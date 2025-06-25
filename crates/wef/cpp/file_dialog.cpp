#include "include/cef_dialog_handler.h"
#include "utils.h"

extern "C" {

void wef_file_dialog_callback_continue(
    CefRefPtr<CefFileDialogCallback>* callback, const char* file_paths) {
  (*callback)->Continue(split_string(file_paths, ";"));
}

void wef_file_dialog_callback_cancel(
    CefRefPtr<CefFileDialogCallback>* callback) {
  (*callback)->Cancel();
}

void wef_file_dialog_callback_destroy(
    CefRefPtr<CefFileDialogCallback>* callback) {
  delete callback;
}

}  // extern "C"