use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{AsyncApp, Bounds, Context, Pixels, Point, Task, WeakEntity, px};

/// Manages timer-based auto-scrolling during drag-selection.
///
/// Delta convention: positive = towards bottom, negative = towards top.
pub struct AutoScroll {
    /// Shared between the main thread and the background task.
    /// Writing `None` is the stop signal; the task exits on its next tick.
    shared: Arc<Mutex<Option<Pixels>>>,
    task: Option<Task<()>>,
    /// Last drag position, used to re-extend selection after each scroll step.
    pub last_drag_position: Option<Point<Pixels>>,
}

impl Default for AutoScroll {
    fn default() -> Self {
        Self {
            shared: Arc::new(Mutex::new(None)),
            task: None,
            last_drag_position: None,
        }
    }
}

impl AutoScroll {
    /// The current scroll delta. Positive = towards bottom.
    pub fn delta(&self) -> Option<Pixels> {
        *self.shared.lock().unwrap()
    }

    /// Compute the scroll delta for a mouse Y position within the given viewport bounds.
    /// Returns positive when near the bottom edge, negative near the top, `None` in the dead zone.
    pub fn compute_delta(y: Pixels, bounds: Bounds<Pixels>) -> Option<Pixels> {
        const DEAD_ZONE: f32 = 16.0;
        const MIN_SPEED: f32 = 15.0;
        const MAX_SPEED: f32 = 80.0;
        const RAMP_DISTANCE: f32 = 200.0;

        let top_trigger = bounds.top() + px(DEAD_ZONE);
        let bottom_trigger = bounds.bottom() - px(DEAD_ZONE);

        if y > bottom_trigger {
            let dist = y - bottom_trigger;
            let t = (dist / px(RAMP_DISTANCE)).min(1.0);
            Some(px(MIN_SPEED + t * (MAX_SPEED - MIN_SPEED)))
        } else if y < top_trigger {
            let dist = top_trigger - y;
            let t = (dist / px(RAMP_DISTANCE)).min(1.0);
            Some(px(-(MIN_SPEED + t * (MAX_SPEED - MIN_SPEED))))
        } else {
            None
        }
    }

    /// Update the scroll delta and (re)start the background task if needed.
    ///
    /// `tick` is called each frame (~60 fps) with the current delta.
    /// It should perform the actual scroll action for this entity.
    pub fn set<T, F>(&mut self, delta: Option<Pixels>, cx: &mut Context<T>, tick: F)
    where
        T: 'static,
        F: Fn(Pixels, &mut T, &mut Context<T>) + Send + 'static,
    {
        let was_idle = self.task.is_none();
        *self.shared.lock().unwrap() = delta;

        if delta.is_none() {
            self.task = None;
            return;
        }

        if was_idle {
            let shared = Arc::clone(&self.shared);
            self.task = Some(cx.spawn(Self::task_loop(shared, tick)));
        }
    }

    fn task_loop<T, F>(
        shared: Arc<Mutex<Option<Pixels>>>,
        tick: F,
    ) -> impl AsyncFnOnce(WeakEntity<T>, &mut AsyncApp) + 'static
    where
        T: 'static,
        F: Fn(Pixels, &mut T, &mut Context<T>) + Send + 'static,
    {
        async move |this: WeakEntity<T>, cx: &mut AsyncApp| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let Some(d) = *shared.lock().unwrap() else {
                    break;
                };
                let alive = this
                    .update(cx, |state, cx| {
                        tick(d, state, cx);
                        true
                    })
                    .unwrap_or(false);
                if !alive {
                    break;
                }
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.delta().is_some()
    }

    pub fn stop(&mut self) {
        *self.shared.lock().unwrap() = None;
        self.task = None;
        self.last_drag_position = None;
    }
}
