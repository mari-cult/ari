use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SafetySetting {
    pub category: SafetyCategory,
    pub threshold: BlockThreshold,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum SafetyCategory {
    #[serde(rename = "HARM_CATEGORY_HARASSMENT")]
    Harassment,
    #[serde(rename = "HARM_CATEGORY_HATE_SPEECH")]
    HateSpeech,
    #[serde(rename = "HARM_CATEGORY_SEXUALLY_EXPLICIT")]
    SexuallyExplicit,
    #[serde(rename = "HARM_CATEGORY_DANGEROUS_CONTENT")]
    DangerousContent,
    #[serde(rename = "HARM_CATEGORY_CIVIC_INTEGRITY")]
    CivicIntegrity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum BlockThreshold {
    #[serde(rename = "BLOCK_NONE")]
    None,
    #[serde(rename = "BLOCK_ONLY_HIGH")]
    Few,
    #[serde(rename = "BLOCK_MEDIUM_AND_ABOVE")]
    Some,
    #[serde(rename = "BLOCK_LOW_AND_ABOVE")]
    Most,
}
