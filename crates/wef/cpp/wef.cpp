#include "wef.h"

#include <algorithm>
#include <iostream>
#include <vector>

#include "app.h"
#include "app_render_process.h"
#include "browser_callbacks.h"
#include "client.h"
#include "frame.h"
#include "include/base/cef_bind.h"
#include "include/base/cef_callback.h"
#include "include/cef_app.h"
#include "include/cef_command_line.h"
#include "include/cef_render_handler.h"
#include "include/cef_task.h"
#include "include/wrapper/cef_closure_task.h"
#include "shutdown_helper.h"

const uint32_t ALL_MOUSE_BUTTONS = EVENTFLAG_LEFT_MOUSE_BUTTON;

struct WefSettings {
  const char* locale;
  const char* cache_path;
  const char* root_cache_path;
  const char* browser_subprocess_path;
};

struct WefBrowserSettings {
  void* parent;
  float device_scale_factor;
  int width;
  int height;
  int frame_rate;
  const char* url;
  const char* inject_javascript;
  BrowserCallbacks callbacks;
  void* userdata;
  DestroyFn destroy_userdata;
};

inline void apply_key_modifiers(uint32_t& m, int modifiers) {
  if (modifiers & 0x1) {
    m |= EVENTFLAG_SHIFT_DOWN;
  }

  if (modifiers & 0x2) {
    m |= EVENTFLAG_CONTROL_DOWN;
  }

  if (modifiers & 0x4) {
    m |= EVENTFLAG_ALT_DOWN;
  }
}

