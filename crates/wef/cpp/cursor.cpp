#include "include/cef_render_handler.h"

extern "C" {

void wef_cursor_info_hotspot(const CefCursorInfo* info, cef_point_t* point) {
  *point = info->hotspot;
}

float wef_cursor_info_image_scale_factor(const CefCursorInfo* info) {
  return info->image_scale_factor;
}

const void* wef_cursor_info_buffer(const CefCursorInfo* info) {
  return info->buffer;
}

void wef_cursor_info_size(const CefCursorInfo* info, cef_size_t* size) {
  *size = info->size;
}

}  // extern "C"