#include "client.h"

#include <iostream>

#include "include/base/cef_bind.h"
#include "include/base/cef_callback.h"
#include "include/cef_browser.h"
#include "include/cef_task.h"
#include "include/wrapper/cef_closure_task.h"

WefClient::WefClient(std::shared_ptr<BrowserSharedState> state)
    : state_(state) {}

WefClient::~WefClient() {
  if (state_->browser) {
    (*state_->browser)->GetHost()->CloseBrowser(true);
  }
}

/////////////////////////////////////////////////////////////////
// CefRenderHandler methods
/////////////////////////////////////////////////////////////////
void WefClient::OnPopupShow(CefRefPtr<CefBrowser> browser, bool show) {
  DCHECK(CefCurrentlyOn(TID_UI));
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_popup_show(userdata, show);
      });
}

void WefClient::OnPopupSize(CefRefPtr<CefBrowser> browser,
                            const CefRect& rect) {
  DCHECK(CefCurrentlyOn(TID_UI));
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_popup_position(userdata, &rect);
      });
}

void WefClient::OnPaint(CefRefPtr<CefBrowser> browser, PaintElementType type,
                        const RectList& dirtyRects, const void* buffer,
                        int width, int height) {
  DCHECK(CefCurrentlyOn(TID_UI));
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_paint(userdata, static_cast<int>(type), &dirtyRects,
                           buffer, static_cast<uint32_t>(width),
                           static_cast<uint32_t>(height));
      });
}

void WefClient::OnImeCompositionRangeChanged(CefRefPtr<CefBrowser> browser,
                                             const CefRange& selected_range,
                                             const RectList& character_bounds) {
  DCHECK(CefCurrentlyOn(TID_UI));

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
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_ime_composition_range_changed(userdata, &rect);
      });
}

bool WefClient::OnCursorChange(CefRefPtr<CefBrowser> browser,
                               CefCursorHandle cursor, cef_cursor_type_t type,
                               const CefCursorInfo& custom_cursor_info) {
  DCHECK(CefCurrentlyOn(TID_UI));

  bool result = false;
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        result = callbacks.on_cursor_changed(
            userdata, static_cast<int>(type),
            type == CT_CUSTOM ? &custom_cursor_info : nullptr);
      });
  return result;
}

/////////////////////////////////////////////////////////////////
// CefDisplayHandler methods
/////////////////////////////////////////////////////////////////
void WefClient::OnAddressChange(CefRefPtr<CefBrowser> browser,
                                CefRefPtr<CefFrame> frame,
                                const CefString& url) {
  DCHECK(CefCurrentlyOn(TID_UI));

  auto url_str = url.ToString();
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_address_changed(userdata, new WefFrame{frame},
                                     url_str.c_str());
      });
}

void WefClient::OnTitleChange(CefRefPtr<CefBrowser> browser,
                              const CefString& title) {
  DCHECK(CefCurrentlyOn(TID_UI));

  auto title_str = title.ToString();
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_title_changed(userdata, title_str.c_str());
      });
}

void WefClient::OnFaviconURLChange(CefRefPtr<CefBrowser> browser,
                                   const std::vector<CefString>& icon_urls) {
  DCHECK(CefCurrentlyOn(TID_UI));

  std::vector<std::string> str_urls;
  std::transform(icon_urls.begin(), icon_urls.end(),
                 std::back_inserter(str_urls),
                 [](const CefString& url) { return url.ToString(); });

  std::vector<const char*> cstr_urls;
  std::transform(str_urls.begin(), str_urls.end(),
                 std::back_inserter(cstr_urls),
                 [](const std::string& url) { return url.c_str(); });

  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_favicon_url_change(userdata, cstr_urls.data(),
                                        static_cast<int>(cstr_urls.size()));
      });
}

bool WefClient::OnTooltip(CefRefPtr<CefBrowser> browser, CefString& text) {
  DCHECK(CefCurrentlyOn(TID_UI));

  auto text_str = text.ToString();
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_tooltip(userdata, text_str.c_str());
      });
  return true;
}

void WefClient::OnStatusMessage(CefRefPtr<CefBrowser> browser,
                                const CefString& value) {
  DCHECK(CefCurrentlyOn(TID_UI));

  auto text_str = value.ToString();
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_status_message(userdata, text_str.c_str());
      });
}

bool WefClient::OnConsoleMessage(CefRefPtr<CefBrowser> browser,
                                 cef_log_severity_t level,
                                 const CefString& message,
                                 const CefString& source, int line) {
  DCHECK(CefCurrentlyOn(TID_UI));

  auto message_str = message.ToString();
  auto source_str = source.ToString();
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_console_message(userdata, message_str.c_str(),
                                     static_cast<int>(level),
                                     source_str.c_str(), line);
      });
  return false;
}

