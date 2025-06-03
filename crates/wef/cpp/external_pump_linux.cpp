#include <errno.h>
#include <fcntl.h>
#include <glib.h>
#include <math.h>

#include <memory>

#include "external_pump.h"
#include "include/base/cef_logging.h"
#include "include/cef_app.h"

#if !defined(HANDLE_EINTR)
#if !DCHECK_IS_ON()

#define HANDLE_EINTR(x)                                     \
  ({                                                        \
    decltype(x) eintr_wrapper_result;                       \
    do {                                                    \
      eintr_wrapper_result = (x);                           \
    } while (eintr_wrapper_result == -1 && errno == EINTR); \
    eintr_wrapper_result;                                   \
  })

#else

#define HANDLE_EINTR(x)                                      \
  ({                                                         \
    int eintr_wrapper_counter = 0;                           \
    decltype(x) eintr_wrapper_result;                        \
    do {                                                     \
      eintr_wrapper_result = (x);                            \
    } while (eintr_wrapper_result == -1 && errno == EINTR && \
             eintr_wrapper_counter++ < 100);                 \
    eintr_wrapper_result;                                    \
  })

#endif  // !DCHECK_IS_ON()
#endif  // !defined(HANDLE_EINTR)

class ExternalPumpLinux : public ExternalPump {
 public:
  ExternalPumpLinux();
  ~ExternalPumpLinux();

  // MainMessageLoopExternalPump methods:
  void OnScheduleMessagePumpWork(int64_t delay_ms) override;

  int HandlePrepare();
  bool HandleCheck();
  void HandleDispatch();

 protected:
  void SetTimer(int64_t delay_ms) override;
  void KillTimer() override;
  bool IsTimerPending() override;

 private:
  GMainContext* context_;
  GSource* work_source_;
  CefTime delayed_work_time_;
  int wakeup_pipe_read_;
  int wakeup_pipe_write_;
  std::unique_ptr<GPollFD> wakeup_gpollfd_;
};

// Return a timeout suitable for the glib loop, -1 to block forever,
// 0 to return right away, or a timeout in milliseconds from now.
int GetTimeIntervalMilliseconds(const CefTime& from) {
  if (from.GetDoubleT() == 0.0) {
    return -1;
  }

  CefTime now;
  now.Now();

  // Be careful here. CefTime has a precision of microseconds, but we want a
  // value in milliseconds. If there are 5.5ms left, should the delay be 5 or
  // 6?  It should be 6 to avoid executing delayed work too early.
  int delay =
      static_cast<int>(ceil((from.GetDoubleT() - now.GetDoubleT()) * 1000.0));

  // If this value is negative, then we need to run delayed work soon.
  return delay < 0 ? 0 : delay;
}

struct WorkSource : public GSource {
  ExternalPumpLinux* pump;
};

gboolean WorkSourcePrepare(GSource* source, gint* timeout_ms) {
  *timeout_ms = static_cast<WorkSource*>(source)->pump->HandlePrepare();
  // We always return FALSE, so that our timeout is honored.  If we were
  // to return TRUE, the timeout would be considered to be 0 and the poll
  // would never block.  Once the poll is finished, Check will be called.
  return FALSE;
}

gboolean WorkSourceCheck(GSource* source) {
  // Only return TRUE if Dispatch should be called.
  return static_cast<WorkSource*>(source)->pump->HandleCheck();
}

gboolean WorkSourceDispatch(GSource* source, GSourceFunc unused_func,
                            gpointer unused_data) {
  static_cast<WorkSource*>(source)->pump->HandleDispatch();
  // Always return TRUE so our source stays registered.
  return TRUE;
}

// I wish these could be const, but g_source_new wants non-const.
GSourceFuncs WorkSourceFuncs = {WorkSourcePrepare, WorkSourceCheck,
                                WorkSourceDispatch, nullptr};

