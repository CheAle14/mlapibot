use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ImgurError {
    #[error("failed to initialize reqwest client")]
    Init(reqwest::Error),

    #[error("invalid header value")]
    InvalidHeaderValue(reqwest::header::InvalidHeaderValue),

    #[error("failed to read image to upload")]
    UploadReadImage(std::io::Error),

    #[error("failed to send request")]
    SendRequest(reqwest::Error),

    #[error("got an error as a response")]
    BadResponse(reqwest::Error),

    #[error("failed to decode body as json")]
    ResponseJson(reqwest::Error),

    #[error("failed to decode body as UTF-8 text")]
    ResponseText(reqwest::Error),

    #[error("failed to decode something as json")]
    DecodeJson(serde_json::Error),

    #[error("failed to write image {0} to path {1:?}")]
    WritingImage(usize, PathBuf, image::ImageError),

    #[error("failed to create tempfile")]
    Tempfile(std::io::Error),
}

pub type Result<T, E = ImgurError> = std::result::Result<T, E>;
