use album::{Album, AlbumBuilder};
use anyhow::Context as AnyhowContext;
use image::{Image, ImageBuilder};
use reqwest::{
    blocking::{multipart, RequestBuilder},
    header::{HeaderMap, HeaderValue},
    Method,
};
use serde::Deserialize;

use crate::{
    analysis::{Analyzer, Detection},
    context::Context,
    ocr::image::ImageSource,
    ImgurCredentials,
};

pub mod album;
pub mod image;

pub struct ImgurClient {
    client: reqwest::blocking::Client,
}

#[derive(Deserialize)]
struct BasicResponse<T> {
    pub data: T,
    pub success: bool,
    pub status: i32,
}

impl ImgurClient {
    const BASE_URL: &str = "https://api.imgur.com/3";
    pub fn new(credentials: &ImgurCredentials) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Client-ID {}", credentials.imgur_client_id))?,
        );

        let client = reqwest::blocking::ClientBuilder::new()
            .default_headers(headers)
            .build()?;

        Ok(Self { client })
    }

    #[inline(always)]
    fn request(&self, method: reqwest::Method, endpoint: &str) -> RequestBuilder {
        let url = format!("{}{}.json", Self::BASE_URL, endpoint);
        self.client.request(method, url)
    }

    #[inline(always)]
    fn get(&self, endpoint: &str) -> RequestBuilder {
        self.request(Method::GET, endpoint)
    }

    #[inline(always)]
    fn post(&self, endpoint: &str) -> RequestBuilder {
        self.request(Method::POST, endpoint)
    }

    #[inline(always)]
    fn delete(&self, endpoint: &str) -> RequestBuilder {
        self.request(Method::DELETE, endpoint)
    }

    // 8jbQ8JO0goORG13

    pub fn create_album(&mut self, album: AlbumBuilder) -> anyhow::Result<Album> {
        let response: BasicResponse<Album> = self
            .post("/album")
            .form(&album)
            .send()?
            .error_for_status()?
            .json()?;
        Ok(response.data)
    }

    pub fn upload_image(&mut self, image: ImageBuilder) -> anyhow::Result<Image> {
        let form_image = match image.image {
            ImageSource::KeepFile(path) => multipart::Part::file(path)?,
            ImageSource::DeleteOnDropFile(guard) => multipart::Part::file(guard)?,
        };

        let form = reqwest::blocking::multipart::Form::new()
            .part("image", form_image)
            .text("type", "image");

        let form = if let Some(title) = image.title {
            form.text("title", title)
        } else {
            form
        };

        let form = if let Some(description) = image.description {
            form.text("description", description)
        } else {
            form
        };

        let response: BasicResponse<Image> = self.post("/image").multipart(form).send()?.json()?;
        Ok(response.data)
    }

    pub fn add_to_album(&mut self, album: &Album, images: &[Image]) -> anyhow::Result<()> {
        let mut hashes = String::with_capacity(images.len() * 10);
        for img in &images[..images.len() - 1] {
            hashes.push_str(&img.delete_hash);
            hashes.push(',');
        }
        hashes.push_str(&images.last().unwrap().delete_hash);

        let form = multipart::Form::new().text("deletehashes", hashes);

        let url = format!("/album/{}/add", album.delete_hash);
        self.post(&url).multipart(form).send()?.error_for_status()?;
        Ok(())
    }

    pub fn delete_album(&mut self, album: Album) -> anyhow::Result<()> {
        let endpoint = format!("/album/{}", album.delete_hash);
        let _ = self.delete(&endpoint).send()?.error_for_status()?;

        Ok(())
    }
}

pub fn upload_images(
    client: &mut ImgurClient,
    context: &Context,
    detection: &Detection,
    analyzer: &Analyzer,
) -> anyhow::Result<Album> {
    let mut images = Vec::new();
    for (idx, image) in context.images.iter().enumerate() {
        let seen = image.get_seen_words_image();
        let tempfile = tempfile::Builder::new().suffix(".png").tempfile()?;
        seen.save(tempfile.path())
            .with_context(|| format!("saving file {idx} to {:?}", tempfile.path()))?;
        let source = ImageSource::DeleteOnDropFile(tempfile);

        let description = if detection.images.contains_key(&idx) {
            "The image's words as they were seen by the bot's OCR. The words which triggered the response are highlighted in a following image"
        } else {
            "The image's words as they were seen by the bot's OCR. No scams were detected in this image."
        };

        let uploaded =
            client.upload_image(ImageBuilder::builder(&source).description(description))?;
        images.push(uploaded);

        if let Some(detected) = detection.images.get(&idx) {
            let trigger = image.get_trigger_words_image(detected);
            let tempfile = tempfile::Builder::new().suffix(".png").tempfile()?;
            trigger
                .save(tempfile.path())
                .with_context(|| format!("save trigger image {idx} to {:?}", tempfile.path()))?;
            let source = ImageSource::DeleteOnDropFile(tempfile);

            let uploaded = client.upload_image(ImageBuilder::builder(&source).description(
                "The words making up the phrase triggering the response is bounded in red boxes.",
            ))?;
            images.push(uploaded);
        }
    }
    let album = client.create_album(AlbumBuilder::builder().title("/u/mlapibot OCR"))?;
    client.add_to_album(&album, &images)?;
    Ok(album)
}
