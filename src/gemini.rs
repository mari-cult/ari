use self::model::outgoing::SetupConfig;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error};

pub use self::settings::{LiveSettings, Modality, Voice};

pub mod model;
mod settings;

type WebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct Outgoing {
    sink: UnboundedSender<Message>,
}

pub struct Incoming {
    stream: UnboundedReceiver<Message>,
}

impl Outgoing {
    pub fn send<T: Serialize>(&mut self, value: &T) -> anyhow::Result<()> {
        let json = serde_json::to_string(value)?;

        self.sink.send(Message::text(json)).map_err(Into::into)
    }
}

impl Incoming {
    pub async fn next(&mut self) -> Option<Message> {
        self.stream.recv().await
    }
}

pub async fn connect(settings: LiveSettings) -> anyhow::Result<(Outgoing, Incoming)> {
    if settings.api_key.is_empty() {
        return Err(anyhow::anyhow!("unauthorized"));
    }

    let url = format!("wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1alpha.GenerativeService.BidiGenerateContent?key={}", settings.api_key);
    let (socket, _response) = tokio_tungstenite::connect_async(url).await?;
    let (mut sink, mut stream) = socket.split();
    let (outgoing_sender, mut outgoing_recevier) = mpsc::unbounded_channel();
    let (incoming_sender, incoming_recevier) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(item) = outgoing_recevier.recv().await {
            debug!("new outgoing message: {item:?}");

            if let Err(error) = sink.send(item).await {
                error!(source = ?error, "error sending message");

                break;
            }
        }

        anyhow::Ok(())
    });

    tokio::spawn(async move {
        while let Some(item) = stream.next().await {
            debug!("new incoming message: {item:?}");

            let Ok(message) = item else {
                error!(source = ?item.unwrap_err(), "error receving message");

                return anyhow::Ok(());
            };

            if let Err(error) = incoming_sender.send(message) {
                error!(source = ?error, "error sending message");

                return anyhow::Ok(());
            }
        }

        anyhow::Ok(())
    });

    let mut outgoing = Outgoing {
        sink: outgoing_sender,
    };

    let mut incoming = Incoming {
        stream: incoming_recevier,
    };

    outgoing.send(&SetupConfig::new(settings))?;
    // TODO: check for correct packet.
    incoming.next().await.unwrap();

    Ok((outgoing, incoming))
}
