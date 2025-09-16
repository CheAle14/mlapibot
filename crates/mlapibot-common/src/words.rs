use std::{borrow::Cow, collections::HashSet};

#[derive(Debug, PartialEq)]
struct WordDef {
    pub start: u32,
    pub len: u32,
}

#[derive(Debug, PartialEq)]
pub struct Words {
    phrase: String,
    words: Vec<WordDef>,
}

impl Words {
    pub fn clean(text: &mut String) {
        text.make_ascii_lowercase();
        text.retain(allowed_char);
    }

    pub fn new(phrase: impl Into<String>) -> Self {
        let mut phrase: String = phrase.into();

        Self::clean(&mut phrase);

        let bytes = phrase.as_bytes();

        if bytes.len() > (u32::MAX as usize) {
            panic!("string is larger than u32");
        }

        let mut last = 0;
        let mut words = Vec::new();

        let mut idx = 0u32;
        while idx < (bytes.len() as u32) {
            let c = bytes[idx as usize];
            if matches!(c, b' ' | b'\t' | b'\n') {
                words.push(WordDef {
                    start: last,
                    len: idx - last,
                });
                last = idx + 1;
            }

            idx += 1;
        }

        if idx > last {
            words.push(WordDef {
                start: last,
                len: idx - last,
            });
        }

        Self { words, phrase }
    }

    pub fn full_text(&self) -> &str {
        &self.phrase
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn iter_words(&self) -> impl Iterator<Item = &str> {
        self.words.iter().map(|w| {
            let s = w.start as usize;
            let l = w.len as usize;

            &self.phrase[s..s + l]
        })
    }

    pub fn as_words(&self) -> Vec<&str> {
        self.iter_words().collect()
    }

    pub fn as_hash_set(&self) -> HashSet<&str> {
        self.iter_words().collect()
    }

    pub fn iter_stemmed_words(&self) -> impl Iterator<Item = Cow<'_, str>> {
        let stemmer = rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::English);
        self.iter_words().map(move |word| stemmer.stem(word))
    }

    pub fn remove_stop_words(&mut self) {
        self.retain(|word| !STOP_WORDS.binary_search(&word).is_ok());
    }

    pub fn retain<F>(&mut self, mut func: F)
    where
        F: FnMut(&str) -> bool,
    {
        let mut offset = 0;
        self.words.retain_mut(|w| {
            w.start = w.start.saturating_sub(offset);

            let s = w.start as usize;
            let l = w.len as usize;

            let word = &self.phrase[s..s + l];

            let retn = func(word);

            if retn {
                true
            } else {
                let rem_left = s > 0;
                let rem_right = !rem_left && (s + l + 1) < self.phrase.len();

                let range = if rem_left {
                    offset += 1;
                    (s - 1)..(s + l)
                } else if rem_right {
                    offset += 1;
                    s..(s + l + 1)
                } else {
                    s..l
                };

                offset += word.len() as u32;
                self.phrase.replace_range(range, "");

                false
            }
        });
    }
}

fn allowed_char(c: char) -> bool {
    match c {
        'a'..='z' => true,
        '0'..='9' => true,
        ' ' | '\t' | '\n' => true,
        _ => false,
    }
}

