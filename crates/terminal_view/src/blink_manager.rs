//! 光标闪烁管理器
//!
//! 参考 Zed 的 BlinkManager 实现，使用 epoch 版本控制防止竞态条件。
//!
//! 特性：
//! - 500ms 闪烁间隔
//! - 输入时暂停闪烁
//! - 焦点获得/失去时自动启用/禁用

use gpui::*;
use std::time::Duration;

/// 光标闪烁间隔
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);
/// 暂停后恢复闪烁的延迟
const PAUSE_RESUME_DELAY: Duration = Duration::from_millis(500);

/// 光标闪烁管理器
pub struct BlinkManager {
    /// 用于取消过时的异步任务
    blink_epoch: usize,
    /// 闪烁是否暂停
    blinking_paused: bool,
    /// 当前光标是否可见
    visible: bool,
    /// 闪烁是否启用
    enabled: bool,
}

impl BlinkManager {
    pub fn new() -> Self {
        Self {
            blink_epoch: 0,
            blinking_paused: false,
            visible: true,
            enabled: false,
        }
    }

    /// 启用光标闪烁（当终端获得焦点时调用）
    pub fn enable(&mut self, cx: &mut Context<Self>) {
        if self.enabled {
            return;
        }
        self.enabled = true;
        self.visible = true;
        self.blink_cursors(self.blink_epoch, cx);
    }

    /// 禁用光标闪烁（当终端失去焦点时调用）
    pub fn disable(&mut self, _cx: &mut Context<Self>) {
        self.enabled = false;
        self.visible = true; // 失焦时保持光标可见
    }

    /// 暂停闪烁并显示光标（用户输入时调用）
    ///
    /// 立即显示光标，并在 500ms 后恢复闪烁
    pub fn pause_blinking(&mut self, cx: &mut Context<Self>) {
        self.show_cursor(cx);
        self.blinking_paused = true;

        let epoch = self.next_epoch();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PAUSE_RESUME_DELAY).await;
            let _ = this.update(cx, |this, cx| this.resume_blinking(epoch, cx));
        })
        .detach();
    }

    /// 立即显示光标
    pub fn show_cursor(&mut self, cx: &mut Context<Self>) {
        self.visible = true;
        cx.notify();
    }

    /// 获取当前光标可见性
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// 检查闪烁是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn next_epoch(&mut self) -> usize {
        self.blink_epoch += 1;
        self.blink_epoch
    }

    fn resume_blinking(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if epoch == self.blink_epoch {
            self.blinking_paused = false;
            self.blink_cursors(epoch, cx);
        }
    }

    fn blink_cursors(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if epoch == self.blink_epoch && self.enabled && !self.blinking_paused {
            self.visible = !self.visible;
            cx.notify();

            let epoch = self.next_epoch();
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(CURSOR_BLINK_INTERVAL).await;
                let _ = this.update(cx, |this, cx| this.blink_cursors(epoch, cx));
            })
            .detach();
        }
    }
}

impl Default for BlinkManager {
    fn default() -> Self {
        Self::new()
    }
}
