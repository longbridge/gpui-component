#pragma once

#if defined(_WIN32) || defined(_WIN64)
#define NOMINMAX
#endif

#include <iostream>
#include <limits>
#include <memory>
#include <optional>

#include "browser_callbacks.h"
#include "frame.h"
#include "include/cef_browser.h"
#include "include/cef_client.h"
#include "include/wrapper/cef_message_router.h"
#include "utils.h"

struct WefBrowser;

enum class BrowserState {
  Creating,
  Created,
  Closing,
  Closed,
};

class BrowserCallbacksTarget {
 private:
  bool disable_;
  BrowserCallbacks callbacks_;
  void* userdata_;
  DestroyFn destroy_userdata_;

 public:
  BrowserCallbacksTarget(BrowserCallbacks callbacks, void* userdata,
                         DestroyFn destroy_userdata)
      : disable_(false),
        callbacks_(callbacks),
        userdata_(userdata),
        destroy_userdata_(destroy_userdata) {}

  BrowserCallbacksTarget(const BrowserCallbacksTarget& other) = delete;
  BrowserCallbacksTarget& operator=(const BrowserCallbacksTarget& other) =
      delete;

  BrowserCallbacksTarget(BrowserCallbacksTarget&& other)
      : disable_(other.disable_),
        callbacks_(std::move(other.callbacks_)),
        userdata_(other.userdata_),
        destroy_userdata_(other.destroy_userdata_) {
    other.disable_ = true;
    other.userdata_ = nullptr;
    other.destroy_userdata_ = nullptr;
  }

  ~BrowserCallbacksTarget() {
    if (destroy_userdata_) {
      destroy_userdata_(userdata_);
    }
  }

  void disable() { disable_ = true; }

  template <typename Callable>
  void call(Callable func) const {
    if (disable_) {
      return;
    }
    func(callbacks_, userdata_);
  }
};

struct BrowserSharedState {
  bool focus;
  int cursorX, cursorY;
  BrowserState browser_state;
  std::optional<CefRefPtr<CefBrowser>> browser;
  int width, height;
  float device_scale_factor;
  BrowserCallbacksTarget callbacks_target;

  BrowserSharedState(BrowserCallbacksTarget&& other)
      : focus(false),
        cursorX(0),
        cursorY(0),
        browser_state(BrowserState::Creating),
        width(800),
        height(600),
        device_scale_factor(1.0f),
        callbacks_target(std::move(other)) {}

  BrowserSharedState(const BrowserSharedState& other) = delete;
  BrowserSharedState& operator=(const BrowserSharedState& other) = delete;
};

