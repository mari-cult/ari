use {
    crate::{model::VisionModel, tokenizer::Tokenizer},
    candle_core::{Device, Tensor},
    candle_transformers::generation::LogitsProcessor,
};

pub struct ImageToText {
    logits_processor: LogitsProcessor,
    model: VisionModel,
    tokenizer: Tokenizer,
    tokens: Vec<u32>,
}

impl ImageToText {
    pub fn new(model: VisionModel, tokenizer: Tokenizer) -> Self {
        Self {
            logits_processor: LogitsProcessor::new(0, None, None),
            model,
            tokenizer,
            tokens: vec![30522],
        }
    }

    pub fn generate(&mut self, image: &Tensor, device: &Device) -> anyhow::Result<String> {
        let image_embedding = self.model.image_to_embedding(image, device)?;

        for index in 0..1000 {
            let context_len = if index > 0 { 1 } else { self.tokens.len() };
            let position = self.tokens.len().saturating_sub(context_len);
            let input = &self.tokens[position..];
            let logits = self
                .model
                .text_decoder_forward(input, &image_embedding, device)?;

            let token = self.logits_processor.sample(&logits)?;

            if token == 102 {
                break;
            }

            self.tokens.push(token);
        }

        let string = self.tokenizer.decode(&self.tokens)?;

        Ok(string)
    }
}
