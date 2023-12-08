use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("load model")]
    LoadModel,
}
