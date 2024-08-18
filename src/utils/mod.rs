mod words;

pub use words::*;

pub fn clamp<'a>(text: &'a str, length: usize) -> &'a str {
    if text.len() <= length {
        text
    } else {
        &text[..length]
    }
}

pub fn as_ref(words: &Vec<String>) -> Vec<&str> {
    words.iter().map(|s| s.as_str()).collect()
}
