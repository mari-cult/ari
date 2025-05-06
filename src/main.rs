use self::gemini::googleapis::google::ai::generativelanguage::v1beta::part::Data;
use self::gemini::googleapis::google::ai::generativelanguage::v1beta::{Content, Part};
use gemini::googleapis::google::ai::generativelanguage::v1beta::{
    Blob, FunctionDeclaration, GenerateContentRequest, Schema, Tool, Type,
};
use image::ImageFormat;
use prost_types::value::Kind;
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::{fs, time};
use tracing::{error, info, warn};
use twilight_cache_inmemory::DefaultInMemoryCache;
use twilight_gateway::{ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt};
use twilight_http::Client as Rest;
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::presence::{Activity, ActivityType, MinimalActivity, Status};
use twilight_model::id::Id;

pub mod gemini;

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

        buf.into_inner()
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

                buf.into_inner()
            } else {
                warn!("failed to capture screen");

                return Err(anyhow::anyhow!("failed to capture screen"));
            };

            let model = "gemini-2.0-flash";

            info!("connect to endpoint");

            let mut gemini = gemini::Gemini::connect(state.api_key.clone()).await?;

            let system_instruction = [
                "ur ari, 22, egirl",
                "ur on ur pc rn, chattin on discord",
                "talk concisely, preferably of few words, lowercase, since ur typing on ur keyboard",
                "talk more or less depending on the topic",
            ]
            .join(".\n")
                + ".";

            let prompt = [
                "always describe the chat u see on ur pc, pay attention to the order and context of messages, replies, attachments, and users in the chat",
                "then carefully respond to users using the send_message function",
                "avoid repeating previously said messages",
            ].join(".\n") + ".";

            info!("generate content");

            let result = gemini
                .generate_content(GenerateContentRequest {
                    model: format!("models/{model}"),
                    system_instruction: Some(Content {
                        parts: vec![Part {
                            data: Some(Data::Text(system_instruction)),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }),
                    tools: vec![Tool {
                        function_declarations: vec![FunctionDeclaration {
                            name: String::from("send_message"),
                            description: String::from("Send a Discord message"),
                            parameters: Some({
                                let mut schema = Schema {
                                    properties: HashMap::from_iter(vec![(
                                        String::from("message"),
                                        {
                                            let mut schema = Schema {
                                                ..Default::default()
                                            };

                                            schema.set_type(Type::String);
                                            schema
                                        },
                                    )]),
                                    required: vec![String::from("message")],
                                    ..Default::default()
                                };

                                schema.set_type(Type::Object);
                                schema
                            }),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }],
                    contents: vec![Content {
                        parts: vec![
                            Part {
                                data: Some(Data::InlineData(Blob {
                                    mime_type: String::from("image/jpeg"),
                                    data: image,
                                })),
                                ..Default::default()
                            },
                            Part {
                                data: Some(Data::Text(prompt)),
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    }],
                    ..Default::default()
                })
                .await;

            let response = match result {
                Ok(response) => response,
                Err(error) => {
                    error!("gemini api response: {error}");

                    return Err(error);
                }
            };

            info!("gemini api response: {response:#?}");

            let candidate = response
                .candidates
                .first()
                .ok_or_else(|| anyhow::anyhow!("no candidates"))?;

            println!("finish_reason = {:?}", candidate.finish_reason());

            let content = candidate
                .content
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no content"))?;

            for part in content.parts.iter() {
                match part {
                    Part {
                        thought: false,
                        data: Some(Data::FunctionCall(function_call)),
                    } => {
                        if &*function_call.name == "send_message" {
                            let Kind::StringValue(content) = function_call
                                .args
                                .as_ref()
                                .unwrap()
                                .fields
                                .get("message")
                                .unwrap()
                                .kind
                                .as_ref()
                                .unwrap()
                            else {
                                panic!()
                            };

                            state
                                .rest
                                .create_message(Id::new(1325549216432394304))
                                .content(content)
                                .await?;
                        }
                    }
                    // may add more
                    _ => {}
                }
            }
        }
        _ => {}
    }

    Ok(())
}
