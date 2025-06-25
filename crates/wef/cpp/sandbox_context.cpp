#include "include/cef_sandbox_mac.h"

extern "C" {

void* wef_sandbox_context_create(char* argv[], int argc) {
  CefScopedSandboxContext* ctx = new CefScopedSandboxContext();
  if (!ctx->Initialize(argc, argv)) {
    delete ctx;
    return nullptr;
  }
  return ctx;
}

void wef_sandbox_context_destroy(void* p) {
  CefScopedSandboxContext* ctx = static_cast<CefScopedSandboxContext*>(p);
  delete ctx;
}

}  // extern "C"