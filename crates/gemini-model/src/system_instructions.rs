use serde::Serialize;

/// `Part` with only a `text` field.
///
/// Serializes to `{ "text": <str> }`.
#[derive(Serialize)]
struct Part<'a> {
    text: &'a str,
}

/// An optional system instruction structure.
///
/// Equivalent to `Contents`, but containing exactly only one `Part`.
///
/// Serializes to `{ "parts": [<Part>] }`.
#[derive(Serialize)]
pub(super) struct SystemInstructions<'a> {
    parts: [Part<'a>; 1],
}

impl<'a> SystemInstructions<'a> {
    /// Create an empty system instruction `Contents`.
    pub(super) const fn new() -> Self {
        Self::from("")
    }

    /// Create a system instruction `Contents` from the specified string.
    pub(super) const fn from(system_instructions: &'a str) -> Self {
        Self {
            parts: [Part {
                text: system_instructions,
            }],
        }
    }

    /// Returns `true` if the inner `text` field is non-empty.
    pub(super) const fn is_empty(&self) -> bool {
        self.parts[0].text.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize() {
        let json = serde_json::to_string(&SystemInstructions::from("ok")).unwrap();

        assert_eq!(json, r#"{"parts":[{"text":"ok"}]}"#);
    }
}
