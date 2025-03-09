use leptess::LepTess;

pub mod error;
pub mod image;
pub mod word;

pub(crate) mod font;

pub(crate) fn get_tesseract() -> crate::error::Result<LepTess> {
    LepTess::new(None, "eng").map_err(crate::error::OcrError::Init)
}
