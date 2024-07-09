use std::collections::BTreeSet;

use ollama_rs::generation::{
    chat::{request::ChatMessageRequest, ChatMessage},
    options::GenerationOptions,
};

#[derive(Debug)]
enum Message {
    Human(String, String),
    Angel(String),
}

#[derive(Debug)]
pub struct Chat {
    messages: Vec<Message>,
    users: BTreeSet<String>,
    pub(crate) name: String,
}

impl Chat {
    pub fn with_name<N: AsRef<str>>(name: N) -> Self {
        let name = name.as_ref().to_string();
        let users = BTreeSet::new();

        //users.insert(name.clone());

        Self {
            users,
            messages: vec![],
            name,
        }
    }

    pub fn human<N: AsRef<str>, C: AsRef<str>>(&mut self, name: N, content: C) {
        let name = name.as_ref().to_string();

        self.users.insert(name.clone());
        self.messages
            .push(Message::Human(name, content.as_ref().into()));
    }

    pub fn angel<C: AsRef<str>>(&mut self, content: C) {
        self.messages.push(Message::Angel(content.as_ref().into()));
    }

    pub(crate) fn to_request(&self) -> ChatMessageRequest {
        let Self {
            messages,
            name,
            users,
        } = self;

        let users = users
            .iter()
            .map(|user| user.as_ref())
            .collect::<Vec<_>>()
            .join(", ");

        let users = format!("Users in this channel: {users}");
        let rules = ["You are a Discord user named Clyde"];
        let system = rules.join(". ") + ".";

        let mut messages = messages
            .iter()
            .map(|message| match message {
                Message::Human(name, content) => ChatMessage::user(format!("{name}: {content}")),
                Message::Angel(content) => ChatMessage::assistant(format!("{name}: {content}")),
            })
            .collect::<Vec<_>>();

        messages.insert(0, ChatMessage::system(system));

        ChatMessageRequest::new("gemma2".into(), messages)
    }
}
