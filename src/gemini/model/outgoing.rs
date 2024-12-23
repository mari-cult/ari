use super::Content;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Modality {
    Audio,
    Image,
    Text,
}

impl Modality {
    fn new(modality: super::Modality) -> Vec<Self> {
        let audio = modality.audio.then_some(Self::Audio);
        let image = modality.image.then_some(Self::Image);
        let text = modality.text.then_some(Self::Text);

        audio.into_iter().chain(image).chain(text).collect()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum Voice {
    Aoede,
    Charon,
    Fenrir,
    Kore,
    Puck,
}

impl Voice {
    fn new(voice: super::Voice) -> Self {
        match voice {
            super::Voice::Aoede => Self::Aoede,
            super::Voice::Charon => Self::Charon,
            super::Voice::Fenrir => Self::Fenrir,
            super::Voice::Kore => Self::Kore,
            super::Voice::Puck => Self::Puck,
        }
    }
}

#[derive(Serialize)]
struct PrebuiltVoiceConfig {
    voice_name: Voice,
}

#[derive(Serialize)]
struct VoiceConfig {
    prebuilt_voice_config: PrebuiltVoiceConfig,
}

#[derive(Serialize)]
struct SpeechConfig {
    voice_config: VoiceConfig,
}

#[derive(Serialize)]
struct LiveGenerationConfig {
    response_modalities: Vec<Modality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speech_config: Option<SpeechConfig>,
}

#[derive(Serialize)]
struct LiveConfig {
    model: String,
    generation_config: LiveGenerationConfig,
    #[serde(skip_serializing_if = "Content::is_empty")]
    system_instruction: Content,
}

#[derive(Serialize)]
pub(crate) struct SetupConfig {
    setup: LiveConfig,
}

impl SetupConfig {
    pub(crate) fn new(config: super::LiveSettings) -> Self {
        let speech_config = SpeechConfig {
            voice_config: VoiceConfig {
                prebuilt_voice_config: PrebuiltVoiceConfig {
                    voice_name: Voice::Aoede,
                },
            },
        };

        let generation_config = LiveGenerationConfig {
            response_modalities: Modality::new(config.modality),
            speech_config: Some(speech_config),
        };

        let system_instruction = Content::new_system(config.system);

        let setup = LiveConfig {
            model: "models/gemini-2.0-flash-exp".into(),
            generation_config,
            system_instruction,
        };

        SetupConfig { setup }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingMessage {
    pub client_content: ClientContent,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientContent {
    pub turns: Vec<Content>,
    pub turn_complete: bool,
}
