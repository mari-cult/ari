use {
    crate::{model::LanguageModel, tokenizer::Tokenizer},
    candle_transformers::{generation::LogitsProcessor, utils},
    std::{slice, time::Instant},
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

        let repeat_last_n = *repeat_last_n;
        let repeat_penalty = *repeat_penalty;

        let start = Instant::now();
        let Some(stop_token) = tokenizer
            .tokenizer
            .get_vocab(true)
            .get("<|endoftext|>")
            .copied()
        else {
            tracing::warn!("no stop token");

            return Err(anyhow::anyhow!("no stop token"));
        };

        let prompt = tokenizer.tokenize_special(input)?;
        let prompt = prompt
            .tokens()
            .iter()
            .copied()
            .take(4096)
            .collect::<Vec<_>>();

        let string = tokenizer.decode(&prompt)?;

        tracing::info!("forward {string:?}");

        let logits = model.forward(&prompt, 0)?;
        let mut tokens = vec![logits_processor.sample(&logits)?];

        for index in 0..50 {
            let input = slice::from_ref(tokens.last().unwrap());
            let string = tokenizer.decode(input)?;

            tracing::info!("forward {string:?}");

            let logits = model.forward(input, prompt.len() + index)?;
            let logits = if repeat_penalty == 1.0 {
                logits
            } else {
                utils::apply_repeat_penalty(
                    &logits,
                    repeat_penalty,
                    &tokens[tokens.len().saturating_sub(repeat_last_n)..],
                )?
            };

            let token = logits_processor.sample(&logits)?;

            tokens.push(token);

            if token == stop_token {
                break;
            }

            let string = tokenizer.decode(&tokens)?;

            if string.contains("<|assistant|>") || string.contains("Clyde:") {
                break;
            }
        }

        // TODO: make this not dumb
        let string = tokenizer.decode(&tokens)?;
        let string = string.trim();
        let string = string.strip_suffix("Clyde:").unwrap_or(string);
        let string = string.trim();
        let string = string.strip_suffix("<|assistant|>").unwrap_or(string);
        let string = string.trim();

        let elapsed = start.elapsed();

        tracing::info!("processed {input:?} in {elapsed:.2?}");
        tracing::info!(
            "generated {} tokens ({:.2?} tokens/s)",
            tokens.len(),
            tokens.len() as f64 / elapsed.as_secs_f64()
        );

        tracing::info!("response: {string:?}");

        Ok(String::from(string))
    }
}
