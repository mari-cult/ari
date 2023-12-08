use {
    crate::{owned_ptr::OwnedPtr, sys, Error},
    std::{any, ffi::CString, fmt, path::Path},
};

pub struct ClipModel {
    pub(crate) clip_model_ptr: OwnedPtr,
}

impl ClipModel {
    pub fn open<P: AsRef<Path>>(path: P, verbosity: u8) -> Result<Self, Error> {
        fn inner(path: &Path, verbosity: u8) -> Result<ClipModel, Error> {
            let bytes = path.as_os_str().as_encoded_bytes();
            let cstr = CString::new(bytes).map_err(|_error| Error::LoadModel)?;

            unsafe {
                let ptr =
                    sys::bindings_clip_model_open(cstr.as_ptr(), verbosity.try_into().unwrap());

                if ptr.is_null() {
                    Err(Error::LoadModel)
                } else {
                    Ok(ClipModel {
                        clip_model_ptr: OwnedPtr::new(ptr, sys::bindings_clip_model_drop),
                    })
                }
            }
        }

        inner(path.as_ref(), verbosity)
    }
}

impl fmt::Debug for ClipModel {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct(any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
