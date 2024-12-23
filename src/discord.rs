use serde::Serialize;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tracing::{debug, warn};
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _};
use twilight_model::id::marker::{ChannelMarker, MessageMarker, UserMarker};
use twilight_model::id::Id;

pub mod typing;

const LIVE_ID: Id<ChannelMarker> = Id::new(1320753531329974374);

#[derive(Serialize)]
pub struct Author {
    pub username: String,
    pub display_name: Option<String>,
    pub user_id: Id<UserMarker>,
}

#[derive(Serialize)]
pub struct Message {
    pub author: Author,
    pub content: String,
    pub message_id: Id<MessageMarker>,
    pub channel_id: Id<ChannelMarker>,
}

pub struct Discord {
    stream: UnboundedReceiver<Message>,
}

impl Discord {
    pub async fn drain(&mut self) -> Vec<Message> {
        let mut messages = Vec::new();

        self.stream.recv_many(&mut messages, usize::MAX).await;

        messages
    }
}

pub fn start(token: String) -> Discord {
    let intents = Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;
    let events = EventTypeFlags::READY | EventTypeFlags::GUILD_MESSAGES;
    let mut shard = Shard::new(ShardId::ONE, token, intents);
    let mut self_id = None;
    let (sender, receiver) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        loop {
            while let Some(item) = shard.next_event(events | EventTypeFlags::READY).await {
                let Ok(event) = item else {
                    warn!(source = ?item.unwrap_err(), "error receving event");

                    continue;
                };

                if sender.is_closed() {
                    debug!("discord event channel is closed");

                    break;
                }

                match event {
                    Event::Ready(ready) => self_id = Some(ready.user.id),
                    Event::MessageCreate(message)
                        if message.channel_id == LIVE_ID
                            && self_id.is_some_and(|id| id != message.author.id) =>
                    {
                        let author = Author {
                            username: message.author.name.clone(),
                            display_name: message.author.global_name.clone(),
                            user_id: message.author.id,
                        };

                        let message = Message {
                            author,
                            content: message.content.clone(),
                            message_id: message.id,
                            channel_id: message.channel_id,
                        };

                        if sender.send(message).is_err() {
                            debug!("discord event channel is closed");

                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    Discord { stream: receiver }
}
