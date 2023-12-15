use candle_core::Device;

pub mod fs;
pub mod huggingface;
pub mod model;
pub mod settings;
pub mod text_generation;
pub mod tokenizer;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let settings = fs::Options::new().toml::<settings::Settings, _>("settings.toml")?;
    let model = settings.language.model;
    let tokenizer = model.load_tokenizer()?;
    let model = model.load_model(&Device::Cpu)?;
    let mut text_generation = text_generation::TextGeneration::new(model, tokenizer);
    let result = text_generation.generate("hi how are you?")?;

    println!("{result:?}");

    Ok(())
}
