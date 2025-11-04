use mlapibot_common::NeedleFinder;
use mlapibot_ocr::image::ImageSource;

use crate::{error::AnalysisError, url::Url};

pub fn parse_url(text: impl AsRef<str>) -> Option<Url> {
    match Url::parse(text.as_ref()).map(|u| u.fix()) {
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

        if let Ok(Some(url)) = Url::parse(link).map(|u| u.fix()) {
            urls.push(url);
        }
    }

    urls
}

pub fn valid_extensions() -> &'static [&'static str] {
    &[".png", ".jpeg", ".jpg"]
}

pub fn extract_image_links(text: &str) -> Vec<Url> {
    let mut all = extract_all_links(text, None);

    all.retain(|u| u.allowed_url());

    all
}

pub fn download_file(url: &Url) -> crate::error::Result<Option<ImageSource>> {
    let text = url.as_str();
    println!("Downloading image from {text}");
    let mut resp = reqwest::blocking::get(text).map_err(AnalysisError::DownloadNetErr)?;
    let len = resp.content_length().unwrap_or_default();
    println!("Image is {len} bytes");

    let Some(filename) = url.filename() else {
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
