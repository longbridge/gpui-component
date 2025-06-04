use gpui::*;
use gpui_component::dock::PanelControl;

pub(crate) mod settings;
pub(crate) mod todo_form;
pub(crate) mod todolist;

/// 故事特征，定义故事组件的基本行为
pub trait View: Focusable + Render + Sized {
    /// 获取故事类名
    fn klass() -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }

    /// 故事标题
    fn title() -> &'static str;

    /// 故事描述
    fn description() -> &'static str {
        ""
    }

    /// 是否可关闭
    fn closable() -> bool {
        true
    }

    /// 是否可缩放
    fn zoomable() -> Option<PanelControl> {
        Some(PanelControl::default())
    }

    /// 标题背景色
    fn title_bg() -> Option<Hsla> {
        None
    }

    /// 创建新视图
    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable>;

    /// 激活状态改变回调
    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut App) {
        let _ = active;
        let _ = window;
        let _ = cx;
    }

    /// 任意视图的激活状态改变回调
    fn on_active_any(view: AnyView, active: bool, window: &mut Window, cx: &mut App)
    where
        Self: 'static,
    {
        if let Some(story) = view.downcast::<Self>().ok() {
            cx.update_entity(&story, |story, cx| {
                story.on_active(active, window, cx);
            });
        }
    }
}