void WefClient::OnLoadingProgressChange(CefRefPtr<CefBrowser> browser,
                                        double progress) {
  DCHECK(CefCurrentlyOn(TID_UI));

  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_loading_progress_changed(userdata,
                                              static_cast<float>(progress));
      });
}

/////////////////////////////////////////////////////////////////
// CefLifeSpanHandler methods
/////////////////////////////////////////////////////////////////
void WefClient::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
  CefMessageRouterConfig config;
  message_router_ = CefMessageRouterBrowserSide::Create(config);
  message_router_->AddHandler(this, false);

  state_->browser = browser;
  state_->browser_state = BrowserState::Created;

  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_created(userdata);
      });

  if (state_->browser_state == BrowserState::Closed) {
    CefPostTask(TID_UI, base::BindOnce(&CefBrowserHost::CloseBrowser,
                                       browser->GetHost(), false));
  }
}

bool WefClient::OnBeforePopup(
    CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, int popup_id,
    const CefString& target_url, const CefString& target_frame_name,
    CefLifeSpanHandler::WindowOpenDisposition target_disposition,
    bool user_gesture, const CefPopupFeatures& popupFeatures,
    CefWindowInfo& windowInfo, CefRefPtr<CefClient>& client,
    CefBrowserSettings& settings, CefRefPtr<CefDictionaryValue>& extra_info,
    bool* no_javascript_access) {
  DCHECK(CefCurrentlyOn(TID_UI));

  auto target_url_str = target_url.ToString();
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_before_popup(userdata, target_url_str.c_str());
      });
  return true;
}

bool WefClient::DoClose(CefRefPtr<CefBrowser> browser) { return false; }

void WefClient::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
  DCHECK(CefCurrentlyOn(TID_UI));

  message_router_->OnBeforeClose(browser);

  state_->browser_state = BrowserState::Closed;
  state_->browser = std::nullopt;

  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_closed(userdata);
      });
}

/////////////////////////////////////////////////////////////////
// CefLoadHandler methods
/////////////////////////////////////////////////////////////////
void WefClient::OnLoadingStateChange(CefRefPtr<CefBrowser> browser,
                                     bool isLoading, bool canGoBack,
                                     bool canGoForward) {
  DCHECK(CefCurrentlyOn(TID_UI));
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_loading_state_changed(userdata, isLoading, canGoBack,
                                           canGoForward);
      });
}

void WefClient::OnLoadStart(CefRefPtr<CefBrowser> browser,
                            CefRefPtr<CefFrame> frame,
                            TransitionType transition_type) {
  DCHECK(CefCurrentlyOn(TID_UI));
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_load_start(userdata, new WefFrame{frame});
      });
}

void WefClient::OnLoadEnd(CefRefPtr<CefBrowser> browser,
                          CefRefPtr<CefFrame> frame, int httpStatusCode) {
  DCHECK(CefCurrentlyOn(TID_UI));

  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_load_end(userdata, new WefFrame{frame});
      });

  if (state_->browser) {
    (*state_->browser)->GetHost()->SetFocus(state_->focus);
  }
}

void WefClient::OnLoadError(CefRefPtr<CefBrowser> browser,
                            CefRefPtr<CefFrame> frame, ErrorCode errorCode,
                            const CefString& errorText,
                            const CefString& failedUrl) {
  DCHECK(CefCurrentlyOn(TID_UI));

  auto error_text_str = errorText.ToString();
  auto failed_url_str = failedUrl.ToString();
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_load_error(userdata, new WefFrame{frame},
                                error_text_str.c_str(), failed_url_str.c_str());
      });
}

/////////////////////////////////////////////////////////////////
// CefDialogHandler methods
/////////////////////////////////////////////////////////////////
bool WefClient::OnFileDialog(CefRefPtr<CefBrowser> browser, FileDialogMode mode,
                             const CefString& title,
                             const CefString& default_file_path,
                             const std::vector<CefString>& accept_filters,
                             const std::vector<CefString>& accept_extensions,
                             const std::vector<CefString>& accept_descriptions,
                             CefRefPtr<CefFileDialogCallback> callback) {
  DCHECK(CefCurrentlyOn(TID_UI));

  auto title_str = title.ToString();
  auto default_file_path_str = default_file_path.ToString();
  auto accept_filters_str = join_strings(accept_filters, "@@@");
  auto accept_extensions_str = join_strings(accept_extensions, "@@@");
  auto accept_descriptions_str = join_strings(accept_descriptions, "@@@");
  CefRefPtr<CefFileDialogCallback>* callback_ptr =
      new CefRefPtr<CefFileDialogCallback>(callback);
  bool result = false;
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        result = callbacks.on_file_dialog(
            userdata, static_cast<int>(mode), title_str.c_str(),
            default_file_path_str.c_str(), accept_filters_str.c_str(),
            accept_extensions_str.c_str(), accept_descriptions_str.c_str(),
            callback_ptr);
      });
  return result;
}

