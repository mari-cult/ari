use {
    serenity::model::{channel::Nonce, user::User},
    std::hash::{DefaultHasher, Hash, Hasher},
};

pub trait UserExt {
    fn display_name(&self) -> &str;
}

impl UserExt for User {
    fn display_name(&self) -> &str {
        self.global_name.as_deref().unwrap_or(&self.name)
    }
}

pub trait StrExt {
    fn to_quoted(&self) -> String;
    fn trim_footer(&self) -> &str;
}

impl StrExt for str {
    fn to_quoted(&self) -> String {
        textwrap::indent(self, "> ").trim_end().to_string()
    }

    fn trim_footer(&self) -> &str {
        self.rsplit_once("-# ")
            .map(|(content, _footer)| content)
            .unwrap_or(self)
            .trim_end()
    }
}

/// Create a message footer from the specified list of items.
///
/// Each item is separated by `U+2022`, the Bullet character.
pub fn footer<I, S>(items: I) -> String
where
    I: IntoIterator<Item = S>,
    S: ToString,
{
    let items = items
        .into_iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(" \u{2022} ");

    String::from("\n-# ") + &items
}

/// Calculate a nonce for the specified `content`.
pub fn nonce_of(content: &str) -> Nonce {
    let mut hasher = DefaultHasher::new();

    content.hash(&mut hasher);

    Nonce::Number(hasher.finish() % i64::MAX as u64)
}
