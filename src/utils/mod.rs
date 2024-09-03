mod lowercase;
mod submission_ext;
mod words;

pub use lowercase::*;
pub use submission_ext::*;
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

#[cfg(feature = "true-run")]
pub const fn is_debug() -> bool {
    false
}

#[cfg(not(feature = "true-run"))]
pub const fn is_debug() -> bool {
    true
}
