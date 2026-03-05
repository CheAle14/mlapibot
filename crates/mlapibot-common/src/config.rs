use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct GlobalSettings {
    pub database_uri: String,
    pub webhook_url: Option<String>,
    pub api: Option<ApiSettings>,
    pub reddit: Option<RedditSettings>,
    pub imgur: Option<ImgurSettings>,
    pub github: Option<GithubSettings>,
}

#[derive(Clone, Deserialize)]
pub struct RedditSettings {
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
    pub user_agent: String,
}

#[derive(Clone, Deserialize)]
pub struct ApiSettings {
    pub bind_address: String,
    pub access_token: String,
}

#[derive(Clone, Deserialize)]
pub struct ImgurSettings {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Clone, Deserialize)]
pub struct GithubSettings {
    pub token: String,
}
