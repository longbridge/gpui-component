#if defined(_WIN32) || defined(_WIN64)
#define NOMINMAX
#endif

#include <algorithm>
#include <iostream>
#include <optional>
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

const uint32_t ALL_MOUSE_BUTTONS = EVENTFLAG_LEFT_MOUSE_BUTTON;

struct WefSettings {
  const char* locale;
  const char* cache_path;
  const char* root_cache_path;
  const char* browser_subprocess_path;
  AppCallbacks callbacks;
  void* userdata;
  DestroyFn destroy_userdata;
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

struct WefBrowser {
  std::shared_ptr<BrowserSharedState> state;
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
  settings.external_message_pump = true;

#ifdef __APPLE__
  settings.no_sandbox = false;
#else
  settings.no_sandbox = true;
#endif

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

  CefRefPtr<WefApp> app(new WefApp(wef_settings->callbacks,
                                   wef_settings->userdata,
                                   wef_settings->destroy_userdata));
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

void wef_shutdown() { CefShutdown(); }

void wef_do_message_work() { CefDoMessageLoopWork(); }

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

  wef_browser->state =
      std::make_shared<BrowserSharedState>(BrowserCallbacksTarget{
          settings->callbacks, settings->userdata, settings->destroy_userdata});
  wef_browser->state->width = settings->width;
  wef_browser->state->height = settings->height;
  wef_browser->state->device_scale_factor = settings->device_scale_factor;

