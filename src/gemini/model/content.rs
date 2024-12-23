use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
pub struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Model,
    User,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Part {
    Text(String),
}

impl Content {
    pub fn new(string: impl ToString) -> Self {
        Self {
            role: Some(Role::User),
            parts: vec![Part::Text(string.to_string())],
        }
    }

    pub fn new_system(string: impl ToString) -> Self {
        Self {
            role: None,
            parts: vec![Part::Text(string.to_string())],
        }
    }

    pub fn is_empty(&self) -> bool {
        parts_is_empty(&self.parts)
    }
}

impl Part {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(string) => Some(string),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Text(string) => string.is_empty(),
            _ => true,
        }
    }

    pub fn into_text(self) -> Option<String> {
        match self {
            Self::Text(string) => Some(string),
            _ => None,
        }
    }
}

fn parts_is_empty(parts: &Vec<Part>) -> bool {
    parts.is_empty() || parts.iter().all(Part::is_empty)
}
