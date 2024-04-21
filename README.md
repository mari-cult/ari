I miss Clyde, so this is a reimplementation (with extras).

Model: LLaVA v1.6 (Mistral) 4-bit K M quant. (`ollama pull llava:7b-v1.6-mistral-q4_K_M`)

The last 50 messages in a channel are passed.

Images are a simple description.

Videos are described by unique frames (Comparsion score above 0.28). The model outputs JSON, with `description`, and `confidence`, invalid JSON or confidence less than 0.5 is discarded. At most 10 descriptions are collected, then summarized.

The final chat inference isn't multi-modal, as there is no visual forwarding, but it is still LLaVA, this has shown improved understanding, however (images aren't merged).
