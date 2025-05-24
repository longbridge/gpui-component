#include <iostream>

#include "include/wrapper/cef_message_router.h"
#include "utils.h"

extern "C" {

void wef_query_callback_success(
    CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback>* callback,
    const char* response) {
  (*callback)->Success(response);
}

void wef_query_callback_failure(
    CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback>* callback,
    const char* error) {
  (*callback)->Failure(-1, error);
}

void wef_query_callback_destroy(
    CefRefPtr<CefMessageRouterBrowserSide::Handler::Callback>* callback) {
  delete callback;
}

}  // extern "C"