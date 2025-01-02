use std::future::{Future, IntoFuture};
use std::pin::Pin;

use tracing::info;

use crate::model::{
    self, BlockThreshold, GeminiMessage, GeminiResponse, SafetyCategory, SafetySetting,
};
use crate::Part;

#[derive(Clone)]
pub struct GenerateContent<'a> {
    http: &'a super::GeminiClient,
    model: &'a str,
    system: &'a str,
    json: bool,
    messages: Vec<GeminiMessage>,
    safety: Vec<SafetySetting>,
}

impl<'a> GenerateContent<'a> {
    pub(crate) const fn new(http: &'a super::GeminiClient, model: &'a str) -> Self {
        Self {
            http,
            model,
            system: "",
            json: false,
            messages: Vec::new(),
            safety: Vec::new(),
        }
    }

    pub const fn system(mut self, system: &'a str) -> Self {
        self.system = system;
        self
    }

    pub const fn json(mut self, json: bool) -> Self {
        self.json = json;
        self
    }

    pub fn message(mut self, message: GeminiMessage) -> Self {
        self.messages.push(message);
        self
    }

    pub fn clear(mut self) -> Self {
        self.messages.clear();
        self
    }

    pub fn safety(mut self, category: SafetyCategory, threshold: BlockThreshold) -> Self {
        self.safety.push(SafetySetting {
            category,
            threshold,
        });

        self
    }
}

impl IntoFuture for GenerateContent<'_> {
    // TODO: employ custom error types
    type Output = anyhow::Result<Vec<Part>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        let model = self.model;
        let mut request = model::GenerateContent::new()
            .system(self.system)
            .json(self.json);

        request.contents = self.messages;

        let url = self
            .http
            .with_base(&format!("v1beta/models/{model}:generateContent"));

        let query = [("key", &self.http.api_key)];

        let request = self.http.client.post(url).query(&query).json(&request);

        Box::pin(async move {
            let bytes = request.send().await?.bytes().await?;

            let string = serde_json::to_string_pretty(
                &serde_json::from_slice::<serde_json::Value>(&bytes)?,
            )?;

            info!("{string}");

            let response = serde_json::from_slice::<GeminiResponse>(&bytes)?;

            let parts = response
                .candidates
                .into_iter()
                .flat_map(|candidate| candidate.content.parts)
                .collect::<Vec<_>>();

            Ok(parts)
        })
    }
}
