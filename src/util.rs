use linkify::{LinkFinder, LinkKind};
use url::Url;

pub fn iter_urls(input: &str) -> impl Iterator<Item = Url> + use<'_> {
    LinkFinder::new()
        .kinds(&[LinkKind::Url])
        .links(input)
        .flat_map(|link| link.as_str().parse().ok())
}
