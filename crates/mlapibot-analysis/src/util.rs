use mlapibot_common::NeedleFinder;
use mlapibot_ocr::image::ImageSource;

use crate::{error::AnalysisError, url::Url};

pub fn parse_url(text: impl AsRef<str>) -> Option<Url> {
    match Url::parse(text.as_ref()).map(fix_url) {
        Ok(Some(url)) => Some(url),
        Ok(None) => None,
        Err(_) => None,
    }
}

pub fn extract_all_links(text: &str, starts_with: Option<&'static str>) -> Vec<Url> {
    let mut urls = Vec::new();

    let mut searcher = NeedleFinder::new(starts_with.unwrap_or("http"), text);

    while let Some(start) = searcher.next() {
        let rest = &text[start..];

        let link = match rest.find([' ', '\t', '\r', '\n', ')']) {
            Some(end) => &rest[..end],
            None => rest,
        };

        if let Ok(Some(url)) = Url::parse(link).map(fix_url) {
            urls.push(url);
        }
    }

    urls
}

pub fn fix_url(mut url: Url) -> Option<Url> {
    if url.scheme() != "https" {
        None
    } else {
        let hostname = url.domain();
        if hostname == "preview.redd.it" {
            let _ = url.set_domain("i.redd.it");
        } else if hostname == "gyazo.com" {
            let _ = url.set_domain("i.gyazo.com");
            let mut path = url.path().to_owned();
            path.push_str(".png");
            url.set_path(&path);
        }

        Some(url)
    }
}

pub fn extract_filename(url: &Url) -> Option<&str> {
    let path = url.path();
    if path.trim().len() == 0 {
        return None;
    }
    let index = path.find('/').unwrap_or_else(|| path.find('\\').unwrap());
    let filename = &path[index + 1..];

    Some(filename)
}

pub fn valid_extensions() -> &'static [&'static str] {
    &[".png", ".jpeg", ".jpg"]
}

pub fn allowed_url(url: &Url) -> bool {
    if let Some(filename) = extract_filename(url) {
        valid_extensions()
            .iter()
            .any(|ext| filename.ends_with(*ext))
    } else {
        false
    }
}

pub fn extract_image_links(text: &str) -> Vec<Url> {
    let mut all = extract_all_links(text, None);

    all.retain(allowed_url);

    all
}

pub fn download_file(url: &Url) -> crate::error::Result<Option<ImageSource>> {
    let text = url.as_str();
    println!("Downloading image from {text}");
    let mut resp = reqwest::blocking::get(text).map_err(AnalysisError::DownloadNetErr)?;
    let len = resp.content_length().unwrap_or_default();
    println!("Image is {len} bytes");

    let Some(filename) = extract_filename(url) else {
        return Ok(None);
    };

    let Some((_, extension)) = filename.rsplit_once('.') else {
        return Ok(None);
    };

    let mut file = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .map_err(AnalysisError::DownloadFileErr)?;

    resp.copy_to(&mut file)
        .map_err(AnalysisError::DownloadNetErr)?;

    Ok(Some(ImageSource::DeleteOnDropFile(file)))
}
