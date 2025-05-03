use {
    crate::{owned_ptr::OwnedPtr, sys, Model},
    std::{any, fmt, slice},
};

/// An inference session.
pub struct Session {
    pub(crate) session_ptr: OwnedPtr,
    pub(crate) sampler_ptr: OwnedPtr,
    model: Model,
}

/// Options and flags which can be used to configure how a session is created.
pub struct SessionOptions {
    pub(crate) options_ptr: OwnedPtr,
    pub(crate) sampler_options_ptr: OwnedPtr,
}

pub struct SessionBatch {
    pub(crate) batch_ptr: OwnedPtr,
    pub(crate) index: usize,
}

impl Session {
    pub fn options() -> SessionOptions {
        SessionOptions::new()
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn into_model(self) -> Model {
        self.model
    }

    pub fn decode(&mut self, batch: &mut SessionBatch) {
        unsafe {
            sys::bindings_session_decode(
                self.session_ptr.as_mut_ptr(),
                batch.batch_ptr.as_mut_ptr(),
            );
        }
    }

    pub fn sample(&mut self) -> i32 {
        unsafe {
            sys::bindings_session_sampler_sample(
                self.sampler_ptr.as_mut_ptr(),
                self.session_ptr.as_mut_ptr(),
            )
        }
    }

    pub fn accept(&mut self, token: i32) {
        unsafe {
            sys::bindings_session_sampler_accept(
                self.sampler_ptr.as_mut_ptr(),
                self.session_ptr.as_mut_ptr(),
                token,
            );
        }
    }

    pub fn reset(&mut self) {
        unsafe {
            sys::bindings_session_sampler_reset(self.sampler_ptr.as_mut_ptr());
        }
    }
}

impl SessionBatch {
    pub fn new(token_capacity: u32, max_sequence_ids: u32) -> Self {
        unsafe {
            Self {
                batch_ptr: OwnedPtr::new(
                    sys::bindings_session_batch_init(token_capacity, 0, max_sequence_ids),
                    sys::bindings_session_batch_drop,
                ),
                index: 0,
            }
        }
    }

    pub fn clear(&mut self) {
        tracing::debug!("clear tokens");

        unsafe {
            sys::bindings_session_batch_clear(self.batch_ptr.as_mut_ptr());
        }
    }

    pub fn len(&self) -> usize {
        unsafe { sys::bindings_session_batch_tokens_len(self.batch_ptr.as_ptr()) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, token: i32, logit: bool) {
        tracing::debug!("push {token} (index={}, len={})", self.index, self.len());

        unsafe {
            sys::bindings_session_batch_add_token(
                self.batch_ptr.as_mut_ptr(),
                token,
                self.index as u32,
                logit,
            );

            self.index += 1;
        }
    }

    pub fn extend<I: IntoIterator<Item = i32>>(&mut self, tokens: I, logit: bool) {
        for token in tokens.into_iter() {
            self.push(token, logit);
        }
    }

    pub fn tokens(&self) -> &[i32] {
        unsafe {
            slice::from_raw_parts(
                sys::bindings_session_batch_tokens_ptr(self.batch_ptr.as_ptr()),
                self.len(),
            )
        }
    }

    pub fn tokens_mut(&mut self) -> &mut [i32] {
        unsafe {
            slice::from_raw_parts_mut(
                sys::bindings_session_batch_tokens_mut_ptr(self.batch_ptr.as_mut_ptr()),
                self.len(),
            )
        }
    }

    pub fn logits(&self) -> &[bool] {
        unsafe {
            slice::from_raw_parts(
                sys::bindings_session_batch_logits_ptr(self.batch_ptr.as_ptr()).cast(),
                self.len(),
            )
        }
    }

    pub fn logits_mut(&mut self) -> &mut [bool] {
        unsafe {
            slice::from_raw_parts_mut(
                sys::bindings_session_batch_logits_mut_ptr(self.batch_ptr.as_mut_ptr()).cast(),
                self.len(),
            )
        }
    }
}

impl SessionOptions {
    /// Creates a new set of session options ready for configuration.
    pub fn new() -> Self {
        unsafe {
            Self {
                options_ptr: OwnedPtr::new(
                    sys::bindings_session_options_new(),
                    sys::bindings_session_options_drop,
                ),
                sampler_options_ptr: OwnedPtr::new(
                    sys::bindings_session_sampler_options_new(),
                    sys::bindings_session_sampler_options_drop,
                ),
            }
        }
    }

    pub fn context_len(&self) -> u32 {
        unsafe { sys::bindings_session_options_context_len(self.options_ptr.as_ptr()) }
    }

    pub fn set_context_len(mut self, value: u32) -> Self {
        unsafe {
            sys::bindings_session_options_set_context_len(self.options_ptr.as_mut_ptr(), value);
        }

        self
    }

    pub fn temperature(&self) -> f32 {
        unsafe {
            sys::bindings_session_sampler_options_temperature(self.sampler_options_ptr.as_ptr())
        }
    }

    pub fn set_temperature(mut self, temperature: f32) -> Self {
        unsafe {
            sys::bindings_session_sampler_options_set_temperature(
                self.sampler_options_ptr.as_mut_ptr(),
                temperature,
            );
        }

        self
    }

    pub fn top_k(&self) -> f32 {
        unsafe { sys::bindings_session_sampler_options_top_k(self.sampler_options_ptr.as_ptr()) }
    }

    pub fn set_top_k(mut self, top_k: f32) -> Self {
        unsafe {
            sys::bindings_session_sampler_options_set_top_k(
                self.sampler_options_ptr.as_mut_ptr(),
                top_k,
            );
        }

        self
    }

    pub fn top_p(&self) -> f32 {
        unsafe { sys::bindings_session_sampler_options_top_p(self.sampler_options_ptr.as_ptr()) }
    }

    pub fn set_top_p(mut self, top_p: f32) -> Self {
        unsafe {
            sys::bindings_session_sampler_options_set_top_p(
                self.sampler_options_ptr.as_mut_ptr(),
                top_p,
            );
        }

        self
    }

    /// Creates a session with the specified model.
    pub fn with_model(mut self, mut model: Model) -> Session {
        unsafe {
            Session {
                session_ptr: OwnedPtr::new(
                    sys::bindings_session_new(
                        model.model_ptr.as_mut_ptr(),
                        self.options_ptr.as_ptr(),
                    ),
                    sys::bindings_session_drop,
                ),
                sampler_ptr: OwnedPtr::new(
                    sys::bindings_session_sampler_new(self.sampler_options_ptr.as_mut_ptr()),
                    sys::bindings_session_sampler_drop,
                ),
                model,
            }
        }
    }
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct(any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for SessionBatch {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct(any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for SessionOptions {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct(any::type_name::<Self>())
            .field("context_len", &self.context_len())
            .field("temperature", &self.temperature())
            .field("top_k", &self.top_k())
            .field("top_p", &self.top_p())
            .finish_non_exhaustive()
    }
}
