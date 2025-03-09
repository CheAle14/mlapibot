use mlapibot_common::Detection;
use serde::Deserialize;

use crate::context::Context;

#[derive(Debug, Deserialize)]
pub struct FuncAnalyzer {
    function: String,
}

impl FuncAnalyzer {
    pub fn analyze(&self, _context: &Context) -> crate::error::Result<Option<Detection>> {
        // TODO
        Ok(None)
    }
}
