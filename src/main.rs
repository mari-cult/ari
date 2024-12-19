use self::screen_recorder::ScreenRecorder;
use google_gemini::{
    GeminiClient, GeminiGenerationConfig, GeminiMessage, GeminiPart, GeminiRequest, GeminiRole,
    GeminiSystemInstruction, GeminiSystemPart,
};
use std::time::Duration;
use std::{env, fs};
use tokio::time;
use tracing::info;

mod screen_recorder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = env::var("GOOGLE_API_KEY")?;
    let gemini = GeminiClient::new(api_key);

    info!("start recording");
    let recording = ScreenRecorder::start()?;

    info!("wait 5s");
    time::sleep(Duration::from_secs(5)).await;

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
                text: include_str!("../personality.txt").into(),
            }],
        }),
        generation_config: Some(GeminiGenerationConfig {
            response_mime_type: "application/json".into(),
        }),
        ..Default::default()
    };

    request.contents.push(GeminiMessage {
        role: GeminiRole::User,
        parts: vec![GeminiPart::Text(
            r#"to use ur keyboard & mouse, respond with a json array of actions, i.e.

    [
        {"action": "keyboard", "value": ["ctrl+a", "ctrl+c", "ctrl+v", "enter"]},
        {"action": "mouse", "value": [{"x": 100, "y": 100}, "left click"]},
    ]
    "#
            .into(),
        )],
    });

    request.contents.push(GeminiMessage {
        role: GeminiRole::User,
        parts: vec![GeminiPart::FileData {
            mime_type: "video/x-matroska".into(),
            file_uri: uri.clone(),
        }],
    });

    let response = gemini.generate(request).await?;

    println!("{response:#?}");

    Ok(())
}
