use gemini::LiveSettings;
use std::env;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};
use twilight_http::Client;

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

    while let Some(message) = messages.next().await {
        let json = serde_json::to_string(&message)?;

        {
            let json = serde_json::to_string_pretty(&message)?;

            info!("sending new discord message: {json}");
        }

        outgoing.send(&gemini::model::outgoing::OutgoingMessage {
            client_content: gemini::model::outgoing::ClientContent {
                turns: vec![gemini::model::Content::new(json)],
                turn_complete: true,
            },
        })?;

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
                .parts.first()
                .and_then(|part| part.as_text());

            if let Some(text) = text {
                response.push_str(text);

                if !response.trim().is_empty() && typing.is_none() {
                    typing = Some(discord::typing::start(
                        Arc::clone(&client),
                        message.channel_id,
                    ))
                }
            }

            if incoming_message.server_content.turn_complete {
                let response = response.trim();

                if !response.is_empty() {
                    if let Err(error) = client
                        .create_message(message.channel_id)
                        .content(response)
                        .await
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

    Ok(())
}
