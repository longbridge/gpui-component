#pragma once

#include <iostream>
#include <limits>

#define NOMINMAX

#include "browser_callbacks.h"
#include "frame.h"
#include "include/cef_browser.h"
#include "include/cef_client.h"
#include "include/wrapper/cef_message_router.h"
#include "utils.h"

struct WefBrowser;

class WefClient : public CefClient,
                  public CefRenderHandler,
                  public CefDisplayHandler,
                  public CefLifeSpanHandler,
                  public CefLoadHandler,
                  public CefDialogHandler,
                  public CefFindHandler,
                  public CefContextMenuHandler,
                  public CefJSDialogHandler,
                  public CefRequestHandler,
                  public CefMessageRouterBrowserSide::Handler {
  IMPLEMENT_REFCOUNTING(WefClient);

 private:
  WefBrowser* wef_browser_;
  int width_, height_;
  float device_scale_factor_;
  BrowserCallbacks callbacks_;
  void* userdata_;
  DestroyFn destroy_userdata_;
  CefRefPtr<CefMessageRouterBrowserSide> message_router_;

 public:
  WefClient(WefBrowser* wef_browser, float device_scale_factor, int width,
            int height, BrowserCallbacks callbacks, void* userdata,
            DestroyFn destroy_userdata);

  virtual ~WefClient();

  bool setSize(int width, int height) {
    if (width_ == width && height_ == height) {
      return false;
    }
    width_ = width;
    height_ = height;
    return true;
  }

  /////////////////////////////////////////////////////////////////
  // CefClient methods
  /////////////////////////////////////////////////////////////////

  bool GetScreenInfo(CefRefPtr<CefBrowser> browser,
                     CefScreenInfo& screen_info) override {
    screen_info.device_scale_factor = device_scale_factor_;
    return true;
  }

  void GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) override {
    rect.Set(
        0, 0,
        static_cast<int>(static_cast<float>(width_) / device_scale_factor_),
        static_cast<int>(static_cast<float>(height_) / device_scale_factor_));
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
  void OnPopupShow(CefRefPtr<CefBrowser> browser, bool show) override {
    callbacks_.on_popup_show(userdata_, show);
  }

  void OnPopupSize(CefRefPtr<CefBrowser> browser,
                   const CefRect& rect) override {
    callbacks_.on_popup_position(userdata_, &rect);
  }

  void OnPaint(CefRefPtr<CefBrowser> browser, PaintElementType type,
               const RectList& dirtyRects, const void* buffer, int width,
               int height) override {
    callbacks_.on_paint(userdata_, static_cast<int>(type), &dirtyRects, buffer,
                        static_cast<uint32_t>(width),
                        static_cast<uint32_t>(height));
  }

  void OnImeCompositionRangeChanged(CefRefPtr<CefBrowser> browser,
                                    const CefRange& selected_range,
                                    const RectList& character_bounds) override {
    int xmin = std::numeric_limits<int>::max();
    int ymin = std::numeric_limits<int>::max();
    int xmax = std::numeric_limits<int>::min();
    int ymax = std::numeric_limits<int>::min();

    for (const auto& r : character_bounds) {
      if (r.x < xmin) {
        xmin = r.x;
      }
      if (r.y < ymin) {
        ymin = r.y;
      }
      if (r.x + r.width > xmax) {
        xmax = r.x + r.width;
      }
      if (r.y + r.height > ymax) {
        ymax = r.y + r.height;
      }
    }

    CefRect rect{int(float(xmin)), int(float(ymin)), int(float(xmax - xmin)),
                 int(float(ymax - ymin))};
    callbacks_.on_ime_composition_range_changed(userdata_, &rect);
  }

  bool OnCursorChange(CefRefPtr<CefBrowser> browser, CefCursorHandle cursor,
                      cef_cursor_type_t type,
                      const CefCursorInfo& custom_cursor_info) override {
    return callbacks_.on_cursor_changed(
        userdata_, static_cast<int>(type),
        type == CT_CUSTOM ? &custom_cursor_info : nullptr);
  }

  /////////////////////////////////////////////////////////////////
  // CefDisplayHandler methods
  /////////////////////////////////////////////////////////////////
  void OnAddressChange(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                       const CefString& url) override {
    auto url_str = url.ToString();
    callbacks_.on_address_changed(userdata_, new WefFrame{frame},
                                  url_str.c_str());
  }

  void OnTitleChange(CefRefPtr<CefBrowser> browser,
                     const CefString& title) override {
    auto title_str = title.ToString();
    callbacks_.on_title_changed(userdata_, title_str.c_str());
  }

  virtual void OnFaviconURLChange(CefRefPtr<CefBrowser> browser,
                                  const std::vector<CefString>& icon_urls) {
    std::vector<std::string> str_urls;
    std::transform(icon_urls.begin(), icon_urls.end(),
                   std::back_inserter(str_urls),
                   [](const CefString& url) { return url.ToString(); });

    std::vector<const char*> cstr_urls;
    std::transform(str_urls.begin(), str_urls.end(),
                   std::back_inserter(cstr_urls),
                   [](const std::string& url) { return url.c_str(); });

    callbacks_.on_favicon_url_change(userdata_, cstr_urls.data(),
                                     static_cast<int>(cstr_urls.size()));
  }

  bool OnTooltip(CefRefPtr<CefBrowser> browser, CefString& text) override {
    auto text_str = text.ToString();
    callbacks_.on_tooltip(userdata_, text_str.c_str());
    return false;
  }

  void OnStatusMessage(CefRefPtr<CefBrowser> browser,
                       const CefString& value) override {
    auto text_str = value.ToString();
    callbacks_.on_status_message(userdata_, text_str.c_str());
  }

  bool OnConsoleMessage(CefRefPtr<CefBrowser> browser, cef_log_severity_t level,
                        const CefString& message, const CefString& source,
                        int line) override {
    auto message_str = message.ToString();
    auto source_str = source.ToString();
    callbacks_.on_console_message(userdata_, message_str.c_str(),
                                  static_cast<int>(level), source_str.c_str(),
                                  line);
    return false;
  }

  void OnLoadingProgressChange(CefRefPtr<CefBrowser> browser,
                               double progress) override {
    callbacks_.on_loading_progress_changed(userdata_,
                                           static_cast<float>(progress));
  }

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
      bool* no_javascript_access) override {
    auto target_url_str = target_url.ToString();
    callbacks_.on_before_popup(userdata_, target_url_str.c_str());
    return true;
  }

  bool DoClose(CefRefPtr<CefBrowser> browser) override { return false; }

  void OnBeforeClose(CefRefPtr<CefBrowser> browser) override;

  /////////////////////////////////////////////////////////////////
  // CefLoadHandler methods
  /////////////////////////////////////////////////////////////////
  void OnLoadingStateChange(CefRefPtr<CefBrowser> browser, bool isLoading,
                            bool canGoBack, bool canGoForward) override {
    callbacks_.on_loading_state_changed(userdata_, isLoading, canGoBack,
                                        canGoForward);
  }

  void OnLoadStart(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                   TransitionType transition_type) override {
    callbacks_.on_load_start(userdata_, new WefFrame{frame});
  }

  void OnLoadEnd(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                 int httpStatusCode) override {
    callbacks_.on_load_end(userdata_, new WefFrame{frame});
  }

  void OnLoadError(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                   ErrorCode errorCode, const CefString& errorText,
                   const CefString& failedUrl) override {
    auto error_text_str = errorText.ToString();
    auto failed_url_str = failedUrl.ToString();
    callbacks_.on_load_error(userdata_, new WefFrame{frame},
                             error_text_str.c_str(), failed_url_str.c_str());
  }

  /////////////////////////////////////////////////////////////////
  // CefDialogHandler methods
  /////////////////////////////////////////////////////////////////
  bool OnFileDialog(CefRefPtr<CefBrowser> browser, FileDialogMode mode,
                    const CefString& title, const CefString& default_file_path,
                    const std::vector<CefString>& accept_filters,
                    const std::vector<CefString>& accept_extensions,
                    const std::vector<CefString>& accept_descriptions,
                    CefRefPtr<CefFileDialogCallback> callback) override {
    auto title_str = title.ToString();
    auto default_file_path_str = default_file_path.ToString();
    auto accept_filters_str = join_strings(accept_filters, "@@@");
    auto accept_extensions_str = join_strings(accept_extensions, "@@@");
    auto accept_descriptions_str = join_strings(accept_descriptions, "@@@");
    CefRefPtr<CefFileDialogCallback>* callback_ptr =
        new CefRefPtr<CefFileDialogCallback>(callback);
    return callbacks_.on_file_dialog(
        userdata_, static_cast<int>(mode), title_str.c_str(),
        default_file_path_str.c_str(), accept_filters_str.c_str(),
        accept_extensions_str.c_str(), accept_descriptions_str.c_str(),
        callback_ptr);
  }

  /////////////////////////////////////////////////////////////////
  // CefContextMenuHandler methods
  /////////////////////////////////////////////////////////////////
  bool RunContextMenu(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                      CefRefPtr<CefContextMenuParams> params,
                      CefRefPtr<CefMenuModel> model,
                      CefRefPtr<CefRunContextMenuCallback> callback) override {
    auto link_url_str = params->GetLinkUrl().ToString();
    auto unfiltered_link_url_str = params->GetUnfilteredLinkUrl().ToString();
    auto source_url_str = params->GetSourceUrl().ToString();
    auto title_text_str = params->GetTitleText().ToString();
    auto page_url_str = params->GetPageUrl().ToString();
    auto frame_url_str = params->GetFrameUrl().ToString();
    auto selection_text_str = params->GetSelectionText().ToString();

    _ContextMenuParams params_{
        params->GetXCoord(),
        params->GetYCoord(),
        static_cast<int>(params->GetTypeFlags()),
        !link_url_str.empty() ? link_url_str.c_str() : nullptr,
        !unfiltered_link_url_str.empty() ? unfiltered_link_url_str.c_str()
                                         : nullptr,
        !source_url_str.empty() ? source_url_str.c_str() : nullptr,
        params->HasImageContents(),
        !title_text_str.empty() ? title_text_str.c_str() : nullptr,
        page_url_str.c_str(),
        frame_url_str.c_str(),
        static_cast<int>(params->GetMediaType()),
        static_cast<int>(params->GetMediaStateFlags()),
        selection_text_str.c_str(),
        params->IsEditable(),
        static_cast<int>(params->GetEditStateFlags()),
    };
    callbacks_.on_context_menu(userdata_, new WefFrame{frame}, &params_);
    return true;
  }

  /////////////////////////////////////////////////////////////////
  // CefFindHandler methods
  /////////////////////////////////////////////////////////////////
  void OnFindResult(CefRefPtr<CefBrowser> browser, int identifier, int count,
                    const CefRect& selectionRect, int activeMatchOrdinal,
                    bool finalUpdate) override {
    callbacks_.on_find_result(userdata_, identifier, count, &selectionRect,
                              activeMatchOrdinal, finalUpdate);
  }

  /////////////////////////////////////////////////////////////////
  // CefJSDialogHandler methods
  /////////////////////////////////////////////////////////////////
  bool OnJSDialog(CefRefPtr<CefBrowser> browser, const CefString& origin_url,
                  JSDialogType dialog_type, const CefString& message_text,
                  const CefString& default_prompt_text,
                  CefRefPtr<CefJSDialogCallback> callback,
                  bool& suppress_message) {
    auto message_text_str = message_text.ToString();
    auto default_prompt_text_str = default_prompt_text.ToString();
    CefRefPtr<CefJSDialogCallback>* callback_ptr =
        new CefRefPtr<CefJSDialogCallback>(callback);

    return callbacks_.on_js_dialog(
        userdata_, static_cast<int>(dialog_type), message_text_str.c_str(),
        default_prompt_text_str.c_str(), callback_ptr);
  }

  /////////////////////////////////////////////////////////////////
  // CefRequestHandler methods
  /////////////////////////////////////////////////////////////////
  void OnRenderProcessTerminated(CefRefPtr<CefBrowser> browser,
                                 TerminationStatus status, int error_code,
                                 const CefString& error_string) override {
    message_router_->OnRenderProcessTerminated(browser);
  }

  bool OnBeforeBrowse(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                      CefRefPtr<CefRequest> request, bool user_gesture,
                      bool is_redirect) override {
    message_router_->OnBeforeBrowse(browser, frame);
    return false;
  }

  /////////////////////////////////////////////////////////////////
  // CefMessageRouterBrowserSide::Handler methods
  /////////////////////////////////////////////////////////////////
  bool OnQuery(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
               int64_t query_id, const CefString& request, bool persistent,
               CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback>
                   callback) override {
    auto request_str = request.ToString();
    CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback>* callback_ptr =
        new CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback>(callback);
    callbacks_.on_query(userdata_, new WefFrame{frame}, request_str.c_str(),
                        callback_ptr);
    return true;
  }
};
