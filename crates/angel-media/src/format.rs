use {
    crate::{error, Buffer},
    ffmpeg::{format::Input as Inner, sys},
    image::error::ImageResult,
    std::{ffi, fmt, mem::ManuallyDrop, ptr, sync::Arc},
};

/// The format of media.
#[derive(Clone)]
pub struct MediaFormat {
    pub(crate) inner: Arc<Inner>,
}

impl MediaFormat {
    /// Returns the name of the format.
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Returns a descrption of the format.
    pub fn description(&self) -> &str {
        self.inner.description()
    }

    /// Returns the file extensions for this format.
    pub fn extensions(&self) -> Vec<&str> {
        self.inner.extensions()
    }

    /// Returns the MIME types for this format.
    pub fn mime_types(&self) -> Vec<&str> {
        self.inner.mime_types()
    }

    /// Guess the format of the provided byte buffer.
    #[doc(alias = "av_probe_input_format")]
    pub fn guess(bytes: &[u8]) -> ImageResult<Self> {
        unsafe { guess_format(bytes) }
    }
}

impl fmt::Debug for MediaFormat {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("MediaFormat")
            .field("name", &self.name())
            .field("description", &self.description())
            .field("extensions", &self.extensions())
            .field("mime_types", &self.mime_types())
            .finish_non_exhaustive()
    }
}

/// Attempt to guess the format of the provided byte slice.
#[doc(alias = "av_probe_input_format")]
pub unsafe fn guess_format(bytes: &[u8]) -> ImageResult<MediaFormat> {
    let len = bytes.len().min(4096);
    let mut buf = Buffer::try_alloc(len + sys::AVPROBE_PADDING_SIZE as usize)?;

    ptr::copy_nonoverlapping(bytes.as_ptr(), buf.as_mut_ptr(), len);

    let mut probe_data = sys::AVProbeData {
        filename: c"stream".as_ptr(),
        buf: buf.as_mut_ptr(),
        buf_size: len as ffi::c_int,
        mime_type: ptr::null_mut(),
    };

    let mut format = sys::av_probe_input_format(&mut probe_data, 1);

    if format.is_null() {
        format = sys::av_probe_input_format(&mut probe_data, 0);
    }

    if format.is_null() {
        return Err(error::unknown_format());
    }

    // `MediaFormat` will take care of dropping the buffer now.
    let _buf = ManuallyDrop::new(buf);

    Ok(MediaFormat {
        inner: Arc::new(Inner::wrap(format.cast_mut())),
    })
}
