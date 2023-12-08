use {
    llama::{Model, Session, SessionBatch},
    std::io::{self, Write},
};

fn main() {
    let model = Model::options()
        .set_gpu_layers(33)
        .open("../models/teknium_openhermes-2.5-mistral-7b.gguf")
        .expect("big oof energy");

    let mut tokens = Vec::new();

    model.tokenize_special("<|im_start|>system\n", &mut tokens);
    model.tokenize(
        "You are named Clyde - and is currently chatting in a Discord server. Communicate responses in lowercase, without punctuation, as one would in chat rooms.",
        &mut tokens,
    );

    model.tokenize_special("<|im_end|>\n", &mut tokens);

    let mut session = Session::options()
        .set_temperature(0.3)
        .set_top_k(50.0)
        .set_top_p(0.95)
        .with_model(model);

    let mut batch = SessionBatch::new(32768, 1);

    batch.extend(tokens.iter().copied(), false);

    let mut lines = io::stdin().lines().flatten();
    let mut stdout = io::stdout().lock();

    while let Some(line) = {
        stdout.write(b"> ").unwrap();
        stdout.flush().unwrap();
        lines.next()
    } {
        let model = session.model();

        tokens.clear();
        model.tokenize_special("<|im_start|>user\n", &mut tokens);
        model.tokenize(&line, &mut tokens);
        model.tokenize_special("<|im_end|>\n<|im_start|>assistant\n", &mut tokens);

        for token in tokens.iter().copied() {
            let mut string = String::new();

            session.model().detokenize(&[token], &mut string);

            println!("{token} -> {string:?}");
        }

        batch.extend(tokens.iter().copied(), false);

        if let Some(logit) = batch.logits_mut().last_mut() {
            *logit = true;
        }

        loop {
            session.decode(&mut batch);

            let token = session.sample();
            let mut string = String::new();

            session.model().detokenize(&[token], &mut string);

            stdout.write(string.as_bytes()).unwrap();
            stdout.flush().unwrap();

            if token == session.model().eos_token() {
                break;
            }

            session.accept(token);
            batch.clear();
            batch.push(token, true);
        }

        writeln!(stdout, "<|done|>").unwrap();
    }
}
