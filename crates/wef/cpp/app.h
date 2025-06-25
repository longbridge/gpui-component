#pragma once

#include <iostream>

#include "app_callbacks.h"
#include "external_pump.h"
#include "include/cef_app.h"
#include "include/wrapper/cef_message_router.h"
#include "utils.h"

const int64_t MAX_TIMER_DELAY = 1000 / 60;

class WefApp : public CefApp, public CefBrowserProcessHandler {
  IMPLEMENT_REFCOUNTING(WefApp);

 private:
  std::optional<std::unique_ptr<ExternalPump>> external_pump_;
  AppCallbacks callbacks_;
  void* userdata_;
  DestroyFn destroy_userdata_;

 public:
  WefApp(AppCallbacks callbacks, void* userdata, DestroyFn destroy_userdata)
      : callbacks_(callbacks),
        userdata_(userdata),
        destroy_userdata_(destroy_userdata),
        external_pump_(std::make_optional(ExternalPump::Create())) {}

  virtual ~WefApp() {
    if (destroy_userdata_) {
      destroy_userdata_(userdata_);
      userdata_ = nullptr;
    }
  }

  /////////////////////////////////////////////////////////////////
  // CefApp methods
  /////////////////////////////////////////////////////////////////
  virtual void OnBeforeCommandLineProcessing(
      const CefString& process_type,
      CefRefPtr<CefCommandLine> command_line) override {
    if (process_type.empty()) {
      // Use software rendering and compositing (disable GPU) for increased FPS
      // and decreased CPU usage. This will also disable WebGL so remove these
      // switches if you need that capability.
      // See https://github.com/chromiumembedded/cef/issues/1257 for details.
      //
      // NOTE: If GPU rendering is not disabled, sometimes there will be issues
      // with incorrect dimensions when changing the window size.
      command_line->AppendSwitch("disable-gpu");
      command_line->AppendSwitch("disable-gpu-compositing");
    }

#ifdef __APPLE__
    command_line->AppendSwitch("use-mock-keychain");
#endif
  }

  CefRefPtr<CefBrowserProcessHandler> GetBrowserProcessHandler() override {
    return this;
  }

  /////////////////////////////////////////////////////////////////
  // CefBrowserProcessHandler methods
  /////////////////////////////////////////////////////////////////
  bool OnAlreadyRunningAppRelaunch(
      CefRefPtr<CefCommandLine> command_line,
      const CefString& current_directory) override {
    return true;
  }

  void OnScheduleMessagePumpWork(int64_t delay_ms) override {
    if (external_pump_) {
      (*external_pump_)->OnScheduleMessagePumpWork(delay_ms);
    }

    callbacks_.on_schedule_message_pump_work(
        userdata_, static_cast<int>(std::min(delay_ms, MAX_TIMER_DELAY)));
  }
};
