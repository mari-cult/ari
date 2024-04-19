use {
    crate::error,
    ffmpeg::sys,
    image::ImageResult,
    std::{mem::MaybeUninit, slice},
};

/// An FFmpeg buffer.
pub struct Buffer {
    ptr: *mut u8,
    capacity: usize,
}

impl Buffer {
    /// Try to allocate a new FFmpeg buffer.
    #[doc(alias = "av_malloc")]
    pub fn try_alloc(capacity: usize) -> ImageResult<Self> {
        let ptr = unsafe { sys::av_malloc(capacity).cast::<u8>() };

        if ptr.is_null() {
            return Err(error::out_of_memory());
        }

        Ok(Self { ptr, capacity })
    }

    /// Return an immutable pointer to the first byte within the buffer.
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.cast_const()
    }

    /// Return a mutable pointer to the first byte within the buffer.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    /// Return the capacity of this buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// View the buffer as a slice of `MaybeUninit<u8>`.
    pub fn slice_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe { slice::from_raw_parts_mut(self.ptr.cast(), self.capacity) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { sys::av_free(self.ptr.cast()) }
    }
}
