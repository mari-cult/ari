use serde::Serialize;

/// Accepted `responseMimeType` values.
#[derive(Serialize)]
enum Mime {
    #[serde(rename = "text/plain")]
    Text,
    #[serde(rename = "text/x.enum")]
    Enum,
    #[serde(rename = "application/json")]
    Json,
}

/// Generation configuration.
#[derive(Serialize)]
pub(super) struct GenerationConfig {
    #[serde(rename = "response_mime_type")]
    mime: Mime,
}

impl GenerationConfig {
    /// Create a default configuration.
    pub const fn new() -> Self {
        Self { mime: Mime::Text }
    }

    /// Toggle whether JSON output is enabled.
    pub(super) const fn json(mut self, json: bool) -> Self {
        self.mime = if json { Mime::Json } else { Mime::Text };
        self
    }

    /// Returns `true` if output mode is text.
    pub(super) const fn is_text(&self) -> bool {
        matches!(self.mime, Mime::Text)
    }
}
