use std::{collections::BinaryHeap, time::Instant};

use job::RateJob;
use task::RateTask;

mod job;
mod task;

pub struct Ratelimiter<Ctx> {
    jobs: BinaryHeap<RateJob<Ctx>>,
}

impl<Ctx> Ratelimiter<Ctx> {
    pub fn new() -> Self {
        Self {
            jobs: BinaryHeap::new(),
        }
    }

    pub fn push<T>(&mut self, name: &'static str, task: T)
    where
        T: RateTask<Ctx> + 'static,
    {
        let job = RateJob::new(name, task);
        self.jobs.push(job);
    }

    pub fn run_immediately(&mut self, name: &str) {
        let mut refill = Vec::new();
        while let Some(mut job) = self.jobs.pop() {
            if job.name() == name {
                job.set_run_now();
            }

            refill.push(job);
        }

        for job in refill {
            self.jobs.push(job);
        }
    }

    /// Runs all pending tasks (if any), returning the Instant when the next
    /// task is due to be run.
    pub fn run(&mut self, ctx: &mut Ctx, now: Instant) -> Instant {
        if let Some(peek) = self.jobs.peek() {
            // Short circuit if none are ready.
            if peek.next() > now {
                return peek.next();
            }
        }

        let mut next: Option<Instant> = None;
        let mut refill = Vec::new();

        while let Some(mut job) = self.jobs.pop() {
            let job_next = job.next();
            if job_next > now {
                // This job isn't ready to run, which means the rest aren't either
                self.jobs.push(job);

                match next {
                    Some(nxt) => next = Some(nxt.min(job_next)),
                    None => next = Some(job_next),
                }

                break;
            }

            println!("[ratelimit] running {}", job.name());
            let job_next = job.run(ctx);

            match next {
                Some(nxt) => next = Some(nxt.min(job_next)),
                None => next = Some(job_next),
            }

            refill.push(job);
        }

        for job in refill {
            self.jobs.push(job);
        }

        next.unwrap_or_else(|| Instant::now())
    }
}
