use crate::gemini::GeminiClient;
use crate::Claide;
use anyhow::Context;
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use reqwest::Url;
use std::borrow::Cow;

const DEFAULT_MIME: &str = "text/plain";
const DEFAULT_FILE_NAME: &str = "file.txt";

fn sanitize_content_type(content_type: Cow<'_, str>) -> anyhow::Result<Cow<'static, str>> {
    let mime = content_type.parse::<Mime>().context("parsing mime")?;

    let mut sanitized_content_type = Cow::Owned(format!(
        "{}/{}",
        mime.type_().as_str(),
        mime.subtype().as_str()
    ));

    if sanitized_content_type == "application/rls-services" {
        sanitized_content_type = DEFAULT_MIME.into();
    }

    tracing::debug!("resolved mime {sanitized_content_type} from {content_type:?}");

    Ok(sanitized_content_type)
}

#[derive(Debug)]
pub struct AttachmentContent<'a> {
    content_type: Cow<'static, str>,
    file_name: Option<&'a str>,
    bytes: Vec<u8>,
}

impl Default for AttachmentContent<'_> {
    fn default() -> Self {
        Self {
            content_type: DEFAULT_FILE_NAME.into(),
            file_name: None,
            bytes: "[empty]".into(),
        }
    }
}

impl<'a> AttachmentContent<'a> {
    fn new(
        content_type: Option<Cow<'a, str>>,
        file_name: Option<&'a str>,
        bytes: Vec<u8>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            content_type: sanitize_content_type(
                content_type.unwrap_or_else(|| DEFAULT_MIME.into()),
            )?,
            file_name,
            bytes,
        })
    }

    async fn upload(self, gemini: &GeminiClient) -> anyhow::Result<String> {
        let file_name = self.file_name.unwrap_or(DEFAULT_FILE_NAME);

        tracing::info!("uploading to gemini: {} - {}", file_name, self.content_type);

        let content_size = self.bytes.len() as u32;

        let url = gemini
            .create_file(file_name, content_size, &self.content_type)
            .await?;

        gemini.upload_file(url, content_size, self.bytes).await
    }
}

/// Registered and uploaded file reference
#[derive(Debug, Clone)]
pub struct GeminiAttachment {
    pub uri: String,
    pub content_type: Cow<'static, str>,
}

/// Uploadable to gemini
pub trait GeminiUpload {
    /// Download file data
    async fn fetch_content(&self, claide: &Claide) -> anyhow::Result<AttachmentContent>;

    /// Upload this to gemini
    async fn upload_into_gemini(&self, claide: &Claide) -> anyhow::Result<GeminiAttachment> {
        let content = self
            .fetch_content(claide)
            .await
            .inspect_err(|err| tracing::warn!("fetch failed: {err}"))
            .unwrap_or_default();
        let content_type = content.content_type.clone();
        let uri = content.upload(&claide.gemini).await?;

        Ok(GeminiAttachment { uri, content_type })
    }
}

impl GeminiUpload for serenity::all::Attachment {
    async fn fetch_content(&self, _claide: &Claide) -> anyhow::Result<AttachmentContent> {
        let file_name = &self.filename;
        let content_type = self.content_type.as_deref().map(Cow::Borrowed);

        tracing::info!("downloading from discord: {}: {}", file_name, self.id);
        let bytes = self.download().await?;

        AttachmentContent::new(content_type, Some(file_name), bytes)
    }
}

impl GeminiUpload for Url {
    async fn fetch_content(&self, claide: &Claide) -> anyhow::Result<AttachmentContent> {
        let file_name = self.path_segments().and_then(|path| path.last());

        tracing::info!("downloading from url: {file_name:?}: {self}");
        let resp = claide
            .http_client
            .get(self.clone())
            .send()
            .await?
            .error_for_status()?;

        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(ToOwned::to_owned)
            .map(Cow::Owned);

        AttachmentContent::new(content_type, file_name, resp.bytes().await?.to_vec())
    }
}

pub enum Attachment {
    Discord(serenity::all::Attachment),
    Url(Url),
}

impl Attachment {
    pub fn url(&self) -> &str {
        match self {
            Self::Discord(attachment) => &attachment.url,
            Self::Url(url) => url.as_str(),
        }
    }
}

impl GeminiUpload for Attachment {
    async fn fetch_content(&self, claide: &Claide) -> anyhow::Result<AttachmentContent> {
        match self {
            Attachment::Discord(discord) => discord.fetch_content(claide).await,
            Attachment::Url(url) => url.fetch_content(claide).await,
        }
    }
}
