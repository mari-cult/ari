use {
    crate::{fs, huggingface, tokenizer::Tokenizer},
    candle_core::{DType, Device, Tensor},
    candle_transformers::{
        models::{
            blip, llama, mistral, mixformer, quantized_blip, quantized_llama, quantized_mistral,
            quantized_mixformer, quantized_stable_lm, stable_lm,
        },
        quantized_var_builder,
    },
    serde::{Deserialize, Serialize},
    std::{fs::File, path::PathBuf},
};

pub enum LanguageModel {
    StableLm(stable_lm::Model),
    QuantizedStableLm(quantized_stable_lm::Model),
    Mixformer(mixformer::MixFormerSequentialForCausalLM),
    QuantizedMixformer(quantized_mixformer::MixFormerSequentialForCausalLM),
    Mistral(mistral::Model),
    QuantizedMistral(quantized_mistral::Model),
    Llama(llama::Llama),
    QuantizedLlama(quantized_llama::ModelWeights),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum LanguageModelType {
    #[serde(rename = "hermes-phi-1-1.3b")]
    HermesPhi1_1_3b,

    #[serde(rename = "hermes-phi-1-1.3b-q4")]
    HermesPhi1_1_3bQ4,

    #[serde(rename = "phi-1-1.3b")]
    Phi1_1_3b,

    #[serde(rename = "phi-1-1.3b-q4")]
    Phi1_1_3bQ4,

    #[serde(rename = "phi-1.5-1.3b")]
    Phi1_5_1_3b,

    #[serde(rename = "phi-1.5-1.3b-q4")]
    Phi1_5_1_3bQ4,

    #[serde(rename = "phi-2-2.7b")]
    Phi2_2_7b,

    #[serde(rename = "phi-2-2.7b-q4")]
    Phi2_2_7bQ4,

    #[serde(rename = "puffin-phi-2-2.7b")]
    PuffinPhi2_2_7b,

    #[serde(rename = "puffin-phi-2-2.7b-q4")]
    PuffinPhi2_2_7bQ4,

    #[serde(rename = "stablelm-4e1t-3b")]
    StableLm4e1t3b,

    #[serde(rename = "stablelm-4e1t-3b-q4")]
    StableLm4e1t3bQ4,

    #[serde(rename = "stablelm-zephyr-3b")]
    StableLmZephyr3b,

    #[serde(rename = "stablelm-zephyr-3b-q4")]
    StableLmZephyr3bQ4,
}

pub enum VisionModel {
    Blip(blip::BlipForConditionalGeneration),
    QuantizedBlip(quantized_blip::BlipForConditionalGeneration),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum VisionModelType {
    #[serde(rename = "blip-large")]
    BlipLarge,

    #[serde(rename = "blip-large-q4")]
    BlipLargeQ4,
}

impl LanguageModelType {
    pub fn repository(&self) -> &'static str {
        match self {
            Self::HermesPhi1_1_3b
            | Self::HermesPhi1_1_3bQ4
            | Self::Phi1_1_3bQ4
            | Self::Phi1_5_1_3bQ4
            | Self::Phi2_2_7bQ4
            | Self::PuffinPhi2_2_7b
            | Self::PuffinPhi2_2_7bQ4 => "lmz/candle-quantized-phi",
            Self::Phi1_1_3b => "microsoft/phi-1",
            Self::Phi1_5_1_3b => "microsoft/phi-1_5",
            Self::Phi2_2_7b => "microsoft/phi-2",
            Self::StableLm4e1t3b | Self::StableLm4e1t3bQ4 => "lmz/candle-stablelm-3b-4e1t",
            Self::StableLmZephyr3b => "stabilityai/stablelm-zephyr-3b",
            Self::StableLmZephyr3bQ4 => "TheBloke/stablelm-zephyr-3b-GGUF",
        }
    }

