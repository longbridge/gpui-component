use std::ffi::c_void;

/// Represents a handler for application events.
#[allow(unused_variables)]
pub trait ApplicationHandler: Send + Sync {
    /// Called from any thread when work has been scheduled for the browser
    /// process main (UI) thread.
    ///
    /// This callback is used in combination with [`crate::do_message_work`] in
    /// cases where the CEF message loop must be integrated into an existing
    /// application message loop.
    ///
    /// This callback should schedule a [`crate::do_message_work`] call to
    /// happen on the main (UI) thread.
    ///
    /// `delay_ms` is the requested delay in milliseconds. If `delay_ms` is <= 0
    /// then the call should happen reasonably soon. If `delay_ms` is > 0
    /// then the call should be scheduled to happen after the specified
    /// delay and any currently pending scheduled call should be cancelled.
    fn on_schedule_message_pump_work(&mut self, delay_ms: i32) {}
}

impl ApplicationHandler for () {}

pub(crate) struct ApplicationState<T> {
    pub(crate) handler: T,
}

pub(crate) extern "C" fn on_schedule_message_pump_work<T: ApplicationHandler>(
    userdata: *mut c_void,
    delay_ms: i32,
) {
    unsafe {
        let state = &mut *(userdata as *mut ApplicationState<T>);
        state.handler.on_schedule_message_pump_work(delay_ms);
    }
}
