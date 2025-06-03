#include "client.h"

#include <iostream>

#include "include/base/cef_bind.h"
#include "include/base/cef_callback.h"
#include "include/cef_browser.h"
#include "include/cef_task.h"
#include "include/wrapper/cef_closure_task.h"
#include "wef.h"

WefClient::WefClient(WefBrowser* wef_browser, float device_scale_factor,
                     int width, int height, BrowserCallbacks callbacks,
                     void* userdata, DestroyFn destroy_userdata)
    : wef_browser_(wef_browser),
      width_(width),
      height_(height),
      device_scale_factor_(device_scale_factor),
      callbacks_(callbacks),
      userdata_(userdata),
      destroy_userdata_(destroy_userdata) {}

WefClient::~WefClient() { destroy_userdata_(userdata_); }

void WefClient::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
  if (wef_browser_->deleteBrowser) {
    CefPostTask(TID_UI, base::BindOnce(&CefBrowserHost::CloseBrowser,
                                       browser->GetHost(), false));
  }

  CefMessageRouterConfig config;
  message_router_ = CefMessageRouterBrowserSide::Create(config);
  message_router_->AddHandler(this, false);

  wef_browser_->browser = browser;

  if (!wef_browser_->url.empty()) {
    browser->GetMainFrame()->LoadURL(wef_browser_->url);
  }

  callbacks_.on_created(userdata_);
}

void WefClient::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
  message_router_->OnBeforeClose(browser);
  delete wef_browser_;
}
