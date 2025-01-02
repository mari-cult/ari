use core::time::Duration;
use mime::Mime;
use reqwest::header::{HeaderName, HeaderValue, CONTENT_LENGTH};
use reqwest::Client;

use gemini_model::{Authentication, CreateFile, CreateFileResponse, State};

extern crate alloc;

pub mod request;

pub use gemini_model as model;

const BASE_URL: &str = "https://generativelanguage.googleapis.com";

const X_GOOG_UPLOAD_COMMAND: HeaderName = HeaderName::from_static("x-goog-upload-command");
const X_GOOG_UPLOAD_HEADER_CONTENT_LENGTH: HeaderName =
    HeaderName::from_static("x-goog-upload-header-content-length");
const X_GOOG_UPLOAD_HEADER_CONTENT_TYPE: HeaderName =
    HeaderName::from_static("x-goog-upload-header-content-type");
const X_GOOG_UPLOAD_OFFSET: HeaderName = HeaderName::from_static("x-goog-upload-offset");
const X_GOOG_UPLOAD_PROTOCOL: HeaderName = HeaderName::from_static("x-goog-upload-protocol");
const X_GOOG_UPLOAD_URL: HeaderName = HeaderName::from_static("x-goog-upload-url");

const RESUMABLE: HeaderValue = HeaderValue::from_static("resumable");
const START: HeaderValue = HeaderValue::from_static("start");
const UPLOAD_FINALIZE: HeaderValue = HeaderValue::from_static("upload, finalize");
const ZERO: HeaderValue = HeaderValue::from_static("0");

pub struct GeminiClient {
    api_key: String,
    base_url: String,
    client: Client,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self::new_with_client(api_key, Client::new())
    }

    pub fn new_with_client(api_key: String, client: Client) -> Self {
        Self::new_with_base_url_and_client(api_key, BASE_URL.into(), client)
    }

    pub fn new_with_base_url_and_client(api_key: String, base_url: String, client: Client) -> Self {
        Self {
            api_key,
            base_url,
            client,
        }
    }

    fn with_base(&self, path: &str) -> String {
        format!("{}/{path}", self.base_url)
    }

    pub async fn create_file(
        &self,
        file_name: &str,
        content_length: u32,
        content_type: &str,
    ) -> anyhow::Result<String> {
        let url = self.with_base("upload/v1beta/files");
        let query = Authentication::new(&self.api_key);
        let request = CreateFile::new(file_name);

        let response = self
            .client
            .post(url)
            .query(&query)
            .header(X_GOOG_UPLOAD_PROTOCOL, RESUMABLE)
            .header(X_GOOG_UPLOAD_COMMAND, START)
            .header(X_GOOG_UPLOAD_HEADER_CONTENT_LENGTH, content_length)
            .header(X_GOOG_UPLOAD_HEADER_CONTENT_TYPE, content_type)
            .json(&request)
            .send()
            .await?;

        let url = response
            .headers()
            .get(X_GOOG_UPLOAD_URL)
            .and_then(|value| value.to_str().map(String::from).ok())
            .ok_or_else(|| anyhow::anyhow!("missing expected x-goog-upload-url"))?;

        Ok(url)
    }

    pub async fn upload_file(
        &self,
        url: String,
        content_length: u32,
        bytes: Vec<u8>,
    ) -> anyhow::Result<String> {
        let query = Authentication::new(&self.api_key);

        let mut response = self
            .client
            .post(url)
            .header(CONTENT_LENGTH, content_length)
            .header(X_GOOG_UPLOAD_OFFSET, ZERO)
            .header(X_GOOG_UPLOAD_COMMAND, UPLOAD_FINALIZE)
            .body(bytes)
            .send()
            .await?
            .json::<CreateFileResponse>()
            .await?;

        while response.file.state == State::Pending {
            tokio::time::sleep(Duration::from_secs(5)).await;

            response.file = self
                .client
                .get(response.file.uri)
                .query(&query)
                .send()
                .await?
                .json()
                .await?;
        }

        Ok(response.file.uri)
    }

    pub const fn generate_content<'a>(&'a self, model: &'a str) -> request::GenerateContent<'a> {
        request::GenerateContent::new(self, model)
    }
}

pub fn is_supported_mime(mime: &Mime) -> bool {
    let mime = (mime.type_().as_str(), mime.subtype().as_str());

    tracing::debug!("{mime:?}");

    matches!(
        mime,
        ("audio", "wav")
            | ("audio", "mpeg")
            | ("audio", "mp3")
            | ("audio", "aiff")
            | ("audio", "aac")
            | ("audio", "ogg")
            | ("audio", "flac")
            | ("application", "pdf")
            | ("application", "x-javascript")
            | ("application", "rls-services")
            | ("text", "javascript")
            | ("application", "x-python")
            | ("text", "x-python")
            | ("text", "plain")
            | ("text", "html")
            | ("text", "css")
            | ("text", "md")
            | ("text", "csv")
            | ("text", "xml")
            | ("text", "rtf")
            | ("image", "png")
            | ("image", "jpeg")
            | ("image", "webp")
            | ("image", "heic")
            | ("image", "heif")
            | ("video", "mp4")
            | ("video", "mpeg")
            | ("video", "mov")
            | ("video", "quicktime")
            | ("video", "avi")
            | ("video", "x-flv")
            | ("video", "mpg")
            | ("video", "webm")
            | ("video", "wmv")
            | ("video", "3gpp")
    )
}
