use {
    serenity::model::channel::Nonce,
    std::hash::{DefaultHasher, Hash, Hasher},
};

/// Calculate a nonce for the specified `content`.
pub fn nonce_of(content: &str) -> Nonce {
    let mut hasher = DefaultHasher::new();

    content.hash(&mut hasher);

    Nonce::Number(hasher.finish() % i64::MAX as u64)
}
