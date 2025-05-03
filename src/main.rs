use base64::Engine;
use base64::engine::general_purpose;
use image::ImageFormat;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::{fs, time};
use tracing::{info, warn};
use twilight_cache_inmemory::DefaultInMemoryCache;
use twilight_gateway::{ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt};
use twilight_http::Client as Rest;
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::presence::{Activity, ActivityType, MinimalActivity, Status};
use twilight_model::id::Id;

#[derive(Clone, Debug, Deserialize)]
struct DiscordOptions {
    token: String,
}

#[derive(Clone, Debug, Deserialize)]
struct GeminiOptions {
    api_key: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Options {
    discord: DiscordOptions,
    gemini: GeminiOptions,
}

struct State {
    api_key: String,
    rest: Rest,
    cache: DefaultInMemoryCache,
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiResponse {
    pub candidates: Vec<Candidate>,
    pub model_version: String,
    pub usage_metadata: UsageMetadata,
    pub error: Option<ErrorResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub avg_logprobs: Option<f64>,
    pub content: Content,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    pub parts: Vec<Part>,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCall {
    pub args: serde_json::Value,
    pub name: String,
}

// Removed the old Args struct as we are using Value

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub candidates_token_count: u32,
    #[serde(default)]
    pub candidates_tokens_details: Vec<TokenDetails>,
    pub prompt_token_count: u32,
    #[serde(default)]
    pub prompt_tokens_details: Vec<TokenDetails>,
    pub total_token_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDetails {
    pub modality: String,
    pub token_count: u32,
}

// --- New structs for handling the error response ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetail {
    pub code: u16, // Use u16 for HTTP status codes
    pub message: String,
    pub status: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let text = fs::read_to_string("options.toml").await?;
    let options: Options = toml::from_str(&text)?;

    let token = options.discord.token;
    let api_key = options.gemini.api_key;

    let output = Command::new("grim")
        .arg("-t")
        .arg("jpeg")
        .arg("-")
        .output()
        .await?;

    let image = if output.status.success() {
        let image = image::load_from_memory_with_format(&output.stdout, ImageFormat::Jpeg)?;
        let mut buf = io::Cursor::new(Vec::new());

        image.write_to(&mut buf, ImageFormat::Jpeg)?;

        let buf = buf.into_inner();

        general_purpose::URL_SAFE.encode(&buf)
    } else {
        warn!("failed to capture screen");

        return Err(anyhow::anyhow!("failed to capture screen"));
    };

    let intents = Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;

    let activity = Activity::from(MinimalActivity {
        kind: ActivityType::Custom,
        name: String::from("soup"),
        url: None,
    });

    let presence = UpdatePresencePayload::new(vec![activity], false, None, Status::Invisible)?;

    let config = ConfigBuilder::new(token.clone(), intents)
        .presence(presence)
        .build();

    let mut shard = Shard::with_config(ShardId::ONE, config);

    let cache = DefaultInMemoryCache::builder()
        .message_cache_size(0)
        .build();

    let client = ClientBuilder::new().user_agent("ari/0.6.0").build()?;

    let state = Arc::new(State {
        api_key,
        rest: Rest::new(token),
        cache,
        client,
    });

    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let Ok(event) = item else {
            warn!("error receiving event: {}", item.unwrap_err());

            continue;
        };

        state.cache.update(&event);

        info!("{event:?}");

        tokio::spawn(run(Arc::clone(&state), event));
    }

