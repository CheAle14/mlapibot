#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("failed to initialize client")]
    InitClient(#[source] reqwest::Error),

    #[error("failed to send request")]
    Send(#[source] reqwest::Error),

    #[error("failed to read response body")]
    Recv(#[source] reqwest::Error),

    #[error("status code indicates error")]
    ApiError(#[source] reqwest::Error, String),
}

pub type Result<T, E = WebhookError> = std::result::Result<T, E>;
