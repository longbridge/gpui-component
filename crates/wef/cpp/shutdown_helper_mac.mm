#include "shutdown_helper.h"

#import <AppKit/AppKit.h>

class ShutdownHelperMac : public ShutdownHelper {
 public:
  ShutdownHelperMac() {}

 protected:
  virtual void run() {
    [NSApp run];
  }

  virtual void quit() {
    [NSApp stop:nil];
}
};

std::unique_ptr<ShutdownHelper>& ShutdownHelper::getSingleton() {
  static std::unique_ptr<ShutdownHelper> instance(new ShutdownHelperMac());
  return instance;
}
