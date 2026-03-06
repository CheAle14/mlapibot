use ::image::DynamicImage;
use album::{Album, AlbumBuilder};
use error::ImgurError;
use image::{Image, ImageBuilder};
use mlapibot_ocr::image::OcrImage;
use reqwest::{
    Method, RequestBuilder,
    header::{HeaderMap, HeaderValue},
    multipart,
};
use serde::Deserialize;

pub mod album;
pub mod error;
pub mod image;

pub struct ImgurClient {
    client: reqwest::Client,
}

#[derive(Deserialize, Debug)]
struct BasicResponse<T> {
    pub data: T,
    // pub success: bool,
    // pub status: i32,
}

impl ImgurClient {
    const BASE_URL: &str = "https://api.imgur.com/3";
    pub fn new(client_id: &str) -> crate::error::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Client-ID {}", client_id))
                .map_err(ImgurError::InvalidHeaderValue)?,
        );

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .map_err(ImgurError::Init)?;

        Ok(Self { client })
    }

    #[inline(always)]
    fn request(&self, method: reqwest::Method, endpoint: &str) -> RequestBuilder {
        let url = format!("{}{}", Self::BASE_URL, endpoint);
        println!("[imgur] {method} {url}");
        self.client.request(method, url)
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn get(&self, endpoint: &str) -> RequestBuilder {
        self.request(Method::GET, endpoint)
    }

    #[inline(always)]
    fn post(&self, endpoint: &str) -> RequestBuilder {
        self.request(Method::POST, endpoint)
    }

    #[inline(always)]
    fn put(&self, endpoint: &str) -> RequestBuilder {
        self.request(Method::PUT, endpoint)
    }

    #[inline(always)]
    fn delete(&self, endpoint: &str) -> RequestBuilder {
        self.request(Method::DELETE, endpoint)
    }

    pub async fn create_album(&mut self, album: AlbumBuilder) -> crate::error::Result<Album> {
        let response = self
            .post("/album")
            .json(&album)
            .send()
            .await
            .map_err(ImgurError::SendRequest)?;

        response
            .json::<BasicResponse<Album>>()
            .await
            .map(|r| r.data)
            .map_err(ImgurError::ResponseJson)
    }

    pub async fn upload_image(&mut self, image: ImageBuilder<'_>) -> crate::error::Result<Image> {
        let form_image = multipart::Part::file(image.path)
            .await
            .map_err(ImgurError::UploadReadImage)?;

        let form = reqwest::multipart::Form::new()
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

        let response = self
            .post("/image")
            .multipart(form)
            .send()
            .await
            .map_err(ImgurError::SendRequest)?;

        let str = response.text().await.map_err(ImgurError::ResponseText)?;
        // let _ = std::fs::write("imgur_out.json", &str);

        serde_json::from_str::<BasicResponse<Image>>(&str)
            .map(|r| r.data)
            .map_err(ImgurError::DecodeJson)
    }

    // pub fn add_to_album(&mut self, album: &Album, images: &[Image]) -> anyhow::Result<()> {
    //     let mut hashes = String::with_capacity(images.len() * 10);
    //     for img in &images[..images.len() - 1] {
    //         hashes.push_str(&img.delete_hash);
    //         hashes.push(',');
    //     }
    //     hashes.push_str(&images.last().unwrap().delete_hash);

    //     let form = multipart::Form::new().text("deletehashes", hashes);

    //     let url = format!("/album/{}/add", album.delete_hash);
    //     self.post(&url).multipart(form).send()?.error_for_status()?;
    //     Ok(())
    // }

    pub async fn update_album(
        &mut self,
        deletehash: &str,
        album: AlbumBuilder,
    ) -> crate::error::Result<()> {
        let url = format!("/album/{deletehash}");

        self.put(&url)
            .json(&album)
            .send()
            .await
            .map_err(ImgurError::SendRequest)?
            .error_for_status()
            .map_err(ImgurError::BadResponse)?;

        Ok(())
    }

    pub async fn delete_album(&mut self, album: Album) -> crate::error::Result<()> {
        let endpoint = format!("/album/{}", album.delete_hash);

        self.delete(&endpoint)
            .send()
            .await
            .map_err(ImgurError::SendRequest)?
            .error_for_status()
            .map_err(ImgurError::BadResponse)?;

        Ok(())
    }
}

pub async fn upload_images<'images>(
    client: &mut ImgurClient,
    images: impl Iterator<Item = &'images OcrImage>,
    get_trigger_words_image: impl Fn(usize) -> Option<DynamicImage>,
) -> crate::error::Result<Album> {
    let mut album_images = Vec::new();
    for (idx, image) in images.enumerate() {
        let seen = image.get_seen_words_image();
        let tempfile = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .map_err(ImgurError::Tempfile)?;

        seen.save(tempfile.path())
            .map_err(|err| ImgurError::WritingImage(idx, tempfile.path().to_owned(), err))?;

        let trigger = get_trigger_words_image(idx);

        let description = if trigger.is_some() {
            "The image's words as they were seen by the bot's OCR. The words which triggered the response are highlighted in a following image"
        } else {
            "The image's words as they were seen by the bot's OCR. No scams were detected in this image."
        };

        let uploaded = client
            .upload_image(ImageBuilder::builder(tempfile.path()).description(description))
            .await?;

        album_images.push(uploaded);

        if let Some(trigger) = trigger {
            let tempfile = tempfile::Builder::new()
                .suffix(".png")
                .tempfile()
                .map_err(ImgurError::Tempfile)?;

            trigger
                .save(tempfile.path())
                .map_err(|err| ImgurError::WritingImage(idx, tempfile.path().to_owned(), err))?;

            let uploaded = client
                .upload_image(ImageBuilder::builder(tempfile.path()).description(
                "The words making up the phrase triggering the response is bounded in red boxes.",
            )).await?;
            album_images.push(uploaded);
        }
    }

    let album = client
        .create_album(AlbumBuilder::builder().title("/u/mlapibot OCR"))
        .await?;

    client
        .update_album(
            &album.delete_hash,
            AlbumBuilder::builder()
                .cover(album_images.first().unwrap().id.as_str())
                .delete_hashes(album_images.iter().map(|x| x.delete_hash.as_str())),
        )
        .await?;

    Ok(album)
}
