use std::time::{Duration, Instant};

use super::task::RateTask;

pub struct RateJob<Ctx> {
    task: Box<dyn RateTask<Ctx>>,
    name: &'static str,
    next: Instant,
    failures: u32,
}

impl<Ctx> RateJob<Ctx> {
    pub fn new<T>(name: &'static str, task: T) -> Self
    where
        T: RateTask<Ctx> + 'static,
    {
        let task = Box::new(task);
        Self {
            task,
            name,
            next: Instant::now(),
            failures: 0,
        }
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn mark_failed(&mut self) {
        self.failures = self.failures.saturating_add(1);
    }

    pub fn mark_success(&mut self) {
        self.failures = self.failures.saturating_sub(2);
    }

    pub fn next(&self) -> Instant {
        self.next
    }

    pub fn set_run_now(&mut self) {
        self.next = Instant::now();
    }

    pub fn set_next_time(&mut self, requested_run_after: Duration) {
        let duration = if self.failures == 0 {
            requested_run_after
        } else {
            requested_run_after + Duration::from_secs(2u64.pow(self.failures))
        };

        self.next = Instant::now()
            .checked_add(duration)
            .unwrap_or_else(|| Instant::now());
    }

    pub fn run(&mut self, ctx: &mut Ctx) -> Instant {
        match self.task.run(ctx) {
            Ok(dur) => {
                self.mark_success();
                self.set_next_time(dur)
            }
            Err(err) => {
                self.mark_failed();
                eprintln!(
                    "[ratelimit] {} failed (#{}): {err:?}",
                    self.name, self.failures
                );

                self.set_next_time(Duration::from_secs(1))
            }
        };

        self.next
    }
}

impl<Ctx> PartialEq for RateJob<Ctx> {
    fn eq(&self, other: &Self) -> bool {
        self.next == other.next
    }
}

impl<Ctx> Eq for RateJob<Ctx> {}

impl<Ctx> Ord for RateJob<Ctx> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.next.cmp(&self.next)
    }
}

impl<Ctx> PartialOrd for RateJob<Ctx> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
