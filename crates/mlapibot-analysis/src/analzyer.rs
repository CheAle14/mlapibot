use std::cell::LazyCell;

use mlapibot_common::{Detection, Words, matchers::Matchers};

use crate::{Context, matcher::Matcher};

pub trait Analyzer {
    fn name(&self) -> &str;
    fn ocr(&self) -> Option<&Matchers>;
    fn title(&self) -> Option<&Matchers>;
    fn body(&self) -> Option<&Matchers>;
    fn title_or_body(&self) -> Option<&Matchers>;

    fn analyze(&self, context: &Context) -> crate::error::Result<Option<Detection>> {
        let mut detection = Detection::new();

        if let Some(ocr) = self.ocr() {
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

        for title in [self.title(), self.title_or_body()] {
            if let Some(title) = title {
                if let Some(words) = &context.title {
                    let words = words.as_words();
                    if let Some(value) = title.best_match(&words, context.debug) {
                        println!("min-max: {:?}", value.min_max_word_indexes());
                        detection.set_title(value);
                    }
                }
            }
        }

        for body in [self.body(), self.title_or_body()] {
            if let Some(body) = body {
                if let Some(words) = &context.body {
                    let words = words.as_words();
                    if let Some(value) = body.best_match(&words, context.debug) {
                        detection.set_body(value);
                    }
                }
            }
        }

        Ok(detection.finish())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct TestAnalyzer {
    pub name: &'static str,
    pub ocr: Option<Matchers>,
    pub title: Option<Matchers>,
    pub body: Option<Matchers>,
    pub title_or_body: Option<Matchers>,
}

#[cfg(test)]
impl Analyzer for TestAnalyzer {
    fn name(&self) -> &str {
        self.name
    }

    fn ocr(&self) -> Option<&Matchers> {
        self.ocr.as_ref()
    }

    fn title(&self) -> Option<&Matchers> {
        self.title.as_ref()
    }

    fn body(&self) -> Option<&Matchers> {
        self.body.as_ref()
    }

    fn title_or_body(&self) -> Option<&Matchers> {
        self.title_or_body.as_ref()
    }
}
