use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot::error::TryRecvError;
use tokio::sync::oneshot::{self, Sender};
use tokio::time;
use tracing::{debug, error};
use twilight_http::Client;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::Id;

pub struct Typing {
    sink: Sender<()>,
}

pub fn start(client: Arc<Client>, channel_id: Id<ChannelMarker>) -> Typing {
    let (sender, mut receiver) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            if let Ok(()) | Err(TryRecvError::Closed) = receiver.try_recv() {
                debug!("typing channel closed");

                break;
            }

            if let Err(error) = client.create_typing_trigger(channel_id).await {
                error!(source = ?error, "error creating typing trigger");

                break;
            };

            time::sleep(Duration::from_secs(8)).await;
        }
    });

    Typing { sink: sender }
}
