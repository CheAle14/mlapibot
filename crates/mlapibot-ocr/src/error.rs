#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("failed to initialize ocr engine")]
    Init(leptess::tesseract::TessInitError),
    #[error("failed to open image for decoding")]
    OpenImage(std::io::Error),
    #[error("failed to decode image")]
    DecodeImage(image::ImageError),
    #[error("failed to set image for OCR use")]
    SetImage(leptess::leptonica::PixError),

    #[error("failed to get image text as UTF-8")]
    NotUtf8(std::str::Utf8Error),

    #[error("did not see any words in the image")]
    NoWords,
}

pub type Result<T, E = OcrError> = std::result::Result<T, E>;
