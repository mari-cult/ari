use std::time::Duration;

use reqwest::{
    header::{HeaderName, HeaderValue, CONTENT_LENGTH},
    Client,
};
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Default, Serialize)]
pub struct GeminiRequest {
    pub system_instruction: GeminiSystemInstruction,
    pub contents: Vec<GeminiMessage>,
    #[serde(rename = "safetySettings")]
    pub safety_settings: Vec<GeminiSafetySetting>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(tag = "category", content = "threshold")]
pub enum GeminiSafetySetting {
    HarmCategoryHarassment(GeminiSafetyThreshold),
    HarmCategoryHateSpeech(GeminiSafetyThreshold),
    HarmCategorySexuallyExplicit(GeminiSafetyThreshold),
    HarmCategoryDangerousContent(GeminiSafetyThreshold),
    HarmCategoryCivicIntegrity(GeminiSafetyThreshold),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GeminiSafetyThreshold {
    BlockNone,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct GeminiSystemInstruction {
    pub parts: Vec<GeminiSystemPart>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct GeminiSystemPart {
    pub text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    pub usage_metadata: GeminiUsageMetadata,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiUsageMetadata {
    pub prompt_token_count: u32,
    #[serde(default)]
    pub candidates_token_count: u32,
    pub total_token_count: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GeminiCandidate {
    pub content: GeminiMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GeminiMessage {
    pub role: GeminiRole,
    pub parts: Vec<GeminiPart>,
}

impl GeminiMessage {
    pub fn new(role: GeminiRole, parts: Vec<GeminiPart>) -> Self {
        Self { role, parts }
    }

    pub fn new_single(role: GeminiRole, text: String) -> Self {
        Self::new(role, vec![GeminiPart::from(text)])
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GeminiPart {
    Text(String),
    FileData { mime_type: String, file_uri: String },
}

impl GeminiPart {
    pub fn file(mime_type: String, file_uri: String) -> Self {
        Self::FileData {
            mime_type,
            file_uri,
        }
    }
}

impl From<String> for GeminiPart {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GeminiRole {
    User,
    Model,
}

#[derive(Serialize)]
struct GeminiCreateFile<'a> {
    file: GeminiFile<'a>,
}

#[derive(Serialize)]
struct GeminiFile<'a> {
    display_name: &'a str,
}

#[derive(Debug, Deserialize)]
struct GeminiFileResponse {
    file: GeminiFileUri,
}

#[derive(Debug, Deserialize)]
struct GeminiFileUri {
    uri: String,
    state: String,
}

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
        let query = [("key", &self.api_key)];
        let request = GeminiCreateFile {
            file: GeminiFile {
                display_name: file_name,
            },
        };

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
        let query = [("key", &self.api_key)];
        let mut response = self
            .client
            .post(url)
            .header(CONTENT_LENGTH, content_length)
            .header(X_GOOG_UPLOAD_OFFSET, ZERO)
            .header(X_GOOG_UPLOAD_COMMAND, UPLOAD_FINALIZE)
            .body(bytes)
            .send()
            .await?
            .json::<GeminiFileResponse>()
            .await?;

        tracing::debug!("initial upload file response: {response:#?}");

        while response.file.state == "PROCESSING" {
            tokio::time::sleep(Duration::from_secs(5)).await;

            response.file = self
                .client
                .get(response.file.uri)
                .query(&query)
                .send()
                .await
                .inspect_err(|error| tracing::error!("processing file: {error}"))?
                .json()
                .await
                .inspect_err(|error| tracing::error!("processing file: {error}"))?;

            tracing::debug!("processing file response: {response:#?}");
        }

        Ok(response.file.uri)
    }

    pub async fn generate(&self, request: GeminiRequest) -> anyhow::Result<String> {
        let url = self.with_base("v1beta/models/gemini-1.5-flash:generateContent");
        let query = [("key", &self.api_key)];

        let response = self
            .client
            .post(url)
            .query(&query)
            .json(&request)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        tracing::debug!(
            "generate response: {}",
            serde_json::to_string_pretty(&response).unwrap()
        );

        let response =
            serde_json::from_str::<GeminiResponse>(&serde_json::to_string(&response).unwrap())?;

        let content = response
            .candidates
            .into_iter()
            .flat_map(|candidate| candidate.content.parts)
            .flat_map(|part| match part {
                GeminiPart::Text(text) => Some(text),
                _ => None,
            })
            .collect::<String>();

        Ok(content)
    }
}

pub fn is_supported_audio_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "audio/wav"
            | "audio/mpeg"
            | "audio/mp3"
            | "audio/aiff"
            | "audio/aac"
            | "audio/ogg"
            | "audio/flac"
    )
}

pub fn is_supported_document_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "application/pdf"
            | "application/x-javascript"
            | "text/javascript"
            | "application/x-python"
            | "text/x-python"
            | "text/plain"
            | "text/html"
            | "text/css"
            | "text/md"
            | "text/csv"
            | "text/xml"
            | "text/rtf"
    )
}

pub fn is_supported_image_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "image/png" | "image/jpeg" | "image/webp" | "image/heic" | "image/heif"
    )
}

pub fn is_supported_video_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "video/mp4"
            | "video/mpeg"
            | "video/mov"
            | "video/quicktime"
            | "video/avi"
            | "video/x-flv"
            | "video/mpg"
            | "video/webm"
            | "video/wmv"
            | "video/3gpp"
    )
}

pub fn is_supported_type(content_type: &str) -> bool {
    is_supported_audio_type(content_type)
        || is_supported_document_type(content_type)
        || is_supported_image_type(content_type)
        || is_supported_video_type(content_type)
}
