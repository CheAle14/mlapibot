use leptess::leptonica::BoxGeometry;

#[derive(Debug)]
pub struct OcrWord<'a> {
    text: &'a str,
    bbox: &'a BoxGeometry,
}

impl<'a> OcrWord<'a> {
    pub fn new(text: &'a str, bbox: &'a BoxGeometry) -> Self {
        Self { text, bbox }
    }

    pub fn text(&self) -> &'a str {
        self.text
    }

    pub fn bbox(&self) -> &'a BoxGeometry {
        self.bbox
    }
}
