use mlapibot_ocr::error::OcrError;

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("OCR-related failure")]
    OCR(OcrError),

    #[error("'{0}' is not a valid URL")]
    Url(String),

    #[error("failed to download image")]
    DownloadNetErr(reqwest::Error),

    #[error("failed to create tempfile for image")]
    DownloadFileErr(std::io::Error),
}

pub type Result<T, E = AnalysisError> = std::result::Result<T, E>;
