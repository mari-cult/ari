use aho_corasick::{AhoCorasick, BuildError};
use anyhow::Context;
use reqwest::Url;
use serde::{de, Deserialize};
use std::fs;

#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct GeminiConfig {
    pub token: String,
}

#[derive(Debug)]
pub struct DomainMatcher {
    compiled: AhoCorasick,
}

impl DomainMatcher {
    pub fn new<I, P>(patterns: I) -> Result<Self, BuildError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<[u8]>,
    {
        Ok(Self {
            compiled: AhoCorasick::new(patterns)?,
        })
    }

    pub fn url_matches(&self, url: &Url) -> bool {
        let Some(domain) = url.domain() else {
            return false;
        };

        self.domain_matches(domain)
    }

    pub fn domain_matches(&self, domain: &str) -> bool {
        let end = domain.chars().count();

        self.compiled.find_iter(domain).any(|mat| mat.end() == end)
    }
}

impl<'de> Deserialize<'de> for DomainMatcher {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let strings: Vec<String> =
            Deserialize::deserialize(deserializer).map_err(de::Error::custom)?;

        Self::new(strings).map_err(de::Error::custom)
    }
}

#[derive(Debug, Deserialize)]
pub struct ClydeConfig {
    pub whitelisted_domains: DomainMatcher,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub discord: DiscordConfig,
    pub gemini: GeminiConfig,
    pub clyde: ClydeConfig,
}

impl Config {
    pub fn read(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path).with_context(|| format!("reading {path}"))?;

        toml::from_str(&content).with_context(|| format!("parsing {path}"))
    }
}
