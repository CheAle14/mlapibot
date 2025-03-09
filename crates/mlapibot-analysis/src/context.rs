use std::path::Path;

use mlapibot_ocr::image::{ImageSource, OcrImage};

use crate::{
    error::AnalysisError,
    url::Url,
    util::{download_file, extract_image_links, fix_url},
};

#[derive(Default)]
pub struct Context {
    pub images: Vec<OcrImage>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub debug: bool,
}

impl Context {
    pub fn new(
        urls: impl Iterator<Item = Url> + ExactSizeIterator,
        title: Option<String>,
        body: Option<String>,
        warnings: &mut Vec<ContextWarning>,
    ) -> crate::error::Result<Self> {
        let mut images = Vec::with_capacity(urls.len());
        for url in urls {
            match download_file(&url) {
                Ok(Some(image)) => match OcrImage::new(image) {
                    Ok(image) => images.push(image),
                    Err(error) => warnings.push(ContextWarning(url, AnalysisError::OCR(error))),
                },
                Ok(None) => (),
                Err(error) => warnings.push(ContextWarning(url, error)),
            }
        }

        Ok(Self {
            images,
            title,
            body,
            debug: false,
        })
    }

    pub fn new_path(path: impl AsRef<Path>) -> crate::error::Result<Self> {
        let source = ImageSource::KeepFile(path.as_ref().to_path_buf());
        let image = OcrImage::new(source).map_err(AnalysisError::OCR)?;

        Ok(Self {
            images: vec![image],
            ..Default::default()
        })
    }

    pub fn new_link(link: &str, warnings: &mut Vec<ContextWarning>) -> crate::error::Result<Self> {
        let Ok(Some(url)) = Url::parse(link).map(fix_url) else {
            return Err(AnalysisError::Url(link.to_string()));
        };

        Self::new(std::iter::once(url), None, None, warnings)
    }

    pub fn new_body(
        body: impl Into<String>,
        warnings: &mut Vec<ContextWarning>,
    ) -> crate::error::Result<Self> {
        let text: String = body.into();
        let links = extract_image_links(&text);

        Self::new(links.into_iter(), None, Some(text), warnings)
    }

    pub fn new_title_and_body(
        title: impl Into<String>,
        body: impl Into<String>,
        warnings: &mut Vec<ContextWarning>,
    ) -> crate::error::Result<Self> {
        Self::new_body(body, warnings).map(|mut this| {
            this.title = Some(title.into());
            this
        })
    }
}

pub struct ContextWarning(Url, AnalysisError);

impl std::fmt::Display for ContextWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unable to process {}: {}", self.0, self.1)
    }
}
