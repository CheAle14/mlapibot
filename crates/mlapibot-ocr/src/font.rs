use std::sync::OnceLock;

use ab_glyph::FontRef;

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
