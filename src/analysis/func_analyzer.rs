use serde::Deserialize;

use crate::context::Context;

use super::Detection;

#[derive(Debug, Deserialize)]
pub struct FuncAnalyzer {
    function: String,
}

impl FuncAnalyzer {
    pub fn analyze(&self, context: &Context) -> anyhow::Result<Option<Detection>> {
        // TODO
        Ok(None)
    }
}
