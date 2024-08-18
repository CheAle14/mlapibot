use ord_many::max_many;
use serde::Deserialize;

use crate::{
    analysis::Detection,
    utils::{as_ref, Words},
};

use super::{
    str_matchers::{Matcher, MatcherKind},
    DetectedItem,
};

#[derive(Debug, Deserialize)]
pub struct StrAnalzyer {
    pub ocr: Option<MatcherKind>,
    pub title: Option<MatcherKind>,
    pub body: Option<MatcherKind>,
}

impl StrAnalzyer {
    pub fn analyze(
        &self,
        context: &crate::context::Context,
    ) -> anyhow::Result<Option<super::Detection>> {
        let mut detection = Detection::new();
        if let Some(ocr) = &self.ocr {
            for (idx, image) in context.images.iter().enumerate() {
                let words = image.words();
                if context.debug {
                    println!("OCR Image {idx}:");
                }
                if let Some(result) = ocr.matches(&words, context.debug) {
                    detection.add_image(idx, result);
                }
            }
        }
        if let Some(title) = &self.title {
            if let Some(ctx) = &context.title {
                let words = Words::new(ctx);
                let words = words.as_words();
                if let Some(value) = title.matches(&words, context.debug) {
                    println!("min-max: {:?}", value.min_max_word_indexes());
                    detection.set_title(value);
                }
            }
        }
        if let Some(body) = &self.body {
            if let Some(ctx) = &context.body {
                let words = Words::new(ctx);
                let words = words.as_words();
                if let Some(value) = body.matches(&words, context.debug) {
                    detection.set_body(value);
                }
            }
        }
        Ok(detection.finish())
    }
}
