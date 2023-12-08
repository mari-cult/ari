#include <clip.h>
#include <ggml-opencl.h>
#include <llama.h>
#include <sampling.h>

/// Library
extern "C" void bindings_init(bool numa_aware) {
  llama_backend_init(numa_aware);
}

// CLIP model
extern "C" void *bindings_clip_model_open(const char *path,
                                          const int verbosity) {
  return static_cast<void *>(clip_model_load(path, verbosity));
}

extern "C" void bindings_clip_model_drop(void *model) {
  clip_free(static_cast<clip_ctx *>(model));
}

// CLIP image
extern "C" bool bindings_clip_image_encode(const void *model, const int threads,
                                           void *image, float *buf) {
  return clip_image_encode(static_cast<const clip_ctx *>(model), threads,
                           static_cast<clip_image_f32 *>(image), buf);
}

extern "C" bool bindings_clip_image_batch_encode(const void *model,
                                                 const int threads,
                                                 void *images, float *buf) {
  return clip_image_batch_encode(static_cast<const clip_ctx *>(model), threads,
                                 static_cast<clip_image_f32_batch *>(images),
                                 buf);
}

// Model
extern "C" void *bindings_model_open(const char *path, const void *options) {
  return static_cast<void *>(llama_load_model_from_file(
      path, *static_cast<const llama_model_params *>(options)));
}

extern "C" int32_t bindings_model_bos_token(const void *model) {
  return llama_token_bos(static_cast<const llama_model *>(model));
}

extern "C" int32_t bindings_model_eos_token(const void *model) {
  return llama_token_eos(static_cast<const llama_model *>(model));
}

extern "C" int32_t bindings_model_nl_token(const void *model) {
  return llama_token_nl(static_cast<const llama_model *>(model));
}

extern "C" int bindings_model_requires_bos_token(const void *model) {
  return llama_add_bos_token(static_cast<const llama_model *>(model));
}

extern "C" int bindings_model_requires_eos_token(const void *model) {
  return llama_add_eos_token(static_cast<const llama_model *>(model));
}

extern "C" int32_t bindings_model_prefix_token(const void *model) {
  return llama_token_prefix(static_cast<const llama_model *>(model));
}

extern "C" int32_t bindings_model_middle_token(const void *model) {
  return llama_token_middle(static_cast<const llama_model *>(model));
}

extern "C" int32_t bindings_model_suffix_token(const void *model) {
  return llama_token_suffix(static_cast<const llama_model *>(model));
}

extern "C" int32_t bindings_model_eot_token(const void *model) {
  return llama_token_eot(static_cast<const llama_model *>(model));
}

extern "C" int bindings_model_tokenize(const void *model,
                                       const char *string_ptr,
                                       const uint32_t string_len,
                                       int32_t *tokens,
                                       const uint32_t tokens_capacity,
                                       const bool add_bos, const bool special) {
  return llama_tokenize(static_cast<const llama_model *>(model), string_ptr,
                        static_cast<int>(string_len), tokens,
                        static_cast<int>(tokens_capacity), add_bos, special);
}

extern "C" int bindings_model_detokenize(const void *model, int32_t token,
                                         char *string_ptr,
                                         uint32_t string_len) {
  return llama_token_to_piece(static_cast<const llama_model *>(model), token,
                              string_ptr, static_cast<int>(string_len));
}

extern "C" void bindings_model_drop(void *model) {
  llama_free_model(static_cast<llama_model *>(model));
}

// Model options
extern "C" void *bindings_model_options_new() {
  return static_cast<void *>(
      new llama_model_params(llama_model_default_params()));
}

extern "C" uint16_t bindings_model_options_gpu_layers(const void *options) {
  return static_cast<uint16_t>(
      static_cast<const llama_model_params *>(options)->n_gpu_layers);
}

extern "C" void bindings_model_options_set_gpu_layers(void *options,
                                                      const uint16_t value) {
  static_cast<llama_model_params *>(options)->n_gpu_layers =
      static_cast<int32_t>(value);
}

extern "C" bool bindings_model_options_use_mlock(const void *options) {
  return static_cast<const llama_model_params *>(options)->use_mlock;
}

extern "C" void bindings_model_options_set_use_mlock(void *options,
                                                     const bool value) {
  static_cast<llama_model_params *>(options)->use_mlock = value;
}

extern "C" bool bindings_model_options_use_mmap(const void *options) {
  return static_cast<const llama_model_params *>(options)->use_mmap;
}

extern "C" void bindings_model_options_set_use_mmap(void *options,
                                                    const bool value) {
  static_cast<llama_model_params *>(options)->use_mmap = value;
}

extern "C" void bindings_model_options_drop(void *options) {
  delete static_cast<llama_model_params *>(options);
}

// Session
extern "C" void *bindings_session_new(void *model, const void *options) {
  return static_cast<void *>(llama_new_context_with_model(
      static_cast<llama_model *>(model),
      *static_cast<const llama_context_params *>(options)));
}

extern "C" int bindings_session_decode(void *session, void *batch) {
  return llama_decode(static_cast<llama_context *>(session),
                      *static_cast<llama_batch *>(batch));
}

extern "C" void bindings_session_drop(void *session) {
  llama_free(static_cast<llama_context *>(session));
}

// Session options
extern "C" void *bindings_session_options_new() {
  return static_cast<void *>(
      new llama_context_params(llama_context_default_params()));
}

extern "C" uint32_t bindings_session_options_context_len(const void *options) {
  return static_cast<const llama_context_params *>(options)->n_ctx;
}

extern "C" void bindings_session_options_set_context_len(void *options,
                                                         uint32_t value) {
  static_cast<llama_context_params *>(options)->n_ctx = value;
}

extern "C" void bindings_session_options_drop(void *options) {
  delete static_cast<llama_context_params *>(options);
}

