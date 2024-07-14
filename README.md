<h1 align="center">Clyde</h1>
<p align="center">A re-creation of Discord's discontinued [Clyde AI experiment](https://discord.fandom.com/wiki/Clyde_(chatbot)).</p>

# How use!?

Install [Rust](https://rustup.rs), and [Ollama](https://github.com/ollama/ollama), download gemma2:

```
ollama pull gemma2
```

Once that works, obtain your Discord Bot token, put it in `.env`, then finally run Clyde:

```
cargo run --release
```
