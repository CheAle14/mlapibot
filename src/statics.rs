use ab_glyph::FontRef;
use std::sync::OnceLock;

use regex::Regex;

#[cfg(windows)]
fn init_font() -> FontRef<'static> {
    FontRef::try_from_slice(include_bytes!("C:\\Windows\\Fonts\\arial.ttf")).unwrap()
}

#[cfg(unix)]
fn init_font() -> FontRef<'static> {
    use std::os::unix::ffi::OsStringExt;
    use std::process::Command;
    use std::{ffi::OsString, path::Path};

    let mut cmd = Command::new("fc-match");
    cmd.arg("-f").arg(r#"%{file}"#).arg("Arial");
    let output = cmd.output().unwrap();
    let os_str: OsString = OsString::from_vec(output.stdout);
    let path: &Path = os_str.as_ref();
    let bytes = std::fs::read(path).unwrap();
    let bytes = bytes.into_boxed_slice();
    let leaked: &'static [u8] = Box::leak(bytes);
    FontRef::try_from_slice(leaked).unwrap()
}

pub fn draw_font() -> &'static ab_glyph::FontRef<'static> {
    static FONT: OnceLock<ab_glyph::FontRef> = OnceLock::new();
    FONT.get_or_init(init_font)
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
