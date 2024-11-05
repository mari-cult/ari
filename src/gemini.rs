use dashmap::DashMap;
use futures::StreamExt;
use rand::distributions::DistString;
use reqwest::header::{HeaderName, HeaderValue, CONTENT_LENGTH, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serenity::{
    all::{Channel, GetMessages, Message, Settings},
    async_trait,
    prelude::*,
};
use std::{env, time::Duration};

struct Claide {
    gemini_api_key: String,
    reqwest: reqwest::Client,
    uploaded_files: DashMap<String, String>,
}

#[derive(Debug, Default, Serialize)]
struct GeminiRequest {
    system_instruction: GeminiSystemInstruction,
    contents: Vec<GeminiMessage>,
}

#[derive(Debug, Default, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiSystemPart>,
}

#[derive(Debug, Default, Serialize)]
struct GeminiSystemPart {
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    usage_metadata: GeminiUsageMetadata,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsageMetadata {
    prompt_token_count: u32,
    candidates_token_count: u32,
    total_token_count: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct GeminiCandidate {
    content: GeminiMessage,
}

#[derive(Debug, Deserialize, Serialize)]
struct GeminiMessage {
    role: GeminiRole,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum GeminiRole {
    User,
    Model,
}

impl Claide {
    async fn process_message(&self, context: Context, message: Message) -> anyhow::Result<()> {
        let current_user_id = context.cache.current_user().id;

        if message.author.id == current_user_id {
            tracing::debug!("ignored self-message");

            return Ok(());
        }

        if !message.mentions_me(&context).await? {
            tracing::debug!("ignored non-mention");

            return Ok(());
        }

        let mut request = GeminiRequest::default();

        request.system_instruction.parts.push(GeminiSystemPart {
            text: "You are named Clyde - and are currently chatting in a Discord server. Communicate responses in lowercase, without punctuation, like a chat user. Don't prefix responses with your name:.".into(),
        });

        {
            let Some(messages) = context.cache.channel_messages(message.channel_id) else {
                return Err(anyhow::anyhow!("no channel messages"));
            };

            for message in messages.values() {
                let (user, role) = if message.author.id == current_user_id {
                    ("Clyde", GeminiRole::Model)
                } else {
                    let user = message
                        .author
                        .global_name
                        .as_deref()
                        .unwrap_or(&message.author.name);

                    (user, GeminiRole::User)
                };

                let content = &message.content;
                let text = format!("{user}: {content}");

                tracing::debug!("add message {role:?} {text:?}");

                request.contents.push(GeminiMessage {
                    role,
                    parts: vec![GeminiPart { text }],
                });

                let iter = message.attachments.iter().flat_map(|attachment| {
                    let content_type = attachment.content_type.as_deref()?;

                    if !is_supported_content_type(content_type) {
                        return None;
                    }

                    if self.uploaded_files.contains_key(&attachment.url) {
                        return None;
                    }

                    Some(async move {
                        let attachment = attachment.clone();
                        let content_length = attachment.size.to_string();

                        let body = self
                            .reqwest
                            .get(&attachment.url)
                            .send()
                            .await?
                            .bytes()
                            .await?
                            .to_vec();

                        let url = start_upload(
                            &self.reqwest,
                            &self.gemini_api_key,
                            &content_length,
                            content_type,
                            &attachment.filename,
                        )
                        .await?;

                        let uri =
                            finalize_upload(&self.reqwest, url, &content_length, body).await?;

                        anyhow::Ok((attachment.url, uri))
                    })
                });

                let mut attachments = futures::stream::iter(iter).buffer_unordered(3);

                while let Some(Ok((key, val))) = attachments.next().await {
                    self.uploaded_files.insert(key, val);
                }
            }
        }

        if request.contents.is_empty() {
            return Err(anyhow::anyhow!("request is empty"));
        }

        tracing::debug!("send request: {request:?}");

        let response = self.reqwest
            .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent")
            .query(&[("key", &self.gemini_api_key)])
            .json(&request)
            .send()
            .await?
            .text()
            .await?;

        tracing::debug!("got response: {response:?}");

        let response = serde_json::from_str::<GeminiResponse>(&response)?;

        tracing::debug!("parsed response: {response:?}");

        let content = response
            .candidates
            .into_iter()
            .flat_map(|candidate| candidate.content.parts)
            .map(|part| part.text)
            .collect::<String>();

        let content = content.trim();

        if content.is_empty() {
            return Err(anyhow::anyhow!("response is empty"));
        }

        message.reply(&context, content).await?;

        Ok(())
    }
}

#[async_trait]
impl EventHandler for Claide {
    async fn message(&self, context: Context, message: Message) {
        if let Err(error) = self.process_message(context, message).await {
            tracing::error!("process_message: {error:?}");
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let discord_token = env::var("DISCORD_TOKEN")?;
    let gemini_api_key = env::var("GEMINI_API_KEY")?;
    let reqwest = reqwest::Client::new();

    let mut cache_settings = Settings::default();

    cache_settings.max_messages = 500;
    cache_settings.time_to_live = Duration::from_secs(24 * 60 * 60);

    let mut client = Client::builder(discord_token, GatewayIntents::all())
        .cache_settings(cache_settings)
        .event_handler(Claide {
            gemini_api_key,
            reqwest,
            uploaded_files: DashMap::new(),
        })
        .await?;

    client.start().await?;

    Ok(())
}

fn is_supported_content_type(content_type: &str) -> bool {
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
            | "image/png"
            | "image/jpeg"
            | "image/webp"
            | "image/heic"
            | "image/heif"
            | "audio/wav"
            | "audio/mp3"
            | "audio/aiff"
            | "audio/aac"
            | "audio/ogg"
            | "audio/flac"
    )
}

#[derive(Debug, Serialize)]
struct GeminiUploadRequest {
    file: GeminiUploadDisplayName,
}

#[derive(Debug, Serialize)]
struct GeminiUploadDisplayName {
    display_name: String,
}

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

async fn finalize_upload(
    client: &reqwest::Client,
    url: String,
    content_length: &str,
    body: Vec<u8>,
) -> anyhow::Result<String> {
    let response = client
        .post(url)
        .header(CONTENT_LENGTH, content_length)
        .header(X_GOOG_UPLOAD_OFFSET, ZERO)
        .header(X_GOOG_UPLOAD_COMMAND, UPLOAD_FINALIZE)
        .body(body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let uri = response["file"]["uri"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("no uri"))?;

    Ok(uri.into())
}

pub struct Client {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl Client {
    pub fn new(api_key: String) -> Self {
        Self::new_with_client(api_key, reqwest::Client::new())
    }

    pub fn new_with_client(api_key: String, client: reqwest::Client) -> Self {
        Self::new_with_base_url_and_client(api_key, BASE_URL.into(), client)
    }

    pub fn new_with_base_url_and_client(
        api_key: String,
        base_url: String,
        client: reqwest::Client,
    ) -> Self {
        Self {
            api_key,
            base_url,
            client,
        }
    }

    fn with_base(&self, path: &str) -> String {
        format!("{}/{path}", self.base_url)
    }

    async fn create_file(
        &self,
        file_name: &str,
        content_length: u64,
        content_type: &str,
    ) -> anyhow::Result<String> {
        let url = self.with_base("upload/v1beta/files");
        let query = [("key", &self.api_key)];
        let content_length = content_length.to_string();

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
}
