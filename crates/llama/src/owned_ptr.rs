use std::{ffi, ptr::NonNull};

/// A type-erased owned non-null pointer.
pub struct OwnedPtr {
    ptr: NonNull<ffi::c_void>,
    drop: unsafe extern "C" fn(ptr: *mut ffi::c_void),
}

impl OwnedPtr {
    /// Create an `OwnedPtr`.
    ///
    /// # Safety
    ///
    /// `ptr` must meet the same requirements as [`NonNull::new_unchecked`].
    #[inline(always)]
    pub unsafe fn new(
        ptr: *mut ffi::c_void,
        drop: unsafe extern "C" fn(ptr: *mut ffi::c_void),
    ) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr),
            drop,
        }
    }

    /// Returns a const pointer to the underlying data.
    #[inline(always)]
    pub fn as_ptr(&self) -> *const ffi::c_void {
        self.ptr.as_ptr().cast_const()
    }

    /// Returns a mutable pointer to the underlying data.
    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::c_void {
        self.ptr.as_ptr()
    }
}

impl Drop for OwnedPtr {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { (self.drop)(self.as_mut_ptr()) }
    }
}
