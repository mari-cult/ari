use serde::Serialize;

#[derive(Serialize)]
struct Part<'a> {
    text: &'a str,
}

#[derive(Serialize)]
pub(super) struct System<'a> {
    parts: [Part<'a>; 1],
}

impl<'a> System<'a> {
    pub(super) const fn new(system: &'a str) -> Self {
        Self {
            parts: [Part { text: system }],
        }
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.parts[0].text.is_empty()
    }
}
