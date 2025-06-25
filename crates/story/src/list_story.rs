// 引入标准库的时间模块
use std::time::Duration;

// 引入假数据生成库
use fake::Fake;
// 引入 GPUI 框架的核心组件和类型
use gpui::{
    actions, div, px, App, AppContext, Context, ElementId, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, RenderOnce, SharedString, Styled,
    Subscription, Task, Timer, Window,
};

// 引入自定义的 UI 组件库
use gpui_component::{
    button::Button,
    checkbox::Checkbox,
    h_flex, hsl,
    label::Label,
    list::{List, ListDelegate, ListEvent, ListItem},
    v_flex, ActiveTheme, Sizable,
};

actions!(story, [SelectedCompany]);

/// 公司数据结构，包含公司的基本信息和股价数据
#[derive(Clone, Default)]
struct Company {
    name: SharedString,     // 公司名称
    industry: SharedString, // 所属行业
    last_done: f64,         // 最新成交价
    prev_close: f64,        // 前收盘价

    change_percent: f64,              // 涨跌幅百分比
    change_percent_str: SharedString, // 涨跌幅百分比字符串
    last_done_str: SharedString,      // 最新成交价字符串
    prev_close_str: SharedString,     // 前收盘价字符串
                                      // description: String,         // 公司描述（注释掉）
}

impl Company {
    /// 预处理公司数据，计算涨跌幅并格式化字符串
    fn prepare(mut self) -> Self {
        // 计算涨跌幅百分比
        self.change_percent = (self.last_done - self.prev_close) / self.prev_close;
        // 格式化涨跌幅为百分比字符串
        self.change_percent_str = format!("{:.2}%", self.change_percent).into();
        // 格式化最新成交价字符串
        self.last_done_str = format!("{:.2}", self.last_done).into();
        // 格式化前收盘价字符串
        self.prev_close_str = format!("{:.2}", self.prev_close).into();
        self
    }

    /// 随机更新股价数据，模拟实时价格变化
    fn random_update(&mut self) {
        // 基于前收盘价，在 -20% 到 +20% 范围内随机变化
        self.last_done = self.prev_close * (1.0 + (-0.2..0.2).fake::<f64>());
    }
}

/// 公司列表项组件，用于渲染单个公司的信息
#[derive(IntoElement)]
struct CompanyListItem {
    base: ListItem,   // 基础列表项组件
    ix: usize,        // 在列表中的索引
    company: Company, // 公司数据
    selected: bool,   // 是否被选中
}

impl CompanyListItem {
    /// 创建新的公司列表项
    pub fn new(id: impl Into<ElementId>, company: Company, ix: usize, selected: bool) -> Self {
        CompanyListItem {
            company,
            ix,
            base: ListItem::new(id),
            selected,
        }
    }
}

impl RenderOnce for CompanyListItem {
    /// 渲染公司列表项的视觉表现
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // 根据选中状态确定文本颜色
        let text_color = if self.selected {
            cx.theme().accent_foreground // 选中时使用强调色
        } else {
            cx.theme().foreground // 未选中时使用默认前景色
        };

        // 根据涨跌幅确定趋势颜色
        let trend_color = match self.company.change_percent {
            change if change > 0.0 => hsl(0.0, 79.0, 53.0), // 上涨：红色
            change if change < 0.0 => hsl(100.0, 79.0, 53.0), // 下跌：绿色
            _ => cx.theme().foreground,                     // 平盘：默认色
        };

        // 根据选中状态和索引确定背景色
        let bg_color = if self.selected {
            cx.theme().list_active // 选中时的背景色
        } else if self.ix % 2 == 0 {
            cx.theme().list // 偶数行背景色
        } else {
            cx.theme().list_even // 奇数行背景色
        };

