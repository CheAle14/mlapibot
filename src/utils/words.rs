#[derive(Debug, PartialEq)]
struct WordDef {
    pub start: usize,
    pub len: usize,
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

        let mut last = 0;
        let mut words = Vec::new();

        let mut idx = 0;
        while idx < bytes.len() {
            let c = bytes[idx];
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
        self.words
            .iter()
            .map(|w| &self.phrase[w.start..w.start + w.len])
    }

    pub fn as_words(&self) -> Vec<&str> {
        self.iter_words().collect()
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

#[cfg(test)]
mod tests {
    use super::Words;

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
}