static STOP_WORDS: &[&str] = &[
    "a",
    "about",
    "above",
    "after",
    "again",
    "against",
    "all",
    "am",
    "an",
    "and",
    "any",
    "anyone",
    "are",
    "as",
    "at",
    "be",
    "because",
    "been",
    "before",
    "being",
    "below",
    "between",
    "both",
    "bro",
    "brother",
    "but",
    "by",
    "can",
    "confused",
    "did",
    "didnt",
    "discord",
    "do",
    "does",
    "doing",
    "don",
    "down",
    "dumb",
    "during",
    "each",
    "easy",
    "else",
    "error",
    "even",
    "exist",
    "expect",
    "feature",
    "few",
    "fix",
    "for",
    "from",
    "further",
    "had",
    "happened",
    "has",
    "have",
    "having",
    "he",
    "help",
    "her",
    "here",
    "hers",
    "herself",
    "him",
    "himself",
    "his",
    "how",
    "i",
    "idk",
    "if",
    "im",
    "in",
    "into",
    "is",
    "issue",
    "it",
    "its",
    "itself",
    "just",
    "know",
    "me",
    "more",
    "most",
    "my",
    "myself",
    "need",
    "no",
    "nor",
    "not",
    "now",
    "of",
    "off",
    "ok",
    "okay",
    "on",
    "once",
    "one",
    "only",
    "or",
    "other",
    "our",
    "ours",
    "ourselves",
    "out",
    "over",
    "own",
    "please",
    "possible",
    "probably",
    "problem",
    "probs",
    "question",
    "really",
    "s",
    "same",
    "she",
    "should",
    "so",
    "some",
    "struggle",
    "such",
    "t",
    "technical",
    "than",
    "that",
    "the",
    "their",
    "theirs",
    "them",
    "themselves",
    "then",
    "there",
    "these",
    "they",
    "think",
    "this",
    "those",
    "thought",
    "thoughts",
    "through",
    "title",
    "to",
    "too",
    "under",
    "until",
    "up",
    "urgent",
    "very",
    "was",
    "we",
    "were",
    "what",
    "when",
    "where",
    "which",
    "while",
    "who",
    "whom",
    "why",
    "will",
    "with",
    "work",
    "working",
    "worried",
    "you",
    "your",
    "yours",
    "yourself",
    "yourselves",
];

#[cfg(test)]
mod tests {
    use crate::words::STOP_WORDS;

    use super::Words;

    #[test]
    pub fn test_stop_word_ordered() {
        let mut vec = STOP_WORDS.to_vec();
        vec.sort();
        vec.dedup();
        if vec != STOP_WORDS {
            panic!("{vec:#?}");
        }
    }

    #[test]
    pub fn test_phrase_matcher_split() {
        let matcher = Words::new("hello world goes here");

        assert_eq!(matcher.as_words(), vec!["hello", "world", "goes", "here"]);
    }

    #[test]
    pub fn test_phrase_matcher_split_numbers() {
        let matcher = Words::new("some 10mb goes 10 mb here");

        assert_eq!(
            matcher.as_words(),
            vec!["some", "10mb", "goes", "10", "mb", "here"]
        );
    }

    #[test]
    pub fn test_remove_stop_words() {
        macro_rules! assert_all_removed {
            ($($words:literal),* $(,)?) => {
                $(
                    let mut words = Words::new($words);
                    words.remove_stop_words();
                    assert_eq!(words.len(), 0, "{:?}", words.full_text());
                )*
            };
        }

        assert_all_removed!(
            "i need help",
            "help me",
            "I need help with this, I’m very worried",
            "What can I do",
            "Has anyone had this issue?",
            "discord is not working at all",
            "i need help… probs an easy fix but",
            "does this exist or is this even possible",
            "is this feature working?",
            "Error ?",
            "Does anyone know how to fix this??",
            "I need urgent help",
            "Bro what is this..",
            "I’m confused please help if possible",
            "Okay please help",
            "any thoughts",
            "i didn't expect this",
            "Idk what to title this"
        );
    }

    #[test]
    pub fn test_words_replace() {
        static WORDS: [&str; 9] = [
            "the", "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog",
        ];

        let joined = WORDS.join(" ");

        let mut words = Words::new(&joined);

        let mut idx = 0;
        words.retain(|word| {
            assert_eq!(WORDS[idx], word, "{idx}");
            idx += 1;

            true
        });

        assert_eq!(joined, words.full_text());

        let mut idx = 0;
        words.retain(|word| {
            assert_eq!(WORDS[idx], word, "{idx}");
            idx += 1;

            false
        });

        assert_eq!("", words.full_text());

        let mut words = Words::new(&joined);

        let mut idx = 0;
        words.retain(|word| {
            assert_eq!(WORDS[idx], word, "{idx}");
            idx += 1;

            word != "the"
        });

        assert_eq!("quick brown fox jumps over lazy dog", words.full_text());

        let mut word_iter = words.iter_words();
        assert_eq!(word_iter.next(), Some("quick"));
        assert_eq!(word_iter.next(), Some("brown"));
        assert_eq!(word_iter.next(), Some("fox"));
        assert_eq!(word_iter.next(), Some("jumps"));
        assert_eq!(word_iter.next(), Some("over"));
        assert_eq!(word_iter.next(), Some("lazy"));
        assert_eq!(word_iter.next(), Some("dog"));
        assert_eq!(word_iter.next(), None);
    }
}
