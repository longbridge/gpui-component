#pragma once

#include <iostream>

#include "include/cef_app.h"
#include "include/wrapper/cef_message_router.h"

class WefRenderProcessHandler : public CefRenderProcessHandler {
  IMPLEMENT_REFCOUNTING(WefRenderProcessHandler);

 private:
  CefRefPtr<CefMessageRouterRendererSide> message_router_;
  std::map<int, CefString> inject_javascript_map_;

 public:
  WefRenderProcessHandler() {
    CefMessageRouterConfig config;
    message_router_ = CefMessageRouterRendererSide::Create(config);
  }

  void OnBrowserCreated(CefRefPtr<CefBrowser> browser,
                        CefRefPtr<CefDictionaryValue> extra_info) override {
    auto inject_javascript = extra_info->GetString("__wef_inject_javascript");
    inject_javascript_map_[browser->GetIdentifier()] = inject_javascript;
  }

  void OnBrowserDestroyed(CefRefPtr<CefBrowser> browser) override {
    inject_javascript_map_.erase(browser->GetIdentifier());
  }

  void OnContextCreated(CefRefPtr<CefBrowser> browser,
                        CefRefPtr<CefFrame> frame,
                        CefRefPtr<CefV8Context> context) override {
    if (frame->IsMain()) {
      auto it = inject_javascript_map_.find(browser->GetIdentifier());
      if (it != inject_javascript_map_.end()) {
        auto inject_javascript = it->second;
        if (!inject_javascript.empty()) {
          frame->ExecuteJavaScript(inject_javascript, frame->GetURL(), 0);
        }
      }
    }
    message_router_->OnContextCreated(browser, frame, context);
  }

  void OnContextReleased(CefRefPtr<CefBrowser> browser,
                         CefRefPtr<CefFrame> frame,
                         CefRefPtr<CefV8Context> context) override {
    message_router_->OnContextReleased(browser, frame, context);
  }

  bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                CefRefPtr<CefFrame> frame,
                                CefProcessId source_process,
                                CefRefPtr<CefProcessMessage> message) override {
    return message_router_->OnProcessMessageReceived(browser, frame,
                                                     source_process, message);
  }
};

class WefRenderProcessApp : public CefApp, public CefRenderProcessHandler {
  IMPLEMENT_REFCOUNTING(WefRenderProcessApp);

 private:
  CefRefPtr<CefRenderProcessHandler> render_process_handler_;

 public:
  WefRenderProcessApp()
      : render_process_handler_(new WefRenderProcessHandler()) {}

  CefRefPtr<CefRenderProcessHandler> GetRenderProcessHandler() {
    return render_process_handler_;
  }
};