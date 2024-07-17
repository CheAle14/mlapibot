use std::{io::Cursor, path::PathBuf};

use ab_glyph::{Font, FontRef, ScaleFont};
use anyhow::anyhow;
use image::{DynamicImage, Rgba};
use imageproc::{
    drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut},
    rect::Rect,
};
use leptess::leptonica::Box;
use tempfile::NamedTempFile;

use crate::{
    analysis::{
        str_analyzer::{clean_string, CleanedWords},
        DetectedItem,
    },
    statics::draw_font,
};

use super::{get_tesseract, word::OcrWord};

pub enum ImageSource {
    /// Image source is a local file that is kept
    KeepFile(PathBuf),
    /// Image source is a local file that will be deleted
    DeleteOnDropFile(NamedTempFile),
}

impl ImageSource {
    pub fn read_image(&self) -> anyhow::Result<DynamicImage> {
        match &self {
            Self::KeepFile(path) => {
                let img = image::io::Reader::open(path)?.decode()?;
                Ok(img)
            }
            Self::DeleteOnDropFile(guard) => {
                let img = image::io::Reader::open(guard)?.decode()?;
                Ok(img)
            }
        }
    }
}

pub struct OcrImage {
    source: ImageSource,
    cached_image: DynamicImage,
    ocr_words: CleanedWords,
    ocr_boxes: Vec<Box>,
}

fn get_size(font: &FontRef, text: &str) -> (i32, i32) {
    let scaled = font.as_scaled(10.0);

    let mut height = 0.0;
    let mut width = 0.0;

    for c in text.chars() {
        let glyph = scaled.scaled_glyph(c);
        let bounds = scaled.glyph_bounds(&glyph);
        let h = bounds.height();
        if h > height {
            height = h;
        }
        width += bounds.width();
    }

    (width.ceil() as i32, height.ceil() as i32)
}

impl OcrImage {
    pub fn new(source: ImageSource) -> anyhow::Result<Self> {
        let mut lt = get_tesseract()?;
        match &source {
            ImageSource::KeepFile(path) => lt.set_image(path)?,
            ImageSource::DeleteOnDropFile(guard) => lt.set_image(guard)?,
        }

        let ocr_text = lt.get_utf8_text()?;
        let components = lt
            .get_component_boxes(leptess::capi::TessPageIteratorLevel_RIL_WORD, true)
            .ok_or(anyhow!("no words"))?;
        let initial_boxes = (&components).into_iter();
        let initial_words: Vec<_> = ocr_text.split_ascii_whitespace().collect();

        let mut ocr_words = Vec::new();
        let mut ocr_boxes = Vec::new();
        for (bx, word) in std::iter::zip(initial_boxes, initial_words) {
            let word = clean_string(word);
            if word.len() > 0 {
                ocr_boxes.push(bx);
                ocr_words.push(word);
            }
        }

        let cached_image = source.read_image()?;
        Ok(Self {
            ocr_words,
            ocr_boxes,
            cached_image,
            source,
        })
    }

    pub fn full_text(&self) -> String {
        self.ocr_words.join(" ")
    }

    pub fn words(&self) -> Vec<&str> {
        self.ocr_words.iter().map(|s| s.as_str()).collect()
    }

    pub fn words_bbox<'this>(&'this self) -> Vec<OcrWord<'this>> {
        self.ocr_boxes
            .iter()
            .zip(&self.ocr_words)
            .map(|(bbox, word)| OcrWord::new(word, bbox))
            .collect()
    }

    /// Returns an image with the words detected drawn over with a box, filled with what text was seen at that position
    pub fn get_seen_words_image(&self) -> DynamicImage {
        const PADDING: i32 = 2;

        let font = draw_font();
        let mut img = self.cached_image.clone();

        for word in self.words_bbox() {
            let text = word.text();
            let bbox = word.bbox();

            let rect = bbox.get_geometry();
            let padded_rect = Rect::at(rect.x - PADDING, rect.y - PADDING).of_size(
                (rect.w + PADDING + PADDING) as u32,
                (rect.h + PADDING + PADDING) as u32,
            );
            let rect = Rect::at(rect.x, rect.y).of_size(rect.w as u32, rect.h as u32);

            draw_filled_rect_mut(&mut img, padded_rect, Rgba([255, 0, 0, 255]));
            draw_filled_rect_mut(&mut img, rect, Rgba([255, 255, 255, 255]));

            let (text_w, text_h) = get_size(font, text);

            let left_x = rect.left();
            let top_y = rect.top();
            let right_x = rect.right();
            let bot_y = rect.bottom();

            let mid_x = (left_x + right_x) / 2;
            let mid_y = (top_y + bot_y) / 2;

            let half_w = text_w / 2;
            let half_h = text_h / 2;

            let x = std::cmp::max(left_x, mid_x - half_w);
            let y = std::cmp::max(top_y, mid_y - half_h);

            draw_text_mut(&mut img, Rgba([0, 0, 0, 0]), x, y, 10.0, &font, text)
        }

        img
    }

    /// Returns an image with the words that were part of the trigger surrounded in a box
    pub fn get_trigger_words_image(&self, detected: &DetectedItem) -> DynamicImage {
        const PADDING: i32 = 2;

        let mut img = self.cached_image.clone();

        for (idx, word) in self.words_bbox().iter().enumerate() {
            if !detected.words.contains_key(&idx) {
                continue;
            }

            let bbox = word.bbox();

            let rect = bbox.get_geometry();
            let padded_rect = Rect::at(rect.x - PADDING, rect.y - PADDING).of_size(
                (rect.w + PADDING + PADDING) as u32,
                (rect.h + PADDING + PADDING) as u32,
            );

            draw_hollow_rect_mut(&mut img, padded_rect, Rgba([255, 0, 0, 255]));
        }

        img
    }
}
