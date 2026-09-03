//! `#[gpui_kit::test]` is GPUI's test attribute reached through the umbrella
//! crate; it needs the `test-support` feature.

use gpui_kit::*;

struct Counter(u32);

#[gpui_kit::test]
fn entity_round_trip(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter(1));
    counter.update(cx, |counter, _| counter.0 += 1);
    assert_eq!(counter.read_with(cx, |counter, _| counter.0), 2);
}

#[gpui_kit::test]
async fn async_test_runs(cx: &mut TestAppContext) {
    let counter = cx.new(|_| Counter(0));
    cx.background_executor.run_until_parked();
    assert_eq!(counter.read_with(cx, |counter, _| counter.0), 0);
}
