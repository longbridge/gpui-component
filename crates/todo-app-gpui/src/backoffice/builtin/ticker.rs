use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use actix::prelude::*;
use std::time::Duration;

#[derive(Message, Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
#[rtype(result = "()")]
pub struct Tick(pub u64);

#[derive(Default, Debug)]
pub struct Ticker;

impl Ticker {
    pub fn global() -> Addr<Self> {
        Self::from_registry()
    }
}

impl Supervised for Ticker {
    fn restarting(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("Ticker is restarting");
    }
}

impl SystemService for Ticker {}

impl Actor for Ticker {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!("Ticker started");
        ctx.run_interval(Duration::from_secs(1), |_ticker, _ctx| {
            let timestamp = chrono::Utc::now().timestamp_millis() as u64;
            CrossRuntimeBridge::global().emit(Tick(timestamp));
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("Ticker stopped");
    }
}
