use mlapibot_ocr::error::OcrError;

use crate::util::DownloadFileError;

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("OCR-related failure")]
    OCR(#[source] OcrError),

    #[error("'{0}' is not a valid URL")]
    Url(String),

    #[error("failed to download image")]
    Download(#[from] DownloadFileError),
}

pub type Result<T, E = AnalysisError> = std::result::Result<T, E>;
