#include <glib.h>

#include "shutdown_helper.h"

class ShutdownHelperLinux : public ShutdownHelper {
 public:
  ShutdownHelperLinux() {}

 protected:
  virtual void run() {
    bool more_work_is_plausible = true;
    for (;;) {
      bool block = !more_work_is_plausible;

      more_work_is_plausible = g_main_context_iteration(context_, block);
      if (should_quit_) {
        break;
      }
    }
  }

  virtual void quit() { should_quit_ = true; }

 private:
  bool should_quit_ = false;
  GMainContext* context_ = g_main_context_default();
};

std::unique_ptr<ShutdownHelper>& ShutdownHelper::getSingleton() {
  static std::unique_ptr<ShutdownHelper> instance(new ShutdownHelperLinux());
  return instance;
}
