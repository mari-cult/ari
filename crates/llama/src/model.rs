use {
    crate::{owned_ptr::OwnedPtr, sys, Error},
    std::{
        any,
        ffi::{self, CString},
        fmt,
        mem::MaybeUninit,
        path::Path,
    },
};

pub struct ModelOptions {
    pub(crate) options_ptr: OwnedPtr,
    pub(crate) verbosity: u8,
}

pub struct Model {
    pub(crate) model_ptr: OwnedPtr,
}

impl Model {
    pub fn bos_token(&self) -> i32 {
        unsafe { sys::bindings_model_bos_token(self.model_ptr.as_ptr()) }
    }

    pub fn eos_token(&self) -> i32 {
        unsafe { sys::bindings_model_eos_token(self.model_ptr.as_ptr()) }
    }

    pub fn nl_token(&self) -> i32 {
        unsafe { sys::bindings_model_nl_token(self.model_ptr.as_ptr()) }
    }

    pub fn requires_bos_token(&self) -> Option<bool> {
        unsafe {
            int_to_requirement(sys::bindings_model_requires_bos_token(
                self.model_ptr.as_ptr(),
            ))
        }
    }

    pub fn requires_eos_token(&self) -> Option<bool> {
        unsafe {
            int_to_requirement(sys::bindings_model_requires_eos_token(
                self.model_ptr.as_ptr(),
            ))
        }
    }

    pub fn prefix_token(&self) -> i32 {
        unsafe { sys::bindings_model_prefix_token(self.model_ptr.as_ptr()) }
    }

    pub fn middle_token(&self) -> i32 {
        unsafe { sys::bindings_model_middle_token(self.model_ptr.as_ptr()) }
    }

    pub fn suffix_token(&self) -> i32 {
        unsafe { sys::bindings_model_suffix_token(self.model_ptr.as_ptr()) }
    }

    pub fn eot_token(&self) -> i32 {
        unsafe { sys::bindings_model_eot_token(self.model_ptr.as_ptr()) }
    }

    unsafe fn tokenize_internal(
        &self,
        string: &str,
        tokens: &mut [MaybeUninit<i32>],
        add_bos: bool,
        special: bool,
    ) -> Result<usize, usize> {
        let len = sys::bindings_model_tokenize(
            self.model_ptr.as_ptr(),
            string.as_ptr().cast(),
            string.len().try_into().unwrap(),
            tokens.as_mut_ptr().cast(),
            tokens.len().try_into().unwrap(),
            add_bos,
            special,
        );

        let is_ok = len > -1;
        let len = len.unsigned_abs().try_into().unwrap();

        if is_ok {
            Ok(len)
        } else {
            Err(len)
        }
    }

    pub fn tokenize(&self, string: &str, tokens: &mut Vec<i32>, add_bos: bool, special: bool) {
        tokens.clear();

        unsafe {
            match self.tokenize_internal(string, tokens.spare_capacity_mut(), add_bos, special) {
                Ok(len) => tokens.set_len(len),
                Err(len) => {
                    tokens.reserve(len - tokens.len());

                    self.tokenize_internal(string, tokens.spare_capacity_mut(), add_bos, special)
                        .unwrap();

                    tokens.set_len(len);
                }
            }
        }
    }

    pub unsafe fn detokenize_internal(
        &self,
        token: i32,
        bytes: &mut [MaybeUninit<u8>],
    ) -> Result<usize, usize> {
        let len = sys::bindings_model_detokenize(
            self.model_ptr.as_ptr(),
            token,
            bytes.as_mut_ptr().cast(),
            bytes.len().try_into().unwrap(),
        );

        let is_ok = len > -1;
        let len = len.unsigned_abs().try_into().unwrap();

        if is_ok {
            Ok(len)
        } else {
            Err(len)
        }
    }

    pub fn detokenize(&self, tokens: &[i32], string: &mut String) {
        string.clear();

        unsafe {
            let bytes = string.as_mut_vec();

            for token in tokens.iter().copied() {
                match self.detokenize_internal(token, bytes.spare_capacity_mut()) {
                    Ok(len) => bytes.set_len(bytes.len() + len),
                    Err(len) => {
                        bytes.reserve(len);

                        self.detokenize_internal(token, bytes.spare_capacity_mut())
                            .unwrap();

                        bytes.set_len(bytes.len() + len);
                    }
                }
            }
        }
    }
}

impl ModelOptions {
    pub fn new() -> Self {
        unsafe {
            Self {
                options_ptr: OwnedPtr::new(
                    sys::bindings_model_options_new(),
                    sys::bindings_model_options_drop,
                ),
                verbosity: 1,
            }
        }
    }

    pub fn gpu_layers(&self) -> u16 {
        unsafe { sys::bindings_model_options_gpu_layers(self.options_ptr.as_ptr()) }
    }

    pub fn set_gpu_layers(&mut self, layers: u16) -> &mut Self {
        unsafe { sys::bindings_model_options_set_gpu_layers(self.options_ptr.as_mut_ptr(), layers) }

        self
    }

    pub fn use_mlock(&self) -> bool {
        unsafe { sys::bindings_model_options_use_mlock(self.options_ptr.as_ptr()) }
    }

    pub fn set_use_mlock(&mut self, mlock: bool) -> &mut Self {
        unsafe { sys::bindings_model_options_set_use_mlock(self.options_ptr.as_mut_ptr(), mlock) }

        self
    }

    pub fn use_mmap(&self) -> bool {
        unsafe { sys::bindings_model_options_use_mmap(self.options_ptr.as_ptr()) }
    }

    pub fn set_use_mmap(&mut self, mmap: bool) -> &mut Self {
        unsafe { sys::bindings_model_options_set_use_mlock(self.options_ptr.as_mut_ptr(), mmap) }

        self
    }

    pub fn open<P: AsRef<Path>>(&self, path: P) -> Result<Model, Error> {
        fn inner(options: &ModelOptions, path: &Path) -> Result<Model, Error> {
            let bytes = path.as_os_str().as_encoded_bytes();
            let cstr = CString::new(bytes).map_err(|_error| Error::LoadModel)?;

            unsafe {
                let ptr = sys::bindings_model_open(cstr.as_ptr(), options.options_ptr.as_ptr());

                if ptr.is_null() {
                    Err(Error::LoadModel)
                } else {
                    Ok(Model {
                        model_ptr: OwnedPtr::new(ptr, sys::bindings_model_drop),
                    })
                }
            }
        }

        inner(self, path.as_ref())
    }
}

impl Default for ModelOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ModelOptions {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct(any::type_name::<Self>())
            .field("gpu_layers", &self.gpu_layers())
            .field("use_mlock", &self.use_mlock())
            .field("use_mmap", &self.use_mmap())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for Model {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct(any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

fn int_to_requirement(token: ffi::c_int) -> Option<bool> {
    match token {
        1 => Some(true),
        0 => Some(false),
        _ => None,
    }
}
