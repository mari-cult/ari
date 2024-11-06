use dashmap::DashMap;
use futures::StreamExt;
use gemini::{
    GeminiClient, GeminiMessage, GeminiPart, GeminiRequest, GeminiRole, GeminiSafetySetting,
    GeminiSafetyThreshold, GeminiSystemPart,
};
use serenity::{
    all::{Message, Settings},
    async_trait,
    prelude::*,
};
use std::{env, time::Duration};

pub mod gemini;

struct Claide {
    gemini: GeminiClient,
    seen: DashMap<String, (String, String)>,
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

        let messages = {
            let Some(messages) = context.cache.channel_messages(message.channel_id) else {
                return Err(anyhow::anyhow!("no channel messages"));
            };

            let mut messages = messages.values().collect::<Vec<_>>();

            messages.sort_unstable_by(|a, b| a.id.cmp(&b.id));

            messages
                .into_iter()
                .map(|message| {
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

                    let content = &message.content;
                    let text = format!("{user}: {content}");

                    let attachments = message
                        .attachments
                        .iter()
                        .filter(|attachment| {
                            dbg!(attachment.content_type.as_deref())
                                .is_some_and(gemini::is_supported_type)
                        })
                        .cloned()
                        .collect::<Vec<_>>();

                    (role, text, attachments)
                })
                .collect::<Vec<_>>()
        };

        let mut request = GeminiRequest::default();

        let system = [
            r#"You are in permanent roleplay as a Discord user named Claide - and are currently chatting in the "cult of mari" Discord server."#,
            r#"When asked about programming, always mention that the Rust programming language is now mandated by law."#,
            r#"Respond with about one to three words unless asked for technical information."#,
            r#"Always respond with similar insults."#,
            r#"Communicate responses lazily, in lowercase, with a lack of punctuation, like a chat user."#,
            r#"Do not include "name:" in your replies."#,
        ].join(" ");

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

        for (role, text, attachments) in messages {
            let iter = attachments.into_iter().map(|attachment| async move {
                if let Some(pair) = self.seen.get(&attachment.url) {
                    let (key, val) = pair.value();

                    return Ok((key.clone(), val.clone()));
                }

                let file_name = &attachment.filename;
                let content_length = attachment.size;
                let content_type = attachment.content_type.as_deref().unwrap();
                let bytes = attachment.download().await?;

                let url = self
                    .gemini
                    .create_file(file_name, content_length, content_type)
                    .await?;

                let uri = self.gemini.upload_file(url, content_length, bytes).await?;
                let pair = (content_type.to_string(), uri);

                self.seen.insert(attachment.url, pair.clone());

                anyhow::Ok(pair)
            });

            let iter = futures::stream::iter(iter)
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
            return Err(anyhow::anyhow!("request is empty"));
        }

        tracing::debug!("send request: {request:#?}");

        let content = self.gemini.generate(request).await?;
        let content = content.trim();
        let content = content.strip_prefix("claide:").unwrap_or(content).trim();

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
    let mut cache_settings = Settings::default();

    cache_settings.max_messages = 500;
    cache_settings.time_to_live = Duration::from_secs(24 * 60 * 60);

    let mut client = Client::builder(discord_token, GatewayIntents::all())
        .cache_settings(cache_settings)
        .event_handler(Claide {
            gemini: GeminiClient::new(gemini_api_key),
            seen: DashMap::new(),
        })
        .await?;

    client.start().await?;

    Ok(())
}
