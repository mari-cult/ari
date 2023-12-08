use {
    llama::{Model, Session, SessionBatch},
    std::{
        collections::hash_map::HashMap,
        env,
        sync::{Arc, Mutex},
        time::Instant,
    },
    twilight_cache_inmemory::InMemoryCache,
    twilight_gateway::{Event, Intents, ShardId},
    twilight_model::channel::message::Message,
    twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder},
};

pub struct Clyde {
    batch: SessionBatch,
    cache: InMemoryCache,
    gateway: twilight_gateway::Shard,
    rest: twilight_http::Client,
    session: Session,
    tokens: Vec<i32>,
    url_cache: Arc<Mutex<HashMap<u16, String>>>,
}

impl Clyde {
    pub fn new(token: String) -> Self {
        let model = Model::options()
            .set_gpu_layers(33)
            .open("../models/teknium_openhermes-2.5-mistral-7b.gguf")
            .expect("big oof energy");

        let mut batch = SessionBatch::new(32786, 1);
        let mut tokens = Vec::new();

        model.tokenize_special("<|im_start|>system\n", &mut tokens);
        model.tokenize(include_str!("personality.txt").trim(), &mut tokens);
        model.tokenize_special("<|im_end|>\n", &mut tokens);

        batch.extend(tokens.iter().copied(), false);

        let session = Session::options()
            .set_context_len(32786)
            .set_temperature(0.2)
            .set_top_k(50.0)
            .set_top_p(0.95)
            .with_model(model);

        Self {
            batch,
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
            tokens,
            url_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn process_message(&mut self, message: &Message) -> anyhow::Result<()> {
        let Self {
            batch,
            cache,
            session,
            tokens,
            rest,
            ..
        } = self;

        let Some(clyde) = cache.current_user() else {
            return Ok(());
        };

        if message.author.id == clyde.id {
            return Ok(());
        }

        let mentions_clyde = message
            .mentions
            .iter()
            .any(|mention| mention.id == clyde.id);

        let replying_to_clyde = message
            .referenced_message
            .as_ref()
            .is_some_and(|message| message.author.id == clyde.id);

        if !(mentions_clyde || replying_to_clyde) {
            return Ok(());
        }

        let model = session.model();
        let content = format!(
            "{}:{}",
            message.author.name.as_str(),
            message.content.trim()
        );

        tokens.clear();
        model.tokenize_special("<|im_start|>user\n", tokens);
        model.tokenize(&content, tokens);
        model.tokenize_special("<|im_end|>\n<|im_start|>assistant\nClyde:", tokens);
        batch.extend(tokens.drain(..), false);

        if let Some(logit) = batch.logits_mut().last_mut() {
            *logit = true;
        }

        let start = Instant::now();

        loop {
            session.decode(batch);

            let token = session.sample();

            session.accept(token);
            batch.clear();
            batch.push(token, true);
            tokens.push(token);

            if token == session.model().eos_token() {
                break;
            }
        }

        let elapsed = Instant::now().duration_since(start);
        let mut bytes = Vec::new();

        session
            .model()
            .detokenize(tokens.iter().copied(), &mut bytes);

        let content = String::from_utf8_lossy(&bytes);
        let content = content
            .trim()
            .trim_matches(|character: char| (character as u32) < 32);

        let content = content.strip_prefix("assistant\n").unwrap_or(&content);
        let content = content.strip_prefix("Clyde:").unwrap_or(&content);

        let content = content
            .strip_prefix("clyde:")
            .unwrap_or(&content)
            .trim()
            .trim_matches(|character: char| (character as u32) < 32);

        tracing::info!("content={content:?}");

        if content.is_empty() {
            return Ok(());
        }

        let embed = EmbedBuilder::new()
            .footer(EmbedFooterBuilder::new(format!(
                "{:?} tokens in {:.2?} ({:.2?} t/s)",
                tokens.len(),
                elapsed,
                (tokens.len() as f32) / elapsed.as_secs_f32(),
            )))
            .build();

        rest.create_message(message.channel_id)
            .content(&content)?
            .embeds(&[embed])?
            .await?;

        Ok(())
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let token = env::var("CLYDE_TOKEN")?;

    Clyde::new(token).run().await?;

    Ok(())
}