ExternalPumpLinux::ExternalPumpLinux()
    : context_(g_main_context_default()), wakeup_gpollfd_(new GPollFD) {
  // Create our wakeup pipe, which is used to flag when work was scheduled.
  int fds[2];
  int ret = pipe(fds);
  DCHECK_EQ(ret, 0);
  (void)ret;  // Prevent warning in release mode.

  wakeup_pipe_read_ = fds[0];
  wakeup_pipe_write_ = fds[1];
  wakeup_gpollfd_->fd = wakeup_pipe_read_;
  wakeup_gpollfd_->events = G_IO_IN;

  work_source_ = g_source_new(&WorkSourceFuncs, sizeof(WorkSource));
  static_cast<WorkSource*>(work_source_)->pump = this;
  g_source_add_poll(work_source_, wakeup_gpollfd_.get());
  // Use a low priority so that we let other events in the queue go first.
  g_source_set_priority(work_source_, G_PRIORITY_DEFAULT_IDLE);
  // This is needed to allow Run calls inside Dispatch.
  g_source_set_can_recurse(work_source_, TRUE);
  g_source_attach(work_source_, context_);
}

ExternalPumpLinux::~ExternalPumpLinux() {
  g_source_destroy(work_source_);
  g_source_unref(work_source_);
  close(wakeup_pipe_read_);
  close(wakeup_pipe_write_);
}

void ExternalPumpLinux::OnScheduleMessagePumpWork(int64_t delay_ms) {
  // This can be called on any thread, so we don't want to touch any state
  // variables as we would then need locks all over. This ensures that if we
  // are sleeping in a poll that we will wake up.
  if (HANDLE_EINTR(write(wakeup_pipe_write_, &delay_ms, sizeof(int64_t))) !=
      sizeof(int64_t)) {
    NOTREACHED() << "Could not write to the UI message loop wakeup pipe!";
  }
}

// Return the timeout we want passed to poll.
int ExternalPumpLinux::HandlePrepare() {
  // We don't think we have work to do, but make sure not to block longer than
  // the next time we need to run delayed work.
  return GetTimeIntervalMilliseconds(delayed_work_time_);
}

bool ExternalPumpLinux::HandleCheck() {
  // We usually have a single message on the wakeup pipe, since we are only
  // signaled when the queue went from empty to non-empty, but there can be
  // two messages if a task posted a task, hence we read at most two bytes.
  // The glib poll will tell us whether there was data, so this read shouldn't
  // block.
  if (wakeup_gpollfd_->revents & G_IO_IN) {
    int64_t delay_ms[2];
    const size_t num_bytes =
        HANDLE_EINTR(read(wakeup_pipe_read_, delay_ms, sizeof(int64_t) * 2));
    if (num_bytes < sizeof(int64_t)) {
      NOTREACHED() << "Error reading from the wakeup pipe.";
    }
    if (num_bytes == sizeof(int64_t)) {
      OnScheduleWork(delay_ms[0]);
    }
    if (num_bytes == sizeof(int64_t) * 2) {
      OnScheduleWork(delay_ms[1]);
    }
  }

  if (GetTimeIntervalMilliseconds(delayed_work_time_) == 0) {
    // The timer has expired. That condition will stay true until we process
    // that delayed work, so we don't need to record this differently.
    return true;
  }

  return false;
}

void ExternalPumpLinux::HandleDispatch() { OnTimerTimeout(); }

void ExternalPumpLinux::SetTimer(int64_t delay_ms) {
  DCHECK_GT(delay_ms, 0);

  CefTime now;
  now.Now();

  delayed_work_time_ =
      CefTime(now.GetDoubleT() + static_cast<double>(delay_ms) / 1000.0);
}

void ExternalPumpLinux::KillTimer() { delayed_work_time_ = CefTime(); }

bool ExternalPumpLinux::IsTimerPending() {
  return GetTimeIntervalMilliseconds(delayed_work_time_) > 0;
}

// static
std::unique_ptr<ExternalPump> ExternalPump::Create() {
  return std::make_unique<ExternalPumpLinux>();
}
