use self::screen_recorder::ScreenRecorder;
use google_gemini::{
    GeminiClient, GeminiGenerationConfig, GeminiMessage, GeminiPart, GeminiRequest, GeminiRole,
    GeminiSystemInstruction, GeminiSystemPart,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::time::Duration;
use std::{env, fs};
use tokio::time;
use tracing::info;

mod physical_control;
mod screen_recorder;

#[derive(Clone, Debug, Deserialize, JsonSchema)]
pub enum Modifier {
    Alt,
    Ctrl,
    Shift,
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
pub enum Button {
    Left,
    Right,
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
pub enum Action {
    KeyboardInput {
        /// A string of key codes to press.
        input: String,
        #[serde(default)]
        /// A list of modifiers to apply to said key codes.
        modifiers: Vec<Modifier>,
    },
    PointerClick {
        /// Which mouse button to press.
        button: Button,
    },
    PointerMoveTo {
        x: u16,
        y: u16,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = env::var("GOOGLE_API_KEY")?;
    let gemini = GeminiClient::new(api_key);
    let schema = schemars::schema_for!(Vec<Action>);
    let schema = serde_json::to_string(&schema)?;

    for _ in 0..10 {
        info!("start recording");
        let recording = ScreenRecorder::start()?;

        info!("wait 5s");
        time::sleep(Duration::from_secs(10)).await;

        info!("stop recording");
        recording.stop().await?;

        let input = fs::read("/tmp/unknown.mkv")?;
        let input_len: u32 = input.len().try_into()?;

        let uri = gemini
            .create_file("unknown.mkv", input_len, "video/x-matroska")
            .await?;

        let uri = gemini.upload_file(uri.clone(), input_len, input).await?;

        let mut request = GeminiRequest {
            system_instruction: Some(GeminiSystemInstruction {
                parts: vec![GeminiSystemPart {
                    text: format!(
                        "{}\nrespond following this json schema:\n{schema}",
                        include_str!("../personality.txt")
                    ),
                }],
            }),
            generation_config: Some(GeminiGenerationConfig {
                response_mime_type: "application/json".into(),
            }),
            ..Default::default()
        };

        request.contents.push(GeminiMessage {
            role: GeminiRole::User,
            parts: vec![
                GeminiPart::FileData {
                    mime_type: "video/x-matroska".into(),
                    file_uri: uri.clone(),
                },
                GeminiPart::Text("this is your pc's screen over the last 10s, respond to the conversation naturally as cleo".into()),
            ],
        });

        let response = gemini.generate(request).await?;
        let result = serde_json::from_str::<Vec<Action>>(&response);

        let Ok(actions) = result else {
            tracing::error!(source = ?result.unwrap_err(), "error parsing response");
            return Ok(());
        };

        println!("{actions:#?}");

        for action in actions {
            match action {
                Action::KeyboardInput { input, modifiers } => {
                    if modifiers.is_empty() {
                        physical_control::keyboard_input(&input)?;
                    } else {
                        let modifiers = modifiers
                            .into_iter()
                            .map(|modifier| match modifier {
                                Modifier::Alt => "alt",
                                Modifier::Ctrl => "ctrl",
                                Modifier::Shift => "shift",
                            })
                            .collect::<Vec<_>>()
                            .join(",");

                        physical_control::keyboard_input_with_modifiers(&input, &modifiers)?;
                    }
                }
                Action::PointerClick { button } => {
                    let button = match button {
                        Button::Left => "left",
                        Button::Right => "right",
                    };

                    physical_control::pointer_click(button)?;
                }
                Action::PointerMoveTo { x, y } => {
                    let x = ((x as f32 / 1000.0) * 2560.0) as u16;
                    let y = ((y as f32 / 1000.0) * 2560.0) as u16;

                    physical_control::pointer_move_to(x, y)?;
                }
            }

            time::sleep(Duration::from_millis(100)).await;
        }
    }

    Ok(())
}
