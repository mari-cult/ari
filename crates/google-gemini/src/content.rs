use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Display, Serialize)]
#[display("{text}")]
#[serde(rename_all = "camelCase")]
pub struct TextPart {
    pub text: String,
    #[serde(default, skip_serializing)]
    pub thought: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineDataPart {
    pub mime_type: String,
    pub data: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallPart {
    pub name: String,
    pub args: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponsePart {
    pub name: String,
    pub response: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDataPart {
    pub mime_type: String,
    pub file_uri: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutableCodeLanguage {
    LanguageUnspecified,
    Python,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutableCodePart {
    pub language: ExecutableCodeLanguage,
    pub code: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    OutcomeOk,
    OutcomeDeadlineExceeded,
    OutcomeFailed,
    OutcomeUnspecified,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeExecutionResultPart {
    pub outcome: Outcome,
    pub output: String,
}

#[derive(Clone, Debug, Deserialize, From, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Part {
    Text(TextPart),
    InlineData(InlineDataPart),
    FunctionCall(FunctionCallPart),
    FunctionResponse(FunctionResponsePart),
    FileData(FileDataPart),
    ExecutableCode(ExecutableCodePart),
    CodeExecutionResult(CodeExecutionResultPart),
}

impl From<&str> for TextPart {
    fn from(text: &str) -> Self {
        Self::from(text.to_string())
    }
}

impl From<String> for TextPart {
    fn from(text: String) -> Self {
        Self {
            text,
            thought: false,
        }
    }
}

impl From<&str> for Part {
    fn from(value: &str) -> Self {
        Self::Text(TextPart::from(value))
    }
}

impl From<String> for Part {
    fn from(value: String) -> Self {
        Self::Text(TextPart::from(value))
    }
}
