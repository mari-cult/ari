use super::Content;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingMessage {
    pub server_content: ServerContent,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerContent {
    pub model_turn: Content,
    pub turn_complete: bool,
}