        // 构建列表项的布局结构
        self.base
            .px_3() // 水平内边距 3 单位
            .py_1() // 垂直内边距 1 单位
            .overflow_x_hidden() // 隐藏水平溢出
            .bg(bg_color) // 设置背景色
            .child(
                h_flex() // 水平弹性布局
                    .items_center() // 垂直居中对齐
                    .justify_between() // 两端对齐
                    .gap_2() // 间距 2 单位
                    .text_color(text_color) // 设置文本颜色
                    .child(
                        // 左侧：公司名称和行业信息
                        v_flex() // 垂直弹性布局
                            .gap_1() // 间距 1 单位
                            .max_w(px(500.)) // 最大宽度 500 像素
                            .overflow_x_hidden() // 隐藏水平溢出
                            .flex_nowrap() // 不换行
                            .child(Label::new(self.company.name.clone()).whitespace_nowrap()) // 公司名称
                            .child(
                                div().text_sm().overflow_x_hidden().child(
                                    Label::new(self.company.industry.clone())
                                        .whitespace_nowrap()
                                        .text_color(text_color.opacity(0.5)), // 行业信息（半透明）
                                ),
                            ),
                    )
                    .child(
                        // 右侧：股价和涨跌幅信息
                        h_flex()
                            .gap_2() // 间距 2 单位
                            .items_center() // 垂直居中
                            .justify_end() // 右对齐
                            .child(
                                // 最新成交价
                                div()
                                    .w(px(65.)) // 固定宽度 65 像素
                                    .text_color(text_color)
                                    .child(self.company.last_done_str.clone()),
                            )
                            .child(
                                // 涨跌幅
                                h_flex().w(px(65.)).justify_end().child(
                                    div()
                                        .rounded(cx.theme().radius) // 圆角
                                        .whitespace_nowrap() // 不换行
                                        .text_size(px(12.)) // 字体大小 12 像素
                                        .px_1() // 水平内边距 1 单位
                                        .text_color(trend_color) // 趋势颜色
                                        .child(self.company.change_percent_str.clone()),
                                ),
                            ),
                    ),
            )
    }
}

/// 公司列表代理，实现列表的数据管理和行为控制
struct CompanyListDelegate {
    companies: Vec<Company>,         // 所有公司数据
    matched_companies: Vec<Company>, // 匹配搜索条件的公司
    selected_index: Option<usize>,   // 当前选中的索引
    confirmed_index: Option<usize>,  // 确认选中的索引
    query: String,                   // 搜索查询字符串
    loading: bool,                   // 是否正在加载
    eof: bool,                       // 是否到达末尾
}

impl ListDelegate for CompanyListDelegate {
    type Item = CompanyListItem;

    /// 返回匹配项的总数
    fn items_count(&self, _: &App) -> usize {
        self.matched_companies.len()
    }

    /// 执行搜索操作，过滤匹配的公司
    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        // 按公司名称进行不区分大小写的搜索
        self.matched_companies = self
            .companies
            .iter()
            .filter(|company| company.name.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();
        Task::ready(())
    }

    /// 确认选择操作
    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        // 分发公司选择动作
        window.dispatch_action(Box::new(SelectedCompany), cx);
    }

    /// 设置选中的索引
    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify(); // 通知界面更新
    }

    /// 渲染指定索引的列表项
    fn render_item(
        &self,
        ix: usize,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        // 判断是否被选中或确认
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(company) = self.matched_companies.get(ix) {
            return Some(CompanyListItem::new(ix, company.clone(), ix, selected));
        }

        None
    }

    /// 返回当前加载状态
    fn loading(&self, _: &App) -> bool {
        self.loading
    }

    /// 判断是否可以加载更多数据
    fn can_load_more(&self, _: &App) -> bool {
        return !self.loading && !self.eof;
    }

    /// 触发加载更多的阈值
    fn load_more_threshold(&self) -> usize {
        150
    }

    /// 加载更多数据的实现
    fn load_more(&mut self, window: &mut Window, cx: &mut Context<List<Self>>) {
        cx.spawn_in(window, async move |view, window| {
            // 模拟网络请求，延迟 1 秒加载数据
            Timer::after(Duration::from_secs(1)).await;

            _ = view.update_in(window, move |view, window, cx| {
                let query = view.delegate().query.clone();
                // 添加 200 个新的随机公司数据
                view.delegate_mut()
                    .companies
                    .extend((0..200).map(|_| random_company()));
                // 重新执行搜索
                _ = view.delegate_mut().perform_search(&query, window, cx);
                // 如果总数据量达到 6000，则设置为末尾
                view.delegate_mut().eof = view.delegate().companies.len() >= 6000;
            });
        })
        .detach(); // 分离任务，在后台运行
    }
}

