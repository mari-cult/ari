use {
    hyphenation::{Language, Load, Standard},
    textwrap::{core::break_words, word_splitters, WordSplitter, WrapAlgorithm},
};

pub struct MessageSplitter {
    options: textwrap::Options<'static>,
}

impl Default for MessageSplitter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageSplitter {
    pub fn new() -> Self {
        let english = Standard::from_embedded(Language::EnglishUS).unwrap();
        let options = textwrap::Options::new(1950)
            .word_splitter(WordSplitter::Hyphenation(english))
            .wrap_algorithm(WrapAlgorithm::new_optimal_fit());

        Self { options }
    }

    pub fn split(&self, content: &str) -> Vec<String> {
        let Self { options } = self;

        let words = options.word_separator.find_words(content);
        let split_words =
            word_splitters::split_words(words, &options.word_splitter).collect::<Vec<_>>();

        let line_widths = [options.width];
        let wrapped_words = options.wrap_algorithm.wrap(&split_words, &line_widths);

        wrapped_words
            .into_iter()
            .map(|words| {
                let words = words.iter().copied();
                let mut broken_words = break_words(words, options.width).into_iter().peekable();
                let mut content = String::with_capacity(options.width);

                while let Some(word) = broken_words.next() {
                    content.push_str(word.word);

                    if (broken_words.peek().is_some() || word.whitespace == "\n")
                        && !content.is_empty()
                    {
                        content.push_str(word.whitespace);
                    } else {
                        content.push_str(word.penalty);
                    }
                }

                content
            })
            .collect()
    }
}
