use {
    bytes::{Buf, BufMut, BytesMut},
    image::ImageError,
    std::{
        ffi, fmt,
        io::{self, Read},
        slice,
    },
};

/// An `io::Read` adapter for use within FFmpeg.
pub struct Reader {
    buffer: BytesMut,
    reader: Box<dyn Read + Send + 'static>,
    pub(crate) error: Option<ImageError>,
}

impl Reader {
    /// Create a new `Reader`.
    pub fn new<R: Read + Send + 'static>(reader: R) -> Box<Self> {
        Box::new(Self {
            buffer: BytesMut::with_capacity(4096),
            reader: Box::from(reader),
            error: None,
        })
    }

    /// Convert an opaque pointer to a mutable reference of this reader (for reading).
    pub unsafe fn opaque_as_mut<'a>(reader: *mut ffi::c_void) -> Option<&'a mut Self> {
        reader.cast::<Self>().as_mut()
    }

    /// Convert an opaque pointer back into a reader (for dropping).
    pub unsafe fn from_opaque(reader: *mut ffi::c_void) -> Box<Self> {
        Box::from_raw(reader.cast::<Reader>())
    }

    /// Convert this reader into an opaque pointer.
    pub fn into_opaque(self: Box<Self>) -> *mut ffi::c_void {
        Box::into_raw(self).cast()
    }

    /// Buffer some data read from the internal reader.
    pub fn read_buf(&mut self) -> io::Result<usize> {
        let len = {
            let chunk = self.buffer.chunk_mut();
            let ptr = chunk.as_mut_ptr();
            let len = chunk.len();
            let buf = unsafe { slice::from_raw_parts_mut(ptr, len) };

            self.reader.read(buf)?
        };

        unsafe {
            self.buffer.advance_mut(len);
        }

        Ok(len)
    }

    /// Read data into the specified buffer.
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = if !self.buffer.is_empty() {
            (&mut self.buffer).reader().read(buf)
        } else {
            self.reader.read(buf)
        };

        match &result {
            Ok(len) => tracing::debug!("read {len} bytes"),
            Err(error) => tracing::debug!("failed to read: {error}"),
        }

        result
    }

    /// Return a reference to the buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl fmt::Debug for Reader {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Reader")
            .field("buffer", &BufferDebug(&self.buffer))
            .field("reader", &"<dyn Read + 'static>")
            .field("error", &ErrorDebug(&self.error))
            .finish_non_exhaustive()
    }
}

struct BufferDebug<'a>(&'a BytesMut);
struct ErrorDebug<'a>(&'a Option<ImageError>);

impl fmt::Debug for BufferDebug<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = self.0.len();

        write!(fmt, "[{len} bytes]")
    }
}

impl fmt::Debug for ErrorDebug<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(error) = self.0.as_ref() {
            write!(fmt, "Some({error})")
        } else {
            write!(fmt, "None")
        }
    }
}
