use gemini::LiveSettings;
use std::env;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};
use twilight_http::Client;
use twilight_model::id::{marker::ChannelMarker, Id};

mod discord;
mod gemini;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = env::var("GOOGLE_AI_API_KEY")?;
    let token = env::var("DISCORD_TOKEN")?;

    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install default crypto provider"))?;

    let client = Arc::new(Client::new(token.clone()));
    let mut messages = discord::start(token);

    let (mut outgoing, mut incoming) = gemini::connect(LiveSettings {
        api_key,
        system: include_str!("lila.txt").trim().into(),
        ..Default::default()
    })
    .await?;

    tokio::spawn(async move {
        loop {
            let messages = messages.drain().await;

            if messages.is_empty() {
                break;
            }

            let mut iter = messages.iter().peekable();

            while let Some(message) = iter.next() {
                let json = serde_json::to_string(&message)?;

                {
                    let json = serde_json::to_string_pretty(&message)?;

                    info!("sending new discord message: {json}");
                }

                outgoing.send(&gemini::model::outgoing::OutgoingMessage {
                    client_content: gemini::model::outgoing::ClientContent {
                        turns: vec![gemini::model::Content::new(json)],
                        turn_complete: iter.peek().is_none(),
                    },
                })?;
            }
        }

        anyhow::Ok(())
    });

    const LIVE_ID: Id<ChannelMarker> = Id::new(1320753531329974374);

    loop {
        let mut response = String::new();
        let mut typing = None;

        loop {
            let Some(Message::Binary(bytes)) = incoming.next().await else {
                continue;
            };

            let incoming_message: gemini::model::incoming::IncomingMessage =
                serde_json::from_slice(&bytes)?;

            let text = incoming_message
                .server_content
                .model_turn
                .parts
                .first()
                .and_then(|part| part.as_text());

            if let Some(text) = text {
                response.push_str(text);

                if !response.trim().is_empty() && typing.is_none() {
                    typing = Some(discord::typing::start(Arc::clone(&client), LIVE_ID))
                }
            }

            if incoming_message.server_content.turn_complete {
                let response = response.trim();

                if !response.is_empty() {
                    /*tokio::time::sleep(std::time::Duration::from_secs_f64(typing_duration(
                        &response,
                    )))
                    .await;*/

                    if let Err(error) = client.create_message(LIVE_ID).content(response).await {
                        warn!(source = ?error, "error creating message");
                    }
                } else {
                    warn!("response is empty");
                }

                break;
            }
        }
    }
}

fn typing_duration(text: &str) -> f64 {
    let avg_wpm = 80.0; // Average words per minute
    let avg_chars_per_word = 5.0; // Average characters per word
    let chars = text.len() as f64;
    let words = chars / avg_chars_per_word;
    let minutes = words / avg_wpm;
    minutes * 60.0 // Convert minutes to seconds
}