class WefClient : public CefClient,
                  public CefRenderHandler,
                  public CefDisplayHandler,
                  public CefLifeSpanHandler,
                  public CefLoadHandler,
                  public CefDialogHandler,
                  public CefFindHandler,
                  public CefContextMenuHandler,
                  public CefRequestHandler,
                  public CefJSDialogHandler,
                  public CefFocusHandler,
                  public CefPermissionHandler,
                  public CefMessageRouterBrowserSide::Handler {
  IMPLEMENT_REFCOUNTING(WefClient);

 private:
  std::shared_ptr<BrowserSharedState> state_;
  CefRefPtr<CefMessageRouterBrowserSide> message_router_;

 public:
  WefClient(std::shared_ptr<BrowserSharedState> state);

  virtual ~WefClient();

  /////////////////////////////////////////////////////////////////
  // CefClient methods
  /////////////////////////////////////////////////////////////////

  bool GetScreenInfo(CefRefPtr<CefBrowser> browser,
                     CefScreenInfo& screen_info) override {
    screen_info.device_scale_factor = state_->device_scale_factor;
    return true;
  }

  void GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) override {
    rect.Set(0, 0,
             static_cast<int>(static_cast<float>(state_->width) /
                              state_->device_scale_factor),
             static_cast<int>(static_cast<float>(state_->height) /
                              state_->device_scale_factor));
  }

  CefRefPtr<CefRenderHandler> GetRenderHandler() override { return this; }
  CefRefPtr<CefDisplayHandler> GetDisplayHandler() override { return this; }
  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() override { return this; }
  CefRefPtr<CefLoadHandler> GetLoadHandler() override { return this; }
  CefRefPtr<CefDialogHandler> GetDialogHandler() override { return this; }
  CefRefPtr<CefContextMenuHandler> GetContextMenuHandler() override {
    return this;
  }
  CefRefPtr<CefFindHandler> GetFindHandler() override { return this; }
  CefRefPtr<CefJSDialogHandler> GetJSDialogHandler() override { return this; }
  CefRefPtr<CefFocusHandler> GetFocusHandler() override { return this; }
  CefRefPtr<CefPermissionHandler> GetPermissionHandler() override {
    return this;
  }

  bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                CefRefPtr<CefFrame> frame,
                                CefProcessId source_process,
                                CefRefPtr<CefProcessMessage> message) override {
    return message_router_->OnProcessMessageReceived(browser, frame,
                                                     source_process, message);
  }

  /////////////////////////////////////////////////////////////////
  // CefRenderHandler methods
  /////////////////////////////////////////////////////////////////
  void OnPopupShow(CefRefPtr<CefBrowser> browser, bool show) override;
  void OnPopupSize(CefRefPtr<CefBrowser> browser, const CefRect& rect) override;
  void OnPaint(CefRefPtr<CefBrowser> browser, PaintElementType type,
               const RectList& dirtyRects, const void* buffer, int width,
               int height) override;
  void OnImeCompositionRangeChanged(CefRefPtr<CefBrowser> browser,
                                    const CefRange& selected_range,
                                    const RectList& character_bounds) override;
  bool OnCursorChange(CefRefPtr<CefBrowser> browser, CefCursorHandle cursor,
                      cef_cursor_type_t type,
                      const CefCursorInfo& custom_cursor_info) override;

  /////////////////////////////////////////////////////////////////
  // CefDisplayHandler methods
  /////////////////////////////////////////////////////////////////
  void OnAddressChange(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                       const CefString& url) override;
  void OnTitleChange(CefRefPtr<CefBrowser> browser,
                     const CefString& title) override;
  void OnFaviconURLChange(CefRefPtr<CefBrowser> browser,
                          const std::vector<CefString>& icon_urls) override;
  bool OnTooltip(CefRefPtr<CefBrowser> browser, CefString& text) override;
  void OnStatusMessage(CefRefPtr<CefBrowser> browser,
                       const CefString& value) override;
  bool OnConsoleMessage(CefRefPtr<CefBrowser> browser, cef_log_severity_t level,
                        const CefString& message, const CefString& source,
                        int line) override;
  void OnLoadingProgressChange(CefRefPtr<CefBrowser> browser,
                               double progress) override;

  /////////////////////////////////////////////////////////////////
  // CefLifeSpanHandler methods
  /////////////////////////////////////////////////////////////////
  void OnAfterCreated(CefRefPtr<CefBrowser> browser) override;
  bool OnBeforePopup(
      CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, int popup_id,
      const CefString& target_url, const CefString& target_frame_name,
      CefLifeSpanHandler::WindowOpenDisposition target_disposition,
      bool user_gesture, const CefPopupFeatures& popupFeatures,
      CefWindowInfo& windowInfo, CefRefPtr<CefClient>& client,
      CefBrowserSettings& settings, CefRefPtr<CefDictionaryValue>& extra_info,
      bool* no_javascript_access) override;
  bool DoClose(CefRefPtr<CefBrowser> browser) override;
  void OnBeforeClose(CefRefPtr<CefBrowser> browser) override;

  /////////////////////////////////////////////////////////////////
  // CefLoadHandler methods
  /////////////////////////////////////////////////////////////////
  void OnLoadingStateChange(CefRefPtr<CefBrowser> browser, bool isLoading,
                            bool canGoBack, bool canGoForward) override;
  void OnLoadStart(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                   TransitionType transition_type) override;
  void OnLoadEnd(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                 int httpStatusCode) override;
  void OnLoadError(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                   ErrorCode errorCode, const CefString& errorText,
                   const CefString& failedUrl) override;

  /////////////////////////////////////////////////////////////////
  // CefDialogHandler methods
  /////////////////////////////////////////////////////////////////
  bool OnFileDialog(CefRefPtr<CefBrowser> browser, FileDialogMode mode,
                    const CefString& title, const CefString& default_file_path,
                    const std::vector<CefString>& accept_filters,
                    const std::vector<CefString>& accept_extensions,
                    const std::vector<CefString>& accept_descriptions,
                    CefRefPtr<CefFileDialogCallback> callback) override;

  /////////////////////////////////////////////////////////////////
  // CefContextMenuHandler methods
  /////////////////////////////////////////////////////////////////
  bool RunContextMenu(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                      CefRefPtr<CefContextMenuParams> params,
                      CefRefPtr<CefMenuModel> model,
                      CefRefPtr<CefRunContextMenuCallback> callback) override;

  /////////////////////////////////////////////////////////////////
  // CefFindHandler methods
  /////////////////////////////////////////////////////////////////
  void OnFindResult(CefRefPtr<CefBrowser> browser, int identifier, int count,
                    const CefRect& selectionRect, int activeMatchOrdinal,
                    bool finalUpdate) override;

  /////////////////////////////////////////////////////////////////
  // CefJSDialogHandler methods
  /////////////////////////////////////////////////////////////////
  bool OnJSDialog(CefRefPtr<CefBrowser> browser, const CefString& origin_url,
                  JSDialogType dialog_type, const CefString& message_text,
                  const CefString& default_prompt_text,
                  CefRefPtr<CefJSDialogCallback> callback,
                  bool& suppress_message) override;
  bool OnBeforeUnloadDialog(CefRefPtr<CefBrowser> browser,
                            const CefString& message_text, bool is_reload,
                            CefRefPtr<CefJSDialogCallback> callback) override;

  /////////////////////////////////////////////////////////////////
  // CefRequestHandler methods
  /////////////////////////////////////////////////////////////////
  void OnRenderProcessTerminated(CefRefPtr<CefBrowser> browser,
                                 TerminationStatus status, int error_code,
                                 const CefString& error_string) override;
  bool OnBeforeBrowse(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                      CefRefPtr<CefRequest> request, bool user_gesture,
                      bool is_redirect) override;

  /////////////////////////////////////////////////////////////////
  // CefFocusHandler methods
  /////////////////////////////////////////////////////////////////
  void OnTakeFocus(CefRefPtr<CefBrowser> browser, bool next) override;
  bool OnSetFocus(CefRefPtr<CefBrowser> browser, FocusSource source) override;

  /////////////////////////////////////////////////////////////////
  // CefPermissionHandler methods
  /////////////////////////////////////////////////////////////////
  bool OnRequestMediaAccessPermission(
      CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
      const CefString& requesting_origin, uint32_t requested_permissions,
      CefRefPtr<CefMediaAccessCallback> callback) override;

  /////////////////////////////////////////////////////////////////
  // CefMessageRouterBrowserSide::Handler methods
  /////////////////////////////////////////////////////////////////
  bool OnQuery(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
               int64_t query_id, const CefString& request, bool persistent,
               CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback>
                   callback) override;
};
