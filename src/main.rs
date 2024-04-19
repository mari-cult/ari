use {
    angel_core::Core,
    discord_content::MessageSplitter,
    ollama_rs::{
        generation::chat::{request::ChatMessageRequest, ChatMessage},
        Ollama as Llm,
    },
    std::{env, sync::Arc},
    tokio::sync::Mutex,
    twilight_cache_inmemory::DefaultInMemoryCache as Cache,
    twilight_gateway::{
        ConfigBuilder, Event, EventTypeFlags, Intents, Shard as Gateway, ShardId, StreamExt as _,
    },
    twilight_http::Client as Rest,
    twilight_model::{
        channel::Message,
        gateway::{
            payload::outgoing::update_presence::UpdatePresencePayload,
            presence::{Activity, ActivityType, MinimalActivity, Status},
        },
        guild::Permissions,
        id::{marker::UserMarker, Id},
    },
};

pub struct State {
    core: Core,
    cache: Cache,
    gateway: Mutex<Gateway>,
    llm: Llm,
    rest: Rest,
    message_splitter: MessageSplitter,
}

impl State {
    pub fn new(token: String) -> anyhow::Result<Arc<Self>> {
        let llm = Llm::default();
        let core = Core::new();
        let cache = Cache::builder().message_cache_size(10).build();
        let config = ConfigBuilder::new(String::from(&token), Intents::all())
            .presence(dnd(String::from("im clyde")))
            .build();
        let gateway = Mutex::new(Gateway::with_config(ShardId::ONE, config));
        let rest = Rest::new(token);

        Ok(Arc::new(Self {
            core,
            cache,
            gateway,
            rest,
            llm,
            message_splitter: MessageSplitter::new(),
        }))
    }

    pub async fn process(self: &Arc<Self>) -> anyhow::Result<()> {
        let Self { cache, gateway, .. } = &**self;
        let mut gateway = gateway.lock().await;

        while let Some(event) = gateway.next_event(EventTypeFlags::all()).await {
            let event = event?;

            cache.update(&event);

            let Event::MessageCreate(message) = event else {
                continue;
            };

            let this = Arc::clone(self);

            tokio::spawn(async move {
                this.process_message(message.0).await?;

                Ok::<_, anyhow::Error>(())
            });
        }

        Ok(())
    }

