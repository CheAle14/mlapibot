use std::time::Duration;

pub type RunAfter = Duration;

pub trait RateTask<Ctx> {
    fn run(&self, ctx: &mut Ctx) -> anyhow::Result<RunAfter>;
}

impl<F, Ctx> RateTask<Ctx> for F
where
    F: Fn(&mut Ctx) -> anyhow::Result<RunAfter>,
{
    fn run(&self, ctx: &mut Ctx) -> anyhow::Result<RunAfter> {
        (self)(ctx)
    }
}
