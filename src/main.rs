use {
    base64::{engine::general_purpose::STANDARD, Engine},
    image::{codecs::jpeg::JpegEncoder, imageops},
    llama::{ModelOptions, Session, SessionBatch, SessionOptions},
    serde::{Deserialize, Serialize},
    std::{
        collections::{
            btree_map::{self, BTreeMap},
            hash_map::{self, DefaultHasher, HashMap},
        },
        env,
        fmt::Write as _,
        hash::{Hash, Hasher},
        io,
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    },
    tracing::{debug, info, warn},
    twilight_cache_inmemory::InMemoryCache,
    twilight_gateway::{Event, Intents, ShardId},
    twilight_model::{
        channel::{message::MessageType, Message},
        id::{
            marker::{ChannelMarker, MessageMarker},
            Id,
        },
    },
    twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder},
};

pub mod discord;
pub mod prompt;

pub struct Clyde {
    cache: InMemoryCache,
    gateway: twilight_gateway::Shard,
    rest: twilight_http::Client,
    session: Session,
    url_cache: Arc<Mutex<HashMap<u16, String>>>,
}

impl Clyde {
    pub fn new(token: String) -> Self {
        let model = ModelOptions::new()
            .set_gpu_layers(33)
            .open("../models/teknium_openhermes-2.5-mistral-7b.gguf")
            .expect("big oof energy");

        let session = SessionOptions::new().with_model(model);

        Self {
            cache: InMemoryCache::builder().message_cache_size(50).build(),
            gateway: twilight_gateway::Shard::new(
                ShardId::ONE,
                token.clone(),
                Intents::GUILDS
                    | Intents::GUILD_MEMBERS
                    | Intents::GUILD_MESSAGES
                    | Intents::DIRECT_MESSAGES
                    | Intents::MESSAGE_CONTENT,
            ),
            rest: twilight_http::Client::new(token),
            session,
            url_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn process_message(&mut self, message: &Message) -> anyhow::Result<()> {
        let Some(clyde) = self.cache.current_user() else {
            return Ok(());
        };

        if message.author.id == clyde.id {
            return Ok(());
        }

        if !message
            .mentions
            .iter()
            .any(|mention| mention.id == clyde.id)
        {
            return Ok(());
        }

        let mut batch = SessionBatch::new(2048, 0, 1);
        let mut tokens = Vec::new();

        self.session
            .model()
            .tokenize(&message.content, &mut tokens, false, false);

        info!(target: "inference", "input={:?} ({} tokens)", message.content, tokens.len());

        for (index, token) in tokens.iter().copied().enumerate() {
            batch.add_token(token, index.try_into().unwrap(), false);
        }

        if let Some(logit) = batch.logits_mut().last_mut() {
            *logit = true;
        }

        let mut then = Instant::now();
        let mut reply_id = None;

        tokens.clear();

        loop {
            self.session.decode(&mut batch);

            let token = self.session.sample();

            if token == self.session.model().eos_token() {
                break;
            }

            self.session.accept(token);
            batch.clear();
            batch.add_token(token, tokens.len().try_into().unwrap(), true);
            tokens.push(token);

            let now = Instant::now();
            let elapsed = now.duration_since(then);

            if elapsed > Duration::from_secs(1) {
                then = now;

                let mut content = String::new();

                self.session.model().detokenize(&tokens, &mut content);

                info!(target: "inference", "output={content:?} ({} tokens)", tokens.len());

                match reply_id {
                    Some(message_id) => {
                        self.update_message(message.channel_id, message_id, &content)
                            .await;
                    }
                    None => {
                        reply_id = self
                            .create_message(message.channel_id, &content)
                            .await
                            .map(|message| message.id);
                    }
                }
            }
        }

        info!(target: "inference", "done");

        Ok(())
    }

    pub async fn create_message(
        &self,
        channel_id: Id<ChannelMarker>,
        content: &str,
    ) -> Option<Message> {
        let content = content.trim();

        if content.is_empty() {
            warn!("Cannot send an empty message.");

            return None;
        }

        let result = self.rest.create_message(channel_id).content(content);

        let result = match result {
            Ok(future) => future.await,
            Err(error) => {
                warn!("Cannot send a message with invalid content: {error}");

                return None;
            }
        };

        let result = match result {
            Ok(response) => response.model().await,
            Err(error) => {
                warn!("Failed to parse create message response: {error}");

                return None;
            }
        };

        match result {
            Ok(message) => Some(message),
            Err(error) => {
                warn!("Failed to deserialize create message response as a message: {error}");

                None
            }
        }
    }

    pub async fn update_message(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
        content: &str,
    ) -> Option<Message> {
        let content = content.trim();

        if content.is_empty() {
            warn!("Cannot update a message to have no content.");

            return None;
        }

        let result = self
            .rest
            .update_message(channel_id, message_id)
            .content(Some(content));

        let result = match result {
            Ok(future) => future.await,
            Err(error) => {
                warn!("Cannot update a message with invalid content: {error}");

                return None;
            }
        };

        let result = match result {
            Ok(response) => response.model().await,
            Err(error) => {
                warn!("Failed to parse update message response: {error}");

                return None;
            }
        };

        match result {
            Ok(message) => Some(message),
            Err(error) => {
                warn!("Failed to deserialize update message response as a message: {error}");

                None
            }
        }
    }

    pub async fn url(&self, url: &str) -> anyhow::Result<(u16, String)> {
        let mut hasher = DefaultHasher::new();

        url.hash(&mut hasher);

        let id = (hasher.finish() % u16::MAX as u64) as u16;

        match self.url_cache.lock().unwrap().entry(id) {
            hash_map::Entry::Occupied(entry) => Ok((id, entry.get().clone())),
            hash_map::Entry::Vacant(entry) => {
                let bytes = reqwest::get(url).await?.bytes().await?;
                let base64 = process_image(&bytes)?;

                Ok((id, entry.insert(base64).clone()))
            }
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            let event = match self.gateway.next_event().await {
                Ok(event) => event,
                Err(error) if error.is_fatal() => {
                    return Err(error.into());
                }
                _ => return Ok(()),
            };

            self.cache.update(&event);

            if let Event::MessageCreate(message) = event {
                self.process_message(&message).await?;
            }
        }
    }
}

/// Process an image.
///
/// - Load image data from `bytes`.
/// - Resize to 512x512, maintaining aspect ratio.
/// - Quantize colour data.
/// - Encode as a JPEG with 65% quality.
/// - Encode as base64.
fn process_image(bytes: &[u8]) -> anyhow::Result<String> {
    debug!("Attempt to parse {} bytes as an image", bytes.len());

    let image = image::load_from_memory(bytes)?;

    debug!("Resize to 256x256");

    let image = image.resize(256, 256, imageops::Triangle);

    debug!("Ensure 8-bit RGBA");

    let mut image = image.into_rgba8();

    debug!("Build NEUQUANT color map");

    let color_map = color_quant::NeuQuant::new(30, 128, image.as_raw());

    debug!("Apply dithering");

    imageops::dither(&mut image, &color_map);

    debug!("Encode as JPEG with 65% quality.");

    let mut jpeg = io::Cursor::new(Vec::new());

    image.write_with_encoder(JpegEncoder::new_with_quality(&mut jpeg, 60))?;

    debug!("Encode as base64");

    let mut base64 = String::new();

    STANDARD.encode_string(&jpeg.into_inner(), &mut base64);

    Ok(base64)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let token = env::var("CLYDE_TOKEN")?;

    Clyde::new(token).run().await?;

    Ok(())
}
