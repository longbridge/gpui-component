#include "shutdown_helper.h"

class ShutdownHelperWin : public ShutdownHelper {
 public:
  ShutdownHelperWin() {}

 protected:
  virtual void run() {
    MSG msg;
    while (GetMessage(&msg, nullptr, 0, 0)) {
      TranslateMessage(&msg);
      DispatchMessage(&msg);
    }
  }

  virtual void quit() { PostMessage(nullptr, WM_QUIT, 0, 0); }
};

std::unique_ptr<ShutdownHelper>& ShutdownHelper::getSingleton() {
  static std::unique_ptr<ShutdownHelper> instance(new ShutdownHelperWin());
  return instance;
}
