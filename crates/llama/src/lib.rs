pub use {
    clip_model::ClipModel,
    error::Error,
    llama_sys as sys,
    model::{Model, ModelOptions},
    session::{Session, SessionOptions},
};

mod clip_model;
mod error;
mod model;
mod owned_ptr;
mod session;

pub fn init(numa_aware: bool) {
    unsafe {
        sys::bindings_init(numa_aware);
    }
}
