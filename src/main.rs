use self::attachment::{Attachment, GeminiAttachment, GeminiUpload};
use core::time::Duration;
use futures_util::StreamExt;
use google_gemini::{
    GeminiClient, GeminiMessage, GeminiPart, GeminiRequest, GeminiRole, GeminiSafetySetting,
    GeminiSafetyThreshold, GeminiSystemPart,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serenity::all::{ChannelId, CreateAttachment, CreateMessage, Message, RoleId, Settings};
use serenity::async_trait;
use serenity::prelude::*;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

extern crate alloc;

mod attachment;
mod model;
mod settings;
mod util;

const CLEO_ID: RoleId = RoleId::new(1317078903348793435);

#[derive(Clone, Debug, Deserialize, JsonSchema)]
pub enum Action {
    SendMessage {
        #[serde(default)]
        referenced_message: Option<model::MessageId>,
        content: String,
    },
    // pin stuff u rly rly like or should remember
    PinMessage {
        message_id: model::MessageId,
    },
    // only delete if you think its a good idea or mari is testing
    DeleteMessages {
        #[serde(default)]
        message_ids: Vec<model::MessageId>,
        #[serde(default)]
        reason: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde_as]
#[serde(transparent)]
pub struct GenResponse(#[serde_as(as = "OneOrMany<_, PreferMany>")] pub Vec<Action>);

#[derive(Serialize)]
struct Un<'a> {
    name: &'a str,
    content: &'a str,
    message_id: u64,
    user_id: u64,
}

struct Claide {
    gemini: GeminiClient,
    seen: tokio::sync::Mutex<HashMap<String, GeminiAttachment>>,
    settings: settings::Settings,
    http_client: reqwest::Client,
}

impl Claide {
    async fn process_message(&self, context: Context, message: Message) -> anyhow::Result<()> {
        if self
            .settings
            .discord
            .blacklisted_users
            .contains(&message.author.id.get())
        {
            tracing::debug!("ignored message by blacklisted user {}", &message.author.id);

            return Ok(());
        }

        let current_user_id = context.cache.current_user().id;

        if message.author.id == current_user_id {
            tracing::debug!("ignored self-message");

            return Ok(());
        }

        let is_mentioned = message
            .mentions
            .iter()
            .any(|user| user.id == current_user_id)
            || message.content.to_lowercase().contains("cleo")
            || message.mention_roles.contains(&CLEO_ID)
            || message.mention_everyone
            || !message.attachments.is_empty();

        if !is_mentioned {
            tracing::debug!("ignored non-mention");

            return Ok(());
        }

        let previous_messages = {
            let Some(cached_messages) = context.cache.channel_messages(message.channel_id) else {
                anyhow::bail!("no channel messages");
            };

            let mut messages = cached_messages
                .values()
                .filter(|msg| {
                    !self
                        .settings
                        .discord
                        .blacklisted_users
                        .contains(&msg.author.id.get())
                })
                .collect::<Vec<_>>();

            messages.sort_unstable_by(|a, b| a.id.cmp(&b.id));

            let mut previous_messages = Vec::with_capacity(messages.len());
            for message in messages {
                let content = match message.kind {
                    serenity::all::MessageType::PinsAdd => {
                        format!("*pinned a message to this channel*")
                    }
                    _ => message.content.clone(),
                };

                let name = message
                    .author
                    .global_name
                    .as_deref()
                    .unwrap_or(&message.author.name);

                let un = Un {
                    name,
                    content: &content,
                    message_id: message.id.get(),
                    user_id: message.author.id.get(),
                };

                let content = serde_json::to_string(&un)?;

                let role = if message.author.id == current_user_id {
                    GeminiRole::Model
                } else {
                    GeminiRole::User
                };

                let iter = message
                    .attachments
                    .iter()
                    .flat_map(|attachment| attachment.proxy_url.parse().ok());

                let iter = util::iter_urls(&message.content)
                    .chain(iter)
                    .filter(|url| self.settings.gemini.whitelisted_domains.url_matches(url))
                    .map(Attachment);

                let attachments: Vec<_> = iter.collect();

                previous_messages.push((role, content, attachments));
            }

            previous_messages
        };

        let mut request = GeminiRequest::default();

        let skeema = schemars::schema_for!(Vec<Action>);
        let skeema = serde_json::to_string(&skeema).unwrap();

        request.system_instruction.parts.push(GeminiSystemPart {
            text: format!(
                "{}\nrespond following this json schema: {skeema}",
                self.settings.gemini.personality.clone(),
            ),
        });

        request
            .generation_config
            .get_or_insert_default()
            .response_mime_type
            .push_str("application/json");

        let settings = [
            GeminiSafetySetting::HarmCategoryHarassment,
            GeminiSafetySetting::HarmCategoryHateSpeech,
            GeminiSafetySetting::HarmCategorySexuallyExplicit,
            GeminiSafetySetting::HarmCategoryDangerousContent,
            GeminiSafetySetting::HarmCategoryCivicIntegrity,
        ];

        let settings = settings.map(|setting| (setting)(GeminiSafetyThreshold::BlockNone));

        request.safety_settings.extend(settings);

        for (role, text, attachments) in previous_messages {
            let attachment = attachments.into_iter().map(|attachment| async move {
                anyhow::Ok(
                    match self.seen.lock().await.entry(attachment.url().to_string()) {
                        Entry::Occupied(occupied) => occupied.get().clone(),
                        Entry::Vacant(vacant) => {
                            let pair = attachment.upload_into_gemini(self).await?;

                            vacant.insert(pair.clone());

                            pair
                        }
                    },
                )
            });

            let iter = futures_util::stream::iter(attachment)
                .buffered(3)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .flatten()
                .map(GeminiPart::from);

            let mut parts = vec![GeminiPart::from(text.to_string())];

            parts.extend(iter);

            request.contents.push(GeminiMessage::new(role, parts));
        }

        if request.contents.is_empty() {
            anyhow::bail!("request is empty");
        }

        tracing::debug!("send request: {request:#?}");

        let content = match self.gemini.generate(request).await {
            Ok(content) => content,
            Err(error) => {
                let mut builder = CreateMessage::new();
                builder = builder.content(format!("```\n{error}```\n-# repor issue to mari",));

                message.channel_id.send_message(&context, builder).await?;

                return Ok(());
            }
        };

        let actions: GenResponse = match serde_json::from_str(&content) {
            Ok(content) => content,
            Err(error) => anyhow::bail!("invalid response: {error}"),
        };

        for action in actions.0 {
            match action {
                Action::SendMessage {
                    referenced_message,
                    content,
                } => {
                    let mut builder = CreateMessage::new();

                    if content.chars().count() > 1950 {
                        builder = builder.add_file(CreateAttachment::bytes(content, "message.txt"));
                    } else {
                        builder = builder.content(content);
                    }

                    if let Some(message_id) = referenced_message {
                        if let Some(messages) = context.cache.channel_messages(message.channel_id) {
                            if let Some(message) = messages.get(&message_id.into()) {
                                builder = builder.reference_message(message);
                            }
                        }
                    }

                    if let Err(error) = message.channel_id.send_message(&context, builder).await {
                        tracing::error!("failed to send message: {error}");
                    }
                }
                Action::PinMessage { message_id } => {
                    let mut target_message = None;

                    if let Some(messages) = context.cache.channel_messages(message.channel_id) {
                        if let Some(message) = messages.get(&message_id.into()) {
                            target_message = Some(message.clone());
                        }
                    }

                    if let Some(message) = target_message {
                        if let Err(error) = message.pin(&context).await {
                            tracing::error!("failed to send message: {error}");
                        }
                    }
                }
                Action::DeleteMessages {
                    message_ids,
                    reason,
                } => {
                    if let Err(error) = delete_messages(
                        message.channel_id,
                        &context,
                        message_ids.into_iter().map(Into::into),
                        &reason,
                    )
                    .await
                    {
                        tracing::error!("failed to send message: {error}");
                    }
                }
            }
        }

        Ok(())
    }
}

async fn delete_messages(
    channel_id: ChannelId,
    context: &Context,
    message_ids: impl IntoIterator<Item = serenity::model::id::MessageId>,
    reason: &str,
) -> anyhow::Result<()> {
    let mut target_message_ids = Vec::new();

    if let Some(messages) = context.cache.channel_messages(channel_id) {
        for message_id in message_ids {
            if messages.contains_key(&message_id) {
                target_message_ids.push(message_id);
            }
        }
    }

    for message_ids in target_message_ids.chunks(100) {
        match message_ids {
            [] => continue,
            [message_id] => {
                context
                    .http
                    .delete_message(channel_id, *message_id, Some(reason))
                    .await?;
            }
            message_ids => {
                let map = serde_json::json!({ "messages": message_ids });

                context
                    .http
                    .delete_messages(channel_id, &map, Some(reason))
                    .await?;
            }
        }
    }

    Ok(())
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
    let settings = settings::try_load()?;

    tracing_subscriber::fmt::init();

    let mut cache_settings = Settings::default();

    cache_settings.max_messages = 500;
    cache_settings.time_to_live = Duration::from_secs(24 * 60 * 60);

    let mut client = Client::builder(
        settings.discord.token.clone(),
        GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILD_MESSAGES,
    )
    .cache_settings(cache_settings)
    .event_handler(Claide {
        gemini: GeminiClient::new(settings.gemini.api_key.clone()),
        seen: Mutex::new(HashMap::new()),
        settings,
        http_client: reqwest::Client::new(),
    })
    .await?;

    client.start().await?;

    Ok(())
}
