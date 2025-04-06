use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ImgurError {
    #[error("failed to initialize reqwest client")]
    Init(#[source] reqwest::Error),

    #[error("invalid header value")]
    InvalidHeaderValue(#[source] reqwest::header::InvalidHeaderValue),

    #[error("failed to read image to upload")]
    UploadReadImage(#[source] std::io::Error),

    #[error("failed to send request")]
    SendRequest(#[source] reqwest::Error),

    #[error("got an error as a response")]
    BadResponse(#[source] reqwest::Error),

    #[error("failed to decode body as json")]
    ResponseJson(#[source] reqwest::Error),

    #[error("failed to decode body as UTF-8 text")]
    ResponseText(#[source] reqwest::Error),

    #[error("failed to decode something as json")]
    DecodeJson(#[source] serde_json::Error),

    #[error("failed to write image {0} to path {1:?}")]
    WritingImage(usize, PathBuf, #[source] image::ImageError),

    #[error("failed to create tempfile")]
    Tempfile(#[source] std::io::Error),
}

pub type Result<T, E = ImgurError> = std::result::Result<T, E>;
