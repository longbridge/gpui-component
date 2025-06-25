#pragma once

#include <cstdint>

struct AppCallbacks {
  void (*on_schedule_message_pump_work)(void* userdata, int delay_ms);
};
