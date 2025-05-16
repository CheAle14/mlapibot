use mlapibot_analysis::{Context, Url, parse_url};
use mlapibot_common::{Detection, Words};
use roux::api::submission::SubmissionDataMediaMetadata;

use crate::Submission;

pub trait SubmissionExt {
    fn has_unknown_media(&self) -> bool;
    fn get_misc_links(&self) -> Vec<Url>;
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

    fn get_misc_links(&self) -> Vec<Url> {
        let mut fixed_urls = Vec::new();

        if self.is_self() {
            // do nothing, this is handled by the `body` passed in
        } else if let Some(gallery) = self.gallery_data() {
            if let Some(metadata) = self.media_metadata() {
                for img in &gallery.items {
                    if let Some(meta) = metadata.get(&img.media_id) {
                        match meta {
                            SubmissionDataMediaMetadata::Image { s, .. } => {
                                if let Some(url) = parse_url(&s.u) {
                                    fixed_urls.push(url);
                                } else {
                                    eprintln!("Invalid url: {meta:?}");
                                }
                            }
                            SubmissionDataMediaMetadata::RedditVideo { .. } => (),
                            SubmissionDataMediaMetadata::AnimatedImage { .. } => (),
                            SubmissionDataMediaMetadata::Unknown => (),
                        }
                    } else {
                        eprintln!("Gallery item not present: {img:?}");
                    }
                }
            }
        } else if self.is_video() {
            // do nothing
        } else if let Some(link) = self.url() {
            // finally, it is a link
            if let Some(url) = parse_url(link) {
                fixed_urls.push(url);
            }
        }

        fixed_urls
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

            if let Some(image) = image.get_trigger_words_image(detected) {
                v.push(image);
            }
        }

        Ok(v)
    }
}