    pub fn file_names(&self) -> &'static [&'static str] {
        match self {
            Self::HermesPhi1_1_3b => &["model-phi-hermes-1_3B.safetensors"],
            Self::HermesPhi1_1_3bQ4 => &["model-phi-hermes-1_3B-q4k.gguf"],
            Self::Phi1_1_3bQ4 => &["model-v1-q4k.gguf"],
            Self::Phi1_5_1_3bQ4 => &["model-q4k.gguf"],
            Self::Phi2_2_7b => &[
                "model-00001-of-00002.safetensors",
                "model-00002-of-00002.safetensors",
            ],
            Self::Phi2_2_7bQ4 => &["model-v2-q4k.gguf"],
            Self::PuffinPhi2_2_7b => &["model-puffin-phi-v2.safetensors"],
            Self::PuffinPhi2_2_7bQ4 => &["model-puffin-phi-v2-q4k.gguf"],
            Self::StableLm4e1t3bQ4 => &["model-q4k.gguf"],
            Self::StableLmZephyr3bQ4 => &["stablelm-zephyr-3b.Q4_K_M.gguf"],
            _ => &["model.safetensors"],
        }
    }

    pub fn tokenizer_repository(&self) -> &'static str {
        match self {
            Self::HermesPhi1_1_3b
            | Self::HermesPhi1_1_3bQ4
            | Self::Phi1_1_3bQ4
            | Self::Phi1_5_1_3bQ4
            | Self::Phi2_2_7bQ4
            | Self::PuffinPhi2_2_7b
            | Self::PuffinPhi2_2_7bQ4 => "lmz/candle-quantized-phi",
            Self::Phi1_1_3b => "microsoft/phi-1",
            Self::Phi1_5_1_3b => "microsoft/phi-1_5",
            Self::Phi2_2_7b => "microsoft/phi-2",
            Self::StableLm4e1t3b | Self::StableLm4e1t3bQ4 => "lmz/candle-stablelm-3b-4e1t",
            Self::StableLmZephyr3b | Self::StableLmZephyr3bQ4 => "stabilityai/stablelm-zephyr-3b",
        }
    }

    pub fn tokenizer_file_name(&self) -> &'static str {
        match self {
            Self::HermesPhi1_1_3b | Self::PuffinPhi2_2_7bQ4 => "tokenizer-puffin-phi-v2.json",
            _ => "tokenizer.json",
        }
    }

    pub fn is_quantized(&self) -> bool {
        matches!(
            self,
            Self::HermesPhi1_1_3bQ4
                | Self::Phi1_1_3bQ4
                | Self::Phi1_5_1_3bQ4
                | Self::Phi2_2_7bQ4
                | Self::StableLm4e1t3bQ4
                | Self::StableLmZephyr3bQ4
        )
    }

    pub fn is_stable_lm(&self) -> bool {
        matches!(
            self,
            Self::StableLm4e1t3b
                | Self::StableLm4e1t3bQ4
                | Self::StableLmZephyr3b
                | Self::StableLmZephyr3bQ4
        )
    }

    pub fn is_phi(&self) -> bool {
        matches!(
            self,
            Self::HermesPhi1_1_3b
                | Self::HermesPhi1_1_3bQ4
                | Self::Phi1_1_3b
                | Self::Phi1_1_3bQ4
                | Self::Phi1_5_1_3b
                | Self::Phi1_5_1_3bQ4
                | Self::Phi2_2_7b
                | Self::Phi2_2_7bQ4
                | Self::PuffinPhi2_2_7b
                | Self::PuffinPhi2_2_7bQ4
        )
    }

    fn phi_config(&self) -> Option<mixformer::Config> {
        let config = match self {
            Self::Phi1_1_3b | Self::Phi1_1_3bQ4 => mixformer::Config::v1(),
            Self::Phi1_5_1_3b | Self::Phi1_5_1_3bQ4 => mixformer::Config::v1_5(),
            Self::Phi2_2_7b | Self::Phi2_2_7bQ4 => mixformer::Config::v2(),
            Self::HermesPhi1_1_3b | Self::HermesPhi1_1_3bQ4 => mixformer::Config::phi_hermes_1_3b(),
            Self::PuffinPhi2_2_7b | Self::PuffinPhi2_2_7bQ4 => mixformer::Config::puffin_phi_v2(),
            _ => return None,
        };

        Some(config)
    }

    pub fn fetch_tokenizer(&self) -> anyhow::Result<Vec<PathBuf>> {
        huggingface::fetch(self.tokenizer_repository(), &[self.tokenizer_file_name()])
            .map_err(Into::into)
    }

    pub fn fetch_model(&self) -> anyhow::Result<Vec<PathBuf>> {
        huggingface::fetch(self.repository(), self.file_names()).map_err(Into::into)
    }

    pub fn load_tokenizer(&self) -> anyhow::Result<Tokenizer> {
        fs::Options::new()
            .tokenizer(&self.fetch_tokenizer()?[0])
            .map_err(Into::into)
    }

    pub fn load_model(&self, device: &Device) -> anyhow::Result<LanguageModel> {
        let paths = self.fetch_model()?;
        let dtype = if device.is_cuda() {
            DType::F16
        } else {
            DType::F16
        };

        if self.is_phi() {
            let config = self.phi_config().unwrap();

            if self.is_quantized() {
                let vars = vars_gguf(paths)?;
                let model = if matches!(self, Self::Phi2_2_7bQ4) {
                    quantized_mixformer::MixFormerSequentialForCausalLM::new_v2(&config, vars)?
                } else {
                    quantized_mixformer::MixFormerSequentialForCausalLM::new(&config, vars)?
                };

                Ok(LanguageModel::QuantizedMixformer(model))
            } else {
                let vars = vars_safetensors(paths, dtype, device)?;
                let model = if matches!(self, Self::Phi2_2_7b) {
                    mixformer::MixFormerSequentialForCausalLM::new_v2(&config, vars)?
                } else {
                    mixformer::MixFormerSequentialForCausalLM::new(&config, vars)?
                };

                Ok(LanguageModel::Mixformer(model))
            }
        } else if self.is_stable_lm() {
            let config = stable_lm::Config::stablelm_3b_4e1t(false);

            if self.is_quantized() {
                let vars = vars_gguf(paths)?;
                let model = quantized_stable_lm::Model::new(&config, vars)?;

                Ok(LanguageModel::QuantizedStableLm(model))
            } else {
                let vars = vars_safetensors(paths, dtype, device)?;
                let model = stable_lm::Model::new(&config, vars)?;

                Ok(LanguageModel::StableLm(model))
            }
        } else {
            unimplemented!()
        }
    }
}

