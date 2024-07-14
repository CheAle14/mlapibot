use std::sync::OnceLock;

use regex::Regex;

pub fn draw_font() -> &'static ab_glyph::FontRef<'static> {
    static FONT: OnceLock<ab_glyph::FontRef> = OnceLock::new();
    FONT.get_or_init(|| {
        ab_glyph::FontRef::try_from_slice(include_bytes!("C:\\Windows\\Fonts\\arial.ttf")).unwrap()
    })
}

pub fn valid_extensions() -> &'static [&'static str] {
    &[".png", ".jpeg", ".jpg"]
}

pub fn link_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();

    REGEX.get_or_init(|| {
        Regex::new(r"(?:\bhttps://)?[-A-Za-z0-9+&@#/%?=~_|!:,.;]+[-A-Za-z0-9+&@#/%=~_|]").unwrap()
    })
}
