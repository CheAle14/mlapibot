use std::time::{Duration, Instant};

use ord_many::min_many;

pub struct Ratelimiter {
    last_inbox: Instant,
    last_subreddits: Instant,
    last_status: Instant,
}

pub enum Rate {
    NoneReadyFor(Duration),
    StatusReady,
    InboxReady,
    SubredditsReady,
}

impl Ratelimiter {
    const REDDIT_SECONDS: u64 = 15;
    const STATUS_SECONDS: u64 = 60 * 5;

    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            last_inbox: now
                .checked_sub(Duration::from_secs(Self::REDDIT_SECONDS * 2))
                .unwrap(),
            last_subreddits: now
                .checked_sub(Duration::from_secs(Self::REDDIT_SECONDS * 2))
                .unwrap(),
            last_status: now
                .checked_sub(Duration::from_secs(Self::STATUS_SECONDS * 2))
                .unwrap(),
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
        let status = now
            .checked_duration_since(self.last_status)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        let least = min_many!(subreddits, inbox, status);

        if status >= Self::STATUS_SECONDS && least >= 5 {
            Rate::StatusReady
        } else if subreddits >= Self::REDDIT_SECONDS && least >= 5 {
            Rate::SubredditsReady
        } else if inbox >= Self::REDDIT_SECONDS && least >= 5 {
            Rate::InboxReady
        } else {
            let reddit_max = std::cmp::max(inbox, subreddits);

            let reddit_secs = if reddit_max >= Self::REDDIT_SECONDS {
                5
            } else {
                Self::REDDIT_SECONDS - reddit_max
            };

            let status_secs = if status >= Self::STATUS_SECONDS {
                5
            } else {
                Self::STATUS_SECONDS - status
            };

            let next = std::cmp::min(reddit_secs, status_secs);

            let secs = if least < 5 {
                std::cmp::max(next, 5)
            } else {
                next
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

    pub fn set_status(&mut self) {
        self.last_status = Instant::now();
    }
}
