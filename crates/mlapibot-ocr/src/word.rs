use leptess::leptonica::Box;

#[derive(Debug)]
pub struct OcrWord<'a> {
    text: &'a str,
    bbox: &'a Box,
}

impl<'a> OcrWord<'a> {
    pub fn new(text: &'a str, bbox: &'a Box) -> Self {
        Self { text, bbox }
    }

    pub fn text(&self) -> &'a str {
        self.text
    }

    pub fn bbox(&self) -> &'a Box {
        self.bbox
    }
}