impl LanguageModel {
    pub fn reset(&mut self) {
        match self {
            Self::StableLm(model) => model.reset_kv_cache(),
            _ => {}
        }
    }

    pub fn forward(&mut self, input: &[u32], position: usize) -> anyhow::Result<Tensor> {
        fn stable_lm(tensor: Tensor) -> anyhow::Result<Tensor> {
            Ok(tensor.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?)
        }

        fn mixformer(tensor: Tensor) -> anyhow::Result<Tensor> {
            Ok(tensor.squeeze(0)?.to_dtype(DType::F32)?)
        }

        fn others(tensor: Tensor) -> anyhow::Result<Tensor> {
            Ok(tensor.squeeze(0)?)
        }

        let device = Device::Cpu;
        let input = Tensor::new(input, &device)?.unsqueeze(0)?;
        let input = &input;

        match self {
            Self::StableLm(model) => stable_lm(model.forward(input, position)?),
            Self::QuantizedStableLm(model) => stable_lm(model.forward(input, position)?),
            Self::Mixformer(model) => mixformer(model.forward(input)?),
            Self::QuantizedMixformer(model) => mixformer(model.forward(input)?),
            Self::Mistral(model) => others(model.forward(input, position)?),
            Self::QuantizedMistral(model) => others(model.forward(input, position)?),
            Self::Llama(model) => others(model.forward(input, position)?),
            Self::QuantizedLlama(model) => others(model.forward(input, position)?),
        }
    }
}

