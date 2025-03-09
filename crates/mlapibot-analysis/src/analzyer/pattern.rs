use mlapibot_common::Detection;
use serde::Deserialize;

use crate::context::Context;

#[derive(Debug, Deserialize)]
pub struct PatternAnalyzer {
    pub img: String,
}

impl PatternAnalyzer {
    pub fn analyze(&self, _context: &Context) -> crate::error::Result<Option<Detection>> {
        // TODO
        Ok(None)
    }
}
