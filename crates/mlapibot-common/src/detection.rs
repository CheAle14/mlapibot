use std::collections::HashMap;

use ord_many::{max_many, partial_max_iter};

use crate::detected_item::DetectedItem;

#[derive(Debug)]
pub struct Detection {
    pub images: HashMap<usize, DetectedItem>,
    pub title: Option<DetectedItem>,
    pub body: Option<DetectedItem>,
}

impl Detection {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
            title: None,
            body: None,
        }
    }

    pub fn add_image(&mut self, index: usize, value: DetectedItem) {
        self.images.insert(index, value);
    }

    pub fn set_title(&mut self, value: DetectedItem) {
        self.title = Some(value);
    }

    pub fn set_body(&mut self, value: DetectedItem) {
        self.body = Some(value);
    }

    pub fn finish(self) -> Option<Self> {
        if self.images.len() > 0 || self.title.is_some() || self.body.is_some() {
            Some(self)
        } else {
            None
        }
    }

    pub fn best_score(&self) -> f32 {
        let iter = self.images.values().map(|v| v.score);
        let best = partial_max_iter(iter).unwrap_or(0.0);
        let best_title = self.title.as_ref().map(|t| t.score).unwrap_or(0.0);
        let best_body = self.body.as_ref().map(|t| t.score).unwrap_or(0.0);
        max_many!(best, best_title, best_body)
    }
}
