#include <windows.h>

#include "external_pump.h"

static const int MSG_HAVE_WORK = WM_USER + 1;

class ExternalPumpWin : public ExternalPump {
 public:
  ExternalPumpWin() {
    HINSTANCE hInstance = GetModuleHandle(nullptr);
    const char* const className = "CEFMainTargetHWND";

    WNDCLASSEX wcex = {};
    wcex.cbSize = sizeof(WNDCLASSEX);
    wcex.lpfnWndProc = WndProc;
    wcex.hInstance = hInstance;
    wcex.lpszClassName = className;
    RegisterClassEx(&wcex);

    main_thread_target_ =
        CreateWindow(className, nullptr, WS_OVERLAPPEDWINDOW, 0, 0, 0, 0,
                     HWND_MESSAGE, nullptr, hInstance, nullptr);

    SetWindowLongPtr(main_thread_target_, GWLP_USERDATA,
                     reinterpret_cast<LONG_PTR>(this));
  }

  ~ExternalPumpWin() override {
    KillTimer();
    if (main_thread_target_) {
      DestroyWindow(main_thread_target_);
    }
  }

  void OnScheduleMessagePumpWork(int64_t delay_ms) override {
    // This method may be called on any thread.
    PostMessage(main_thread_target_, MSG_HAVE_WORK, 0,
                static_cast<LPARAM>(delay_ms));
  }

 protected:
  void SetTimer(int64_t delay_ms) override {
    timer_pending_ = true;
    ::SetTimer(main_thread_target_, 1, static_cast<UINT>(delay_ms), nullptr);
  }

  void KillTimer() override {
    if (timer_pending_) {
      ::KillTimer(main_thread_target_, 1);
      timer_pending_ = false;
    }
  }

  bool IsTimerPending() override { return timer_pending_; }

 private:
  static LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wparam,
                                  LPARAM lparam) {
    if (msg == WM_TIMER || msg == MSG_HAVE_WORK) {
      ExternalPumpWin* message_loop = reinterpret_cast<ExternalPumpWin*>(
          GetWindowLongPtr(hwnd, GWLP_USERDATA));
      if (msg == MSG_HAVE_WORK) {
        const int64_t delay_ms = static_cast<int64_t>(lparam);
        message_loop->OnScheduleWork(delay_ms);
      } else {
        message_loop->OnTimerTimeout();
      }
    }
    return DefWindowProc(hwnd, msg, wparam, lparam);
  }

  // True if a timer event is currently pending.
  bool timer_pending_ = false;

  // HWND owned by the thread that CefDoMessageLoopWork should be invoked on.
  HWND main_thread_target_ = nullptr;
};

std::unique_ptr<ExternalPump> ExternalPump::Create() {
  return std::make_unique<ExternalPumpWin>();
}
