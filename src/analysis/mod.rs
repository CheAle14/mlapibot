use std::collections::HashMap;

use func_analyzer::FuncAnalyzer;
use image::DynamicImage;
use ord_many::{max_many, partial_max_iter};
use pattern_analyzer::PatternAnalyzer;
use serde::Deserialize;
use str_analyzer::{get_words, StrAnalzyer, WordMatcher};

use crate::context::Context;

pub mod func_analyzer;
pub mod pattern_analyzer;
pub mod str_analyzer;

#[derive(Debug, Clone)]
pub struct DetectedWord {
    /// whether this word was part of the threshold triggering phrase
    pub matched: bool,
}

#[derive(Debug)]
pub struct DetectedItem {
    /// the words that were present or triggered
    pub words: HashMap<usize, DetectedWord>,
    pub score: f32,
}

impl DetectedItem {
    pub fn new(score: f32) -> Self {
        Self {
            words: HashMap::new(),
            score,
        }
    }

    pub fn mark_match(&mut self, index: usize) {
        self.words
            .entry(index)
            .and_modify(|w| w.matched = true)
            .or_insert_with(|| DetectedWord { matched: true });
    }

    pub fn write_markdown<W: std::fmt::Write>(
        &self,
        text: &[impl AsRef<str>],
        output: &mut W,
    ) -> std::fmt::Result {
        for (idx, word) in text.iter().enumerate() {
            let word = word.as_ref();
            if let Some(_) = self.words.get(&idx) {
                write!(output, "**{word}**")?;
            } else {
                write!(output, "{word}")?;
            }
            write!(output, " ")?;
        }

        Ok(())
    }
}

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

    pub fn get_markdown(&self, ctx: &Context) -> anyhow::Result<Vec<String>> {
        let mut v = Vec::new();
        for (index, img) in &self.images {
            let text = ctx.images[*index].words();
            let mut s = String::new();
            img.write_markdown(&text, &mut s)?;
            v.push(s);
        }

        if let Some(title) = &self.title {
            let text = ctx.title.as_ref().unwrap();
            let words = get_words(&text);
            let mut s = String::new();
            title.write_markdown(&words, &mut s)?;
            v.push(s);
        }

        if let Some(body) = &self.body {
            let text = ctx.body.as_ref().unwrap();
            let words = get_words(&text);
            let mut s = String::new();
            body.write_markdown(&words, &mut s)?;
            v.push(s);
        }

        Ok(v)
    }

    pub fn get_trigger_images(&self, ctx: &Context) -> anyhow::Result<Vec<DynamicImage>> {
        let mut v = Vec::new();
        for (index, detected) in &self.images {
            let image = &ctx.images[*index];
            let image = image.get_trigger_words_image(detected);
            v.push(image);
        }

        Ok(v)
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

fn default_template() -> String {
    String::from("default")
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug)]
pub struct Analyzer {
    pub name: String,
    #[serde(default)]
    pub report: bool,
    #[serde(default = "default_true")]
    pub ignore_self_posts: bool,
    #[serde(default = "default_template")]
    pub template: String,
    blacklist: Option<WordMatcher>,
    #[serde(flatten)]
    kind: AnalyzerKind,
}

impl Analyzer {
    pub fn analyze(&self, context: &Context) -> anyhow::Result<Option<Detection>> {
        let result = self.kind.analyze(context)?;
        if let Some(result) = result {
            if let Some(blacklist) = &self.blacklist {
                if blacklist.any_matches(context) {
                    return Ok(None);
                }
            }

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AnalyzerKind {
    #[serde(rename = "function")]
    Function(FuncAnalyzer),
    #[serde(rename = "img")]
    Pattern(PatternAnalyzer),
    #[serde(untagged)]
    Text(StrAnalzyer),
}

impl AnalyzerKind {
    fn analyze(&self, context: &Context) -> anyhow::Result<Option<Detection>> {
        match self {
            AnalyzerKind::Text(v) => v.analyze(context),
            AnalyzerKind::Function(v) => v.analyze(context),
            AnalyzerKind::Pattern(v) => v.analyze(context),
        }
    }
}

pub fn load_scams() -> anyhow::Result<Vec<Analyzer>> {
    static FILE: &str = include_str!("../../data/scams.json");

    #[derive(Deserialize)]
    struct SaveFile {
        pub scams: Vec<Analyzer>,
    }

    let scams: SaveFile = serde_json::from_str(FILE)?;
    Ok(scams.scams)
}

pub fn get_best_analysis<'yzer>(
    ctx: &Context,
    analyzer: &'yzer [Analyzer],
) -> anyhow::Result<Option<(Detection, &'yzer Analyzer)>> {
    let mut best: Option<(f32, Detection, &Analyzer)> = None;
    for next in analyzer {
        if let Some(detection) = next.analyze(ctx)? {
            let score = detection.best_score();
            if best.is_none() || score > best.as_ref().unwrap().0 {
                best = Some((score, detection, next));
                if score >= 1.0 {
                    break;
                }
            }
        }
    }

    let best = best.map(|(_, d, a)| (d, a));
    Ok(best)
}
