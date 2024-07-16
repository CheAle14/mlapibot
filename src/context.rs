use std::path::{Path, PathBuf};

use regex::Regex;
use url::Url;

use crate::{
    ocr::image::{ImageSource, OcrImage},
    statics::{link_regex, valid_extensions},
};

pub enum ContextKind<'a> {
    CliPath(PathBuf),
    CliLink(Url),
    Submission(&'a roux::submission::SubmissionData),
    Comment(&'a roux::comment::CommentData),
    DirectMessage(&'a roux::inbox::InboxData),
}

fn parse_url(text: impl AsRef<str>) -> Option<Url> {
    match Url::parse(text.as_ref()).map(fix_url) {
        Ok(Some(url)) => Some(url),
        Ok(None) => None,
        Err(_) => None,
    }
}

fn extract_all_links(text: &str, rgx: &Regex) -> Vec<Url> {
    rgx.captures_iter(text)
        .filter_map(|capt| {
            let (text, []) = capt.extract();
            parse_url(text)
        })
        .collect()
}

fn fix_url(mut url: Url) -> Option<Url> {
    if url.scheme() != "https" {
        None
    } else {
        let hostname = url.host_str()?;
        if hostname == "preview.redd.it" {
            let _ = url.set_host(Some("i.redd.it"));
        } else if hostname == "gyazo.com" {
            let _ = url.set_host(Some("i.gyazo.com"));
            let mut path = url.path().to_owned();
            path.push_str(".png");
            url.set_path(&path);
        }

        Some(url)
    }
}

fn extract_filename(url: &Url) -> Option<&str> {
    let path = url.path();
    let index = path.find('/').unwrap_or_else(|| path.find('\\').unwrap());
    let filename = &path[index + 1..];

    Some(filename)
}

fn allowed_url(url: &Url) -> bool {
    if let Some(filename) = extract_filename(url) {
        valid_extensions()
            .iter()
            .any(|ext| filename.ends_with(*ext))
    } else {
        false
    }
}

fn extract_image_links(text: &str, rgx: &Regex) -> Vec<Url> {
    let mut all = extract_all_links(text, rgx);

    all.retain(allowed_url);

    all
}

fn download_file(url: &Url) -> anyhow::Result<ImageSource> {
    let text = url.as_str();
    println!("Downloading image from {text}");
    let mut resp = reqwest::blocking::get(text)?;
    let len = resp.content_length().unwrap_or_default();
    println!("Image is {len} bytes");
    let path = url.path();
    let filename = if let Some(idx) = path.rfind('/') {
        &path[idx..]
    } else {
        &path[..]
    };

    if cfg!(windows) || len > (1024 * 1024 * 10) {
        // download to file on windows since not all image types can be read from memory, or
        // on other OS if the image is too large
        let extension = if filename.ends_with(".jpeg") {
            ".jpeg"
        } else if filename.ends_with(".jpg") {
            ".jpg"
        } else {
            ".png"
        };

        let mut file = tempfile::Builder::new().suffix(extension).tempfile()?;
        let _ = resp.copy_to(&mut file)?;
        Ok(ImageSource::DeleteOnDropFile(file))
    } else {
        // download to memory for small images
        let mut v = Vec::with_capacity(len as usize);
        let _ = resp.copy_to(&mut v)?;

        Ok(ImageSource::MemoryOnly {
            filename: filename.to_string(),
            bytes: v,
        })
    }
}

impl<'a> ContextKind<'a> {
    pub fn get_images(&self) -> anyhow::Result<Vec<OcrImage>> {
        let pattern = link_regex();
        let mut fixed_urls = Vec::new();

        match self {
            ContextKind::CliPath(path) => {
                let image = OcrImage::new(ImageSource::KeepFile(path.clone()))?;
                return Ok(vec![image]);
            }
            ContextKind::CliLink(link) => {
                let url = fix_url(link.clone()).expect("link is https to image");
                fixed_urls.push(url);
            }
            ContextKind::Submission(submission) => {
                if submission.is_self {
                    for url in extract_image_links(&submission.selftext, pattern) {
                        fixed_urls.push(url)
                    }
                } else if let Some(gallery) = &submission.gallery_data {
                    if let Some(metadata) = &submission.media_metadata {
                        for img in &gallery.items {
                            if let Some(meta) = metadata.get(&img.media_id) {
                                if meta.e != "Image" {
                                    continue;
                                }
                                if let Some(url) = parse_url(&meta.s.u) {
                                    fixed_urls.push(url);
                                } else {
                                    eprintln!("Invalid url: {meta:?}");
                                }
                            } else {
                                eprintln!("Gallery item not present: {img:?}");
                            }
                        }
                    }
                } else if let Some(text) = &submission.url {
                    if let Some(url) = parse_url(text) {
                        fixed_urls.push(url);
                    }
                }
                // TODO: gallery
            }
            ContextKind::Comment(comment) => {
                if let Some(body) = &comment.body {
                    fixed_urls.extend(extract_image_links(body, pattern))
                }
            }
            ContextKind::DirectMessage(message) => {
                fixed_urls.extend(extract_image_links(&message.body, pattern))
            }
        };

        let mut images = Vec::with_capacity(fixed_urls.len());
        for url in fixed_urls {
            let downloaded = download_file(&url)?;
            let image = OcrImage::new(downloaded)?;
            images.push(image);
        }
        Ok(images)
    }

    pub fn get_title_and_body(&self) -> anyhow::Result<(Option<String>, Option<String>)> {
        match self {
            ContextKind::CliPath(_) | ContextKind::CliLink(_) => Ok((None, None)),
            ContextKind::Submission(submission) => {
                let (t, c) = match (submission.title.len() > 0, submission.selftext.len() > 0) {
                    (true, true) => (
                        Some(submission.title.clone()),
                        Some(submission.selftext.clone()),
                    ),
                    (true, false) => (Some(submission.title.clone()), None),
                    (false, true) => (None, Some(submission.selftext.clone())),
                    (false, false) => (None, None),
                };

                Ok((t, c))
            }
            ContextKind::Comment(_) => Ok((None, None)),
            ContextKind::DirectMessage(_) => Ok((None, None)),
        }
    }
}

pub struct Context<'a> {
    pub kind: ContextKind<'a>,
    pub images: Vec<OcrImage>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub debug: bool,
}

impl<'a> Context<'a> {
    fn from_kind(kind: ContextKind<'a>) -> anyhow::Result<Self> {
        let images = kind.get_images()?;
        let (title, body) = kind.get_title_and_body()?;

        Ok(Self {
            kind,
            images,
            title,
            body,
            debug: false,
        })
    }
    pub fn from_cli_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let kind = ContextKind::CliPath(path.to_owned());

        Self::from_kind(kind)
    }
    pub fn from_cli_link(link: &Url) -> anyhow::Result<Self> {
        let kind = ContextKind::CliLink(link.clone());

        Self::from_kind(kind)
    }

    pub fn from_submission(
        submission: &'a roux::submission::SubmissionData,
    ) -> anyhow::Result<Self> {
        Self::from_kind(ContextKind::Submission(submission))
    }
}
