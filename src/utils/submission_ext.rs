use crate::reddit::Submission;

pub trait SubmissionExt {
    fn has_unknown_media(&self) -> bool;
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
}
