use {
    crate::model::{LanguageModelType, VisionModelType},
    serde::{Deserialize, Serialize},
    std::num::NonZeroU16,
    ubyte::{ByteUnit, ToByteUnit},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Cache {
    pub max_entries: NonZeroU16,
    // TODO: Restrict to sane values.
    pub max_file_size: ByteUnit,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Discord {
    pub token: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Language {
    pub model: LanguageModelType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Vision {
    pub model: VisionModelType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub cache: Cache,
    pub discord: Discord,
    pub language: Language,
    pub vision: Vision,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            max_entries: NonZeroU16::new(32).unwrap(),
            max_file_size: 8.megabytes(),
        }
    }
}