impl CompanyListDelegate {
    /// 获取当前选中的公司
    fn selected_company(&self) -> Option<Company> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.companies.get(ix).cloned()
    }
}

/// 列表故事主组件，展示公司列表的功能
pub struct ListStory {
    focus_handle: FocusHandle,                       // 焦点处理句柄
    company_list: Entity<List<CompanyListDelegate>>, // 公司列表组件
    selected_company: Option<Company>,               // 当前选中的公司
    _subscriptions: Vec<Subscription>,               // 事件订阅列表
}

/// 实现 Story trait，定义组件的基本信息
impl super::Story for ListStory {
    /// 返回组件标题
    fn title() -> &'static str {
        "List"
    }

    /// 返回组件描述
    fn description() -> &'static str {
        "A list displays a series of items."
    }

    /// 创建新的视图实例
    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl ListStory {
    /// 创建 ListStory 视图的公共方法
    /// 使用工厂模式，通过 cx.new() 创建新的实体
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    /// ListStory 的构造函数，初始化所有必要的组件和状态
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 生成 1000 个随机公司数据作为示例数据
        let companies = (0..1_000)
            .map(|_| random_company())
            .collect::<Vec<Company>>();

        // 创建列表代理对象，管理列表的状态和行为
        let delegate = CompanyListDelegate {
            matched_companies: companies.clone(), // 匹配搜索条件的公司列表
            companies,                            // 所有公司的完整列表
            selected_index: Some(0),              // 当前选中的索引，默认选中第一项
            confirmed_index: None,                // 确认选中的索引
            query: "".to_string(),                // 当前搜索查询字符串
            loading: false,                       // 是否正在加载数据
            eof: false,                           // 是否已经到达数据末尾
        };

        // 创建 List 组件实体，传入代理对象和上下文
        let company_list = cx.new(|cx| List::new(delegate, window, cx));

        // 注释掉的代码：可以手动设置选中索引
        // company_list.update(cx, |list, cx| {
        //     list.set_selected_index(Some(3), cx);
        // });

        // 订阅列表事件，监听选择、确认和取消操作
        let _subscriptions =
            vec![
                cx.subscribe(&company_list, |_, _, ev: &ListEvent, _| match ev {
                    // 当列表项被选中时触发
                    ListEvent::Select(ix) => {
                        println!("List Selected: {:?}", ix);
                    }
                    // 当列表项被确认（如按回车键）时触发
                    ListEvent::Confirm(ix) => {
                        println!("List Confirmed: {:?}", ix);
                    }
                    // 当列表操作被取消时触发
                    ListEvent::Cancel => {
                        println!("List Cancelled");
                    }
                }),
            ];

        // 启动后台任务，定期随机更新公司数据以模拟实时数据变化
        cx.spawn(async move |this, cx| {
            // 更新当前组件实例
            this.update(cx, |this, cx| {
                // 更新公司列表，遍历所有公司并随机更新其数据
                this.company_list.update(cx, |picker, _| {
                    picker
                        .delegate_mut() // 获取代理对象的可变引用
                        .companies // 访问公司列表
                        .iter_mut() // 创建可变迭代器
                        .for_each(|company| {
                            // 对每个公司执行随机更新
                            company.random_update();
                        });
                });
                // 通知界面需要重新渲染
                cx.notify();
            })
            .ok(); // 忽略更新可能的错误
        })
        .detach(); // 分离任务，让其在后台独立运行

        // 返回新创建的 ListStory 实例
        Self {
            focus_handle: cx.focus_handle(), // 创建焦点处理句柄
            company_list,                    // 公司列表组件
            selected_company: None,          // 当前选中的公司，初始为空
            _subscriptions,                  // 事件订阅列表
        }
    }

    /// 处理公司选择动作的方法
    /// 当用户确认选择某个公司时调用
    fn selected_company(&mut self, _: &SelectedCompany, _: &mut Window, cx: &mut Context<Self>) {
        // 读取公司列表组件的状态
        let picker = self.company_list.read(cx);

        // 如果有选中的公司，则更新当前组件的选中状态
        if let Some(company) = picker.delegate().selected_company() {
            self.selected_company = Some(company);
        }
    }
}

