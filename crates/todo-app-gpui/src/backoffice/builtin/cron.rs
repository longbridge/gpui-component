use crate::{
    backoffice::{builtin::ticker::Tick, cross_runtime::CrossRuntimeBridge},
    xbus::Subscription,
};
use actix::prelude::*;

#[derive(Default, Debug)]
pub struct JobRegistry {
    subscription: Option<Subscription>,
}

impl JobRegistry {
    pub fn global() -> Addr<Self> {
        Self::from_registry()
    }
}

impl Handler<Tick> for JobRegistry {
    type Result = ();

    fn handle(&mut self, msg: Tick, _ctx: &mut Self::Context) -> Self::Result {
        tracing::info!("JobRegistry received tick: {}", msg.0);
        // Here you can implement the logic to handle the tick event
        // For example, you might want to trigger some jobs or tasks
    }
}

impl Supervised for JobRegistry {
    fn restarting(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("JobRegistry is restarting");
    }
}

impl SystemService for JobRegistry {}

impl Actor for JobRegistry {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!("JobRegistry started");
        let addr = ctx.address();
        self.subscription = Some(CrossRuntimeBridge::global().subscribe(move |tick: &Tick| {
            addr.try_send(tick.clone()).unwrap_or_else(|err| {
                tracing::error!("Failed to send tick to JobRegistry: {}", err);
            })
        }));
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("JobRegistry stopped");
    }
}