impl VisionModelType {
    pub fn repository(&self) -> &'static str {
        match self {
            Self::BlipLarge => "Salesforce/blip-image-captioning-large",
            Self::BlipLargeQ4 => "lmz/candle-blip",
        }
    }

    pub fn file_names(&self) -> &'static [&'static str] {
        match self {
            Self::BlipLargeQ4 => &["blip-image-captioning-large-q4k.gguf"],
            _ => &["model.safetensors"],
        }
    }

    pub fn tokenizer_repository(&self) -> &'static str {
        "Salesforce/blip-image-captioning-large"
    }

    pub fn tokenizer_file_name(&self) -> &'static str {
        "tokenizer.json"
    }

    pub fn fetch_tokenizer(&self) -> anyhow::Result<Vec<PathBuf>> {
        huggingface::fetch(self.tokenizer_repository(), &[self.tokenizer_file_name()])
            .map_err(Into::into)
    }

    pub fn fetch_model(&self) -> anyhow::Result<Vec<PathBuf>> {
        huggingface::fetch(self.repository(), self.file_names()).map_err(Into::into)
    }

    pub fn load_tokenizer(&self) -> anyhow::Result<Tokenizer> {
        fs::Options::new()
            .tokenizer(&self.fetch_tokenizer()?[0])
            .map_err(Into::into)
    }

    pub fn load_model(&self, device: &Device) -> anyhow::Result<VisionModel> {
        let paths = self.fetch_model()?;
        let config = blip::Config::image_captioning_large();

        match self {
            Self::BlipLarge => {
                let vars = vars_safetensors(paths, DType::F32, device)?;
                let model = blip::BlipForConditionalGeneration::new(&config, vars)?;

                Ok(VisionModel::Blip(model))
            }
            Self::BlipLargeQ4 => {
                let vars = vars_gguf(paths)?;
                let model = quantized_blip::BlipForConditionalGeneration::new(&config, vars)?;

                Ok(VisionModel::QuantizedBlip(model))
            }
        }
    }
}

impl VisionModel {
    pub fn image_to_embedding(&self, image: &Tensor, device: &Device) -> anyhow::Result<Tensor> {
        let input = image.to_device(device)?.unsqueeze(0)?;

        match self {
            Self::Blip(model) => Ok(input.apply(model.vision_model())?),
            Self::QuantizedBlip(model) => Ok(input.apply(model.vision_model())?),
        }
    }

    pub fn reset(&mut self) {
        match self {
            VisionModel::Blip(model) => model.reset_kv_cache(),
            VisionModel::QuantizedBlip(model) => model.reset_kv_cache(),
        }
    }

    pub fn text_decoder_forward(
        &mut self,
        input: &[u32],
        embedding: &Tensor,
        device: &Device,
    ) -> anyhow::Result<Tensor> {
        let input = Tensor::new(input, device)?.unsqueeze(0)?;
        let input = &input;
        let logits = match self {
            Self::Blip(model) => model.text_decoder().forward(input, embedding)?,
            Self::QuantizedBlip(model) => model.text_decoder().forward(input, embedding)?,
        };

        let logits = logits.squeeze(0)?;

        Ok(logits.get(logits.dim(0)? - 1)?)
    }
}

fn vars_gguf(paths: Vec<PathBuf>) -> anyhow::Result<quantized_var_builder::VarBuilder> {
    assert_eq!(paths.len(), 1);

    let path = &paths[0];
    let bytes = unsafe { memmap2::Mmap::map(&File::open(path)?)? };
    let vars = quantized_var_builder::VarBuilder::from_gguf_buffer(&bytes)?;

    Ok(vars)
}

fn vars_safetensors<'a>(
    paths: Vec<PathBuf>,
    dtype: DType,
    device: &Device,
) -> anyhow::Result<candle_nn::VarBuilder<'a>> {
    let vars = unsafe { candle_nn::VarBuilder::from_mmaped_safetensors(&paths, dtype, device)? };

    Ok(vars)
}
