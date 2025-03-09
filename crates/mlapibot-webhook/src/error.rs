#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("failed to initialize client")]
    InitClient(reqwest::Error),

    #[error("failed to send request")]
    Send(reqwest::Error),

    #[error("failed to read response body")]
    Recv(reqwest::Error),

    #[error("status code indicates error")]
    ApiError(reqwest::Error, String),
}

pub type Result<T, E = WebhookError> = std::result::Result<T, E>;
