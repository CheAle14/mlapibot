use std::{
    pin::Pin,
    time::{Duration, Instant},
};

pub struct Cached<T, Ctx, Err> {
    data: T,
    last: Instant,
    cache_for: Duration,
    func: HydrateFn<Ctx, T, Err>,
}

impl<T, Ctx, Err> Cached<T, Ctx, Err> {
    pub async fn new(
        cache_for: Duration,
        ctx: &Ctx,
        func: HydrateFn<Ctx, T, Err>,
    ) -> Result<Self, Err> {
        let data = func(ctx).await?;
        let last = Instant::now();

        Ok(Self {
            data,
            last,
            cache_for,
            func,
        })
    }

    pub async fn flush(&mut self, ctx: &Ctx) -> Result<(), Err> {
        self.data = (self.func)(ctx).await?;
        self.last = Instant::now();
        Ok(())
    }

    pub async fn data(&mut self, ctx: &Ctx) -> Result<&T, Err> {
        if self.last.elapsed() > self.cache_for {
            self.flush(ctx).await?;
        }

        Ok(&self.data)
    }
}

type HydrateFn<Ctx, T, Err> =
    for<'a> fn(&'a Ctx) -> Pin<Box<dyn Future<Output = Result<T, Err>> + 'a>>;

pub struct LazyCached<T, Ctx, Err> {
    data: Option<T>,
    last: Instant,
    cache_for: Duration,
    func: HydrateFn<Ctx, T, Err>,
}

impl<T, Ctx, Err> LazyCached<T, Ctx, Err> {
    pub fn new(cache_for: Duration, func: HydrateFn<Ctx, T, Err>) -> Self {
        Self {
            data: None,
            last: Instant::now(),
            cache_for,
            func,
        }
    }

    pub async fn flush(&mut self, ctx: &Ctx) -> Result<(), Err> {
        self.data = Some((self.func)(ctx).await?);
        self.last = Instant::now();
        Ok(())
    }

    pub async fn data(&mut self, ctx: &Ctx) -> Result<&T, Err> {
        if self.data.is_none() || self.last.elapsed() > self.cache_for {
            self.flush(ctx).await?;
        }

        Ok(self.data.as_ref().unwrap())
    }
}
