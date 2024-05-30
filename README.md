<h1 align="center">Clyde</h1>
<p align="center">A Discord chatbot</p>

A re-creation of Discord's discontinued [Clyde AI experiment](https://discord.fandom.com/wiki/Clyde_(chatbot)).

# How use!?

Install [Rust](https://rustup.rs), and [Ollama](https://github.com/ollama/ollama), download the LLaVA LLama 3 model:

```
ollama pull llava-llama3
```

Once that works, obtain your Discord Bot token, put it in `.env`, then finally run Clyde:

```
cargo run --release
```
