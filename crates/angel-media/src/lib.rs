#![deny(clippy::print_stderr)]
#![deny(clippy::print_stdout)]

pub use {
    self::{
        buffer::Buffer,
        context::Context,
        format::MediaFormat,
        reader::Reader,
        source::{MediaSource, VideoSource},
    },
    ffmpeg,
};

pub mod buffer;
pub mod context;
pub mod error;
pub mod format;
pub mod reader;
pub mod source;
