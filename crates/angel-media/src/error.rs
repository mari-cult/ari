use {
    image::error::{DecodingError, ImageError, ImageFormatHint},
    std::io,
};

/// Out of memory error.
pub fn out_of_memory() -> ImageError {
    io::Error::from(io::ErrorKind::OutOfMemory).into()
}

/// Unknown or unsupported format error.
pub fn unknown_format() -> ImageError {
    ImageError::Decoding(DecodingError::new(
        ImageFormatHint::Name(String::from("stream")),
        "Unknown or unsupported format",
    ))
}

/// Convert an FFmpeg error into an `ImageError`.
pub fn ffmpeg_into_image(error: ffmpeg::Error) -> ImageError {
    let image_error = match error {
        ffmpeg::Error::Other { errno } => ImageError::IoError(io::Error::from_raw_os_error(errno)),
        // FIXME: Proper conversions.
        error => ImageError::IoError(io::Error::other(error)),
    };

    tracing::debug!("{error} -> {image_error}");

    image_error
}

/// Convert an `io::Error` to an FFmpeg error.
pub fn io_to_ffmpeg(error: &io::Error) -> ffmpeg::Error {
    let ffmpeg_error = error
        .raw_os_error()
        .map(|errno| ffmpeg::Error::Other { errno })
        .unwrap_or(ffmpeg::Error::Unknown);

    tracing::debug!("{error} -> {ffmpeg_error}");

    ffmpeg_error
}
