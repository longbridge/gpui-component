#include "include/cef_render_handler.h"

extern "C" {

int wef_dirty_rects_len(const CefRenderHandler::RectList* dirtyRects) {
  return static_cast<int>(dirtyRects->size());
}

void wef_dirty_rects_get(const CefRenderHandler::RectList* dirtyRects, int i,
                         CefRect* rect) {
  *rect = dirtyRects->at(i);
}

}  // extern "C"