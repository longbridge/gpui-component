use gpui::prelude::*;
use gpui::*;
use gpui_component::{context_menu::ContextMenuExt, h_flex, v_flex, ActiveTheme};
/// 故事章节组件，用于组织和展示相关的故事内容
#[derive(IntoElement)]
pub struct StorySection {
    base: Div,                 // 基础 div 元素
    title: AnyElement,         // 标题元素
    children: Vec<AnyElement>, // 子元素列表
}

impl StorySection {
    /// 设置最大宽度为中等（48rem）
    #[allow(unused)]
    pub fn max_w_md(mut self) -> Self {
        self.base = self.base.max_w(rems(48.));
        self
    }

    /// 设置最大宽度为大（64rem）
    #[allow(unused)]
    pub fn max_w_lg(mut self) -> Self {
        self.base = self.base.max_w(rems(64.));
        self
    }

    /// 设置最大宽度为超大（80rem）
    #[allow(unused)]
    pub fn max_w_xl(mut self) -> Self {
        self.base = self.base.max_w(rems(80.));
        self
    }

    /// 设置最大宽度为 2 倍超大（96rem）
    #[allow(unused)]
    pub fn max_w_2xl(mut self) -> Self {
        self.base = self.base.max_w(rems(96.));
        self
    }
}

// 实现父元素特征，允许添加子元素
impl ParentElement for StorySection {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

// 实现样式特征，允许应用样式
impl Styled for StorySection {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for StorySection {
    /// 渲染故事章节
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .gap_2() // 间距 2 单位
            .mb_5() // 底部外边距 5 单位
            .w_full() // 全宽
            .child(
                h_flex()
                    .justify_between() // 两端对齐
                    .w_full() // 全宽
                    .gap_4() // 间距 4 单位
                    .child(self.title), // 标题
            )
            .child(
                v_flex()
                    .p_4() // 内边距 4 单位
                    .overflow_x_hidden() // 隐藏水平溢出
                    .border_1() // 1 像素边框
                    .border_color(cx.theme().border) // 主题边框颜色
                    .rounded_lg() // 大圆角
                    .items_center() // 垂直居中
                    .justify_center() // 水平居中
                    .child(self.base.children(self.children)), // 内容
            )
    }
}

// 实现上下文菜单扩展
impl ContextMenuExt for StorySection {}

/// 创建新的故事章节
pub fn section(title: impl IntoElement) -> StorySection {
    StorySection {
        title: title.into_any_element(),
        base: h_flex()
            .flex_wrap() // 允许换行
            .justify_center() // 水平居中
            .items_center() // 垂直居中
            .w_full() // 全宽
            .gap_4(), // 间距 4 单位
        children: vec![],
    }
}
