#pragma once

#include <iostream>

#include "include/cef_app.h"

class ShutdownHelper {
 protected:
  ShutdownHelper() {}
  ShutdownHelper(ShutdownHelper&) = delete;

 public:
  static std::unique_ptr<ShutdownHelper>& getSingleton();

  void browserCreated() { ++alive_browsers_; }

  void browserDestroyed() {
    if (alive_browsers_ > 0) {
      --alive_browsers_;
    }
    if (alive_browsers_ == 0 && shutting_down_) {
      quit();
    }
  }

  void shutdown() {
    if (alive_browsers_ > 0) {
      shutting_down_ = true;
      run();
    }
  }

 protected:
  virtual void run() = 0;
  virtual void quit() = 0;

 private:
  unsigned int alive_browsers_ = 0;
  bool shutting_down_ = false;
};