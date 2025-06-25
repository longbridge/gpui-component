#include "frame.h"

#include <algorithm>

#include "include/cef_frame.h"

extern "C" {
void wef_frame_destroy(WefFrame* frame) { delete frame; }

bool wef_frame_is_valid(WefFrame* frame) { return frame->frame->IsValid(); }

bool wef_frame_is_main(WefFrame* frame) { return frame->frame->IsMain(); }

void wef_frame_name(WefFrame* frame, void* userdata,
                    void (*callback)(void*, const char*)) {
  auto name = frame->frame->GetName().ToString();
  callback(userdata, name.c_str());
}

void wef_frame_identifier(WefFrame* frame, void* userdata,
                          void (*callback)(void*, const char*)) {
  auto id = frame->frame->GetIdentifier().ToString();
  callback(userdata, id.c_str());
}

void wef_frame_get_url(WefFrame* frame, void* userdata,
                       void (*callback)(void*, const char*)) {
  auto url = frame->frame->GetURL().ToString();
  callback(userdata, url.c_str());
}

void wef_frame_load_url(WefFrame* frame, const char* url) {
  if (strlen(url) > 0) {
    frame->frame->LoadURL(url);
  }
}

WefFrame* wef_frame_parent(WefFrame* frame) {
  auto parent = frame->frame->GetParent();
  return parent ? new WefFrame{parent} : nullptr;
}

void wef_frame_undo(WefFrame* frame) { frame->frame->Undo(); }

void wef_frame_redo(WefFrame* frame) { frame->frame->Redo(); }

void wef_frame_cut(WefFrame* frame) { frame->frame->Cut(); }

void wef_frame_copy(WefFrame* frame) { frame->frame->Copy(); }

void wef_frame_paste(WefFrame* frame) { frame->frame->Paste(); }

void wef_frame_paste_and_match_style(WefFrame* frame) {
  frame->frame->PasteAndMatchStyle();
}

void wef_frame_delete(WefFrame* frame) { frame->frame->Delete(); }

void wef_frame_select_all(WefFrame* frame) { frame->frame->SelectAll(); }

void wef_frame_execute_javascript(WefFrame* frame, const char* code) {
  frame->frame->ExecuteJavaScript(code, frame->frame->GetURL(), 0);
}

}  // extern "C"
