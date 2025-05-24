#pragma once

#include <optional>

#include "client.h"
#include "include/cef_browser.h"

struct WefBrowser {
  std::string url;
  CefRefPtr<WefClient> client;
  bool deleteBrowser = false;
  std::optional<CefRefPtr<CefBrowser>> browser;
  int cursorX, cursorY;
};
