use roux::builders::submission::SubmissionSubmitBuilder;

pub struct CachedSubmission {
    title: String,
    body: String,
    hash: String,
}

impl CachedSubmission {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        let title: String = title.into();
        let body: String = body.into();

        let hash = Self::create_hash(&body);

        Self { title, body, hash }
    }

    pub fn to_builder(&self) -> SubmissionSubmitBuilder {
        SubmissionSubmitBuilder::text(&self.title, &self.body).with_send_replies(false)
    }

    pub fn get_body(&self) -> &str {
        &self.body
    }

    // pub fn set_body(&mut self, body: impl Into<String>) {
    //     self.body = body.into();
    //     self.hash = Self::create_hash(&self.body)
    // }

    pub fn get_hash(&self) -> &str {
        &self.hash
    }

    fn create_hash(text: &str) -> String {
        use base64ct::{Base64, Encoding};
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(text);
        let output = hasher.finalize();

        Base64::encode_string(&output)
    }
}
