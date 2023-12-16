use {
    crate::{model::LanguageModel, tokenizer::Tokenizer},
    candle_transformers::{generation::LogitsProcessor, utils},
};

pub struct TextGeneration {
    logits_processor: LogitsProcessor,
    model: LanguageModel,
    repeat_last_n: usize,
    repeat_penalty: f32,
    tokenizer: Tokenizer,
}

impl TextGeneration {
    pub fn new(model: LanguageModel, tokenizer: Tokenizer) -> Self {
        Self {
            logits_processor: LogitsProcessor::new(0, Some(0.2), None),
            model,
            repeat_last_n: 64,
            repeat_penalty: 1.2,
            tokenizer,
        }
    }

    pub fn generate(&mut self, input: &str) -> anyhow::Result<String> {
        let Self {
            logits_processor,
            model,
            repeat_last_n,
            repeat_penalty,
            tokenizer,
        } = self;

        let mut tokens = tokenizer.tokenize(input)?.tokens().to_vec();

        for index in 0..50 {
            let context_len = if index > 0 { 1 } else { tokens.len() };
            let position = tokens.len().saturating_sub(context_len);
            let input = &tokens[position..];

            let logits = model.forward(input, 0)?;
            let logits = if *repeat_penalty == 1.0 {
                logits
            } else {
                let start_at = tokens.len().saturating_sub(*repeat_last_n);

                utils::apply_repeat_penalty(&logits, *repeat_penalty, &tokens[start_at..])?
            };

            tokens.push(logits_processor.sample(&logits)?);
        }

        let string = tokenizer.decode(&tokens)?;

        Ok(string)
    }
}
