use {
    hf_hub::api::sync::Api as HuggingFace,
    std::{path::PathBuf, sync::OnceLock},
};

pub use hf_hub::api::sync::ApiError as HuggingFaceError;

static HUGGING_FACE: OnceLock<HuggingFace> = OnceLock::new();

fn hugging_face() -> &'static HuggingFace {
    HUGGING_FACE.get_or_init(|| HuggingFace::new().unwrap())
}

pub fn fetch(repository: &str, file_names: &[&str]) -> Result<Vec<PathBuf>, HuggingFaceError> {
    let mut paths = Vec::new();

    for file_name in file_names {
        tracing::info!("fetch {file_name} from {repository}");

        let path = hugging_face()
            .model(String::from(repository))
            .get(file_name)?;

        tracing::info!("done, file is located at {}", path.display());

        paths.push(path);
    }

    Ok(paths)
}
