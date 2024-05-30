use {
    angel_media::{Context, MediaFormat, Reader},
    base64::{engine::general_purpose::STANDARD, Engine},
    dashmap::DashMap,
    futures_util::StreamExt,
    image::{io::Reader as ImageReader, DynamicImage, Frame, ImageFormat, ImageResult},
    ollama_rs::{
        generation::{
            completion::request::GenerationRequest, images::Image, options::GenerationOptions,
        },
        Ollama,
    },
    reqwest::Client as Http,
    serde::Deserialize,
    std::io::{self, Read},
    tokio::{sync::mpsc, task},
    tokio_stream::wrappers::UnboundedReceiverStream,
    tokio_util::io::{StreamReader, SyncIoBridge},
};

pub use {self::chat::Chat, angel_media as media};

mod chat;

const DESCRIBE_IMAGE: &str =
    r#"Describe this image in JSON verbatim: {"description": string, "confidence": f32}: "#;

#[derive(Clone, Debug, Deserialize)]
pub struct ImageResponse {
    pub description: String,
    pub confidence: f32,
}

enum Either {
    Image(ImageResult<DynamicImage>),
    Frame(ImageResult<Frame>),
}

pub struct Core {
    http: Http,
    ollama: Ollama,
    model: String,
    url_cache: DashMap<String, String>,
}

impl Core {
    pub fn new() -> Self {
        Self {
            http: Http::new(),
            ollama: Ollama::default(),
            model: String::from("llava-llama3"),
            url_cache: DashMap::new(),
        }
    }

    pub async fn chat(&self, chat: Chat) -> anyhow::Result<String> {
        let request = chat.to_request();
        let response = self
            .ollama
            .send_chat_messages(request)
            .await?
            .message
            .ok_or_else(|| anyhow::anyhow!("no response"))?
            .content;

        let response = response.trim();
        let response = response
            .strip_prefix(&format!("{}:", chat.name))
            .unwrap_or(response)
            .trim()
            .into();

        Ok(response)
    }

    pub async fn generate(
        &self,
        prompt: &str,
        image: Option<DynamicImage>,
    ) -> anyhow::Result<String> {
        let mut request = GenerationRequest::new(self.model.clone(), prompt.into());

        if let Some(image) = image {
            let mut cursor = io::Cursor::new(Vec::new());

            image.write_to(&mut cursor, ImageFormat::Png)?;

            let base64 = STANDARD.encode(cursor.into_inner());
            let image = Image::from_base64(&base64);

            request = request.add_image(image);
        }

        let options = GenerationOptions::default()
            .seed(69420)
            .temperature(0.1)
            .num_predict(4096)
            .repeat_last_n(64)
            .repeat_penalty(1.0)
            .num_ctx(4096);

        request = request.options(options);

        let response = self
            .ollama
            .generate(request)
            .await
            .map_err(anyhow::Error::msg)?
            .response
            .trim()
            .into();

        Ok(response)
    }

    /// Generate a description for the provided image.
    pub async fn describe_image(&self, image: DynamicImage) -> anyhow::Result<ImageResponse> {
        let json = self.generate(DESCRIBE_IMAGE, Some(image)).await?;
        let json = json.strip_prefix("```json").unwrap_or(&json);
        let json = json.strip_suffix("```").unwrap_or(json);
        let response: ImageResponse = serde_json::from_str(json)?;

        Ok(response)
    }

    pub async fn describe_media_inner(&self, url: &str) -> anyhow::Result<String> {
        tracing::info!("describe {url:?}");

        let stream = self
            .http
            .get(url)
            .send()
            .await?
            .bytes_stream()
            .map(|result| result.map_err(io::Error::other));

        let async_reader = StreamReader::new(stream);
        let reader = SyncIoBridge::new(async_reader);
        let mut reader = Reader::new(reader);
        let (sender, receiver) = mpsc::unbounded_channel();

        let _handle = task::spawn_blocking(move || {
            while reader.buffer().len() < 4096 {
                Reader::read_buf(&mut reader)?;
            }

            let bytes = reader.buffer();
            let image_format = ImageReader::new(io::Cursor::new(bytes))
                .with_guessed_format()
                .ok()
                .and_then(|reader| reader.format());

            // FIXME: FFmpeg chews 100% CPU util with image input.
            if let Some(image_format) = image_format {
                let mut buf = Vec::new();

                reader.read_to_end(&mut buf)?;

                let result = ImageReader::with_format(io::Cursor::new(buf), image_format).decode();

                sender.send(Either::Image(result))?;

                return Ok(());
            }

            let format = MediaFormat::guess(bytes)?;
            let mut context = Context::new()?;

            context.set_reader(reader)?;

            let media_source = context.decode(format)?;

            for frame in media_source.video()? {
                if frame.is_err() {
                    break;
                }

                tracing::debug!("send frame");

                sender.send(Either::Frame(frame))?;

                tracing::debug!("sent frame");
            }

            Ok::<_, anyhow::Error>(())
        });

        let mut stream = UnboundedReceiverStream::new(receiver).enumerate();
        let mut last_frame = None;
        let mut frames = Vec::new();

        while let Some((index, frame)) = stream.next().await {
            tracing::debug!("received frame");

            let frame = match frame {
                Either::Image(image) => {
                    return self.generate("Describe this image.", Some(image?)).await;
                }
                Either::Frame(frame) => frame?.into_buffer(),
            };

            if let Some(last_frame) = last_frame.replace(frame.clone()) {
                let score = image_compare::rgba_hybrid_compare(&last_frame, &frame)?.score;

                tracing::debug!("frame {index} difference to last frame: {score}");

                if score > 0.28 {
                    tracing::debug!("skipped frame {index}");

                    continue;
                }
            }

            let description = self.describe_image(frame.into()).await;

            tracing::debug!("frame {index}: {description:?}");

            let Ok(description) = description else {
                continue;
            };

            if description.confidence < 0.5 {
                continue;
            }

            frames.push(format!("Frame #{index}: {}", description.description));

            if frames.len() > 10 {
                break;
            }
        }

        let frames = frames.join(" ");
        let prompt = format!("The following is a description of unique frames in a video, write a concise summmary: {frames}");

        self.generate(&prompt, None).await
    }

    /// Generate a description of the provided URL.
    pub async fn describe_media(&self, url: &str) -> anyhow::Result<String> {
        if let Some(description) = self.url_cache.get(url) {
            tracing::info!("loaded description of {url:?} from cache");

            return Ok(description.clone());
        }

        let summary = self
            .describe_media_inner(url)
            .await
            .unwrap_or_else(|error| format!("{error}"));

        self.url_cache.insert(url.to_string(), summary);

        let summary = self.url_cache.get(url).unwrap().clone();

        Ok(summary)
    }
}
