use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
use std::{fmt, iter};

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    input: Vec<&'a str>,
}

#[derive(Deserialize)]
struct Response {
    embeddings: Vec<Vector>,
}

#[derive(Default)]
pub struct Ollama {
    client: Client,
}

#[derive(Clone, Deserialize, PartialEq, PartialOrd, Serialize)]
#[repr(transparent)]
pub struct Vector(Vec<f32>);

impl Vector {
    pub fn dot(&self, other: &Vector) -> f32 {
        iter::zip(&self[..], &other[..]).map(|(x, y)| x * y).sum()
    }

    pub fn magnitude(&self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn distance_euclid(&self, other: &Vector) -> f32 {
        iter::zip(&self[..], &other[..])
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    pub fn cos_similarity(&self, other: &Vector) -> f32 {
        let a = self.magnitude();
        let b = other.magnitude();

        if a == 0.0 || b == 0.0 {
            0.0
        } else {
            self.dot(other) / (a * b)
        }
    }
}

impl fmt::Debug for Vector {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self[..], fmt)
    }
}

impl Deref for Vector {
    type Target = [f32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Vector {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Ollama {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn embed<'a, I>(&self, model: &str, input: I) -> anyhow::Result<Vec<Vector>>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let input = input.into_iter().collect();
        let embeddings = reqwest::Client::default()
            .post("http://localhost:11434/api/embed")
            .json(&Request { model, input })
            .send()
            .await?
            .json::<Response>()
            .await?
            .embeddings;

        Ok(embeddings)
    }
}
