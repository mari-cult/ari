use {
    angel_media::{Context, MediaFormat, Reader},
    base64::{engine::general_purpose::STANDARD, Engine},
    dashmap::DashMap,
    futures_util::StreamExt,
    image::{DynamicImage, ImageFormat},
    ollama_rs::{
        generation::{completion::request::GenerationRequest, images::Image},
        Ollama,
    },
    reqwest::Client as Http,
    serde::Deserialize,
    std::io,
    tokio::{sync::mpsc, task},
    tokio_stream::wrappers::UnboundedReceiverStream,
    tokio_util::io::{StreamReader, SyncIoBridge},
};

pub use angel_media as media;

#[derive(Clone, Debug, Deserialize)]
pub struct ImageResponse {
    pub description: String,
    pub confidence: f32,
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
            model: String::from("llava"),
            url_cache: DashMap::new(),
        }
    }

    /// Generate a description for the provided image.
    pub async fn describe_image(&self, image: DynamicImage) -> anyhow::Result<ImageResponse> {
        let mut cursor = io::Cursor::new(Vec::new());

        image.write_to(&mut cursor, ImageFormat::Png)?;

        let base64 = STANDARD.encode(cursor.into_inner());
        let image = Image::from_base64(&base64);
        let request = GenerationRequest::new(
            self.model.clone(),
            String::from(r#"Describe this image in JSON verbatim: {"description": string, "confidence": f32}: "#),
        )
        .add_image(image);

        let json = self
            .ollama
            .generate(request)
            .await
            .map_err(anyhow::Error::msg)?
            .response;

        let json = json.trim();
        let json = json.strip_prefix("```json").unwrap_or(json);
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
                reader.read_buf()?;
            }

            let format = MediaFormat::guess(reader.buffer())?;
            let mut context = Context::new()?;

            context.set_reader(reader)?;

            let media_source = context.decode(format)?;

            for frame in media_source.video()? {
                tracing::debug!("send frame");

                sender.send(frame)?;

                tracing::debug!("sent frame");
            }

            Ok::<_, anyhow::Error>(())
        });

        let mut stream = UnboundedReceiverStream::new(receiver).enumerate();
        let mut last_frame = None;
        let mut frames = Vec::new();

        while let Some((index, frame)) = stream.next().await {
            tracing::debug!("received frame");

            let frame = frame?.into_buffer();

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
        let request = GenerationRequest::new(self.model.clone(), prompt);
        let summary = self
            .ollama
            .generate(request)
            .await
            .map_err(anyhow::Error::msg)?
            .response
            .trim()
            .to_string();

        Ok(summary)
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