/////////////////////////////////////////////////////////////////
// CefContextMenuHandler methods
/////////////////////////////////////////////////////////////////
bool WefClient::RunContextMenu(CefRefPtr<CefBrowser> browser,
                               CefRefPtr<CefFrame> frame,
                               CefRefPtr<CefContextMenuParams> params,
                               CefRefPtr<CefMenuModel> model,
                               CefRefPtr<CefRunContextMenuCallback> callback) {
  DCHECK(CefCurrentlyOn(TID_UI));

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
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_context_menu(userdata, new WefFrame{frame}, &params_);
      });
  return true;
}

/////////////////////////////////////////////////////////////////
// CefFindHandler methods
/////////////////////////////////////////////////////////////////
void WefClient::OnFindResult(CefRefPtr<CefBrowser> browser, int identifier,
                             int count, const CefRect& selectionRect,
                             int activeMatchOrdinal, bool finalUpdate) {
  DCHECK(CefCurrentlyOn(TID_UI));
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        callbacks.on_find_result(userdata, identifier, count, &selectionRect,
                                 activeMatchOrdinal, finalUpdate);
      });
}

/////////////////////////////////////////////////////////////////
// CefJSDialogHandler methods
/////////////////////////////////////////////////////////////////
bool WefClient::OnJSDialog(CefRefPtr<CefBrowser> browser,
                           const CefString& origin_url,
                           JSDialogType dialog_type,
                           const CefString& message_text,
                           const CefString& default_prompt_text,
                           CefRefPtr<CefJSDialogCallback> callback,
                           bool& suppress_message) {
  DCHECK(CefCurrentlyOn(TID_UI));

  auto message_text_str = message_text.ToString();
  auto default_prompt_text_str = default_prompt_text.ToString();
  CefRefPtr<CefJSDialogCallback>* callback_ptr =
      new CefRefPtr<CefJSDialogCallback>(callback);

  bool result = false;
  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        result = callbacks.on_js_dialog(
            userdata, static_cast<int>(dialog_type), message_text_str.c_str(),
            default_prompt_text_str.c_str(), callback_ptr);
      });
  return result;
}

bool WefClient::OnBeforeUnloadDialog(CefRefPtr<CefBrowser> browser,
                                     const CefString& message_text,
                                     bool is_reload,
                                     CefRefPtr<CefJSDialogCallback> callback) {
  callback->Continue(true, "");
  return true;
}

/////////////////////////////////////////////////////////////////
// CefRequestHandler methods
/////////////////////////////////////////////////////////////////
void WefClient::OnRenderProcessTerminated(CefRefPtr<CefBrowser> browser,
                                          TerminationStatus status,
                                          int error_code,
                                          const CefString& error_string) {
  message_router_->OnRenderProcessTerminated(browser);
}

bool WefClient::OnBeforeBrowse(CefRefPtr<CefBrowser> browser,
                               CefRefPtr<CefFrame> frame,
                               CefRefPtr<CefRequest> request, bool user_gesture,
                               bool is_redirect) {
  message_router_->OnBeforeBrowse(browser, frame);
  return false;
}

/////////////////////////////////////////////////////////////////
// CefFocusHandler methods
/////////////////////////////////////////////////////////////////
void WefClient::OnTakeFocus(CefRefPtr<CefBrowser> browser, bool next) {}

bool WefClient::OnSetFocus(CefRefPtr<CefBrowser> browser, FocusSource source) {
  return false;
}

/////////////////////////////////////////////////////////////////
// CefPermissionHandler methods
/////////////////////////////////////////////////////////////////
bool WefClient::OnRequestMediaAccessPermission(
    CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
    const CefString& requesting_origin, uint32_t requested_permissions,
    CefRefPtr<CefMediaAccessCallback> callback) {
  callback->Continue(CEF_MEDIA_PERMISSION_NONE);
  return true;
}

/////////////////////////////////////////////////////////////////
// CefMessageRouterBrowserSide::Handler methods
/////////////////////////////////////////////////////////////////
bool WefClient::OnQuery(
    CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, int64_t query_id,
    const CefString& request, bool persistent,
    CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback> callback) {
  DCHECK(CefCurrentlyOn(TID_UI));

  auto request_str = request.ToString();
  CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback>* callback_ptr =
      new CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback>(callback);

  state_->callbacks_target.call(
      [&](const BrowserCallbacks& callbacks, void* userdata) {
        return callbacks.on_query(userdata, new WefFrame{frame},
                                  request_str.c_str(), callback_ptr);
      });
  return true;
}
