use self::attachment::{Attachment, GeminiAttachment, GeminiUpload};
use futures_util::StreamExt;
use google_gemini::{
    GeminiClient, GeminiMessage, GeminiPart, GeminiRequest, GeminiRole, GeminiSafetySetting,
    GeminiSafetyThreshold, GeminiSystemPart,
};
use mime::Mime;
use regex::Regex;
use reqwest::Url;
use serde::Serialize;
use serenity::all::{CreateAttachment, CreateMessage, Message, Settings};
use serenity::async_trait;
use serenity::prelude::*;
use settings::GeminiSettings;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

mod attachment;
mod settings;

static REGEX_URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bhttps://\S+").unwrap());

struct Claide {
    gemini: GeminiClient,
    seen: tokio::sync::Mutex<HashMap<String, GeminiAttachment>>,
    settings: GeminiSettings,
    http_client: reqwest::Client,
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

        let previous_messages = {
            let Some(cached_messages) = context.cache.channel_messages(message.channel_id) else {
                anyhow::bail!("no channel messages");
            };

            let mut messages = cached_messages.values().collect::<Vec<_>>();

            messages.sort_unstable_by(|a, b| a.id.cmp(&b.id));

            #[derive(Serialize)]
            struct Un<'a> {
                name: &'a str,
                content: &'a str,
            }

            let mut previous_messages = Vec::with_capacity(messages.len());
            for message in messages {
                let content = &message.content;

                let (role, content) = if message.author.id == current_user_id {
                    (GeminiRole::Model, content.to_string())
                } else {
                    let name = message
                        .author
                        .global_name
                        .as_deref()
                        .unwrap_or(&message.author.name);

                    let un = Un { name, content };

                    let con = serde_json::to_string(&un)?;

                    (GeminiRole::User, con)
                };

                let mut attachments = Vec::new();

                attachments.extend(
                    REGEX_URL
                        .find_iter(&content)
                        .map(|m| m.as_str())
                        .filter_map(|s| Url::try_from(s).ok())
                        .filter(|url| self.settings.whitelisted_domains.url_matches(url))
                        .map(Attachment::Url),
                );

                attachments.extend(
                    message
                        .attachments
                        .iter()
                        .filter(|attachment| {
                            attachment
                                .content_type
                                .as_deref()
                                .and_then(|content_type| content_type.parse::<Mime>().ok())
                                .is_some_and(|mime| google_gemini::is_supported_mime(&mime))
                        })
                        .cloned()
                        .map(Attachment::Discord),
                );

                previous_messages.push((role, content, attachments));
            }

            previous_messages
        };

        let mut request = GeminiRequest::default();

        request.system_instruction.parts.push(GeminiSystemPart {
            text: include_str!("personality.txt").into(),
        });

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

        let content = content.trim();
        let content = content.strip_prefix("claide:").unwrap_or(content).trim();

        if content.is_empty() {
            anyhow::bail!("response is empty");
        }

        let mut builder = CreateMessage::new();

        if content.chars().count() > 1950 {
            builder = builder.add_file(CreateAttachment::bytes(content, "message.txt"));
        } else {
            builder = builder.content(content);
        }

        message.channel_id.send_message(&context, builder).await?;

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
    let settings = settings::try_load()?;

    tracing_subscriber::fmt::init();

    let mut cache_settings = Settings::default();

    cache_settings.max_messages = 500;
    cache_settings.time_to_live = Duration::from_secs(24 * 60 * 60);

    let mut client = Client::builder(
        settings.discord.token,
        GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILD_MESSAGES,
    )
    .cache_settings(cache_settings)
    .event_handler(Claide {
        gemini: GeminiClient::new(settings.gemini.api_key.clone()),
        seen: Mutex::new(HashMap::new()),
        settings: settings.gemini,
        http_client: reqwest::Client::new(),
    })
    .await?;

    client.start().await?;

    Ok(())
}