// Session sampling
extern "C" void *bindings_session_sampler_new(const void *options) {
  return static_cast<void *>(llama_sampling_init(
      *static_cast<const llama_sampling_params *>(options)));
}

extern "C" void bindings_session_sampler_reset(void *sampler) {
  llama_sampling_reset(static_cast<llama_sampling_context *>(sampler));
}

extern "C" int32_t bindings_session_sampler_sample(void *sampler,
                                                   void *session) {
  return llama_sampling_sample(static_cast<llama_sampling_context *>(sampler),
                               static_cast<llama_context *>(session), nullptr,
                               0);
}

extern "C" void bindings_session_sampler_accept(void *sampler, void *session,
                                                int32_t token) {
  llama_sampling_accept(

      static_cast<llama_sampling_context *>(sampler),
      static_cast<llama_context *>(session), token, false);
}

extern "C" void bindings_session_sampler_drop(void *sampler) {
  llama_sampling_free(static_cast<llama_sampling_context *>(sampler));
}

// Session sampling options
extern "C" void *bindings_session_sampler_options_new() {
  return static_cast<void *>(new llama_sampling_params);
}

extern "C" float
bindings_session_sampler_options_temperature(const void *options) {
  return static_cast<const llama_sampling_params *>(options)->temp;
}

extern "C" void
bindings_session_sampler_options_set_temperature(void *options,
                                                 const float value) {
  static_cast<llama_sampling_params *>(options)->temp = value;
}

extern "C" float bindings_session_sampler_options_top_k(const void *options) {
  return static_cast<const llama_sampling_params *>(options)->top_k;
}

extern "C" void bindings_session_sampler_options_set_top_k(void *options,
                                                           const float value) {
  static_cast<llama_sampling_params *>(options)->top_k = value;
}

extern "C" float bindings_session_sampler_options_top_p(const void *options) {
  return static_cast<const llama_sampling_params *>(options)->top_p;
}

extern "C" void bindings_session_sampler_options_set_top_p(void *options,
                                                           const float value) {
  static_cast<llama_sampling_params *>(options)->top_p = value;
}

extern "C" void bindings_session_sampler_options_drop(void *options) {
  delete static_cast<llama_sampling_params *>(options);
}

/// Session batch
extern "C" void *bindings_session_batch_init(const uint32_t token_capacity,
                                             const uint32_t embedding_size,
                                             const uint32_t max_sequence_ids) {
  return static_cast<void *>(new llama_batch(
      llama_batch_init(static_cast<int32_t>(token_capacity),
                       static_cast<int32_t>(embedding_size),
                       static_cast<int32_t>(max_sequence_ids))));
}

extern "C" void bindings_session_batch_clear(void *batch) {
  llama_batch_clear(*static_cast<llama_batch *>(batch));
}

extern "C" void bindings_session_batch_add_token(void *batch, int32_t token,
                                                 uint32_t index, bool logits) {
  llama_batch_add(*static_cast<llama_batch *>(batch), token,
                  static_cast<uint32_t>(index), {0}, logits);
}

extern "C" uint32_t bindings_session_batch_tokens_len(const void *batch) {
  return static_cast<uint32_t>(
      static_cast<const llama_batch *>(batch)->n_tokens);
}

extern "C" void bindings_session_batch_tokens_set_len(void *batch,
                                                      uint32_t value) {
  static_cast<llama_batch *>(batch)->n_tokens = static_cast<int32_t>(value);
}

extern "C" const int32_t *bindings_session_batch_tokens_ptr(const void *batch) {
  return const_cast<const int32_t *>(
      static_cast<const llama_batch *>(batch)->token);
}

extern "C" int32_t *bindings_session_batch_tokens_mut_ptr(void *batch) {
  return static_cast<llama_batch *>(batch)->token;
}

extern "C" const float *
bindings_session_batch_embedding_ptr(const void *batch) {
  return const_cast<const float *>(
      static_cast<const llama_batch *>(batch)->embd);
}

extern "C" float *bindings_session_batch_embedding_mut_ptr(void *batch) {
  return static_cast<llama_batch *>(batch)->embd;
}

extern "C" const int32_t *bindings_session_batch_pos_ptr(const void *batch) {
  return const_cast<const int32_t *>(
      static_cast<const llama_batch *>(batch)->pos);
}

extern "C" int32_t *bindings_session_batch_pos_mut_ptr(void *batch) {
  return static_cast<llama_batch *>(batch)->pos;
}

extern "C" const int32_t *
bindings_session_batch_sequence_id_len_ptr(const void *batch) {
  return const_cast<const int32_t *>(
      static_cast<const llama_batch *>(batch)->n_seq_id);
}

extern "C" int32_t *
bindings_session_batch_sequence_id_len_mut_ptr(void *batch) {
  return static_cast<llama_batch *>(batch)->n_seq_id;
}

extern "C" const int32_t **
bindings_session_batch_sequence_id_ptr(const void *batch) {
  return const_cast<const int32_t **>(
      static_cast<const llama_batch *>(batch)->seq_id);
}

extern "C" int32_t **bindings_session_batch_sequence_id_mut_ptr(void *batch) {
  return static_cast<llama_batch *>(batch)->seq_id;
}

extern "C" const int8_t *bindings_session_batch_logits_ptr(const void *batch) {
  return const_cast<const int8_t *>(
      static_cast<const llama_batch *>(batch)->logits);
}

extern "C" int8_t *bindings_session_batch_logits_mut_ptr(void *batch) {
  return static_cast<llama_batch *>(batch)->logits;
}

extern "C" void bindings_session_batch_drop(void *batch) {
  llama_batch_free(*static_cast<llama_batch *>(batch));

  delete static_cast<llama_batch *>(batch);
}
