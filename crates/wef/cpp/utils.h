#pragma once

#include <numeric>
#include <string>
#include <vector>

#include "include/internal/cef_string.h"

typedef void (*DestroyFn)(void*);

inline std::string join_strings(const std::vector<CefString>& strings,
                                const std::string& delimiter) {
  return std::accumulate(strings.begin(), strings.end(), std::string(),
                         [&](const std::string& a, const CefString& b) {
                           auto b_ = b.ToString();
                           return a.empty() ? b_ : a + delimiter + b_;
                         });
}

inline std::vector<CefString> split_string(const std::string& str,
                                           const std::string& delimiter) {
  std::vector<CefString> result;
  std::string::size_type start = 0;
  std::string::size_type end;

  while ((end = str.find(delimiter, start)) != std::string::npos) {
    result.emplace_back(CefString(str.substr(start, end - start)));
    start = end + delimiter.length();
  }
  auto last = str.substr(start);
  if (!last.empty()) {
    result.emplace_back(CefString(last));
  }
  return result;
}