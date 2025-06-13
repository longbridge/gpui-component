#include "include/cef_jsdialog_handler.h"
#include "utils.h"

extern "C" {

void wef_js_dialog_callback_continue(CefRefPtr<CefJSDialogCallback>* callback,
                                     bool success, const char* user_input) {
  (*callback)->Continue(success, user_input);
}

void wef_js_dialog_callback_destroy(CefRefPtr<CefJSDialogCallback>* callback) {
  delete callback;
}

}  // extern "C"