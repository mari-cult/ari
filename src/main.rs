use attachment::{Attachment, GeminiUpload};
use config::{ClydeConfig, Config};
use dashmap::DashMap;
use futures::StreamExt;
use gemini::{
    GeminiClient, GeminiMessage, GeminiPart, GeminiRequest, GeminiRole, GeminiSafetySetting,
    GeminiSafetyThreshold, GeminiSystemPart,
};
use mime::Mime;
use regex::Regex;
use reqwest::Url;
use serenity::{
    all::{CreateAttachment, CreateMessage, Message, Settings},
    async_trait,
    prelude::*,
};
use std::{sync::LazyLock, time::Duration};

mod attachment;
mod config;
mod gemini;

static REGEX_URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bhttps://\S+").unwrap());

struct Claide {
    config: ClydeConfig,
    gemini: GeminiClient,
    seen: DashMap<String, (String, String)>,
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

            let mut previous_messages = Vec::with_capacity(messages.len());
            for message in messages {
                let (user, role) = if message.author.id == current_user_id {
                    ("claide", GeminiRole::Model)
                } else {
                    let user = message
                        .author
                        .global_name
                        .as_deref()
                        .unwrap_or(&message.author.name);

                    (user, GeminiRole::User)
                };

                let mut attachments = Vec::new();

                let content = &message.content;
                attachments.extend(
                    REGEX_URL
                        .find_iter(content)
                        .map(|m| m.as_str())
                        .filter_map(|s| Url::try_from(s).ok())
                        .filter(|url| self.config.whitelisted_domains.url_matches(url))
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
                                .is_some_and(|mime| gemini::is_supported_mime(&mime))
                        })
                        .cloned()
                        .map(Attachment::Discord),
                );

                previous_messages.push((role, format!("{user}: {content}"), attachments));
            }

            previous_messages
        };

        let mut request = GeminiRequest::default();

        let system = [
            "You are claide a software engineer for the cult of mari.",
            "Respond like a Discord user - few words, lack of punctuation, always lowercase, occasional typos.",
            "When asked about programming, provide your best effort response and code.",
        ]
        .join(" ");

        request
            .system_instruction
            .parts
            .push(GeminiSystemPart { text: system });

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
                anyhow::Ok(match self.seen.entry(attachment.url().to_string()) {
                    dashmap::Entry::Occupied(occupied) => occupied.get().clone(),
                    dashmap::Entry::Vacant(vacant) => {
                        let pair = attachment.upload_into_gemini(self).await?;

                        vacant.insert(pair.clone());

                        pair
                    }
                })
            });

            let iter = futures::stream::iter(attachment)
                .buffered(3)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .flatten()
                .map(|(content_type, file_uri)| GeminiPart::file(content_type, file_uri));

            let mut parts = vec![GeminiPart::from(text)];

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
                builder = builder.content(format!("<@&1308647289589334067> fix ```\n{error}```"));

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
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::read("clyde.toml")?;
    let mut cache_settings = Settings::default();

    cache_settings.max_messages = 500;
    cache_settings.time_to_live = Duration::from_secs(24 * 60 * 60);

    let mut client = Client::builder(
        config.discord.token,
        GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILD_MESSAGES,
    )
    .cache_settings(cache_settings)
    .event_handler(Claide {
        config: config.clyde,
        gemini: GeminiClient::new(config.gemini.token),
        seen: DashMap::new(),
        http_client: reqwest::Client::new(),
    })
    .await?;

    client.start().await?;

    Ok(())
}
