pub fn clamp<'a>(text: &'a str, length: usize) -> &'a str {
    if text.len() <= length {
        text
    } else {
        &text[..length]
    }
}
