use anyhow::Context;
use mlapibot_analysis::Url;
use mlapibot_imgur::image::ImageBuilder;

pub struct CdnLinks;

impl CdnLinks {
    const ALLOWED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];

    fn extract_cdn_links(text: &str) -> Vec<Url> {
        mlapibot_analysis::extract_all_links(text, Some("https://cdn.discordapp.com/attachments"))
    }
}

impl super::Module for CdnLinks {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn name(&self) -> &'static str {
        "comment_cdn_links"
    }

    fn wants(&self) -> super::ModuleWants {
        super::ModuleWants::COMMENTS
    }

    super::impl_mask_subreddits!(comments_cdn => comments);

    fn run_comment<'client>(
        &mut self,
        client: &mut crate::client::ModuleRedditClient<'client>,
        comment: &roux::models::LatestComment<roux::client::AuthedClient>,
    ) -> anyhow::Result<()> {
        let links = Self::extract_cdn_links(comment.body().as_str());

        let http = reqwest::blocking::Client::new();

        let mut uploaded = Vec::new();

        if let Some(imgur) = client.imgur.as_mut() {
            for link in links {
                let Some((_, extension)) = link.path().rsplit_once('.') else {
                    continue;
                };

                if !Self::ALLOWED_EXTENSIONS
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(extension))
                {
                    continue;
                }

                let mut response = http
                    .get(link.as_str())
                    .send()
                    .with_context(|| format!("sending {}", link.as_str()))?
                    .error_for_status()
                    .with_context(|| format!("status {}", link.as_str()))?;

                let mut temp = tempfile::NamedTempFile::with_suffix(extension)
                    .with_context(|| format!("tempfile {}", link.as_str()))?;

                response.copy_to(&mut temp)?;

                let link = imgur.upload_image(ImageBuilder::builder(temp.path()))?;
                uploaded.push(link);
            }
        }

        let text = if uploaded.len() > 0 {
            let mut text = String::from(
                "Discord's CDN links will expire after a relatively short duration; any such links found in your comment have been re-uploaded to Imgur so they last a bit longer:\r\n",
            );

            for link in uploaded {
                text.push_str("\n- https://imgur.com/");
                text.push_str(&link.id);
            }

            text
        } else {
            String::from(
                "Discord's CDN links will expire after a relatively short duration. \
                To help those in the future, especially if this is a bug report, \
                you should consider uploading it directly to Reddit or re-uploading it \
                to a dedicated media host (e.g. Imgur) and updating the link(s) in your message.",
            )
        };

        let reply = comment.reply(&text)?;

        if reply.can_mod_post() {
            reply.lock()?;
        }

        Ok(())
    }
}
