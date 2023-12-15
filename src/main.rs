use {
    crate::{settings::Settings, text_generation::TextGeneration},
    candle_core::Device,
    futures_util::stream::StreamExt,
    image::{io::Reader as ImageReader, DynamicImage},
    lru::LruCache,
    std::io::Cursor,
    twilight_cache_inmemory::InMemoryCache,
    twilight_gateway::{Event, Intents, Shard as Gateway, ShardId},
    twilight_http::Client as Rest,
};

pub mod fs;
pub mod huggingface;
pub mod model;
pub mod settings;
pub mod text_generation;
pub mod tokenizer;

pub enum Content {
    Text(String),
    Image(DynamicImage),
}

pub struct Clyde {
    cache: twilight_cache_inmemory::InMemoryCache,
    gateway: twilight_gateway::Shard,
    rest: twilight_http::Client,
    settings: Settings,
    text_generation: TextGeneration,
    content_cache: LruCache<String, Content>,
}

impl Clyde {
    pub fn new() -> anyhow::Result<Self> {
        let settings = fs::Options::new().toml::<settings::Settings, _>("settings.toml")?;
        let model = settings.language.model;
        let tokenizer = model.load_tokenizer()?;
        let model = model.load_model(&Device::Cpu)?;
        let text_generation = text_generation::TextGeneration::new(model, tokenizer);
        let max_entries = settings.cache.max_entries.try_into().unwrap();

        Ok(Self {
            cache: InMemoryCache::new(),
            gateway: Gateway::new(ShardId::ONE, settings.discord.token.clone(), Intents::all()),
            rest: Rest::new(settings.discord.token.clone()),
            settings,
            text_generation,
            content_cache: LruCache::new(max_entries),
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

    pub async fn fetch_url(&mut self, url: &str) -> anyhow::Result<()> {
        let mut stream = reqwest::get(url).await?.bytes_stream();
        let mut bytes = Vec::new();

        while let Some(result) = stream.next().await {
            bytes.extend_from_slice(&result?);

            if u128::try_from(bytes.len()).unwrap() > self.settings.cache.max_file_size.as_u128() {
                return Ok(());
            }
        }

        let content_type = content_inspector::inspect(&bytes);
        let bytes = Cursor::new(bytes);

        if content_type.is_text() {
            let _string = html2text::from_read(bytes, usize::MAX);
        } else if content_type.is_binary() {
            let _image = ImageReader::new(bytes).with_guessed_format()?.decode()?;
        }

        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mut clyde = Clyde::new()?;

    loop {
        let event = clyde.next_event().await?;

        clyde.cache.update(&event);

        println!("{event:?}");
    }

    Ok(())
}
