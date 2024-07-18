use std::path::PathBuf;

use roux::util::FeedOption;

pub struct SeenTracker {
    seen_file: PathBuf,
    seen_id: Option<String>,
}

impl SeenTracker {
    pub fn new(seen_file: PathBuf) -> Self {
        let seen_id = std::fs::read_to_string(&seen_file).ok();

        Self { seen_file, seen_id }
    }

    pub fn get_options(&self) -> Option<FeedOption> {
        self.seen_id
            .as_ref()
            .map(|id| FeedOption::new().before(&id))
    }

    pub fn set_seen(&mut self, value: &str) {
        if let Some(existing) = &self.seen_id {
            if existing == value {
                return;
            }
        }

        self.seen_id = Some(value.to_string());
        std::fs::write(&self.seen_file, value).expect("can write seen id to file");
    }
}
