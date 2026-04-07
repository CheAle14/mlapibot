use serde::{Deserialize, Serialize};

use crate::Words;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Matchers {
    Phrase(PhraseMatcher),
    Any(AnyMatcher),
    Ordered(OrderedMatcher),
    All(AllMatcher),
    Exact(ExactMatcher),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhraseMatcher {
    pub phrase: Words,
}

impl PhraseMatcher {
    pub fn new(phrase: impl Into<Words>) -> Self {
        Self {
            phrase: phrase.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnyMatcher {
    pub children: Vec<Matchers>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderedMatcher {
    pub children: Vec<Matchers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<usize>,
}
impl OrderedMatcher {
    pub fn new(children: Vec<Matchers>) -> Self {
        Self {
            children,
            max_steps: None,
        }
    }

    pub fn new_with_steps(children: Vec<Matchers>, max_steps: usize) -> Self {
        Self {
            children,
            max_steps: Some(max_steps),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AllMatcher {
    pub children: Vec<Matchers>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoneMatcher {
    pub children: Vec<Matchers>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExactMatcher {
    pub phrase: Words,
}

impl ExactMatcher {
    pub fn new(phrase: impl Into<Words>) -> Self {
        Self {
            phrase: phrase.into(),
        }
    }
}
