use llama::{Model, Session, SessionBatch};
use std::io::{self, Write};

fn main() {
    let model = Model::options()
        .set_gpu_layers(33)
        .open("../models/teknium_openhermes-2.5-mistral-7b.gguf")
        .expect("big oof energy");

    let mut prompt = Vec::new();

    model.tokenize_special("<|im_start|>system\n", &mut prompt);
    model.tokenize("You are named Clyde", &mut prompt);
    model.tokenize_special("<|im_end|>\n", &mut prompt);

    let mut session = Session::options()
        .set_temperature(0.2)
        .set_top_k(50.0)
        .set_top_p(0.95)
        .with_model(model);

    let mut batch = SessionBatch::new(32768, 0, 1);
    let mut tokens = prompt.clone();
    let model = session.model();

    model.tokenize_special("<|im_start|>user\n", &mut tokens);
    model.tokenize("hi clyde", &mut tokens);
    model.tokenize_special("<|im_end|>\n<|im_start|>assistant\n", &mut tokens);

    for token in tokens.iter().copied() {
        let mut string = String::new();

        session.model().detokenize(&[token], &mut string);

        println!("{token} -> {string:?}");
    }

    for (index, token) in tokens.iter().copied().enumerate() {
        batch.add_token(token, index.try_into().unwrap(), false);
    }

    if let Some(logit) = batch.logits_mut().last_mut() {
        *logit = true;
    }

    let mut stdout = io::stdout().lock();

    loop {
        session.decode(&mut batch);

        let token = session.sample();

        {
            let mut string = String::new();

            session.model().detokenize(&[token], &mut string);

            stdout.write(string.as_bytes()).unwrap();
            stdout.flush().unwrap();
        }

        if token == session.model().eos_token() {
            break;
        }

        session.accept(token);
        batch.clear();
        batch.add_token(token, tokens.len() as u32, true);
        tokens.push(token);
    }

    session.reset();
}