    Ok(())
}

async fn run(state: Arc<State>, event: Event) -> anyhow::Result<()> {
    let Some(self_id) = state.cache.current_user().map(|user| user.id) else {
        return Ok(());
    };

    match event {
        Event::MessageCreate(message)
            if message.author.id != self_id
                && (message.mentions.iter().any(|mention| mention.id == self_id)
                    || message
                        .referenced_message
                        .as_ref()
                        .is_some_and(|message| message.author.id == self_id)) =>
        {
            info!("wait 150ms");

            time::sleep(Duration::from_millis(150)).await;

            let output = Command::new("grim")
                .arg("-t")
                .arg("jpeg")
                .arg("-")
                .output()
                .await?;

            let image = if output.status.success() {
                let image = image::load_from_memory_with_format(&output.stdout, ImageFormat::Jpeg)?;
                let mut buf = io::Cursor::new(Vec::new());

                image.write_to(&mut buf, ImageFormat::Jpeg)?;

                let buf = buf.into_inner();

                general_purpose::URL_SAFE.encode(&buf)
            } else {
                warn!("failed to capture screen");

                return Err(anyhow::anyhow!("failed to capture screen"));
            };

            let system = "ur ari, 22, egirl\n\
            ur on ur pc rn, chatting on discord\n\
            talk concisely, few words, lowercase, as if ur typing on ur keyboard, yk.";

            let prompt = "always describe the chat u see on ur pc, pay attention to the order and context of messages, replies, attachments, and users in the chat. \
            then carefully respond to users using the send_message function. \
            avoid repeating previously said messages.";

            let request = json!({
                "contents": [
                    {
                        "parts": [
                            {
                                "text": prompt,
                            },
                            {
                                "inline_data": {
                                    "mime_type": "image/jpeg",
                                    "data": image,
                                }
                            }
                        ]
                    }
                ],
                /*"generationConfig": {
                    "responseModalities": ["IMAGE", "TEXT"],
                },*/
                "system_instruction": {
                    "parts": [
                        {
                            "text": system,
                        }
                    ]
                },
                "tools": [
                    //{ "code_execution": {} },
                    {
                        "functionDeclarations": [
                            {
                                "name": "send_message",
                                "description": "Send a Discord message in the channel you are looking at",
                                "parameters": {
                                    "type": "object",
                                    "properties": {
                                        "content": {
                                            "type": "string",
                                            "description": "The message contents"
                                        }
                                    },
                                    "required": ["content"]
                                }
                            }
                        ]
                    }
                ]
            });

            // info!(
            //     "gemini api request: {}",
            //     serde_json::to_string_pretty(&request)?
            // );

            //let model = "gemini-2.5-flash-preview-04-17";
            // most stable model
            let model = "gemini-2.0-flash";
            let mut result = Vec::new();

            for attempt in 0..5 {
                info!("send gemini request");

                let response: serde_json::Value = state.client.post(format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={}", state.api_key))
                    .json(&request)
                    .send()
                    .await?
                    .json()
                    .await?;

                let response = serde_json::to_string_pretty(&response)?;

                info!("gemini api response: {response}");

                match serde_json::from_str(&response) {
                    Ok(GeminiResponse {
                        error: Some(error), ..
                    }) if error.error.code == 503 => {
                        warn!("gemini api unavailable (attempt {attempt} of 5)");

                        time::sleep(Duration::from_secs(attempt)).await;
                    }
                    Ok(GeminiResponse {
                        error: Some(error), ..
                    }) => {
                        warn!(
                            "gemini api error: {:#?}",
                            serde_json::to_string_pretty(&error.error)?
                        );

                        return Err(anyhow::anyhow!("fatal gemini api error"));
                    }
                    Ok(GeminiResponse { candidates, .. }) => {
                        result = candidates;

                        break;
                    }
                    Err(error) => {
                        warn!("gemini api deserialization error: {error}");

                        return Err(error.into());
                    }
                }
            }

            let Some(candidate) = result.first() else {
                return Err(anyhow::anyhow!("gemini api unavilable"));
            };

            for part in &candidate.content.parts {
                let Some(function_call) = &part.function_call else {
                    continue;
                };

                if &*function_call.name == "send_message" {
                    let content = function_call.args["content"].as_str().unwrap();

                    if content.is_empty() {
                        continue;
                    }

                    state
                        .rest
                        .create_message(Id::new(1325549216432394304))
                        .content(content)
                        .await?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}
