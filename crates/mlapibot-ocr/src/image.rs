use std::path::{Path, PathBuf};

use ab_glyph::{Font, FontRef, ScaleFont};
use image::{DynamicImage, Rgba};
use imageproc::{
    drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut},
    rect::Rect,
};
use leptess::leptonica::BoxGeometry;
use mlapibot_common::{DetectedItem, Words};
use tempfile::NamedTempFile;

// use crate::{analysis::DetectedItem, statics::draw_font, utils::Words};

use crate::error::OcrError;

use super::{font::draw_font, get_tesseract, word::OcrWord};

pub enum ImageSource {
    /// Image source is a local file that is kept
    KeepFile(PathBuf),
    /// Image source is a local file that will be deleted
    DeleteOnDropFile(NamedTempFile),
}

impl ImageSource {
    pub fn read_image(&self) -> crate::error::Result<DynamicImage> {
        match &self {
            Self::KeepFile(path) => image::ImageReader::open(path)
                .map_err(OcrError::OpenImage)?
                .decode()
                .map_err(OcrError::DecodeImage),
            Self::DeleteOnDropFile(guard) => image::ImageReader::open(guard)
                .map_err(OcrError::OpenImage)?
                .decode()
                .map_err(OcrError::DecodeImage),
        }
    }

    pub fn move_and_keep(self, dest: &Path) -> crate::error::Result<Self> {
        match self {
            ImageSource::KeepFile(path_buf) => {
                std::fs::rename(&path_buf, dest)
                    .map_err(|e| OcrError::KeepMoveImage(path_buf, dest.to_path_buf(), e))?;
            }
            ImageSource::DeleteOnDropFile(named_temp_file) => match named_temp_file.persist(dest) {
                Ok(_file) => (),
                Err(err) if err.error.kind() == std::io::ErrorKind::CrossesDevices => {
                    std::fs::copy(err.file.path(), dest).map_err(|e| {
                        OcrError::KeepMoveImage(
                            err.file.path().to_path_buf(),
                            dest.to_path_buf(),
                            e,
                        )
                    })?;
                }
                Err(err) => return Err(OcrError::KeepPersistImage(dest.to_path_buf(), err)),
            },
        }

        Ok(Self::KeepFile(dest.to_path_buf()))
    }
}

pub struct OcrImage {
    #[allow(unused)]
    source: ImageSource,
    cached_image: DynamicImage,
    ocr_words: Vec<String>,
    ocr_boxes: Vec<BoxGeometry>,
}

impl std::fmt::Debug for OcrImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::with_capacity(self.ocr_words.iter().map(|i| i.len()).sum());

        for word in &self.ocr_words {
            s.push_str(&word);
            s.push(' ');
        }

        f.debug_struct("OcrImage").field("ocr_words", &s).finish()
    }
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
    pub fn new(source: ImageSource) -> crate::error::Result<Self> {
        let mut lt = get_tesseract()?;
        match &source {
            ImageSource::KeepFile(path) => lt.set_image(path).map_err(OcrError::SetImage)?,
            ImageSource::DeleteOnDropFile(guard) => {
                lt.set_image(guard).map_err(OcrError::SetImage)?
            }
        }

        let ocr_text = lt.get_utf8_text().map_err(OcrError::NotUtf8)?;
        let components = lt
            .get_component_boxes(leptess::capi::TessPageIteratorLevel_RIL_WORD, true)
            .ok_or(OcrError::NoWords)?;
        let initial_boxes = (&components).into_iter();
        let initial_words: Vec<_> = ocr_text.split_ascii_whitespace().collect();

        let mut ocr_words = Vec::new();
        let mut ocr_boxes = Vec::new();
        for (bx, word) in std::iter::zip(initial_boxes, initial_words) {
            let mut owned = word.to_owned();
            Words::clean(&mut owned);

            // tesseract returns "hello-world" as one word, but Words::clean will strip
            // the `-`, leaving two words in one 'word'.
            // This means all sub-words will have the same bounding box,
            // but that's not really used anyway so.
            // It looks as though leptonica's `Box` should be refcounted, but it doesn't
            // provide any way to clone them. As such, we just clone the xywh geometry itself.
            if owned.contains(' ') {
                let geom = bx.get_geometry();
                for word in owned.split_ascii_whitespace() {
                    if word.len() > 0 {
                        ocr_boxes.push(geom.clone());
                        ocr_words.push(word.to_owned());
                    }
                }
            } else if word.len() > 0 {
                ocr_boxes.push(bx.get_geometry().clone());
                ocr_words.push(owned);
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

    pub fn image(&self) -> &DynamicImage {
        &self.cached_image
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
            let rect = word.bbox();

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
    pub fn get_trigger_words_image(&self, detected: &DetectedItem) -> Option<DynamicImage> {
        if detected.words.len() == 0 {
            return None;
        }

        const PADDING: i32 = 2;

        let mut img = self.cached_image.clone();

        for (idx, word) in self.words_bbox().iter().enumerate() {
            if !detected.words.contains_key(&idx) {
                continue;
            }

            let rect = word.bbox();
            let padded_rect = Rect::at(rect.x - PADDING, rect.y - PADDING).of_size(
                (rect.w + PADDING + PADDING) as u32,
                (rect.h + PADDING + PADDING) as u32,
            );

            draw_hollow_rect_mut(&mut img, padded_rect, Rgba([255, 0, 0, 255]));
        }

        Some(img)
    }
}