  CefRefPtr<WefClient> client(new WefClient(wef_browser->state));
  CefBrowserHost::CreateBrowser(window_info, client, settings->url,
                                browser_settings, extra_info, nullptr);
  return wef_browser;
}

void wef_browser_close(WefBrowser* browser) {
  if (browser->state->browser_state == BrowserState::Creating) {
    browser->state->browser_state = BrowserState::Closed;
  } else if (browser->state->browser_state == BrowserState::Created) {
    browser->state->browser_state = BrowserState::Closing;
    (*browser->state->browser)->GetHost()->CloseBrowser(false);
  }
}

void wef_browser_destroy(WefBrowser* browser) {
  if (browser->state->browser_state == BrowserState::Creating) {
    browser->state->browser_state = BrowserState::Closed;
  } else if (browser->state->browser_state == BrowserState::Created) {
    browser->state->browser_state = BrowserState::Closed;
    (*browser->state->browser)->GetHost()->CloseBrowser(true);
  }
  browser->state->callbacks_target.disable();
  delete browser;
}

bool wef_browser_is_created(WefBrowser* browser) {
  return browser->state->browser_state == BrowserState::Created;
}

void wef_browser_set_size(WefBrowser* browser, int width, int height) {
  browser->state->width = width;
  browser->state->height = height;
  if (browser->state->browser) {
    (*browser->state->browser)->GetHost()->WasResized();
  }
}

void wef_browser_load_url(WefBrowser* browser, const char* url) {
  if (strlen(url) == 0) {
    return;
  }

  if (!browser->state->browser) {
    return;
  }

  CefPostTask(TID_UI, base::BindOnce(&CefFrame::LoadURL,
                                     (*browser->state->browser)->GetMainFrame(),
                                     CefString(url)));
}

bool wef_browser_can_go_forward(WefBrowser* browser) {
  if (!browser->state->browser) {
    return false;
  }
  return (*browser->state->browser)->CanGoForward();
}

bool wef_browser_can_go_back(WefBrowser* browser) {
  if (!browser->state->browser) {
    return false;
  }
  return (*browser->state->browser)->CanGoBack();
}

void wef_browser_go_forward(WefBrowser* browser) {
  if (!browser->state->browser) {
    return;
  }
  (*browser->state->browser)->GoForward();
}

void wef_browser_go_back(WefBrowser* browser) {
  if (!browser->state->browser) {
    return;
  }
  (*browser->state->browser)->GoBack();
}

void wef_browser_reload(WefBrowser* browser) {
  if (!browser->state->browser) {
    return;
  }
  (*browser->state->browser)->Reload();
}

void wef_browser_reload_ignore_cache(WefBrowser* browser) {
  if (!browser->state->browser) {
    return;
  }
  (*browser->state->browser)->ReloadIgnoreCache();
}

void wef_browser_send_mouse_click_event(WefBrowser* browser,
                                        int mouse_button_type, bool mouse_up,
                                        int click_count, int modifiers) {
  if (!browser->state->browser) {
    return;
  }

  CefMouseEvent mouse_event;
  mouse_event.x = browser->state->cursorX;
  mouse_event.y = browser->state->cursorY;
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

  (*browser->state->browser)
      ->GetHost()
      ->SendMouseClickEvent(mouse_event, btn_type, mouse_up,
                            std::max(click_count, 3));
}

void wef_browser_send_mouse_move_event(WefBrowser* browser, int x, int y,
                                       int modifiers) {
  if (!browser->state->browser) {
    return;
  }

  CefMouseEvent mouse_event;
  mouse_event.x = x;
  mouse_event.y = y;
  mouse_event.modifiers = ALL_MOUSE_BUTTONS;
  apply_key_modifiers(mouse_event.modifiers, modifiers);
  (*browser->state->browser)->GetHost()->SendMouseMoveEvent(mouse_event, false);

  browser->state->cursorX = mouse_event.x;
  browser->state->cursorY = mouse_event.y;
}

void wef_browser_send_mouse_wheel_event(WefBrowser* browser, int delta_x,
                                        int delta_y) {
  if (!browser->state->browser) {
    return;
  }

  CefMouseEvent mouse_event;
  mouse_event.x = browser->state->cursorX;
  mouse_event.y = browser->state->cursorY;
  mouse_event.modifiers = ALL_MOUSE_BUTTONS;
  (*browser->state->browser)
      ->GetHost()
      ->SendMouseWheelEvent(mouse_event, delta_x, delta_y);
}

void wef_browser_send_key_event(WefBrowser* browser, bool is_down, int key_code,
                                int modifiers) {
  if (!browser->state->browser) {
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
  (*browser->state->browser)->GetHost()->SendKeyEvent(key_event);
}

void wef_browser_send_char_event(WefBrowser* browser, char16_t ch) {
  if (!browser->state->browser) {
    return;
  }

  CefKeyEvent key_event;
  key_event.type = KEYEVENT_CHAR;
  key_event.modifiers = EVENTFLAG_NONE;
  key_event.windows_key_code = static_cast<int>(ch);
  key_event.native_key_code = static_cast<int>(ch);
  key_event.character = static_cast<char16_t>(ch);
  (*browser->state->browser)->GetHost()->SendKeyEvent(key_event);
}

void wef_browser_ime_set_composition(WefBrowser* browser, const char* text,
                                     uint32_t cursor_begin,
                                     uint32_t cursor_end) {
  if (!browser->state->browser) {
    return;
  }
  (*browser->state->browser)
      ->GetHost()
      ->ImeSetComposition(text, {}, CefRange::InvalidRange(),
                          CefRange(cursor_begin, cursor_end));
}

void wef_browser_ime_commit(WefBrowser* browser, const char* text) {
  if (!browser->state->browser) {
    return;
  }
  (*browser->state->browser)
      ->GetHost()
      ->ImeCommitText(text, CefRange::InvalidRange(), 0);
}

WefFrame* wef_browser_get_main_frame(WefBrowser* browser) {
  if (!browser->state->browser) {
    return nullptr;
  }
  auto main_frame = (*browser->state->browser)->GetMainFrame();
  return main_frame ? new WefFrame{main_frame} : nullptr;
}

WefFrame* wef_browser_get_focused_frame(WefBrowser* browser) {
  if (!browser->state->browser) {
    return nullptr;
  }
  auto frame = (*browser->state->browser)->GetFocusedFrame();
  return frame ? new WefFrame{frame} : nullptr;
}

WefFrame* wef_browser_get_frame_by_name(WefBrowser* browser, const char* name) {
  if (!browser->state->browser) {
    return nullptr;
  }
  auto frame = (*browser->state->browser)->GetFrameByName(name);
  return frame ? new WefFrame{frame} : nullptr;
}

WefFrame* wef_browser_get_frame_by_identifier(WefBrowser* browser,
                                              const char* id) {
  if (!browser->state->browser) {
    return nullptr;
  }
  auto frame = (*browser->state->browser)->GetFrameByIdentifier(id);
  return frame ? new WefFrame{frame} : nullptr;
}

bool wef_browser_is_audio_muted(WefBrowser* browser, bool mute) {
  if (browser->state->browser) {
    return false;
  }
  return (*browser->state->browser)->GetHost()->IsAudioMuted();
}

void wef_browser_set_audio_mute(WefBrowser* browser, bool mute) {
  if (browser->state->browser) {
    return;
  }
  (*browser->state->browser)->GetHost()->SetAudioMuted(mute);
}

void wef_browser_find(WefBrowser* browser, const char* search_text,
                      bool forward, bool match_case, bool find_next) {
  if (!browser->state->browser) {
    return;
  }
  (*browser->state->browser)
      ->GetHost()
      ->Find(search_text, forward, match_case, find_next);
}

void wef_browser_set_focus(WefBrowser* browser, bool focus) {
  if (!browser->state->browser) {
    browser->state->focus = true;
    return;
  }
  (*browser->state->browser)->GetHost()->SetFocus(focus);
}

}  // extern "C"