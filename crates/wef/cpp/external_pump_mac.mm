#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>

#include "external_pump.h"

@class EventHandler;

class ExternalPumpMac : public ExternalPump {
 public:
  ExternalPumpMac();
  ~ExternalPumpMac();   

  void OnScheduleMessagePumpWork(int64_t delay_ms) override;
  void HandleScheduleWork(int64_t delay_ms);
  void HandleTimerTimeout();

 protected:
  void SetTimer(int64_t delay_ms) override;
  void KillTimer() override;
  bool IsTimerPending() override { return timer_ != nil; }

 private:
  NSThread* owner_thread_;
  NSTimer* timer_;
  EventHandler* event_handler_;
};

// Object that handles event callbacks on the owner thread.
@interface EventHandler : NSObject {
 @private
  ExternalPumpMac* pump_;
}

- (id)initWithPump:(ExternalPumpMac*)pump;
- (void)scheduleWork:(NSNumber*)delay_ms;
- (void)timerTimeout:(id)obj;
@end

@implementation EventHandler

- (id)initWithPump:(ExternalPumpMac*)pump {
  if (self = [super init]) {
    pump_ = pump;
  }
  return self;
}

- (void)scheduleWork:(NSNumber*)delay_ms {
  pump_->HandleScheduleWork([delay_ms integerValue]);
}

- (void)timerTimeout:(id)obj {
  pump_->HandleTimerTimeout();
}

@end

ExternalPumpMac::ExternalPumpMac()
    : owner_thread_([NSThread currentThread]), timer_(nil) {
#if !__has_feature(objc_arc)
  [owner_thread_ retain];
#endif  // !__has_feature(objc_arc)
  event_handler_ = [[EventHandler alloc] initWithPump:this];
}

ExternalPumpMac::~ExternalPumpMac() {
  KillTimer();
#if !__has_feature(objc_arc)
  [owner_thread_ release];
  [event_handler_ release];
#endif  // !__has_feature(objc_arc)
  owner_thread_ = nil;
  event_handler_ = nil;
}

void ExternalPumpMac::OnScheduleMessagePumpWork(
    int64_t delay_ms) {
  // This method may be called on any thread.
  NSNumber* number = [NSNumber numberWithInt:static_cast<int>(delay_ms)];
  [event_handler_ performSelector:@selector(scheduleWork:)
                         onThread:owner_thread_
                       withObject:number
                    waitUntilDone:NO];
}

void ExternalPumpMac::HandleScheduleWork(int64_t delay_ms) {
  OnScheduleWork(delay_ms);
}

void ExternalPumpMac::HandleTimerTimeout() {
  OnTimerTimeout();
}

void ExternalPumpMac::SetTimer(int64_t delay_ms) {
  const double delay_s = static_cast<double>(delay_ms) / 1000.0;
  timer_ = [NSTimer timerWithTimeInterval:delay_s
                                   target:event_handler_
                                 selector:@selector(timerTimeout:)
                                 userInfo:nil
                                  repeats:NO];
#if !__has_feature(objc_arc)
  [timer_ retain];
#endif  // !__has_feature(objc_arc)

  // Add the timer to default and tracking runloop modes.
  NSRunLoop* owner_runloop = [NSRunLoop currentRunLoop];
  [owner_runloop addTimer:timer_ forMode:NSRunLoopCommonModes];
  [owner_runloop addTimer:timer_ forMode:NSEventTrackingRunLoopMode];
}

void ExternalPumpMac::KillTimer() {
  if (timer_ != nil) {
    [timer_ invalidate];
#if !__has_feature(objc_arc)
    [timer_ release];
#endif  // !__has_feature(objc_arc)
    timer_ = nil;
  }
}

std::unique_ptr<ExternalPump> ExternalPump::Create() {
  return std::make_unique<ExternalPumpMac>();
}
