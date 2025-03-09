use mlapibot_analysis::Context;
use mlapibot_common::{Detection, Words};

use crate::Submission;

pub trait SubmissionExt {
    fn has_unknown_media(&self) -> bool;
}

impl SubmissionExt for Submission {
    fn has_unknown_media(&self) -> bool {
        if let Some(metadata) = self.media_metadata() {
            for value in metadata.values() {
                match value {
                    roux::api::submission::SubmissionDataMediaMetadata::Unknown => return true,
                    _ => (),
                }
            }
        }
        false
    }
}

pub trait DetectionExt {
    fn get_markdown(&self, ctx: &Context) -> anyhow::Result<Vec<String>>;
    fn get_trigger_images(&self, ctx: &Context) -> anyhow::Result<Vec<image::DynamicImage>>;
}

impl DetectionExt for Detection {
    fn get_markdown(&self, ctx: &Context) -> anyhow::Result<Vec<String>> {
        let mut v = Vec::new();
        for (index, img) in &self.images {
            let text = ctx.images[*index].words();
            let mut s = String::new();
            img.write_markdown(&text, &mut s)?;
            v.push(s);
        }

        if let Some(title) = &self.title {
            let text = ctx.title.as_ref().unwrap();
            let words = Words::new(text);
            let mut s = String::new();
            title.write_markdown(&words.as_words(), &mut s)?;
            v.push(s);
        }

        if let Some(body) = &self.body {
            let text = ctx.body.as_ref().unwrap();
            let words = Words::new(text);
            let mut s = String::new();
            body.write_markdown(&words.as_words(), &mut s)?;
            v.push(s);
        }

        Ok(v)
    }

    fn get_trigger_images(&self, ctx: &Context) -> anyhow::Result<Vec<image::DynamicImage>> {
        let mut v = Vec::new();
        for (index, detected) in &self.images {
            let image = &ctx.images[*index];
            let image = image.get_trigger_words_image(detected);
            v.push(image);
        }

        Ok(v)
    }
}
