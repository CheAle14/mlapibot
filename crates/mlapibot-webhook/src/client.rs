use crate::{error::WebhookError, message::Message};

pub struct WebhookClient {
    client: reqwest::Client,
    url: String,
}

impl WebhookClient {
    pub fn new(url: impl Into<String>) -> crate::error::Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .user_agent("DiscordBot (https://github.com/CheAle14/mlapibot, rust-1)")
            .build()
            .map_err(WebhookError::InitClient)?;

        Ok(Self {
            url: url.into(),
            client,
        })
    }

    pub async fn send(&mut self, message: &Message) -> crate::error::Result<()> {
        let response = self
            .client
            .post(&self.url)
            .json(message)
            .send()
            .await
            .map_err(WebhookError::Send)?;

        match response.error_for_status_ref() {
            Ok(_) => Ok(()),
            Err(err) => {
                let body = response.text().await.map_err(WebhookError::Recv)?;
                eprintln!("Response body: {body}");
                Err(WebhookError::ApiError(err, body))
            }
        }
    }
}
