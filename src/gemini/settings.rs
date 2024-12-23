#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Voice {
    #[default]
    Aoede,
    Charon,
    Fenrir,
    Kore,
    Puck,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Modality {
    pub audio: bool,
    pub image: bool,
    pub text: bool,
}

impl Default for Modality {
    fn default() -> Self {
        Self {
            audio: false,
            image: false,
            text: true,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct LiveSettings {
    pub api_key: String,
    pub modality: Modality,
    pub system: String,
    pub voice: Option<Voice>,
}
