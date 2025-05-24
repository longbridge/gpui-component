#pragma once

#include <iostream>

#include "external_pump.h"
#include "include/cef_app.h"
#include "include/wrapper/cef_message_router.h"
#include "utils.h"

class WefApp : public CefApp, public CefBrowserProcessHandler {
  IMPLEMENT_REFCOUNTING(WefApp);

 private:
  std::unique_ptr<ExternalPump> external_pump_;

 public:
  WefApp() : external_pump_(ExternalPump::Create()) {}

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
    external_pump_->OnScheduleMessagePumpWork(delay_ms);
  }
};
