use self::gemini::googleapis::google::ai::generativelanguage::v1alpha::generation_config::Modality;
use self::gemini::googleapis::google::ai::generativelanguage::v1alpha::part::Data;
use self::gemini::googleapis::google::ai::generativelanguage::v1alpha::{
    self, BidiGenerateContentClientContent, BidiGenerateContentClientMessage,
    BidiGenerateContentServerMessage, BidiGenerateContentSetup, BidiGenerateContentToolResponse,
    CodeExecution, Content, FunctionDeclaration, FunctionResponse, GenerationConfig, Part, Schema,
    Tool, Type, bidi_generate_content_client_message,
};
use prost_types::value::Kind;
use prost_types::{Struct, Value};
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use time::OffsetDateTime;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use tokio::fs;
use tracing::{info, warn};
use twilight_cache_inmemory::DefaultInMemoryCache;
use twilight_gateway::{
    ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _,
};
use twilight_http::Client as Rest;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::presence::{Activity, ActivityType, MinimalActivity, Status};

pub mod gemini;

const TIME: &[BorrowedFormatItem<'_>] =
    format_description!("[hour]:[minute]:[second] [weekday], [month], [day] [week_number], [year]");

#[derive(Clone, Debug, Deserialize)]
struct DiscordOptions {
    token: String,
}

#[derive(Clone, Debug, Deserialize)]
struct GeminiOptions {
    api_key: String,
    system_instructions: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Options {
    discord: DiscordOptions,
    gemini: GeminiOptions,
}

struct State {
    options: Options,
    rest: Rest,
    cache: DefaultInMemoryCache,
    client: Client,
}

fn new_schema(kind: Type) -> Schema {
    let mut schema = Schema::default();

    schema.set_type(kind);
    schema
}

fn fields<K: Into<String>>(
    fields: impl IntoIterator<Item = (K, Schema)>,
) -> HashMap<String, Schema> {
    fields
        .into_iter()
        .map(|(key, value)| (key.into(), value))
        .collect()
}

fn required<T: ToString>(fields: impl IntoIterator<Item = T>) -> Vec<String> {
    fields.into_iter().map(|field| field.to_string()).collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let text = fs::read_to_string("options.toml").await?;
    let options: Options = toml::from_str(&text)?;

    let mut intents = Intents::all();

    intents.remove(Intents::GUILD_PRESENCES);

    let activity = Activity::from(MinimalActivity {
        kind: ActivityType::Custom,
        name: String::from("soup"),
        url: None,
    });

    let presence = UpdatePresencePayload::new(vec![activity], false, None, Status::Invisible)?;

    let config = ConfigBuilder::new(options.discord.token.clone(), intents)
        .presence(presence)
        .build();

    let mut shard = Shard::with_config(ShardId::ONE, config);

    let cache = DefaultInMemoryCache::builder()
        .message_cache_size(0)
        .build();

    let client = ClientBuilder::new().user_agent("ari/0.6.0").build()?;

    let state = Arc::new(State {
        rest: Rest::new(options.discord.token.clone()),
        options,
        cache,
        client,
    });

    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

    let system_instructions = &state.options.gemini.system_instructions;

    let model = "gemini-2.0-flash-live-001";

    info!("send setup");
    sender.send(BidiGenerateContentClientMessage {
        message_type: Some(bidi_generate_content_client_message::MessageType::Setup(
            BidiGenerateContentSetup {
                model: format!("models/{model}"),
                generation_config: Some({
                    let mut generation_config = GenerationConfig::default();

                    generation_config.push_response_modalities(Modality::Text);
                    generation_config
                }),
                system_instruction: Some(Content {
                    parts: vec![Part {
                        data: Some(Data::Text(system_instructions.to_string())),
                    }],
                    ..Default::default()
                }),
                tools: vec![
                    Tool {
                        code_execution: Some(CodeExecution {}),
                        ..Default::default()
                    },
                    Tool {
                        function_declarations: vec![
                            FunctionDeclaration {
                                name: String::from("discord_send_message"),
                                description: String::from("send a message in a channel"),
                                parameters: Some(Schema {
                                    properties: fields([
                                        ("channel_id", new_schema(Type::String)),
                                        ("content", new_schema(Type::String)),
                                    ]),
                                    required: required(["channel_id", "content"]),
                                    ..new_schema(Type::Object)
                                }),
                                ..Default::default()
                            },
                            FunctionDeclaration {
                                name: String::from("discord_react_to_message"),
                                description: String::from(
                                    "react to a message with a single unicode emoji",
                                ),
                                parameters: Some(Schema {
                                    properties: fields([
                                        ("channel_id", new_schema(Type::String)),
                                        ("message_id", new_schema(Type::String)),
                                        ("emoji", new_schema(Type::String)),
                                    ]),
                                    required: required(["channel_id", "message_id", "emoji"]),
                                    ..new_schema(Type::Object)
                                }),
                                ..Default::default()
                            },
                            FunctionDeclaration {
                                name: String::from("discord_edit_message"),
                                description: String::from("edit one of your own messages"),
                                parameters: Some(Schema {
                                    properties: fields([
                                        ("channel_id", new_schema(Type::String)),
                                        ("message_id", new_schema(Type::String)),
                                        ("new_content", new_schema(Type::String)),
                                    ]),
                                    required: required(["channel_id", "message_id", "new_content"]),
                                    ..new_schema(Type::Object)
                                }),
                                ..Default::default()
                            },
                            FunctionDeclaration {
                                name: String::from("discord_delete_message"),
                                description: String::from("delete a message"),
                                parameters: Some(Schema {
                                    properties: fields([
                                        ("channel_id", new_schema(Type::String)),
                                        ("message_id", new_schema(Type::String)),
                                    ]),
                                    required: required(["channel_id", "message_id"]),
                                    ..new_schema(Type::Object)
                                }),
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    },
                ],
            },
        )),
    })?;

    info!("connect to endpont");
    let mut gemini = gemini::GeminiLive::connect(state.options.gemini.api_key.clone()).await?;
    info!("start bidi");
    let mut receiver = gemini.bidi(receiver).await?;

    // setupcomplete
    info!("recv setupcomple");
    receiver.message().await?;

    info!("do discord");
    'outer: while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let Ok(event) = item else {
            warn!("error receiving event: {}", item.unwrap_err());

            continue;
        };

        state.cache.update(&event);

        match event {
            Event::Ready(..) => info!("ari is ready"),
            Event::MessageCreate(message)
                if state
                    .cache
                    .current_user()
                    .is_some_and(|user| user.id != message.author.id) =>
            {
                let channel = state.cache.channel(message.channel_id).unwrap();
                let channel_name = channel.name.as_deref().unwrap_or("unknown");
                let now = OffsetDateTime::now_local()?;

                let content = format!(
                    "A new message by {author} in #{channel_name} {time}: channel_id={channel_id} message_id={message_id} content={content:?}",
                    channel_id = channel.id,
                    message_id = message.id,
                    author = message.author.name,
                    content = message.content,
                    time = now.format(&TIME)?,
                );

                sender.send(BidiGenerateContentClientMessage {
                    message_type: Some(
                        bidi_generate_content_client_message::MessageType::ClientContent(
                            BidiGenerateContentClientContent {
                                turns: vec![Content {
                                    parts: vec![Part {
                                        data: Some(Data::Text(content)),
                                    }],
                                    ..Default::default()
                                }],
                                turn_complete: true,
                            },
                        ),
                    ),
                })?;

                use gemini::googleapis::google::ai::generativelanguage::v1alpha;

                use v1alpha::bidi_generate_content_server_message::MessageType;
                use v1alpha::{BidiGenerateContentServerContent, BidiGenerateContentToolCall};

                while let Some(BidiGenerateContentServerMessage {
                    message_type: Some(message_type),
                }) = dbg!(receiver.message().await?)
                {
                    match message_type {
                        MessageType::ServerContent(BidiGenerateContentServerContent {
                            turn_complete,
                            interrupted,
                            ..
                        }) => {
                            if interrupted || turn_complete {
                                info!("model completed turn, break loop");

                                continue 'outer;
                            }
                        }
                        MessageType::ToolCall(BidiGenerateContentToolCall { function_calls }) => {
                            for function_call in function_calls {
                                info!("model executed {} tool", function_call.name);

                                let status = match &*function_call.name {
                                    "discord_send_message" => {
                                        let args = function_call.args.unwrap();

                                        let Some(Value {
                                            kind: Some(Kind::StringValue(channel_id)),
                                        }) = args.fields.get("channel_id")
                                        else {
                                            warn!("`channel_id` field is missing");

                                            continue;
                                        };

                                        let Some(Value {
                                            kind: Some(Kind::StringValue(content)),
                                        }) = args.fields.get("content")
                                        else {
                                            warn!("`content` field is missing");

                                            continue;
                                        };

                                        let future = async {
                                            let channel_id =
                                                channel_id.parse().map_err(|_error| {
                                                    anyhow::anyhow!("failed to parse channel_id")
                                                })?;

                                            info!(
                                                "discord_send_message(channel_id={channel_id}, content={content:?})"
                                            );

                                            let response = state
                                                .rest
                                                .create_message(channel_id)
                                                .content(content)
                                                .await?;

                                            anyhow::Ok(response)
                                        };

                                        match future.await {
                                            Ok(response) => format!(
                                                "successfully sent your message to channel_id={channel_id} message_id={}",
                                                response.model().await?.id,
                                            ),
                                            Err(error) => {
                                                format!(
                                                    "failed to send your message to channel_id={channel_id}, maybe try again with channel_id={suggested_channel_id}? heres the error: {error}",
                                                    suggested_channel_id = message.channel_id,
                                                )
                                            }
                                        }
                                    }
                                    "discord_react_to_message" => {
                                        let args = function_call.args.unwrap();

                                        let Some(Value {
                                            kind: Some(Kind::StringValue(channel_id)),
                                        }) = args.fields.get("channel_id")
                                        else {
                                            warn!("`channel_id` field is missing");

                                            continue;
                                        };

                                        let Some(Value {
                                            kind: Some(Kind::StringValue(message_id)),
                                        }) = args.fields.get("message_id")
                                        else {
                                            warn!("`message_id` field is missing");

                                            continue;
                                        };

                                        let Some(Value {
                                            kind: Some(Kind::StringValue(emoji)),
                                        }) = args.fields.get("emoji")
                                        else {
                                            warn!("`emoji` field is missing");

                                            continue;
                                        };

                                        let future = async {
                                            let channel_id =
                                                channel_id.parse().map_err(|_error| {
                                                    anyhow::anyhow!("failed to parse channel_id")
                                                })?;

                                            let message_id =
                                                message_id.parse().map_err(|_error| {
                                                    anyhow::anyhow!("failed to parse message_id")
                                                })?;

                                            info!(
                                                "discord_react_to_message(channel_id={channel_id}, message_id={message_id}, emoji={emoji})"
                                            );

                                            state
                                                .rest
                                                .create_reaction(
                                                    channel_id,
                                                    message_id,
                                                    &RequestReactionType::Unicode { name: emoji },
                                                )
                                                .await?;

                                            anyhow::Ok(())
                                        };

                                        match future.await {
                                            Ok(_message) => format!(
                                                "successfully reacted to channel_id={channel_id} message_id={message_id}"
                                            ),
                                            Err(error) => {
                                                format!(
                                                    "failed to react to channel_id={channel_id} message_id={message_id}, maybe try again with channel_id={suggested_channel_id} message_id={suggested_message_id}? heres the error: {error}",
                                                    suggested_channel_id = message.channel_id,
                                                    suggested_message_id = message.id,
                                                )
                                            }
                                        }
                                    }
                                    "discord_edit_message" => {
                                        let args = function_call.args.unwrap();

                                        let Some(Value {
                                            kind: Some(Kind::StringValue(channel_id)),
                                        }) = args.fields.get("channel_id")
                                        else {
                                            warn!("`channel_id` field is missing");

                                            continue;
                                        };

                                        let Some(Value {
                                            kind: Some(Kind::StringValue(message_id)),
                                        }) = args.fields.get("message_id")
                                        else {
                                            warn!("`message_id` field is missing");

                                            continue;
                                        };

                                        let Some(Value {
                                            kind: Some(Kind::StringValue(new_content)),
                                        }) = args.fields.get("new_content")
                                        else {
                                            warn!("`new_content` field is missing");

                                            continue;
                                        };

                                        let future = async {
                                            let channel_id =
                                                channel_id.parse().map_err(|_error| {
                                                    anyhow::anyhow!("failed to parse channel_id")
                                                })?;

                                            let message_id =
                                                message_id.parse().map_err(|_error| {
                                                    anyhow::anyhow!("failed to parse message_id")
                                                })?;

                                            info!(
                                                "discord_edit_message(channel_id={channel_id}, message_id={message_id}, new_content={new_content})"
                                            );

                                            state
                                                .rest
                                                .update_message(channel_id, message_id)
                                                .content(Some(new_content))
                                                .await?;

                                            anyhow::Ok(())
                                        };

                                        match future.await {
                                            Ok(_message) => format!(
                                                "successfully reacted to channel_id={channel_id} message_id={message_id}"
                                            ),
                                            Err(error) => {
                                                format!(
                                                    "failed to react to channel_id={channel_id} message_id={message_id}, maybe try again with channel_id={suggested_channel_id} message_id={suggested_message_id}? heres the error: {error}",
                                                    suggested_channel_id = message.channel_id,
                                                    suggested_message_id = message.id,
                                                )
                                            }
                                        }
                                    }
                                    "discord_delete_message" => {
                                        let args = function_call.args.unwrap();

                                        let Some(Value {
                                            kind: Some(Kind::StringValue(channel_id)),
                                        }) = args.fields.get("channel_id")
                                        else {
                                            warn!("`channel_id` field is missing");

                                            continue;
                                        };

                                        let Some(Value {
                                            kind: Some(Kind::StringValue(message_id)),
                                        }) = args.fields.get("message_id")
                                        else {
                                            warn!("`message_id` field is missing");

                                            continue;
                                        };

                                        let future = async {
                                            let channel_id =
                                                channel_id.parse().map_err(|_error| {
                                                    anyhow::anyhow!("failed to parse channel_id")
                                                })?;

                                            let message_id =
                                                message_id.parse().map_err(|_error| {
                                                    anyhow::anyhow!("failed to parse message_id")
                                                })?;

                                            info!(
                                                "discord_delete_message(channel_id={channel_id}, message_id={message_id})"
                                            );

                                            state
                                                .rest
                                                .delete_message(channel_id, message_id)
                                                .await?;

                                            anyhow::Ok(())
                                        };

                                        match future.await {
                                            Ok(_message) => format!(
                                                "successfully deleted message channel_id={channel_id} message_id={message_id}"
                                            ),
                                            Err(error) => {
                                                format!(
                                                    "failed to delete message channel_id={channel_id} message_id={message_id}, heres the error: {error}"
                                                )
                                            }
                                        }
                                    }
                                    _ => {
                                        continue;
                                    }
                                };

                                sender.send(BidiGenerateContentClientMessage {
                                    message_type: Some(
                                        bidi_generate_content_client_message::MessageType::ToolResponse(
                                            BidiGenerateContentToolResponse {
                                                function_responses: vec![
                                                    FunctionResponse {
                                                        id: function_call.id.clone(),
                                                        name: function_call.name.clone(),
                                                        response: Some(Struct {
                                                            fields: {
                                                                let mut fields = BTreeMap::new();

                                                                fields.insert("output".to_string(), Value {
                                                                    kind: Some(Kind::StringValue(status)),
                                                                });

                                                                fields
                                                            }
                                                        })
                                                    }
                                                ]
                                            }
                                        )
                                    )
                                })?;
                            }
                        }
                        _ => {}
                    };
                }
            }

            _ => {}
        }
    }

    Ok(())
}
