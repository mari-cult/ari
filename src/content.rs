use {
    crate::{ImageToText, TextGeneration},
    candle_core::{DType, Device, Tensor},
    futures_util::stream::StreamExt,
    html2text::render::text_renderer::TrivialDecorator,
    image::io::Reader as ImageReader,
    lru::LruCache,
    std::{io::Cursor, num::NonZeroU16, rc::Rc, time::Instant},
    ubyte::ByteUnit,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentKind {
    Text,
    Html,
    Image,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Content {
    kind: ContentKind,
    summary: Box<str>,
}

pub struct ContentCache {
    lru: LruCache<Box<str>, Rc<Content>>,
    max_file_size: ByteUnit,
}

impl ContentCache {
    pub fn new(capacity: NonZeroU16, max_file_size: ByteUnit) -> Self {
        let capacity = capacity.into();

        Self {
            lru: LruCache::new(capacity),
            max_file_size,
        }
    }

    fn get(&mut self, url: &str) -> Option<Rc<Content>> {
        self.lru.get(url).cloned()
    }

    fn insert(&mut self, url: &str, content: Content) -> Rc<Content> {
        self.lru.put(Box::from(url), Rc::new(content));
        self.lru.get(url).unwrap().clone()
    }

    async fn download_url(
        &mut self,
        url: &str,
        text_generation: &mut TextGeneration,
        image_to_text: &mut ImageToText,
    ) -> anyhow::Result<Rc<Content>> {
        let max_file_size = self.max_file_size;

        tracing::info!("download {url}");

        let start = Instant::now();
        let mut stream = reqwest::get(url).await?.bytes_stream();
        let mut bytes = Vec::new();

        while let Some(result) = stream.next().await {
            bytes.extend_from_slice(&result?);

            if u128::try_from(bytes.len()).unwrap() > max_file_size.as_u128() {
                tracing::warn!("{url} exceeded {max_file_size:.2}");

                return Ok(self.insert(
                    url,
                    Content::unknown(format!("{url} exceeded {max_file_size:.2}")),
                ));
            }
        }

        let size = ByteUnit::from(bytes.len());
        let elapsed = start.elapsed();

        tracing::info!("downloaded {url}, {size:.2} in {elapsed:.2?}");
        tracing::info!("process {url}");

        let start = Instant::now();
        let content = Content::from_bytes(&bytes, text_generation, image_to_text)?;
        let summary = content.summary();
        let elapsed = start.elapsed();

        tracing::info!("processed {url}, {summary} in {elapsed:.2?}");

        Ok(self.insert(url, content))
    }

    pub async fn fetch_url(
        &mut self,
        url: &str,
        text_generation: &mut TextGeneration,
        image_to_text: &mut ImageToText,
    ) -> Rc<Content> {
        if let Some(content) = self.get(url) {
            return content;
        }

        match self.download_url(url, text_generation, image_to_text).await {
            Ok(content) => content,
            Err(error) => self.insert(url, Content::unknown(format!("{error}"))),
        }
    }
}

impl Content {
    pub fn new<S: Into<Box<str>>>(kind: ContentKind, summary: S) -> Self {
        Self {
            kind,
            summary: summary.into(),
        }
    }

    pub fn text<S: Into<Box<str>>>(summary: S) -> Self {
        Self::new(ContentKind::Text, summary)
    }

    pub fn html<S: Into<Box<str>>>(summary: S) -> Self {
        Self::new(ContentKind::Html, summary)
    }

    pub fn image<S: Into<Box<str>>>(summary: S) -> Self {
        Self::new(ContentKind::Image, summary)
    }

    pub fn unknown<S: Into<Box<str>>>(summary: S) -> Self {
        Self::new(ContentKind::Unknown, summary)
    }

    pub fn from_bytes(
        bytes: &[u8],
        _text_generation: &mut TextGeneration,
        image_to_text: &mut ImageToText,
    ) -> anyhow::Result<Self> {
        let content_type = content_inspector::inspect(bytes);

        if content_type.is_text() {
            Self::from_html(bytes).or_else(|_error| Self::from_text(bytes))
        } else if content_type.is_binary() {
            Self::from_image(bytes, image_to_text)
        } else {
            Ok(Self::new(
                ContentKind::Unknown,
                "unimplemented content type",
            ))
        }
    }

    fn from_text(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self::text(String::from_utf8_lossy(bytes)))
    }

    fn from_html(bytes: &[u8]) -> anyhow::Result<Self> {
        let string = html2text::from_read_with_decorator(
            Cursor::new(bytes),
            usize::MAX,
            TrivialDecorator::new(),
        );

        Ok(Self::html(string))
    }

    fn from_image(bytes: &[u8], image_to_text: &mut ImageToText) -> anyhow::Result<Self> {
        let image = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()?
            .decode()?
            .resize_to_fill(384, 384, image::imageops::Triangle)
            .into_rgb8()
            .into_raw();

        let image = Tensor::from_vec(image, (384, 384, 3), &Device::Cpu)?.permute((2, 0, 1))?;
        let mean = Tensor::new(&[0.48145466f32, 0.4578275, 0.40821073], &Device::Cpu)?
            .reshape((3, 1, 1))?;

        let std = Tensor::new(&[0.26862954f32, 0.261_302_6, 0.275_777_1], &Device::Cpu)?
            .reshape((3, 1, 1))?;

        let image = (image.to_dtype(DType::F32)? / 255.0)?
            .broadcast_sub(&mean)?
            .broadcast_div(&std)?;

        let summary = image_to_text.generate(&image, &Device::cuda_if_available(0)?)?;

        Ok(Self::image(summary))
    }

    pub fn kind(&self) -> ContentKind {
        self.kind
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }
}
