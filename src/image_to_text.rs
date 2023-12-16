use {
    crate::{model::VisionModel, tokenizer::Tokenizer},
    candle_core::{Device, Tensor},
    candle_transformers::generation::LogitsProcessor,
    std::time::Instant,
};

pub struct ImageToText {
    logits_processor: LogitsProcessor,
    model: VisionModel,
    tokenizer: Tokenizer,
}

impl ImageToText {
    pub fn new(model: VisionModel, tokenizer: Tokenizer) -> Self {
        Self {
            logits_processor: LogitsProcessor::new(0, None, None),
            model,
            tokenizer,
        }
    }

    pub fn generate(&mut self, image: &Tensor, device: &Device) -> anyhow::Result<String> {
        let Self {
            logits_processor,
            model,
            tokenizer,
        } = self;

        tracing::info!("processing image");

        let start = Instant::now();
        let mut tokens = vec![30522];
        let image_embedding = model.image_to_embedding(image, device)?;

        for index in 0..1000 {
            let context_len = if index > 0 { 1 } else { tokens.len() };
            let position = tokens.len().saturating_sub(context_len);
            let input = &tokens[position..];
            let logits = model.text_decoder_forward(input, &image_embedding, device)?;

            let token = logits_processor.sample(&logits)?;

            if token == 102 {
                break;
            }

            tokens.push(token);
        }

        model.reset();
        let string = tokenizer.decode(&tokens)?;
        let elapsed = start.elapsed();

        tracing::info!("processed image in {elapsed:.2?}");
        tracing::info!(
            "generated {} tokens ({:.2?} tokens/s)",
            tokens.len(),
            tokens.len() as f64 / elapsed.as_secs_f64()
        );

        tracing::info!("response: {string}");

        Ok(string)
    }
}
