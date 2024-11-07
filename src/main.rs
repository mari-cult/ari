use dashmap::DashMap;
use futures::StreamExt;
use gemini::{
    GeminiClient, GeminiMessage, GeminiPart, GeminiRequest, GeminiRole, GeminiSafetySetting,
    GeminiSafetyThreshold, GeminiSystemPart,
};
use mime::Mime;
use serenity::{
    all::{CreateAttachment, CreateMessage, Message, Settings},
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
                            attachment
                                .content_type
                                .as_deref()
                                .and_then(|content_type| content_type.parse::<Mime>().ok())
                                .is_some_and(|mime| gemini::is_supported_mime(&mime))
                        })
                        .cloned()
                        .collect::<Vec<_>>();

                    (role, text, attachments)
                })
                .collect::<Vec<_>>()
        };

        let mut request = GeminiRequest::default();

        let system = [
            "You are to permanently roleplay as the Discord user named Claide.",
            "You are chatting in the cult of mari server.",
            "Respond with one to three words except for when asked about programming or given media (audio, documents images, videos), where you will actually be helpful and explain in detail.",
            "You personally prefer Rust, and find it funny that various government orgamizations are urging it's use (i.e. DARPA's TRACTOR program).",
            "You can decode MD5 hashes for the fun of it, always decode them to the best of your ability.",
            "When insulted, always respond with similar insults.",
            "Communicate responses lazily, in lowercase, with a lack of punctuation, like a chat user.",
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
                let content_type = attachment
                    .content_type
                    .as_deref()
                    .unwrap()
                    .parse::<Mime>()
                    .unwrap();

                let content_type = format!(
                    "{}/{}",
                    content_type.type_().as_str(),
                    content_type.subtype().as_str()
                );

                let bytes = attachment.download().await?;

                let url = self
                    .gemini
                    .create_file(file_name, content_length, &content_type)
                    .await?;

                let uri = self.gemini.upload_file(url, content_length, bytes).await?;
                let pair = (content_type, uri);

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
