use aho_corasick::{AhoCorasick, BuildError};
use anyhow::Context;
use reqwest::Url;
use serde::{de, Deserialize};
use std::{borrow::Cow, fmt::Display, fs};

#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct GeminiConfig {
    pub token: String,
}

/// Checks wether domain overlaps with given list
#[derive(Debug)]
pub struct DomainMatcher {
    backend: AhoCorasick,
}

impl DomainMatcher {
    pub fn new<I, P>(patterns: I) -> Result<Self, BuildError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<[u8]> + Display,
    {
        Ok(Self {
            backend: AhoCorasick::new(patterns.into_iter().map(|pattern| format!(".{pattern}")))?,
        })
    }

    /// Check domain in url
    ///
    /// This fails if url does not have domain
    pub fn url_matches(&self, url: &Url) -> bool {
        let Some(domain) = url.domain() else {
            return false;
        };

        self.domain_matches(domain)
    }

    /// Checks if domain overlaps
    pub fn domain_matches(&self, domain: &str) -> bool {
        let domain = if domain.starts_with('.') {
            Cow::Borrowed(domain)
        } else {
            Cow::Owned(format!(".{domain}"))
        };

        if self.backend.patterns_len() == 0 {
            return true;
        }

        let end = domain.chars().count();

        self.backend
            .find_iter(domain.as_ref())
            .any(|mat| mat.end() == end)
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
    pub owner_id: u64,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_matcher_empty() {
        let whitelist = DomainMatcher::new(Vec::<String>::new()).unwrap();

        assert!(whitelist.domain_matches("aaaa"));
        assert!(whitelist.domain_matches(".aaaa.bbb"));
    }

    #[test]
    fn domain_matcher() {
        let whitelist = DomainMatcher::new(vec!["example.com", "foo.bar"]).unwrap();

        assert!(whitelist.domain_matches("example.com"));

        assert!(!whitelist.domain_matches("discord.gg"));
        assert!(!whitelist.domain_matches("example.net"));
        assert!(!whitelist.domain_matches("example.comm"));
    }

    #[test]
    fn domain_matcher_leading_dot() {
        let whitelist = DomainMatcher::new(vec!["foo.bar"]).unwrap();

        assert!(whitelist.domain_matches("foo.bar"));
        assert!(whitelist.domain_matches("baz.foo.bar"));

        assert!(whitelist.domain_matches(".foo.bar"));
        assert!(whitelist.domain_matches(".baz.foo.bar"));

        assert!(!whitelist.domain_matches("fffoo.bar"));
    }

    #[test]
    fn domain_matcher_subdomains() {
        let whitelist = DomainMatcher::new(vec!["com"]).unwrap();

        assert!(whitelist.domain_matches("discord.com"));
        assert!(whitelist.domain_matches("cdn.whatever.discord.com"));
    }

    #[test]
    fn domain_matcher_url() {
        let whitelist = DomainMatcher::new(vec!["discord.gg", "google.com"]).unwrap();

        assert!(whitelist.url_matches(&"https://google.com/test".parse().expect("url parsing")));
        assert!(!whitelist.url_matches(&"foo:bar".parse().expect("url parsing")));
    }
}