    async fn process_message(self: &Arc<Self>, message: Message) -> anyhow::Result<()> {
        let Self {
            cache,
            llm,
            rest,
            message_splitter,
            ..
        } = &**self;

        let Some(current_user) = cache.current_user() else {
            return Ok(());
        };

        // Don't reply to yourself.
        if message.author.id == current_user.id {
            return Ok(());
        }

        // The channel needs to be cached...
        /*let Some(channel) = cache.channel(message.channel_id) else {
            return Ok(());
        };*/

        // ...with message history to iterate.
        let Some(channel_messages) = cache.channel_messages(message.channel_id) else {
            return Ok(());
        };

        //println!("{channel:#?}");
        println!("{channel_messages:#?}");

        let is_a_dm = message.guild_id.is_none();

        let can_reply = cache
            .permissions()
            .in_channel(current_user.id, message.channel_id)
            .ok()
            .is_some_and(|permissions| permissions.contains(Permissions::SEND_MESSAGES))
            || is_a_dm;

        // Don't even bother if you can't reply.
        if !can_reply {
            return Ok(());
        }

        let is_reply_to_you = message.is_reply_to(current_user.id);
        let mentions_you = message.mentions(current_user.id);

        // Reply if the message..
        // ..is a reply to you.
        // ..mentions you.
        // ..is in a dm channel.
        if !(is_reply_to_you || mentions_you || is_a_dm) {
            return Ok(());
        }

        let channel_id = message.channel_id;
        let channel = "cunt";
        let user_id = message.author.id;
        let user = &message.author.name;
        let content = &message.content;
        let guild = message.guild_id.and_then(|guild_id| cache.guild(guild_id));

        if let Some(guild) = guild {
            let guild_id = guild.id();
            let guild = guild.name();

            tracing::info!(
                "@{user} ({user_id}) #{channel} ({channel_id}) {guild} ({guild_id}): {content}"
            );
        } else {
            tracing::info!("@{user} ({user_id}) #{channel} ({channel_id}): {content}");
        }

        rest.create_typing_trigger(message.channel_id).await?;

        let rules = [
            "You are named Clyde - and is currently chatting in a Discord server",
            //"You are extremely racist, like a 4chan /pol/ user",
            "Do not talk about yourself in third person",
            "Do not refer to yourself as an AI, LLM, assistant",
            "Communicate responses in lowercase, without punctuation",
        ]
        .join(". ");

        let mut messages = vec![ChatMessage::system(rules)];

        for message_id in channel_messages.iter().copied().rev() {
            let Some(message) = cache.message(message_id) else {
                continue;
            };

            let author_id = message.author();
            let mut attachments = String::new();

            for attachment in message.attachments() {
                let name = &attachment.filename;
                let url = &attachment.url;
                let description = self.core.describe_media(url).await?;

                attachments.push_str(&format!("{name}: {description}"));
            }

            let content = message.content();
            let content = if attachments.is_empty() {
                content.to_string()
            } else {
                format!("{content}\n{attachments}")
            };

            let chat_message = if author_id == current_user.id {
                ChatMessage::assistant(format!("Clyde: {content}"))
            } else {
                let Some(author) = cache.user(author_id) else {
                    continue;
                };

                let name = &author.name;

                ChatMessage::user(format!("{name}: {content}"))
            };

            messages.push(chat_message);
        }

        let request = ChatMessageRequest::new("llava".into(), messages);

        tracing::debug!("{request:#?}");

        let response = llm.send_chat_messages(request).await?;
        let content = response.message.unwrap().content;
        let content = content.trim();

        if content.is_empty() {
            return Ok(());
        }

        let mut reply_to = if is_a_dm { None } else { Some(message.id) };

        for content in message_splitter.split(content) {
            let mut create_message = rest.create_message(message.channel_id).content(&content);

            if let Some(reply) = reply_to.take() {
                create_message = create_message.reply(reply);
            }

            create_message.await?;
        }

        Ok(())
    }
}

/// Returns a do not disturn presence payload.
fn dnd(status: String) -> UpdatePresencePayload {
    let activity = Activity::from(MinimalActivity {
        kind: ActivityType::Custom,
        name: status,
        url: None,
    });

    UpdatePresencePayload {
        activities: vec![activity],
        afk: false,
        since: None,
        status: Status::DoNotDisturb,
    }
}

pub trait MessageExt {
    fn mentions(&self, id: Id<UserMarker>) -> bool;
    fn mentions_any(&self, ids: &[Id<UserMarker>]) -> bool;

    fn is_reply_to(&self, id: Id<UserMarker>) -> bool;
    fn is_reply_to_any(&self, id: &[Id<UserMarker>]) -> bool;
}

impl MessageExt for Message {
    fn mentions(&self, id: Id<UserMarker>) -> bool {
        self.mentions.iter().any(|mention| mention.id == id)
    }

    fn mentions_any(&self, ids: &[Id<UserMarker>]) -> bool {
        self.mentions
            .iter()
            .any(|mention| ids.contains(&mention.id))
    }

    fn is_reply_to(&self, id: Id<UserMarker>) -> bool {
        self.referenced_message
            .as_ref()
            .is_some_and(|referenced_message| referenced_message.author.id == id)
    }

    fn is_reply_to_any(&self, ids: &[Id<UserMarker>]) -> bool {
        self.referenced_message
            .as_ref()
            .is_some_and(|referenced_message| ids.contains(&referenced_message.author.id))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    angel_core::media::ffmpeg::init()?;

    let token = env::var("DISCORD_TOKEN")?;
    let state = State::new(token)?;

    state.process().await?;

    Ok(())
}
