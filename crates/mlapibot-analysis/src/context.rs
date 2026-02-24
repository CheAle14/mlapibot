use std::path::Path;

use mlapibot_ocr::image::{ImageSource, OcrImage};

use crate::{
    error::AnalysisError,
    url::Url,
    util::{download_all_files, download_file, extract_image_links},
};

#[derive(Debug, Default)]
pub struct Context {
    pub images: Vec<OcrImage>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub debug: bool,
}

impl Context {
    pub async fn new(
        urls: impl Iterator<Item = Url> + ExactSizeIterator,
        title: Option<String>,
        body: Option<String>,
        warnings: &mut Vec<ContextWarning>,
    ) -> crate::error::Result<Self> {
        let mut images = Vec::with_capacity(urls.len());

        let result = download_all_files(urls).await;

        for (url, error) in result.failures {
            warnings.push(ContextWarning(url, AnalysisError::from(error)));
        }

        for (url, file) in result.success {
            match OcrImage::new(ImageSource::DeleteOnDropFile(file)) {
                Ok(image) => images.push(image),
                Err(error) => warnings.push(ContextWarning(url, AnalysisError::OCR(error))),
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

    pub async fn new_link(
        link: &str,
        warnings: &mut Vec<ContextWarning>,
    ) -> crate::error::Result<Self> {
        let Ok(Some(url)) = Url::parse(link).map(|u| u.fix()) else {
            return Err(AnalysisError::Url(link.to_string()));
        };

        Self::new(std::iter::once(url), None, None, warnings).await
    }

    pub async fn new_body(
        body: impl Into<String>,
        warnings: &mut Vec<ContextWarning>,
    ) -> crate::error::Result<Self> {
        let text: String = body.into();
        let links = extract_image_links(&text);

        Self::new(links.into_iter(), None, Some(text), warnings).await
    }

    pub async fn new_submission(
        urls: impl Iterator<Item = Url> + ExactSizeIterator,
        title: impl Into<String>,
        body: impl Into<String>,
        warnings: &mut Vec<ContextWarning>,
    ) -> crate::error::Result<Self> {
        let body: String = body.into();

        let links = extract_image_links(&body).into_iter();

        Self::new(
            ChainedExactIter(urls, false, links),
            Some(title.into()),
            Some(body),
            warnings,
        )
        .await
    }
}

pub struct ContextWarning(Url, AnalysisError);

impl std::fmt::Display for ContextWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unable to process {}: ", self.0)?;
        mlapibot_common::write_error_chain(f, &self.1)
    }
}

struct ChainedExactIter<I>(I, bool, std::vec::IntoIter<Url>);

impl<I: Iterator<Item = Url>> Iterator for ChainedExactIter<I> {
    type Item = Url;

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 {
            self.2.next()
        } else {
            match self.0.next() {
                Some(v) => Some(v),
                None => {
                    self.1 = true;
                    self.2.next()
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min0, _) = self.0.size_hint();
        let (min1, _) = self.2.size_hint();

        (min0 + min1, Some(min0 + min1))
    }
}

impl<I: Iterator<Item = Url> + ExactSizeIterator> ExactSizeIterator for ChainedExactIter<I> {}
