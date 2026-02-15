use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use gpui::App;

/// A simple debouncer that delays execution until a period of inactivity.
///
/// When `debounce` is called, it waits for the delay period and returns `true`
/// only if no newer calls have been made. This allows the caller to decide
/// whether to proceed with the action.
///
/// # Example
///
/// ```ignore
/// let debouncer = Arc::new(Debouncer::new(Duration::from_millis(250)));
///
/// cx.spawn(async move |view, cx| {
///     if debouncer.debounce().await {
///         // This runs after 250ms of inactivity
///         let _ = view.update(cx, |this, cx| {
///             this.do_something(cx);
///         });
///     }
/// }).detach();
/// ```
pub struct Debouncer {
    delay: Duration,
    seq: AtomicU64,
}

impl Debouncer {
    /// Create a new debouncer with the specified delay.
    pub fn new(delay: Duration) -> Self {
        Debouncer {
            delay,
            seq: AtomicU64::new(0),
        }
    }

    /// Wait for the delay and return whether this call should proceed.
    ///
    /// Returns `true` if no newer calls have been made during the delay,
    /// `false` otherwise (meaning a newer call has superseded this one).
    pub async fn debounce(&self, cx: &mut App) -> bool {
        // Increment sequence number to invalidate any pending calls
        let my_seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;

        // Wait for the delay
        cx.background_executor().timer(self.delay).await;

        // Only return true if no newer call has been made
        self.seq.load(Ordering::SeqCst) == my_seq
    }
}
