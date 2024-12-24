use gemini::LiveSettings;
use std::env;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};
use twilight_http::Client;
use twilight_model::id::{marker::ChannelMarker, Id};

mod discord;
mod gemini;

const LIVE_ID: Id<ChannelMarker> = Id::new(1320753531329974374);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install default crypto provider"))?;

    let api_key = env::var("GOOGLE_AI_API_KEY")?;
    let token = env::var("DISCORD_TOKEN")?;
    let client = Arc::new(Client::new(token.clone()));
    let mut discord = discord::start(token);
    let mut message_queue = Vec::new();

    'outermost: while {
        message_queue.extend(discord.drain().await);

        !message_queue.is_empty()
    } {
        debug!("starting new live connection");

        let (mut outgoing, mut incoming) = gemini::connect(LiveSettings {
            api_key: api_key.clone(),
            system: include_str!("lila.txt").trim().into(),
            ..Default::default()
        })
        .await?;

        let mut message_queue = std::mem::take(&mut message_queue);
        let discord = discord.clone();
        let client = Arc::clone(&client);

        tokio::spawn(async move {
            'outer: loop {
                debug!("outer incoming loop");

                let mut response = String::new();
                let mut typing = None;

                loop {
                    debug!("inner incoming loop");

                    let bytes = match incoming.next().await {
                        Some(Message::Binary(bytes)) => bytes,
                        Some(Message::Close(_)) | None => {
                            warn!("ws closed");

                            break 'outer;
                        }
                        _ => continue,
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
                            debug!("start typing");
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

                            debug!("send message");

                            if let Err(error) =
                                client.create_message(LIVE_ID).content(response).await
                            {
                                warn!(source = ?error, "error creating message");
                            }
                        } else {
                            warn!("response is empty");
                        }

                        break;
                    }
                }
            }

            anyhow::Ok(())
        });

        while !message_queue.is_empty() {
            let next_message = message_queue.remove(0);
            let json = serde_json::to_string(&next_message)?;

            {
                let json = serde_json::to_string_pretty(&next_message)?;

                info!("sending new discord message: {json}");
            }

            if let Err(error) = outgoing.send(&gemini::model::outgoing::OutgoingMessage {
                client_content: gemini::model::outgoing::ClientContent {
                    turns: vec![gemini::model::Content::new(json)],
                    turn_complete: message_queue.is_empty(),
                },
            }) {
                error!(source = ?error, "sending outgoing message(s) failed");
                message_queue.insert(0, next_message);
                break;
            }
        }
    }

    Ok(())
}

fn typing_duration(text: &str) -> f64 {
    let avg_wpm = 80.0; // Average words per minute
    let avg_chars_per_word = 5.0; // Average characters per word
    let chars = text.len() as f64;
    let words = chars / avg_chars_per_word;
    let minutes = words / avg_wpm;
    minutes * 60.0 // Convert minutes to seconds
}
