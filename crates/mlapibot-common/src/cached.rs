use std::time::{Duration, Instant};

pub struct Cached<T, Ctx, Err> {
    data: T,
    last: Instant,
    cache_for: Duration,
    func: fn(&Ctx) -> Result<T, Err>,
}

impl<T, Ctx, Err> Cached<T, Ctx, Err> {
    pub fn new(
        cache_for: Duration,
        ctx: &Ctx,
        func: fn(&Ctx) -> Result<T, Err>,
    ) -> Result<Self, Err> {
        func(ctx).map(|data| {
            let last = Instant::now();

            Self {
                data,
                last,
                cache_for,
                func,
            }
        })
    }

    pub fn flush(&mut self, ctx: &Ctx) -> Result<(), Err> {
        self.data = (self.func)(ctx)?;
        self.last = Instant::now();
        Ok(())
    }

    pub fn data(&mut self, ctx: &Ctx) -> Result<&T, Err> {
        if self.last.elapsed() > self.cache_for {
            self.flush(ctx)?;
        }

        Ok(&self.data)
    }
}
