use serde::Deserialize;

use crate::context::Context;

use super::Detection;

#[derive(Debug, Deserialize)]
pub struct PatternAnalyzer {
    pub img: String,
}

impl PatternAnalyzer {
    pub fn analyze(&self, context: &Context) -> anyhow::Result<Option<Detection>> {
        // TODO
        Ok(None)
    }
}
