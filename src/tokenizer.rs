use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("tokenize")]
    Tokenize,

    #[error("detokenize")]
    Detokenize,
}

pub struct Tokenizer {
    pub(crate) tokenizer: tokenizers::Tokenizer,
}

pub struct Tokens {
    pub(crate) encoding: tokenizers::Encoding,
}

impl Tokenizer {
    pub fn tokenize(&self, input: &str) -> Result<Tokens, Error> {
        self.tokenizer
            .encode(input, false)
            .map(|encoding| {
                for (token, id) in encoding.get_tokens().iter().zip(encoding.get_ids().iter()) {
                    let token = token.replace('▁', " ").replace("<0x0A>", "\n");

                    tracing::debug!("{id:7} -> {token:?}");
                }

                Tokens { encoding }
            })
            .map_err(|_error| Error::Tokenize)
    }

    pub fn tokenize_special(&self, input: &str) -> Result<Tokens, Error> {
        self.tokenizer
            .encode(input, true)
            .map(|encoding| Tokens { encoding })
            .map_err(|_error| Error::Tokenize)
    }

    pub fn decode(&self, tokens: &[u32]) -> Result<String, Error> {
        self.tokenizer
            .decode(tokens, false)
            .map_err(|_error| Error::Detokenize)
    }

    pub fn decode_special(&self, tokens: &[u32]) -> Result<String, Error> {
        self.tokenizer
            .decode(tokens, true)
            .map_err(|_error| Error::Detokenize)
    }
}

impl Tokens {
    pub fn tokens(&self) -> &[u32] {
        self.encoding.get_ids()
    }
}
