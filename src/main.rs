use {
    crate::{content::ContentCache, image_to_text::ImageToText, text_generation::TextGeneration},
    candle_core::Device,
    std::fmt::Write,
    twilight_cache_inmemory::InMemoryCache,
    twilight_gateway::{Event, Intents, Shard as Gateway, ShardId},
    twilight_http::Client as Rest,
    twilight_mention::{parse::MentionType, ParseMention},
    twilight_model::{
        channel::Message,
        id::{marker::ChannelMarker, Id},
    },
};

const SANDBOX_ID: Id<ChannelMarker> = Id::new(1185415937780883456);

pub mod content;
pub mod fs;
pub mod huggingface;
pub mod image_to_text;
pub mod model;
pub mod settings;
pub mod text_generation;
pub mod tokenizer;

pub struct Clyde {
    cache: twilight_cache_inmemory::InMemoryCache,
    content_cache: ContentCache,
    gateway: twilight_gateway::Shard,
    image_to_text: ImageToText,
    rest: twilight_http::Client,
    text_generation: TextGeneration,
}

impl Clyde {
    pub fn new() -> anyhow::Result<Self> {
        let settings::Settings {
            cache,
            discord,
            language,
            vision,
        } = fs::Options::new().toml("settings.toml")?;

        let tokenizer = language.model.load_tokenizer()?;
        let model = language.model.load_model(&Device::Cpu)?;
        let text_generation = text_generation::TextGeneration::new(model, tokenizer);

        let tokenizer = vision.model.load_tokenizer()?;
        let model = vision.model.load_model(&Device::new_cuda(0)?)?;
        let image_to_text = ImageToText::new(model, tokenizer);

        Ok(Self {
            cache: InMemoryCache::new(),
            content_cache: ContentCache::new(cache.max_entries, cache.max_file_size),
            gateway: Gateway::new(ShardId::ONE, discord.token.clone(), Intents::all()),
            image_to_text,
            rest: Rest::new(discord.token.clone()),
            text_generation,
        })
    }

    pub async fn next_event(&mut self) -> anyhow::Result<Event> {
        loop {
            match self.gateway.next_event().await {
                Ok(event) => break Ok(event),
                Err(error) if error.is_fatal() => break Err(error.into()),
                Err(error) => {
                    tracing::warn!("{error}");
                }
            }
        }
    }

    fn should_reply(&self, message: &Message) -> bool {
        let Some(current_user) = self.cache.current_user() else {
            return false;
        };

        if message.author.id == current_user.id {
            return false;
        }

        let in_dm = message.guild_id.is_none();
        let in_sandbox = message.channel_id == SANDBOX_ID;
        let mentions_clyde = message
            .mentions
            .iter()
            .any(|mention| mention.id == current_user.id);

        let reply_to_clyde = message
            .referenced_message
            .as_ref()
            .is_some_and(|message| message.author.id == current_user.id);

        in_dm || in_sandbox || mentions_clyde || reply_to_clyde
    }

    pub async fn process_message(&mut self, message: &Message) -> anyhow::Result<()> {
        let Some(current_user) = self.cache.current_user() else {
            return Ok(());
        };

        if !self.should_reply(message) {
            return Ok(());
        }

        let mut prompt = String::new();
        let message_ids = self.cache.channel_messages(message.channel_id).unwrap();

        for message_id in message_ids.iter().copied() {
            let message = self.cache.message(message_id).unwrap();
            let content = message.content();
            let author_id = message.author();

            if author_id == current_user.id {
                write!(prompt, "<|assistant|>\nClyde: {content}<|assistant|>\n")?;

                continue;
            }

            let author = self.cache.user(author_id).unwrap();
            let name = author.global_name.as_deref().unwrap_or(&author.name);

            write!(prompt, "<|user|>\n{name}: {content}<|endoftext|>\n")?;

            if let Some(hash) = author.avatar {
                let url =
                    format!("https://cdn.discordapp.com/avatars/{author_id}/{hash}.webp?size=80");

                let content = self
                    .content_cache
                    .fetch_url(&url, &mut self.text_generation, &mut self.image_to_text)
                    .await;

                let summary = content.summary();

                write!(
                    prompt,
                    "<|user|>\n{url}: Avatar is {summary}<|endoftext|>\n"
                )?;
            }

            for attachment in message.attachments() {
                let url = &attachment.proxy_url;
                let content = self
                    .content_cache
                    .fetch_url(url, &mut self.text_generation, &mut self.image_to_text)
                    .await;

                let summary = content.summary();

                write!(
                    prompt,
                    "<|user|>\n{url}: Attachment is {summary}<|endoftext|>\n"
                )?;
            }

            for mention in MentionType::iter(content) {
                match mention {
                    (MentionType::Emoji(emoji_id), _, _) => {
                        let url = format!("https://cdn.discordapp.com/emojis/{emoji_id}.webp");

                        let content = self
                            .content_cache
                            .fetch_url(&url, &mut self.text_generation, &mut self.image_to_text)
                            .await;

                        let summary = content.summary();

                        write!(
                            prompt,
                            "<|user|>\n{emoji_id}: Emoji is {summary}<|endoftext|>\n"
                        )?;
                    }
                    _ => {}
                }
            }

            for url in urls(content) {
                let content = self
                    .content_cache
                    .fetch_url(url, &mut self.text_generation, &mut self.image_to_text)
                    .await;

                let summary = content.summary();

                write!(prompt, "<|user|>\n{url}: URL is {summary}<|endoftext|>\n")?;
            }
        }

        write!(prompt, "<|assistant|>\nClyde:")?;

        let response = self.text_generation.generate(&prompt)?;

        if response.is_empty() {
            return Ok(());
        }

        let Ok(create_message) = self
            .rest
            .create_message(message.channel_id)
            .content(&response)
        else {
            return Ok(());
        };

        create_message.await?;

        Ok(())
    }
}

fn urls(string: &str) -> impl Iterator<Item = &str> {
    let mut options = linkify::LinkFinder::new();

    options.kinds(&[linkify::LinkKind::Url]);
    options.links(string).map(|line| line.as_str())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mut clyde = Clyde::new()?;

    loop {
        let event = clyde.next_event().await?;

        clyde.cache.update(&event);

        let Event::MessageCreate(message) = event else {
            continue;
        };

        clyde.process_message(&message).await?;
    }
}
