#pragma once

#include <stdint.h>

#include <memory>

class ExternalPump {
 public:
  ExternalPump() {}
  virtual ~ExternalPump() {}
  ExternalPump(const ExternalPump&) = delete;
  ExternalPump& operator=(const ExternalPump&) = delete;
  static std::unique_ptr<ExternalPump> Create();
  virtual void OnScheduleMessagePumpWork(int64_t delay_ms) = 0;

 protected:
  void OnScheduleWork(int64_t delay_ms);
  void OnTimerTimeout();

  virtual void SetTimer(int64_t delay_ms) = 0;
  virtual void KillTimer() = 0;
  virtual bool IsTimerPending() = 0;

 private:
  void DoWork();
  bool PerformMessageLoopWork();

  bool is_active_ = false;
  bool reentrancy_detected_ = false;
};
