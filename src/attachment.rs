use crate::{gemini::GeminiClient, Claide};
use anyhow::{Context, Result};
use mime::Mime;
use reqwest::{header::CONTENT_TYPE, Url};

const DEFAULT_MIME: &str = "text/plain";
const DEFAULT_FILE_NAME: &str = "file.txt";

fn sanitize_content_type(content_type: Option<&str>) -> Result<String> {
    let mime = content_type
        .unwrap_or(DEFAULT_MIME)
        .parse::<Mime>()
        .context("parsing mime")?;

    let mut sanitized_content_type =
        format!("{}/{}", mime.type_().as_str(), mime.subtype().as_str());

    if sanitized_content_type == "application/rls-services" {
        sanitized_content_type = DEFAULT_MIME.into();
    }

    tracing::debug!("resolved mime {sanitized_content_type} from {content_type:?}");

    Ok(sanitized_content_type)
}

async fn upload(
    gemini: &GeminiClient,
    file_name: &str,
    content_type: &str,
    bytes: Vec<u8>,
) -> Result<String> {
    tracing::info!("uploading to gemini: {file_name} - {content_type}");

    let content_size = bytes.len() as u32;

    let url = gemini
        .create_file(file_name, content_size, content_type)
        .await?;

    gemini.upload_file(url, content_size, bytes).await
}

pub trait GeminiUpload {
    async fn upload_into_gemini(self, claide: &Claide) -> Result<(String, String)>;
}

impl GeminiUpload for serenity::all::Attachment {
    async fn upload_into_gemini(self, claide: &Claide) -> Result<(String, String)> {
        let file_name = &self.filename;
        let content_type = sanitize_content_type(self.content_type.as_deref())?;

        tracing::info!("downloading from discord: {}: {}", file_name, self.id);
        let bytes = serenity::all::Attachment::download(&self).await?;

        let uri = upload(&claide.gemini, file_name, &content_type, bytes).await?;

        Ok((content_type, uri))
    }
}

impl GeminiUpload for Url {
    async fn upload_into_gemini(self, claide: &Claide) -> Result<(String, String)> {
        let file_name = self
            .path_segments()
            .and_then(|path| path.last())
            .unwrap_or(DEFAULT_FILE_NAME)
            .to_owned();

        tracing::info!("downloading from url: {file_name}: {self}");
        let resp = claide.http_client.get(self).send().await?;
        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(DEFAULT_MIME);

        let content_type = sanitize_content_type(Some(content_type))?;

        let uri = upload(
            &claide.gemini,
            &file_name,
            &content_type,
            resp.bytes().await?.to_vec(),
        )
        .await?;

        Ok((content_type, uri))
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
    async fn upload_into_gemini(self, claide: &Claide) -> Result<(String, String)> {
        match self {
            Attachment::Discord(discord) => discord.upload_into_gemini(claide).await,
            Attachment::Url(url) => url.upload_into_gemini(claide).await,
        }
    }
}
