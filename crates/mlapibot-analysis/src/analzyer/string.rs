use mlapibot_common::{Detection, Words};
use serde::Deserialize;

use crate::matcher::{AnyMatcher, Matcher, MatcherKind};

#[derive(Deserialize)]
struct StrAnalyzerRepr {
    ocr: Option<MatcherKind>,
    title: Option<MatcherKind>,
    body: Option<MatcherKind>,
    #[serde(rename = "title+body")]
    title_or_body: Option<MatcherKind>,
}

#[derive(Debug, Deserialize)]
#[serde(from = "StrAnalyzerRepr")]
pub struct StrAnalzyer {
    pub ocr: Option<MatcherKind>,
    pub title: Option<MatcherKind>,
    pub body: Option<MatcherKind>,
}

fn extend(first: &mut Option<MatcherKind>, extend_with: Option<MatcherKind>) {
    if first.is_none() {
        *first = extend_with;
    } else if let Some(with) = extend_with {
        let first = first.as_mut().unwrap();
        let replaced = std::mem::replace(first, MatcherKind::None);
        let any = MatcherKind::Any(AnyMatcher::new(vec![replaced, with]));
        *first = any;
    }
}

impl From<StrAnalyzerRepr> for StrAnalzyer {
    fn from(value: StrAnalyzerRepr) -> Self {
        let StrAnalyzerRepr {
            ocr,
            mut title,
            mut body,
            title_or_body,
        } = value;

        extend(&mut title, title_or_body.clone());
        extend(&mut body, title_or_body);

        Self { ocr, title, body }
    }
}

impl StrAnalzyer {
    pub fn analyze(
        &self,
        context: &crate::context::Context,
    ) -> crate::error::Result<Option<Detection>> {
        let mut detection = Detection::new();
        if let Some(ocr) = &self.ocr {
            for (idx, image) in context.images.iter().enumerate() {
                let words = image.words();
                if context.debug {
                    println!("OCR Image {idx}:");
                }
                if let Some(result) = ocr.best_match(&words, context.debug) {
                    detection.add_image(idx, result);
                }
            }
        }
        if let Some(title) = &self.title {
            if let Some(ctx) = &context.title {
                let words = Words::new(ctx);
                let words = words.as_words();
                if let Some(value) = title.best_match(&words, context.debug) {
                    println!("min-max: {:?}", value.min_max_word_indexes());
                    detection.set_title(value);
                }
            }
        }
        if let Some(body) = &self.body {
            if let Some(ctx) = &context.body {
                let words = Words::new(ctx);
                let words = words.as_words();
                if let Some(value) = body.best_match(&words, context.debug) {
                    detection.set_body(value);
                }
            }
        }
        Ok(detection.finish())
    }
}
