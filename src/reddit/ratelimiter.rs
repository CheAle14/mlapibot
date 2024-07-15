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
        let inbox = now
            .checked_duration_since(self.last_inbox)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        let subreddits = now
            .checked_duration_since(self.last_subreddits)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        if inbox >= Self::REDDIT_SECONDS && subreddits >= 5 {
            Rate::InboxReady
        } else if subreddits >= Self::REDDIT_SECONDS && inbox >= 5 {
            Rate::SubredditsReady
        } else {
            let max = std::cmp::max(inbox, subreddits);

            if max >= Self::REDDIT_SECONDS {
                Rate::NoneReadyFor(Duration::from_secs(5))
            } else {
                Rate::NoneReadyFor(Duration::from_secs(Self::REDDIT_SECONDS - max))
            }
        }
    }

    pub fn set_inbox(&mut self) {
        self.last_inbox = Instant::now();
    }

    pub fn set_subreddits(&mut self) {
        self.last_subreddits = Instant::now();
    }
}
