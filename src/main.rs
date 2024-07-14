use {
    pyo3::{conversion, prelude::*, types::PyDict, PyTypeCheck},
    python::langchain::LangGraph,
    serenity::{
        all::{
            ActivityData, Channel, Client, Context, CreateMessage, GatewayIntents, GetMessages,
            Message, OnlineStatus, Settings,
        },
        prelude::*,
    },
    std::{env, process, sync::Arc, time::Instant},
    time::Duration,
    tokio::{signal, task},
};

pub mod python;
pub mod util;

struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, message: Message) {
        handle_message(context, message).await.ok();
    }
}

async fn handle_message(context: Context, message: Message) -> anyhow::Result<()> {
    // Ignore messages from yourself.
    if message.author.id == context.cache.current_user().id {
        return Ok(());
    }

    if message.channel_id != 1261932165453582348 {
        return Ok(());
    }

    let channel = message.channel(&context).await?;
    let get_messages = GetMessages::new().before(message.id).limit(25);
    let messages = match channel {
        Channel::Guild(channel) => channel.messages(&context, get_messages).await?,
        Channel::Private(channel) => channel.messages(&context, get_messages).await?,
        _ => return Ok(()),
    };

    let name = "Clyde";
    let location = "Server";
    let users = "mari (kalmari246):";

    // rustfmt wants the string on its own lines, and that looks horrid.
    #[rustfmt::skip]
    let system_prompt = indoc::formatdoc!(r#"
        You are named {name} - and are currently chatting in a Discord {location}.
        Do not include name: or message: in your response.
        If insulted, reply with a similar insult.
        Communicate responses in lowercase, without punctuation, like a chat user.
        Information about your environment:
        You can use this information about the chat participants in the conversation in your replies. Use this information to answer questions.
        {users}
    "#);

    let system_prompt = python::langchain::Message::system(&system_prompt);
    let mut conversation = vec![system_prompt];

    for message in messages.iter().take(15).rev() {
        let name = message.author_nick(&context).await;
        let name = name.as_deref().unwrap_or_else(|| {
            message
                .author
                .global_name
                .as_deref()
                .unwrap_or(&message.author.name)
        });

        let content = message.content_safe(&context);

        let message = if message.author.id == context.cache.current_user().id {
            let content = content
                .rsplit_once("\n-# ")
                .map(|(content, _footer)| content)
                .unwrap_or(&content);

            let content = format!("{name}: {content}");

            python::langchain::Message::assistant(&content)
        } else {
            let content = format!("{name}: {content}");
            python::langchain::Message::user(&content)
        };

        conversation.push(message);
    }

    println!("{conversation:#?}");

    let start = Instant::now();
    let content = {
        let data = context.data.read().await;
        let lang_graph = data.get::<LangGraphKey>().unwrap();

        lang_graph.invoke_async(conversation).await?
    };

    let duration = start.elapsed();

    let footer = [
        &format!("{duration:.02?}"),
        "gemma2",
        "[Support](<https://discord.gg/PB3kcvCnub>)",
        "[GitHub](<https://github.com/mizz1e/clyde>)",
    ];

    let content = format!("{content}\n-# {}", footer.join(" • "));
    let nonce = util::nonce_of(&content);

    let new_message = CreateMessage::new()
        .content(content)
        .enforce_nonce(true)
        .nonce(nonce);

    message
        .channel_id
        .send_message(&context.http, new_message)
        .await?;

    Ok(())
}

pub struct LangGraphKey;

impl TypeMapKey for LangGraphKey {
    type Value = LangGraph;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    pyo3::prepare_freethreaded_python();

    task::spawn(async move {
        signal::ctrl_c().await.ok();
        process::exit(1);
    });

    let lang_graph = LangGraph::new();

    let token = env::var("DISCORD_TOKEN")?;
    let intents = GatewayIntents::DIRECT_MESSAGE_POLLS
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_EMOJIS_AND_STICKERS
        | GatewayIntents::GUILD_INVITES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_POLLS
        | GatewayIntents::GUILD_MODERATION
        | GatewayIntents::GUILD_PRESENCES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut cache_settings = Settings::default();

    cache_settings.max_messages = 1000;
    cache_settings.time_to_live = Duration::hours(24).try_into().unwrap();

    let mut client = Client::builder(token, intents)
        .activity(ActivityData::custom("hi im clyde"))
        .cache_settings(cache_settings)
        .event_handler(Handler)
        .status(OnlineStatus::Invisible)
        .await?;

    {
        let mut data = client.data.write().await;

        data.insert::<LangGraphKey>(lang_graph);
    }

    client.start().await?;

    Ok(())
}