extern "C" {

bool wef_init(const WefSettings* wef_settings) {
  CefSettings settings;
  settings.windowless_rendering_enabled = true;

#ifdef __APPLE__
  settings.no_sandbox = false;
#else
  settings.no_sandbox = true;
#endif

  settings.external_message_pump = true;

  if (wef_settings->locale) {
    CefString(&settings.locale) = wef_settings->locale;
  }

  if (wef_settings->cache_path) {
    CefString(&settings.cache_path) = wef_settings->cache_path;
  }

  if (wef_settings->root_cache_path) {
    CefString(&settings.root_cache_path) = wef_settings->root_cache_path;
  }

  if (wef_settings->browser_subprocess_path) {
    CefString(&settings.browser_subprocess_path) =
        wef_settings->browser_subprocess_path;
  }

  CefRefPtr<WefApp> app(new WefApp());
  return CefInitialize(CefMainArgs(), settings, app, nullptr);
}

bool wef_exec_process(char* argv[], int argc) {
#ifdef WIN32
  CefMainArgs args(GetModuleHandle(NULL));
#else
  CefMainArgs args(argc, argv);
#endif

  CefRefPtr<WefRenderProcessApp> app(new WefRenderProcessApp());
  return CefExecuteProcess(args, app, nullptr) >= 0;
}

void wef_shutdown() {
  ShutdownHelper::getSingleton()->shutdown();
  CefShutdown();
}

WefBrowser* wef_browser_create(const WefBrowserSettings* settings) {
  CefWindowInfo window_info;
  window_info.SetAsWindowless(
      reinterpret_cast<CefWindowHandle>(settings->parent));
  window_info.runtime_style = CEF_RUNTIME_STYLE_ALLOY;

  CefBrowserSettings browser_settings;
  browser_settings.windowless_frame_rate = settings->frame_rate;
  browser_settings.background_color = CefColorSetARGB(255, 255, 255, 255);

  WefBrowser* wef_browser = new WefBrowser;
  CefRefPtr<CefDictionaryValue> extra_info = CefDictionaryValue::Create();
  extra_info->SetString("__wef_inject_javascript", settings->inject_javascript);

  CefRefPtr<WefClient> client(
      new WefClient(wef_browser, settings->device_scale_factor, settings->width,
                    settings->height, settings->callbacks, settings->userdata,
                    settings->destroy_userdata));
  CefBrowserHost::CreateBrowser(window_info, client, "about:blank",
                                browser_settings, extra_info, nullptr);

  wef_browser->url = settings->url;
  wef_browser->focus = false;
  wef_browser->client = client;
  wef_browser->browser = std::nullopt;
  wef_browser->cursorX = 0;
  wef_browser->cursorY = 0;

  ShutdownHelper::getSingleton()->browserCreated();
  return wef_browser;
}

void wef_browser_destroy(WefBrowser* browser) {
  if (browser->browser) {
    (*browser->browser)->GetHost()->CloseBrowser(false);
  } else {
    browser->deleteBrowser = true;
  }
}

bool wef_browser_is_created(WefBrowser* browser) {
  return browser->browser.has_value();
}

void wef_browser_set_size(WefBrowser* browser, int width, int height) {
  if (browser->client->setSize(width, height)) {
    if (browser->browser) {
      (*browser->browser)->GetHost()->WasResized();
    }
  }
}

void wef_browser_load_url(WefBrowser* browser, const char* url) {
  if (strlen(url) > 0) {
    if (!browser->browser) {
      browser->url = url;
      return;
    }

    CefString url_str = url;
    CefPostTask(TID_UI,
                base::BindOnce(&CefFrame::LoadURL,
                               (*browser->browser)->GetMainFrame(), url_str));
  }
}

bool wef_browser_can_go_forward(WefBrowser* browser) {
  if (!browser->browser) {
    return false;
  }
  return (*browser->browser)->CanGoForward();
}

bool wef_browser_can_go_back(WefBrowser* browser) {
  if (!browser->browser) {
    return false;
  }
  return (*browser->browser)->CanGoBack();
}

void wef_browser_go_forward(WefBrowser* browser) {
  if (!browser->browser) {
    return;
  }
  (*browser->browser)->GoForward();
}

void wef_browser_go_back(WefBrowser* browser) {
  if (!browser->browser) {
    return;
  }
  (*browser->browser)->GoBack();
}

void wef_browser_reload(WefBrowser* browser) {
  if (!browser->browser) {
    return;
  }
  (*browser->browser)->Reload();
}

void wef_browser_reload_ignore_cache(WefBrowser* browser) {
  if (!browser->browser) {
    return;
  }
  (*browser->browser)->ReloadIgnoreCache();
}

void wef_browser_send_mouse_click_event(WefBrowser* browser,
                                        int mouse_button_type, bool mouse_up,
                                        int click_count, int modifiers) {
  if (!browser->browser) {
    return;
  }

  CefMouseEvent mouse_event;
  mouse_event.x = browser->cursorX;
  mouse_event.y = browser->cursorY;
  mouse_event.modifiers = EVENTFLAG_NONE;

  CefBrowserHost::MouseButtonType btn_type;
  switch (mouse_button_type) {
    case 1:
      btn_type = MBT_MIDDLE;
      mouse_event.modifiers |= EVENTFLAG_MIDDLE_MOUSE_BUTTON;
      break;
    case 2:
      btn_type = MBT_RIGHT;
      mouse_event.modifiers |= EVENTFLAG_RIGHT_MOUSE_BUTTON;
      break;
    default:
      btn_type = MBT_LEFT;
      mouse_event.modifiers |= EVENTFLAG_LEFT_MOUSE_BUTTON;
  }

  apply_key_modifiers(mouse_event.modifiers, modifiers);
  (*browser->browser)
      ->GetHost()
      ->SendMouseClickEvent(mouse_event, btn_type, mouse_up,
                            std::min(click_count, 3));
}

void wef_browser_send_mouse_move_event(WefBrowser* browser, int x, int y,
                                       int modifiers) {
  if (!browser->browser) {
    return;
  }

  CefMouseEvent mouse_event;
  mouse_event.x = x;
  mouse_event.y = y;
  mouse_event.modifiers = ALL_MOUSE_BUTTONS;
  apply_key_modifiers(mouse_event.modifiers, modifiers);
  (*browser->browser)->GetHost()->SendMouseMoveEvent(mouse_event, false);

  browser->cursorX = mouse_event.x;
  browser->cursorY = mouse_event.y;
}

void wef_browser_send_mouse_wheel_event(WefBrowser* browser, int delta_x,
                                        int delta_y) {
  if (!browser->browser) {
    return;
  }

  CefMouseEvent mouse_event;
  mouse_event.x = browser->cursorX;
  mouse_event.y = browser->cursorY;
  mouse_event.modifiers = ALL_MOUSE_BUTTONS;
  (*browser->browser)
      ->GetHost()
      ->SendMouseWheelEvent(mouse_event, delta_x, delta_y);
}

void wef_browser_send_key_event(WefBrowser* browser, bool is_down, int key_code,
                                int modifiers) {
  if (!browser->browser) {
    return;
  }

  CefKeyEvent key_event;
  key_event.type = is_down ? KEYEVENT_KEYDOWN : KEYEVENT_KEYUP;
  key_event.modifiers = EVENTFLAG_NONE;
  key_event.focus_on_editable_field = false;
  key_event.is_system_key = false;
  key_event.windows_key_code = key_code;
  key_event.native_key_code = key_code;
  key_event.modifiers = 0;
  apply_key_modifiers(key_event.modifiers, modifiers);
  (*browser->browser)->GetHost()->SendKeyEvent(key_event);
}

void wef_browser_send_char_event(WefBrowser* browser, char16_t ch) {
  if (!browser->browser) {
    return;
  }

  CefKeyEvent key_event;
  key_event.type = KEYEVENT_CHAR;
  key_event.modifiers = EVENTFLAG_NONE;
  key_event.windows_key_code = static_cast<int>(ch);
  key_event.native_key_code = static_cast<int>(ch);
  key_event.character = static_cast<char16_t>(ch);
  (*browser->browser)->GetHost()->SendKeyEvent(key_event);
}

void wef_browser_ime_set_composition(WefBrowser* browser, const char* text,
                                     uint32_t cursor_begin,
                                     uint32_t cursor_end) {
  if (!browser->browser) {
    return;
  }
  (*browser->browser)
      ->GetHost()
      ->ImeSetComposition(text, {}, CefRange::InvalidRange(),
                          CefRange(cursor_begin, cursor_end));
}

void wef_browser_ime_commit(WefBrowser* browser, const char* text) {
  if (!browser->browser) {
    return;
  }
  (*browser->browser)
      ->GetHost()
      ->ImeCommitText(text, CefRange::InvalidRange(), 0);
}

WefFrame* wef_browser_get_main_frame(WefBrowser* browser) {
  if (!browser->browser) {
    return nullptr;
  }
  auto main_frame = (*browser->browser)->GetMainFrame();
  return main_frame ? new WefFrame{main_frame} : nullptr;
}

WefFrame* wef_browser_get_focused_frame(WefBrowser* browser) {
  if (!browser->browser) {
    return nullptr;
  }
  auto frame = (*browser->browser)->GetFocusedFrame();
  return frame ? new WefFrame{frame} : nullptr;
}

WefFrame* wef_browser_get_frame_by_name(WefBrowser* browser, const char* name) {
  if (!browser->browser) {
    return nullptr;
  }
  auto frame = (*browser->browser)->GetFrameByName(name);
  return frame ? new WefFrame{frame} : nullptr;
}

WefFrame* wef_browser_get_frame_by_identifier(WefBrowser* browser,
                                              const char* id) {
  if (!browser->browser) {
    return nullptr;
  }
  auto frame = (*browser->browser)->GetFrameByIdentifier(id);
  return frame ? new WefFrame{frame} : nullptr;
}

bool wef_browser_is_audio_muted(WefBrowser* browser, bool mute) {
  if (browser->browser) {
    return false;
  }
  return (*browser->browser)->GetHost()->IsAudioMuted();
}

void wef_browser_set_audio_mute(WefBrowser* browser, bool mute) {
  if (browser->browser) {
    return;
  }
  (*browser->browser)->GetHost()->SetAudioMuted(mute);
}

void wef_browser_find(WefBrowser* browser, const char* search_text,
                      bool forward, bool match_case, bool find_next) {
  if (!browser->browser) {
    return;
  }
  (*browser->browser)
      ->GetHost()
      ->Find(search_text, forward, match_case, find_next);
}

void wef_browser_set_focus(WefBrowser* browser, bool focus) {
  if (!browser->browser) {
    browser->focus = true;
    return;
  }
  (*browser->browser)->GetHost()->SetFocus(focus);
}

}  // extern "C"