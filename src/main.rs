use aho_corasick::AhoCorasick;
use core::time::Duration;
use futures_util::StreamExt;
use gemini::model::content::{Part, TextPart};
use gemini::model::safety_setting::{BlockThreshold, SafetyCategory};
use gemini::model::{GeminiMessage, GeminiRole};
use gemini::GeminiClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock};
use tracing::warn;
use twilight_cache_inmemory::iter::IterReference;
use twilight_cache_inmemory::DefaultInMemoryCache;
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _};
use twilight_http::Client;
use twilight_model::channel::Channel;
use twilight_model::guild::Permissions;
use twilight_model::id::marker::{ChannelMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;

extern crate alloc;

mod settings;

#[derive(Clone, Copy, Debug)]
struct ChannelPermissions {
    manage_messages: bool,
    manage_roles: bool,
    send_messages: bool,
    view_channel: bool,
}

impl From<Permissions> for ChannelPermissions {
    fn from(permissions: Permissions) -> Self {
        Self {
            manage_messages: permissions.contains(Permissions::MANAGE_MESSAGES),
            manage_roles: permissions.contains(Permissions::MANAGE_ROLES),
            send_messages: permissions.contains(Permissions::SEND_MESSAGES),
            view_channel: permissions.contains(Permissions::VIEW_CHANNEL),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    return Ok(());

    let settings = settings::try_load()?;

    let cache: Arc<_> = DefaultInMemoryCache::builder()
        .message_cache_size(25)
        .build()
        .into();

    let client: Arc<_> = Client::new(settings.discord.token.clone()).into();

    let mut shard = Shard::new(ShardId::ONE, settings.discord.token, Intents::all());

    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let Ok(event) = item else {
            warn!(source = ?item.unwrap_err(), "error receiving event");

            continue;
        };

        cache.update(&event);

        // let Some(user_id) = cache.current_user().map(|user| user.id) else {
        //     continue;
        // };

        // let channels: Vec<_> = cache
        //     .iter()
        //     .channels()
        //     .flat_map(|channel| {
        //         cache
        //             .permissions()
        //             .in_channel(user_id, channel.id)
        //             .map(ChannelPermissions::from)
        //             .map(|permissions| (channel, permissions))
        //     })
        //     .filter(|(_channel, permissions)| permissions.view_channel)
        //     //.flat_map(|(channel, permissions)| channel.name.clone().map(|name| (name, permissions)))
        //     .collect();

        // let mut users = HashSet::new();

        // let messages: Vec<_> = channels
        //     .iter()
        //     .map(|(channel, _permissions)| {
        //         let messages: Vec<_> = cache
        //             .channel_messages(channel.id)
        //             .as_deref()
        //             .into_iter()
        //             .flatten()
        //             .copied()
        //             .flat_map(|message_id| cache.message(message_id))
        //             .inspect(|message| {
        //                 users.insert(message.author());
        //             })
        //             .collect();

        //         messages
        //     })
        //     .collect();

        // let users: Vec<_> = users
        //     .into_iter()
        //     .flat_map(|user_id| cache.user(user_id))
        //     .collect();

        // println!("{users:#?}");
        // println!("{messages:#?}");

        // match event {
        //     Event::MessageCreate(message) => {
        //         if cache
        //             .current_user()
        //             .is_none_or(|user| user.id == message.author.id)
        //         {
        //             continue;
        //         }

        //         let mut users = HashSet::new();

        //         let messages: Vec<_> = cache
        //             .channel_messages(message.channel_id)
        //             .as_deref()
        //             .into_iter()
        //             .flatten()
        //             .copied()
        //             .flat_map(|message_id| cache.message(message_id))
        //             .inspect(|message| {
        //                 users.insert(message.author());
        //             })
        //             .collect();

        //         let users: Vec<_> = users
        //             .into_iter()
        //             .flat_map(|user_id| cache.user(user_id))
        //             .collect();

        //         println!("{users:#?}");
        //         println!("{messages:#?}");
        //     }
        //     Event::ReactionAdd(reaction) => {
        //         if cache
        //             .current_user()
        //             .is_none_or(|user| user.id == reaction.user_id)
        //         {
        //             continue;
        //         }
        //     }
        //     _ => {}
        // }
    }

    Ok(())
}
