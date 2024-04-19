use {
    crate::{error, Buffer, MediaFormat, MediaSource, Reader},
    ffmpeg::{format::context::Input as Inner, sys, Error::Eof},
    image::ImageResult,
    std::{
        ffi, fmt,
        mem::ManuallyDrop,
        ptr, slice,
        sync::{Arc, Mutex},
    },
};

/// An FFmpeg `AVFormatContext`.
pub struct Context {
    inner: Inner,
}

impl Context {
    /// Try to allocate a new FFmpeg `AVFormatContext`.
    #[doc(alias = "avformat_alloc_context")]
    pub fn new() -> ImageResult<Self> {
        unsafe { new_context() }
    }

    /// Try to set the AVIO context to use the provided reader.
    #[doc(alias = "avio_alloc_context")]
    pub fn set_reader(&mut self, reader: Box<Reader>) -> ImageResult<()> {
        unsafe { set_reader(self, reader) }
    }

    /// Try to decode the provided context, and return a media source.
    #[doc(alias = "avformat_open_input")]
    pub fn decode(self, format: MediaFormat) -> ImageResult<MediaSource> {
        unsafe { decode(self, format) }
    }

    /// Returns an immutable reference to the AVIO context.
    unsafe fn avio(&self) -> &*mut sys::AVIOContext {
        &(*self.inner.as_ptr()).pb
    }

    /// Returns a mutable reference to the AVIO context.
    unsafe fn avio_mut(&mut self) -> &mut *mut sys::AVIOContext {
        &mut (*self.inner.as_mut_ptr()).pb
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Context")
            .field("avio", unsafe { self.avio() })
            .finish_non_exhaustive()
    }
}

/// Try to allocate a new FFmpeg `AVFormatContext`.
#[doc(alias = "avformat_alloc_context")]
pub unsafe fn new_context() -> ImageResult<Context> {
    let ptr = sys::avformat_alloc_context();

    if ptr.is_null() {
        return Err(error::out_of_memory());
    }

    Ok(Context {
        inner: Inner::wrap(ptr),
    })
}

/// Try to set the AVIO context to use the provided reader.
#[doc(alias = "avio_alloc_context")]
pub unsafe fn set_reader(context: &mut Context, reader: Box<Reader>) -> ImageResult<()> {
    let mut buf = Buffer::try_alloc(4096)?;
    let io = sys::avio_alloc_context(
        buf.as_mut_ptr(),
        buf.capacity() as ffi::c_int,
        0,
        Reader::into_opaque(reader),
        Some(read),
        None,
        None,
    );

    if io.is_null() {
        return Err(error::out_of_memory());
    }

    // `Context` will take care of dropping the buffer now.
    let _buf = ManuallyDrop::new(buf);

    unsafe {
        *context.avio_mut() = io;
    }

    Ok(())
}

/// AVIO `read_packet` function.
#[doc(alias = "read_packet")]
pub unsafe extern "C" fn read(
    state: *mut ffi::c_void,
    buf: *mut u8,
    len: ffi::c_int,
) -> ffi::c_int {
    read_inner(state, buf, len).unwrap_or_else(Into::into)
}

/// AVIO `read_packet` function - internal implementation.
pub unsafe fn read_inner(
    opaque: *mut ffi::c_void,
    buf: *mut u8,
    len: ffi::c_int,
) -> Result<ffi::c_int, ffmpeg::Error> {
    let Some(reader) = Reader::opaque_as_mut(opaque) else {
        return Err(Eof);
    };

    let Some(buf) = slice_mut(buf, len) else {
        return Err(Eof);
    };

    match reader.read(buf) {
        Ok(0) => Err(Eof),
        Ok(bytes) => i32::try_from(bytes).map_err(|_error| Eof),
        Err(error) => {
            let ffmpeg_error = error::io_to_ffmpeg(&error);

            reader.error = Some(error.into());

            Err(ffmpeg_error)
        }
    }
}

/// Convert a pointer, and int buffer provided by FFmpeg to a mutable byte slice that is not empty.
pub unsafe fn slice_mut<'a>(buf: *mut u8, len: ffi::c_int) -> Option<&'a mut [u8]> {
    if buf.is_null() {
        return None;
    }

    if len <= 0 {
        return None;
    }

    Some(slice::from_raw_parts_mut(buf, len as usize))
}

/// Try to decode the provided context, and return a media source.
#[doc(alias = "avformat_open_input")]
pub unsafe fn decode(mut context: Context, format: MediaFormat) -> ImageResult<MediaSource> {
    let result = sys::avformat_open_input(
        &mut context.inner.as_mut_ptr(),
        ptr::null(),
        format.inner.as_ptr(),
        ptr::null_mut(),
    );

    if result < 0 {
        //Reader::from_opaque((*context.inner.as_mut_ptr()).opaque);
        //(*context.inner.as_mut_ptr()).opaque = ptr::null_mut();

        Err(error::ffmpeg_into_image(result.into()))
    } else {
        Ok(MediaSource {
            inner: Arc::new(Mutex::new(context.inner)),
        })
    }
}
