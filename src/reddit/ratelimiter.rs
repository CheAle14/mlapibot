use std::time::{Duration, Instant};

use ord_many::{max_many, min_many};
use statuspage::status::StatusIndicator;

pub struct Ratelimiter {
    last_inbox: Instant,
    last_subreddits: Instant,
    last_status: Instant,
    last_downvotes: Instant,

    last_webhook: Instant,
}

pub enum Rate {
    NoneReadyFor(Duration),
    StatusReady,
    InboxReady,
    SubredditsReady,
    DownvotesReady,

    WebhookCheck,
}

impl Ratelimiter {
    const REDDIT_SECONDS: u64 = 15;
    const STATUS_SECONDS: u64 = 60 * 5;
    const REDDIT_DELAY: u64 = 6;
    const ENSURE_WEBHOOK_CHECKED: u64 = 30;

    const fn delay_for(status: StatusIndicator) -> u64 {
        match status {
            StatusIndicator::None | StatusIndicator::Maintenance => Self::STATUS_SECONDS,
            StatusIndicator::Minor => 180,
            StatusIndicator::Major => 120,
            StatusIndicator::Critical => 60,
        }
    }

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
            last_downvotes: now
                .checked_sub(Duration::from_secs(Self::REDDIT_SECONDS * 2))
                .unwrap(),
            last_webhook: now
                .checked_sub(Duration::from_secs(Self::ENSURE_WEBHOOK_CHECKED * 2))
                .unwrap(),
        }
    }

    pub fn get(&self, current_status: StatusIndicator) -> Rate {
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
        let downvotes = now
            .checked_duration_since(self.last_downvotes)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        let webhook = now
            .checked_duration_since(self.last_webhook)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        let least = min_many!(subreddits, inbox, status, downvotes);

        if webhook >= Self::ENSURE_WEBHOOK_CHECKED {
            Rate::WebhookCheck
        } else if status >= Self::delay_for(current_status) && least >= Self::REDDIT_DELAY {
            Rate::StatusReady
        } else if subreddits >= Self::REDDIT_SECONDS && least >= Self::REDDIT_DELAY {
            Rate::SubredditsReady
        } else if inbox >= Self::REDDIT_SECONDS && least >= Self::REDDIT_DELAY {
            Rate::InboxReady
        } else if downvotes >= Self::REDDIT_SECONDS && least >= Self::REDDIT_DELAY {
            Rate::DownvotesReady
        } else {
            let reddit_max = max_many!(inbox, subreddits, downvotes);

            let reddit_secs = if reddit_max >= Self::REDDIT_SECONDS {
                Self::REDDIT_DELAY
            } else {
                Self::REDDIT_SECONDS - reddit_max
            };

            let status_secs = if status >= Self::STATUS_SECONDS {
                Self::REDDIT_DELAY
            } else {
                Self::STATUS_SECONDS - status
            };

            let next = std::cmp::min(reddit_secs, status_secs);

            let secs = if least < Self::REDDIT_DELAY {
                std::cmp::max(next, Self::REDDIT_DELAY)
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

    pub fn set_downvotes(&mut self) {
        self.last_downvotes = Instant::now();
    }

    pub fn set_webhook(&mut self) {
        self.last_webhook = Instant::now();
    }
}
