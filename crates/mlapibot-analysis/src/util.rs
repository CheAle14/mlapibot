use futures::{StreamExt, stream::FuturesUnordered};
use mlapibot_common::NeedleFinder;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;

use crate::url::Url;

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

#[derive(Debug, thiserror::Error)]
pub enum DownloadFileError {
    #[error("sending request")]
    SendRequest(#[source] reqwest::Error),
    #[error("not image {0}")]
    NonImageURL(&'static str),
    #[error("creating temp file")]
    CreateTempfile(std::io::Error),
    #[error("fetching next data chunk")]
    FetchData(reqwest::Error),
    #[error("write data chunk to file")]
    WriteData(std::io::Error),
}

#[derive(Default)]
pub struct DownloadAllFiles {
    pub success: Vec<(Url, NamedTempFile)>,
    pub failures: Vec<(Url, DownloadFileError)>,
}

pub async fn download_all_files(urls: impl Iterator<Item = Url>) -> DownloadAllFiles {
    let client = reqwest::Client::default();

    let mut result = DownloadAllFiles::default();
    let mut futs = futures::stream::iter(urls)
        .map(|url| async {
            match download_file(&client, &url).await {
                Ok(file) => Ok((url, file)),
                Err(err) => Err((url, err)),
            }
        })
        .buffer_unordered(5);

    while let Some(next) = futs.next().await {
        match next {
            Ok(r) => result.success.push(r),
            Err(r) => result.failures.push(r),
        }
    }

    result
}

pub async fn download_file(
    client: &reqwest::Client,
    url: &Url,
) -> Result<NamedTempFile, DownloadFileError> {
    let text = url.as_str();

    let Some(filename) = url.filename() else {
        return Err(DownloadFileError::NonImageURL("no filename part of path"));
    };

    let Some((_, extension)) = filename.rsplit_once('.') else {
        return Err(DownloadFileError::NonImageURL("no extension in filename"));
    };

    println!("Downloading image from {text}");

    let file = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .map_err(DownloadFileError::CreateTempfile)?;

    let (file, temppath) = file.into_parts();
    let mut file = tokio::fs::File::from_std(file);

    let resp = client
        .get(text)
        .send()
        .await
        .map_err(DownloadFileError::SendRequest)?;

    let len = resp.content_length().unwrap_or_default();
    println!("Fetching {len} bytes: {text}");

    let mut bytes = resp.bytes_stream();

    while let Some(chunk) = bytes.next().await {
        match chunk {
            Ok(chunk) => {
                // println!("{} {}", text, chunk.len());
                file.write_all(&chunk)
                    .await
                    .map_err(DownloadFileError::WriteData)?;
            }
            Err(err) => return Err(DownloadFileError::FetchData(err)),
        }
    }

    let file = file.into_std().await;
    let file = NamedTempFile::from_parts(file, temppath);

    Ok(file)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn can_download_all_files() {
        let urls = vec![
            "https://http.cat/images/100.jpg",
            "https://http.cat/images/101.jpg",
            "https://http.cat/images/404.jpg",
            "https://http.cat/images/500.jpg",
            "https://http.cat/images/200.jpg",
            "https://http.cat/images/201.jpg",
            "https://http.cat/images/202.jpg",
            "https://http.cat/images/204.jpg",
            "https://http.cat/images/400.jpg",
            "https://http.cat/images/401.jpg",
            "https://http.cat/images/403.jpg",
            "https://http.cat/images/404.jpg",
        ]
        .into_iter()
        .filter_map(|text| crate::parse_url(text));

        let result = super::download_all_files(urls).await;

        if result.failures.len() > 0 {
            panic!("{:#?}", result.failures);
        }

        assert_eq!(result.success.len(), 12);
    }
}
