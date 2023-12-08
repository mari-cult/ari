#![no_std]

use core::ffi;

extern "C" {
    pub fn bindings_clip_model_drop(model: *mut ffi::c_void);
    pub fn bindings_clip_model_open(
        path: *const ffi::c_char,
        verbosity: ffi::c_int,
    ) -> *mut ffi::c_void;
    pub fn bindings_init(numa_aware: bool);
    pub fn bindings_model_bos_token(model: *const ffi::c_void) -> i32;
    pub fn bindings_model_detokenize(
        model: *const ffi::c_void,
        token: i32,
        string_ptr: *mut ffi::c_char,
        string_len: u32,
    ) -> i32;
    pub fn bindings_model_drop(model: *mut ffi::c_void);
    pub fn bindings_model_eos_token(model: *const ffi::c_void) -> i32;
    pub fn bindings_model_eot_token(model: *const ffi::c_void) -> i32;
    pub fn bindings_model_middle_token(model: *const ffi::c_void) -> i32;
    pub fn bindings_model_nl_token(model: *const ffi::c_void) -> i32;
    pub fn bindings_model_open(
        path: *const ffi::c_char,
        options: *const ffi::c_void,
    ) -> *mut ffi::c_void;
    pub fn bindings_model_options_drop(options: *mut ffi::c_void);
    pub fn bindings_model_options_gpu_layers(options: *const ffi::c_void) -> u16;
    pub fn bindings_model_options_new() -> *mut ffi::c_void;
    pub fn bindings_model_options_set_gpu_layers(options: *mut ffi::c_void, value: u16);
    pub fn bindings_model_options_set_use_mlock(options: *mut ffi::c_void, value: bool);
    pub fn bindings_model_options_set_use_mmap(options: *mut ffi::c_void, value: bool);
    pub fn bindings_model_options_use_mlock(options: *const ffi::c_void) -> bool;
    pub fn bindings_model_options_use_mmap(options: *const ffi::c_void) -> bool;
    pub fn bindings_model_prefix_token(model: *const ffi::c_void) -> i32;
    pub fn bindings_model_requires_bos_token(model: *const ffi::c_void) -> ffi::c_int;
    pub fn bindings_model_requires_eos_token(model: *const ffi::c_void) -> ffi::c_int;
    pub fn bindings_model_suffix_token(model: *const ffi::c_void) -> i32;
    pub fn bindings_model_tokenize(
        model: *const ffi::c_void,
        string_ptr: *const ffi::c_char,
        string_len: u32,
        tokens: *const i32,
        tokens_capacity: u32,
        add_bos: bool,
        special: bool,
    ) -> i32;
    pub fn bindings_session_batch_drop(batch: *mut ffi::c_void);
    pub fn bindings_session_batch_embedding_mut_ptr(batch: *mut ffi::c_void) -> *mut f32;
    pub fn bindings_session_batch_embedding_ptr(batch: *const ffi::c_void) -> *const f32;
    pub fn bindings_session_batch_init(
        token_capacity: u32,
        embedding_size: u32,
        max_sequence_ids: u32,
    ) -> *mut ffi::c_void;
    pub fn bindings_session_batch_clear(batch: *mut ffi::c_void);
    pub fn bindings_session_batch_add_token(
        batch: *mut ffi::c_void,
        token: i32,
        index: u32,
        logits: bool,
    );
    pub fn bindings_session_batch_logits_mut_ptr(batch: *mut ffi::c_void) -> *mut i8;
    pub fn bindings_session_batch_logits_ptr(batch: *const ffi::c_void) -> *const i8;
    pub fn bindings_session_batch_pos_mut_ptr(batch: *mut ffi::c_void) -> *mut i32;
    pub fn bindings_session_batch_pos_ptr(batch: *const ffi::c_void) -> *const i32;
    pub fn bindings_session_batch_sequence_id_len_mut_ptr(batch: *mut ffi::c_void) -> *mut i32;
    pub fn bindings_session_batch_sequence_id_len_ptr(batch: *const ffi::c_void) -> *const i32;
    pub fn bindings_session_batch_sequence_id_mut_ptr(batch: *mut ffi::c_void) -> *mut *mut i32;
    pub fn bindings_session_batch_sequence_id_ptr(batch: *const ffi::c_void) -> *const *mut i32;
    pub fn bindings_session_batch_tokens_len(batch: *const ffi::c_void) -> u32;
    pub fn bindings_session_batch_tokens_set_len(batch: *mut ffi::c_void, value: u32);
    pub fn bindings_session_batch_tokens_mut_ptr(batch: *mut ffi::c_void) -> *mut i32;
    pub fn bindings_session_batch_tokens_ptr(batch: *const ffi::c_void) -> *const i32;
    pub fn bindings_session_decode(
        session: *mut ffi::c_void,
        batch: *mut ffi::c_void,
    ) -> ffi::c_int;
    pub fn bindings_session_drop(session: *mut ffi::c_void);
    pub fn bindings_session_new(
        model: *mut ffi::c_void,
        options: *const ffi::c_void,
    ) -> *mut ffi::c_void;
    pub fn bindings_session_options_drop(options: *mut ffi::c_void);
    pub fn bindings_session_options_context_len(options: *const ffi::c_void) -> u32;
    pub fn bindings_session_options_set_context_len(options: *mut ffi::c_void, value: u32);
    pub fn bindings_session_options_new() -> *mut ffi::c_void;
    pub fn bindings_session_sampler_drop(sampler: *mut ffi::c_void);
    pub fn bindings_session_sampler_new(options: *mut ffi::c_void) -> *mut ffi::c_void;
    pub fn bindings_session_sampler_options_drop(options: *mut ffi::c_void);
    pub fn bindings_session_sampler_options_new() -> *mut ffi::c_void;
    pub fn bindings_session_sampler_options_set_temperature(options: *mut ffi::c_void, value: f32);
    pub fn bindings_session_sampler_options_set_top_k(options: *mut ffi::c_void, value: f32);
    pub fn bindings_session_sampler_options_set_top_p(options: *mut ffi::c_void, value: f32);
    pub fn bindings_session_sampler_options_temperature(options: *const ffi::c_void) -> f32;
    pub fn bindings_session_sampler_options_top_k(options: *const ffi::c_void) -> f32;
    pub fn bindings_session_sampler_options_top_p(options: *const ffi::c_void) -> f32;
    pub fn bindings_session_sampler_reset(sampler: *mut ffi::c_void);
    pub fn bindings_session_sampler_sample(
        sampler: *mut ffi::c_void,
        session: *mut ffi::c_void,
    ) -> i32;
    pub fn bindings_session_sampler_accept(
        sampler: *mut ffi::c_void,
        session: *mut ffi::c_void,
        token: i32,
    );
}
