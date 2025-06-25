#pragma once

#include "include/cef_client.h"
#include "include/cef_frame.h"
#include "include/wrapper/cef_message_router.h"

struct _ContextMenuParams {
  int x_crood;
  int y_crood;
  int type_flags;
  const char* link_url;
  const char* unfiltered_link_url;
  const char* source_url;
  bool has_image_contents;
  const char* title_text;
  const char* page_url;
  const char* frame_url;
  int media_type;
  int media_state_flags;
  const char* selection_text;
  bool is_editable;
  int edit_state_flags;
};

struct BrowserCallbacks {
  void (*on_created)(void* userdata);
  void (*on_closed)(void* userdata);
  void (*on_popup_show)(void* userdata, bool show);
  void (*on_popup_position)(void* userdata, const CefRect* rect);
  void (*on_paint)(void* userdata, int type, const void* dirty_rects,
                   const void* buffer, unsigned int width, unsigned int height);
  void (*on_address_changed)(void* userdata, void* frame, const char* url);
  void (*on_title_changed)(void* userdata, const char* title);
  void (*on_favicon_url_change)(void* userdata, const char** urls, int);
  void (*on_tooltip)(void* userdata, const char* text);
  void (*on_status_message)(void* userdata, const char* text);
  void (*on_console_message)(void* userdata, const char* message, int level,
                             const char* source, int line);
  bool (*on_cursor_changed)(void* userdata, int cursor_type,
                            const void* custom_cursor_info);
  void (*on_before_popup)(void* userdata, const char* url);
  void (*on_loading_progress_changed)(void* userdata, float progress);
  void (*on_loading_state_changed)(void* userdata, bool is_loading,
                                   bool can_go_back, bool can_go_forward);
  void (*on_load_start)(void* userdata, void* frame);
  void (*on_load_end)(void* userdata, void* frame);
  void (*on_load_error)(void* userdata, void* frame, const char* error_text,
                        const char* failed_url);
  void (*on_ime_composition_range_changed)(void* userdata, const CefRect* rect);
  bool (*on_file_dialog)(void* userdata, int mode, const char* title,
                         const char* default_file_path,
                         const char* accept_filters,
                         const char* accept_extensions,
                         const char* accept_descriptions,
                         CefRefPtr<CefFileDialogCallback>* callback);
  void (*on_context_menu)(void* userdata, void* frame,
                          const _ContextMenuParams* params);
  void (*on_find_result)(void* userdata, int identifier, int count,
                         const CefRect* selection_rect,
                         int active_match_ordinal, bool final_update);
  bool (*on_js_dialog)(void* userdata, int type, const char* message_text,
                       const char* default_prompt_text,
                       CefRefPtr<CefJSDialogCallback>* callback);
  void (*on_query)(
      void* userdata, void* frame, const char* payload,
      CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback>* callback);
};