/// 生成随机公司数据的工具函数
fn random_company() -> Company {
    // 生成随机的最新成交价
    let last_done = (0.0..999.0).fake::<f64>();
    // 基于最新成交价生成前收盘价（在 -10% 到 +10% 范围内变化）
    let prev_close = last_done * (-0.1..0.1).fake::<f64>();

    Company {
        // 生成假的公司名称
        name: fake::faker::company::en::CompanyName()
            .fake::<String>()
            .into(),
        // 生成假的行业名称
        industry: fake::faker::company::en::Industry().fake::<String>().into(),
        last_done,
        prev_close,
        ..Default::default() // 其他字段使用默认值
    }
    .prepare() // 预处理数据，计算涨跌幅等
}

/// 实现焦点管理接口
impl Focusable for ListStory {
    /// 返回焦点处理句柄
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// 实现渲染接口，定义组件的视觉表现
impl Render for ListStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex() // 垂直弹性布局
            .track_focus(&self.focus_handle) // 跟踪焦点
            .on_action(cx.listener(Self::selected_company)) // 监听公司选择动作
            .size_full() // 占满整个空间
            .gap_4() // 间距 4 单位
            .child(
                // 顶部按钮组
                h_flex() // 水平弹性布局
                    .gap_2() // 间距 2 单位
                    .flex_wrap() // 允许换行
                    .child(
                        // 滚动到顶部按钮
                        Button::new("scroll-top")
                            .child("Scroll to Top")
                            .small() // 小尺寸
                            .on_click(cx.listener(|this, _, window, cx| {
                                // 滚动到第一项
                                this.company_list.update(cx, |list, cx| {
                                    list.scroll_to_item(0, window, cx);
                                })
                            })),
                    )
                    .child(
                        // 滚动到底部按钮
                        Button::new("scroll-bottom")
                            .child("Scroll to Bottom")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                // 滚动到最后一项
                                this.company_list.update(cx, |list, cx| {
                                    list.scroll_to_item(
                                        list.delegate().items_count(cx) - 1,
                                        window,
                                        cx,
                                    );
                                })
                            })),
                    )
                    .child(
                        // 滚动到选中项按钮
                        Button::new("scroll-to-selected")
                            .child("Scroll to Selected")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                // 滚动到当前选中的项
                                this.company_list.update(cx, |list, cx| {
                                    if let Some(selected) = list.selected_index() {
                                        list.scroll_to_item(selected, window, cx);
                                    }
                                })
                            })),
                    )
                    .child(
                        // 加载状态复选框
                        Checkbox::new("loading")
                            .label("Loading")
                            .checked(self.company_list.read(cx).delegate().loading) // 绑定加载状态
                            .on_click(cx.listener(|this, check: &bool, _, cx| {
                                // 切换加载状态
                                this.company_list.update(cx, |this, cx| {
                                    this.delegate_mut().loading = *check;
                                    cx.notify();
                                })
                            })),
                    ),
            )
            .child(
                // 主列表容器
                div()
                    .flex_1() // 占满剩余空间
                    .w_full() // 全宽
                    .border_1() // 1 像素边框
                    .border_color(cx.theme().border) // 主题边框颜色
                    .rounded(cx.theme().radius) // 主题圆角
                    .child(self.company_list.clone()), // 包含公司列表组件
            )
    }
}
