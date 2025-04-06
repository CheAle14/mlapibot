use mlapibot_ocr::error::OcrError;

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("OCR-related failure")]
    OCR(#[source] OcrError),

    #[error("'{0}' is not a valid URL")]
    Url(String),

    #[error("failed to download image")]
    DownloadNetErr(#[source] reqwest::Error),

    #[error("failed to create tempfile for image")]
    DownloadFileErr(#[source] std::io::Error),
}

pub type Result<T, E = AnalysisError> = std::result::Result<T, E>;
