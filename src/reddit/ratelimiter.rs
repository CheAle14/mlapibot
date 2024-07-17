use std::time::{Duration, Instant};

pub struct Ratelimiter {
    last_inbox: Instant,
    last_subreddits: Instant,
}

pub enum Rate {
    NoneReadyFor(Duration),
    InboxReady,
    SubredditsReady,
}

impl Ratelimiter {
    const REDDIT_SECONDS: u64 = 15;

    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            last_inbox: now.checked_sub(Duration::from_secs(30)).unwrap(),
            last_subreddits: now.checked_sub(Duration::from_secs(60)).unwrap(),
        }
    }

    pub fn get(&self) -> Rate {
        let now = Instant::now();
        let subreddits = now
            .checked_duration_since(self.last_subreddits)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        let inbox = now
            .checked_duration_since(self.last_inbox)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        let least = std::cmp::min(subreddits, inbox);

        if subreddits >= Self::REDDIT_SECONDS && least >= 5 {
            Rate::SubredditsReady
        } else if inbox >= Self::REDDIT_SECONDS && least >= 5 {
            Rate::InboxReady
        } else {
            let max = std::cmp::max(inbox, subreddits);

            let secs = if max >= Self::REDDIT_SECONDS {
                5
            } else {
                Self::REDDIT_SECONDS - max
            };

            let secs = if least < 5 {
                std::cmp::max(secs, 5)
            } else {
                secs
            };

            Rate::NoneReadyFor(Duration::from_secs(secs))
        }
    }

    pub fn set_inbox(&mut self) {
        self.last_inbox = Instant::now();
    }

    pub fn set_subreddits(&mut self) {
        self.last_subreddits = Instant::now();
    }
}
