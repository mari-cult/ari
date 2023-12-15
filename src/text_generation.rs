use {
    crate::{model::LanguageModel, tokenizer::Tokenizer},
    candle_transformers::{generation::LogitsProcessor, utils},
};

pub struct TextGeneration {
    logits_processor: LogitsProcessor,
    model: LanguageModel,
    repeat_penalty: f32,
    repeat_last_n: usize,
    tokenizer: Tokenizer,
    tokens: Vec<u32>,
}

impl TextGeneration {
    pub fn new(model: LanguageModel, tokenizer: Tokenizer) -> Self {
        Self {
            logits_processor: LogitsProcessor::new(0, Some(0.2), None),
            model,
            repeat_penalty: 1.2,
            repeat_last_n: 64,
            tokenizer,
            tokens: Vec::new(),
        }
    }

    pub fn generate(&mut self, input: &str) -> anyhow::Result<String> {
        let input = self.tokenizer.tokenize(input)?;

        self.tokens.extend(input.tokens().iter().copied());

        for index in 0..50 {
            let context_len = if index > 0 { 1 } else { self.tokens.len() };
            let position = self.tokens.len().saturating_sub(context_len);
            let input = &self.tokens[position..];
            let logits = self.model.forward(input, 0)?;

            let logits = if self.repeat_penalty == 1.0 {
                logits
            } else {
                let start_at = self.tokens.len().saturating_sub(self.repeat_last_n);

                utils::apply_repeat_penalty(&logits, self.repeat_penalty, &self.tokens[start_at..])?
            };

            let next_token = self.logits_processor.sample(&logits)?;

            self.tokens.push(next_token);
        }

        let string = self.tokenizer.decode(&self.tokens)?;

        Ok(string)
    }
}
