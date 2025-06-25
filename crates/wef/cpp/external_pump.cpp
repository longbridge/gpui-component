#include "external_pump.h"

#include <climits>

#include "include/cef_app.h"

// Special timer delay placeholder value. Intentionally 32-bit for Windows and
// OS X platform API compatibility.
const int32_t TIMER_DELAY_PLACE_HOLDER = INT_MAX;

// The maximum number of milliseconds we're willing to wait between calls to
// DoWork().
const int64_t MAX_TIMER_DELAY = 1000 / 60;  // 60fps

void ExternalPump::OnScheduleWork(int64_t delay_ms) {
  if (delay_ms == TIMER_DELAY_PLACE_HOLDER && IsTimerPending()) {
    // Don't set the maximum timer requested from DoWork() if a timer event is
    // currently pending.
    return;
  }

  KillTimer();

  if (delay_ms <= 0) {
    // Execute the work immediately.
    DoWork();
  } else {
    // Never wait longer than the maximum allowed time.
    if (delay_ms > MAX_TIMER_DELAY) {
      delay_ms = MAX_TIMER_DELAY;
    }

    // Results in call to OnTimerTimeout() after the specified delay.
    SetTimer(delay_ms);
  }
}

void ExternalPump::OnTimerTimeout() {
  KillTimer();
  DoWork();
}

void ExternalPump::DoWork() {
  const bool was_reentrant = PerformMessageLoopWork();
  if (was_reentrant) {
    // Execute the remaining work as soon as possible.
    OnScheduleMessagePumpWork(0);
  } else if (!IsTimerPending()) {
    // Schedule a timer event at the maximum allowed time. This may be dropped
    // in OnScheduleWork() if another timer event is already in-flight.
    OnScheduleMessagePumpWork(TIMER_DELAY_PLACE_HOLDER);
  }
}

bool ExternalPump::PerformMessageLoopWork() {
  if (is_active_) {
    // When CefDoMessageLoopWork() is called there may be various callbacks
    // (such as paint and IPC messages) that result in additional calls to this
    // method. If re-entrancy is detected we must repost a request again to the
    // owner thread to ensure that the discarded call is executed in the future.
    reentrancy_detected_ = true;
    return false;
  }

  reentrancy_detected_ = false;

  is_active_ = true;
  CefDoMessageLoopWork();
  is_active_ = false;

  // |reentrancy_detected_| may have changed due to re-entrant calls to this
  // method.
  return reentrancy_detected_;
}