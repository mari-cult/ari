use serde::Serialize;

#[derive(Serialize)]
struct Key<'a> {
    key: &'a str,
}

/// Gemini API authorization.
///
/// Serializes to `?key=<API KEY>`.
#[derive(Serialize)]
#[serde(transparent)]
pub struct Authentication<'a> {
    query: [Key<'a>; 1],
}

impl<'a> Authentication<'a> {
    /// Create an authorization query string from the provided key.
    pub const fn new(api_key: &'a str) -> Self {
        Self {
            query: [Key { key: api_key }],
        }
    }
}
