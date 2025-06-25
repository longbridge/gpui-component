#include "include/wrapper/cef_library_loader.h"

extern "C" {

void* wef_load_library(bool helper) {
  CefScopedLibraryLoader* library_loader = new CefScopedLibraryLoader();
  if (!helper) {
    if (!library_loader->LoadInMain()) {
      delete library_loader;
      return nullptr;
    }
  } else {
    if (!library_loader->LoadInHelper()) {
      delete library_loader;
      return nullptr;
    }
  }
  return library_loader;
}

void wef_unload_library(void* loader) {
  CefScopedLibraryLoader* library_loader =
      static_cast<CefScopedLibraryLoader*>(loader);
  delete library_loader;
}

}  // extern "C"