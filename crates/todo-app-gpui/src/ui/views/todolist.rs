use gpui::prelude::*;
use gpui::*;
use std::time::Duration;

use gpui_component::{
    button::{Button, ButtonGroup, ButtonVariants},
    h_flex,
    indicator::Indicator,
    label::Label,
    list::{List, ListDelegate, ListEvent, ListItem},
    popup_menu::PopupMenu,
    tab::TabBar,
    v_flex, ActiveTheme, IconName, Sizable, *,
};

use crate::ui::{views::todo_thread::TodoThreadChat, AppExt};

use super::todo_thread_edit::TodoThreadEdit;

actions!(
    list_story,
    [
        SelectedCompany,
        Open,
        Edit,
        Completed,
        Pause,
        Clone,
        Star,
        Delete
    ]
);

#[derive(Clone, Default)]
struct Todo {
    title: SharedString,
    description: SharedString,
}

#[derive(IntoElement)]
struct TodoItem {
    base: ListItem,
    ix: usize,
    item: Todo,
    selected: bool,
    in_progress: bool,
    star: bool,
    alert: bool,
    completed: bool,
}

impl TodoItem {
    pub fn new(id: impl Into<ElementId>, item: Todo, ix: usize, selected: bool) -> Self {
        TodoItem {
            item,
            ix,
            base: ListItem::new(id),
            selected,
            in_progress: true,
            star: false,
            alert: true,
            completed: false,
        }
    }
}

impl RenderOnce for TodoItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let selected = self.selected;
        let text_color = if self.selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().foreground
        };

        let bg_color = if self.selected {
            cx.theme().list_active
        } else if self.ix % 2 == 0 {
            cx.theme().list
        } else {
            cx.theme().list_even
        };

        self.base
            .px_3()
            .py_1()
            .overflow_x_hidden()
            .bg(bg_color)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_color(text_color)
                    .child(
                        v_flex()
                            .gap_2()
                            .max_w(px(500.))
                            .overflow_x_hidden()
                            .text_sm()
                            .child(Label::new(self.item.title.clone()).whitespace_nowrap())
                            .child(
                                div().text_ellipsis().text_sm().overflow_x_hidden().child(
                                    Label::new(self.item.description.clone())
                                        .text_color(text_color.opacity(0.5)),
                                ),
                            ),
                    )
                    .child(
                        v_flex()
                            .h_full()
                            .gap_1()
                            .items_end()
                            .justify_end()
                            .when(selected, |div| {
                                div.child(
                                    h_flex()
                                        .gap_1()
                                        .items_center()
                                        .justify_end()
                                        .when(self.in_progress, |div| {
                                            div.child(
                                                Indicator::new()
                                                    .with_size(px(16.))
                                                    .icon(IconName::RefreshCW)
                                                    .color(blue_500()),
                                            )
                                        })
                                        .when(!self.in_progress, |div| {
                                            div.child(
                                                Button::new("button-refresh")
                                                    .ghost()
                                                    .icon(IconName::RefreshCW)
                                                    .small()
                                                    .on_click(|event, win, app| {}),
                                            )
                                        })
                                        .child(
                                            Button::new("button-copy")
                                                .ghost()
                                                .icon(IconName::Copy)
                                                .small()
                                                .on_click(|event, win, app| {}),
                                        )
                                        .child(
                                            Button::new("button-star")
                                                .ghost()
                                                .icon(IconName::Star)
                                                .small()
                                                .on_click(|event, win, app| {}),
                                        ), // .child(
                                           //     Button::new("button-trash")
                                           //         .ghost()
                                           //         .icon(IconName::Trash)
                                           //         .small()
                                           //         .on_click(|event, win, app| {}),
                                           // ),
                                )
                            })
                            .child(
                                h_flex()
                                    // .child(IconName::Calendar)
                                    .child(
                                        Label::new("10/01 17:36")
                                            .whitespace_nowrap()
                                            .text_xs()
                                            .text_color(text_color.opacity(0.5)),
                                    ),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_color(text_color)
                    .child(
                        //todo信息
                        h_flex()
                            .items_center()
                            .justify_start()
                            .gap_2()
                            .when(self.alert, |div| {
                                div.child(
                                    Icon::new(IconName::TriangleAlert)
                                        .xsmall()
                                        .text_color(yellow_500()),
                                )
                            })
                            .child(Icon::new(IconName::Paperclip).xsmall())
                            .child(if self.completed {
                                Icon::new(IconName::RefreshCW).xsmall()
                            } else {
                                Icon::new(IconName::TimerReset)
                                    .xsmall()
                                    .text_color(green_500())
                            }),
                    )
                    .child(
                        // 模型信息
                        h_flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(Icon::new(IconName::Mic).xsmall())
                            .child(Icon::new(IconName::Image).xsmall())
                            .child(Icon::new(IconName::Brain).xsmall())
                            .child(Icon::new(IconName::Wrench).xsmall()),
                    ),
            )
    }
}

struct TodoListDelegate {
    companies: Vec<Todo>,
    matched_companies: Vec<Todo>,
    selected_index: Option<usize>,
    confirmed_index: Option<usize>,
    query: String,
    loading: bool,
    eof: bool,
}

impl ListDelegate for TodoListDelegate {
    type Item = TodoItem;

    fn items_count(&self, _: &App) -> usize {
        self.matched_companies.len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.matched_companies = self
            .companies
            .iter()
            .filter(|company| company.title.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();
        Task::ready(())
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedCompany), cx);
    }

    fn on_double_click(
        &mut self,
        ev: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        println!("Double clicked: {:?} {:?}", ev, self.selected_index);
        window.dispatch_action(Box::new(Open), cx);
    }

    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn render_item(
        &self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(company) = self.matched_companies.get(ix) {
            return Some(TodoItem::new(ix, company.clone(), ix, selected));
        }

        None
    }

    fn context_menu(
        &self,
        row_ix: usize,
        menu: PopupMenu,
        _window: &Window,
        _cx: &App,
    ) -> PopupMenu {
        println!("Context menu for row: {}", row_ix);
        menu.external_link_icon(true)
            // .link("About", "https://github.com/longbridge/gpui-component")
            .menu("打开", Box::new(Open))
            .menu("编辑", Box::new(Edit))
            .separator()
            .menu_with_icon("克隆", IconName::Copy, Box::new(Clone))
            .menu_with_icon("暂停", IconName::Pause, Box::new(Pause))
            .menu_with_icon("完成", IconName::Done, Box::new(Completed))
            .menu_with_icon("关注", IconName::Star, Box::new(Completed))
            // .separator()
            // .menu_with_check("删除", true, Box::new(ToggleCheck))
            .separator()
            .menu_with_icon("删除", IconName::Trash, Box::new(Delete))
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
    }

    fn can_load_more(&self, _: &App) -> bool {
        return !self.loading && !self.eof;
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<List<Self>>) {
        cx.spawn_in(window, async move |view, window| {
            // Simulate network request, delay 1s to load data.
            Timer::after(Duration::from_secs(1)).await;

            _ = view.update_in(window, move |view, window, cx| {
                let query = view.delegate().query.clone();
                view.delegate_mut()
                    .companies
                    .extend((0..200).map(|_| random_todo()));
                _ = view.delegate_mut().perform_search(&query, window, cx);
                view.delegate_mut().eof = view.delegate().companies.len() >= 6000;
            });
        })
        .detach();
    }
}

impl TodoListDelegate {
    fn selected_company(&self) -> Option<Todo> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.companies.get(ix).cloned()
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TodoFilter {
    #[default]
    All,
    Planned,
    Completed,
}
pub struct TodoList {
    focus_handle: FocusHandle,
    company_list: Entity<List<TodoListDelegate>>,
    selected_company: Option<Todo>,
    _subscriptions: Vec<Subscription>,
    todo_filter: TodoFilter,
    active_tab_ix: usize,
}

impl TodoList {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let companies = (0..1_000).map(|_| random_todo()).collect::<Vec<Todo>>();

        let delegate = TodoListDelegate {
            matched_companies: companies.clone(),
            companies,
            selected_index: Some(0),
            confirmed_index: None,
            query: "".to_string(),
            loading: false,
            eof: false,
        };

        let company_list = cx.new(|cx| List::new(delegate, window, cx));
        // company_list.update(cx, |list, cx| {
        //     list.set_selected_index(Some(3), cx);
        // });

        let _subscriptions = vec![
            // cx.subscribe(&company_list, |_, _, ev: &ListEvent, _| match ev {
            //     ListEvent::Select(ix) => {
            //         println!("List Selected: {:?}", ix);
            //     }
            //     ListEvent::Confirm(ix) => {
            //         println!("List Confirmed: {:?}", ix);
            //     }
            //     ListEvent::Cancel => {
            //         println!("List Cancelled");
            //     }
            // }),
        ];

        // Spawn a background to random refresh the list
        // cx.spawn(async move |this, cx| {
        //     this.update(cx, |this, cx| {
        //         this.company_list.update(cx, |picker, _| {
        //             picker
        //                 .delegate_mut()
        //                 .companies
        //                 .iter_mut()
        //                 .for_each(|company| {
        //                     company.random_update();
        //                 });
        //         });
        //         cx.notify();
        //     })
        //     .ok();
        // })
        // .detach();

        Self {
            focus_handle: cx.focus_handle(),
            company_list,
            selected_company: None,
            _subscriptions,
            todo_filter: TodoFilter::default(),
            active_tab_ix: 0,
        }
    }

    fn selected_company(&mut self, _: &SelectedCompany, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.company_list.read(cx);
        self.selected_company = picker.delegate().selected_company();
    }

    fn clone(&mut self, _: &Clone, _: &mut Window, cx: &mut Context<Self>) {
        println!("Clone action triggered");
    }

    fn open_todo(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(todo) = self.selected_company.clone() {
            cx.activate(true);
            let window_size = size(px(600.0), px(800.0));
            let window_bounds = Bounds::centered(None, window_size, cx);
            let options = WindowOptions {
                app_id: Some("x-todo-app".to_string()),
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(600.),
                    height: px(800.),
                }),
                kind: WindowKind::Normal,
                #[cfg(target_os = "linux")]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui::WindowDecorations::Client),
                ..Default::default()
            };
            cx.create_normal_window(
                format!("todo-{}", todo.title),
                options,
                move |window, cx| TodoThreadChat::view(window, cx),
            );
        }
    }

    fn edit_todo(&mut self, _: &Edit, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(todo) = self.selected_company.clone() {
            cx.activate(true);
            let window_size = size(px(600.0), px(650.0));
            let window_bounds = Bounds::centered(None, window_size, cx);
            let options = WindowOptions {
                app_id: Some("x-todo-app".to_string()),
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(600.),
                    height: px(650.),
                }),
                kind: WindowKind::PopUp,
                #[cfg(target_os = "linux")]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui::WindowDecorations::Client),
                ..Default::default()
            };
            cx.create_normal_window(
                format!("todo-{}", todo.title),
                options,
                move |window, cx| TodoThreadEdit::view(window, cx),
            );
        }
    }

    fn set_todo_filter(&mut self, filter: TodoFilter, _: &mut Window, cx: &mut Context<Self>) {
        self.todo_filter = filter;
        cx.notify();
    }

    fn set_active_tab(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.active_tab_ix = ix;
        match ix {
            0 => self.set_todo_filter(TodoFilter::All, window, cx),
            1 => self.set_todo_filter(TodoFilter::Planned, window, cx),
            2 => self.set_todo_filter(TodoFilter::Completed, window, cx),
            _ => {}
        }
        cx.notify();
    }
    fn set_scroll(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        println!("Scroll to: {}", ix);
        match ix {
            0 => self.company_list.update(cx, |list, cx| {
                list.scroll_to_item(0, window, cx);
            }),

            1 => self.company_list.update(cx, |list, cx| {
                if let Some(selected) = list.selected_index() {
                    list.scroll_to_item(selected, window, cx);
                }
            }),
            2 => self.company_list.update(cx, |list, cx| {
                list.scroll_to_item(list.delegate().items_count(cx) - 1, window, cx);
            }),
            _ => {}
        }
        cx.notify();
    }
}

fn random_todo() -> Todo {
    use fake::faker::name::zh_cn::Name;
    use fake::Fake;
    use rand::seq::SliceRandom;

    // 100条常见待办事项模板
    static TITLES: &[&str] = &[
        "完成{}的日报",
        "联系{}",
        "整理{}资料",
        "参加{}会议",
        "审核{}文档",
        "准备{}演示",
        "更新{}进度",
        "提交{}申请",
        "学习{}课程",
        "预约{}",
        "购买办公用品",
        "检查邮件",
        "备份数据",
        "清理桌面",
        "安排下周计划",
        "阅读新通知",
        "回复客户信息",
        "完善项目文档",
        "测试新功能",
        "优化代码",
        "部署到生产环境",
        "修复Bug",
        "撰写周报",
        "整理会议纪要",
        "统计本月数据",
        "制定预算",
        "确认会议时间",
        "准备面试材料",
        "安排培训",
        "更新简历",
        "打扫卫生",
        "采购物资",
        "归还借用物品",
        "打印文件",
        "扫描合同",
        "签署协议",
        "上传资料",
        "下载报告",
        "同步进度",
        "提醒同事",
        "预约体检",
        "缴纳费用",
        "续签合同",
        "申请报销",
        "整理发票",
        "归档文件",
        "检查设备",
        "维护服务器",
        "更新软件",
        "更换密码",
        "设置权限",
        "备份数据库",
        "清理缓存",
        "巡检网络",
        "测试打印机",
        "检查安全隐患",
        "安排值班",
        "统计考勤",
        "整理客户名单",
        "发送通知",
        "准备礼品",
        "安排聚餐",
        "预定场地",
        "确认嘉宾",
        "制作宣传海报",
        "发布公告",
        "收集反馈",
        "分析数据",
        "制定KPI",
        "组织团建",
        "安排面谈",
        "准备PPT",
        "整理照片",
        "编辑视频",
        "上传作品",
        "申请加班",
        "审批请假",
        "安排轮休",
        "检查库存",
        "补充物料",
        "联系供应商",
        "核对账单",
        "催收款项",
        "安排发货",
        "确认收货",
        "处理投诉",
        "回访客户",
        "更新官网",
        "维护公众号",
        "发布推文",
        "整理代码",
        "合并分支",
        "代码评审",
        "编写测试用例",
        "运行自动化测试",
        "生成报告",
        "优化性能",
        "升级依赖",
        "修订计划",
        "总结经验",
        "制定目标",
        "安排复盘",
    ];

    static DESCS: &[&str] = &[
        "请在今天下班前完成该任务，并将结果通过邮件反馈给负责人。如有疑问请及时沟通，确保进度顺利推进。",
        "与{}详细沟通需求，整理会议纪要并上传至系统，确保团队成员都能及时了解最新进展。",
        "将与{}相关的所有资料进行分类整理，按时间顺序归档，并在资料库中建立索引以便后续查找。",
        "请于明天下午三点准时参加与{}的线上会议，提前准备需要讨论的议题和相关文档，会议结束后整理纪要。",
        "对{}提交的文档进行全面审核，重点检查数据准确性和逻辑完整性，发现问题及时反馈并协助修改。",
        "为即将到来的{}演示准备PPT和讲稿，确保内容详实、逻辑清晰，并提前进行彩排保证顺利进行。",
        "每日更新{}的项目进度，将最新进展同步到团队群，并在周会时进行简要汇报，确保信息同步。",
        "填写并提交{}相关的申请表格，确保所有信息准确无误，提交后请关注审批进度并及时跟进。",
        "利用业余时间学习{}课程，做好学习笔记，遇到不懂的问题及时向讲师或同学请教，提升专业能力。",
        "提前预约{}的相关服务，确认时间和地点，避免与其他重要事项冲突，如有变动请及时调整。",
        "根据实际需求填写办公用品采购清单，控制预算，采购完成后做好入库登记并通知相关同事领取。",
        "每天定时检查工作邮箱，及时回复重要邮件，对需要处理的事项做好标记，避免遗漏关键信息。",
        "定期备份重要数据文件，将备份文件存储在安全位置，并做好备份记录，防止数据丢失造成损失。",
        "每周五下班前清理办公桌面和电脑桌面，归还借用物品，保持办公环境整洁有序，提高工作效率。",
        "根据本周工作情况，制定下周详细计划，明确每项任务的负责人和截止时间，并在周一早会进行说明。",
        "认真阅读公司发布的新通知，了解最新政策和流程变化，确保自己的工作符合公司要求。",
        "及时回复客户的各类信息，耐心解答客户疑问，维护良好的客户关系，提升客户满意度。",
        "完善项目文档，补充缺失部分，确保文档结构清晰、内容详实，方便团队成员查阅和维护。",
        "对新开发的功能进行全面测试，记录测试结果和发现的问题，及时反馈给开发人员进行修复。",
        "对现有代码进行优化，提升运行效率和可维护性，优化后请进行回归测试确保功能正常。",
        "统计本月数据，整理成报表，分析关键指标，为下月计划和决策提供数据支持。",
        "制定部门预算，合理分配各项支出，确保资金使用高效合规，及时向财务部门报备。",
        "确认下周会议时间，提前通知参会人员，准备好相关资料，确保会议顺利进行。",
        "准备面试材料，整理候选人简历，安排面试时间并通知相关人员，确保流程顺畅。",
        "安排新员工培训，准备培训资料，确保培训内容覆盖岗位要求，帮助新员工快速上手。",
        "更新个人简历，补充近期项目经验，突出个人优势，提升求职竞争力。",
        "打扫办公室卫生，清理公共区域，营造良好工作环境，提升团队士气。",
        "采购日常物资，确保库存充足，满足团队日常需求，避免因物资短缺影响工作。",
        "归还借用物品，做好登记，避免物品遗失，保持物品管理有序。",
        "打印重要文件，检查内容无误后分发给相关人员，确保信息传递准确。",
        "扫描合同文件，保存电子版并归档，便于后续查找和管理，提升工作效率。",
        "签署合作协议，确认条款无误后完成签字流程，确保双方权益得到保障。",
        "上传项目资料至云盘，设置访问权限，确保数据安全并方便团队成员查阅。",
        "下载最新报告，阅读并整理要点，准备汇报材料，确保信息传递到位。",
        "同步项目进度，定期与团队成员沟通，解决遇到的问题，保证项目顺利推进。",
        "提醒同事完成分配任务，协助解决遇到的困难，促进团队协作。",
        "预约年度体检，确认时间地点，提前安排好工作，确保健康检查顺利进行。",
        "缴纳各项费用，保存缴费凭证，及时报销，确保财务流程合规。",
        "续签合同，确认条款变更，完成签署流程，确保合作关系持续稳定。",
        "申请差旅报销，整理发票和行程单，提交审批，确保报销流程顺畅。",
        "整理本月发票，分类归档，方便财务查账，提高工作效率。",
        "归档历史文件，建立索引，便于后续查找和管理，提升资料管理水平。",
        "检查办公设备运行状态，发现故障及时报修，保障日常工作顺利进行。",
        "维护服务器，定期检查系统安全和性能，防止出现故障影响业务。",
        "更新常用软件，确保版本最新，避免安全风险，提升工作效率。",
        "更换重要账号密码，提升账户安全性，防止信息泄露。",
        "设置系统权限，合理分配访问级别，保障数据安全和合规性。",
        "备份数据库，定期检查备份有效性，防止数据丢失造成损失。",
        "清理系统缓存，释放存储空间，提升运行速度和系统稳定性。",
        "巡检公司网络，排查安全隐患，确保网络畅通和数据安全。",
        "测试打印机功能，发现异常及时维修，保障日常办公需求。",
        "检查办公区域安全隐患，完善应急预案，提升安全管理水平。",
        "安排本月值班表，确保各时段有人值守，保障公司正常运转。",
        "统计员工考勤数据，核对异常情况，及时反馈并处理。",
        "整理客户名单，补充联系方式，便于后续跟进和客户维护。",
        "发送会议通知，附上议程和相关资料，确保参会人员提前知晓。",
        "准备节日礼品，提前采购并包装，安排发放，提升员工归属感。",
        "安排团队聚餐，预定餐厅，通知所有成员，增强团队凝聚力。",
        "预定会议场地，确认设备齐全，提前布置现场，确保会议顺利进行。",
        "确认活动嘉宾名单，发送邀请函，跟进回复，确保嘉宾准时出席。",
        "制作宣传海报，突出活动主题，设计美观大方，吸引更多参与者。",
        "发布公司公告，确保所有员工及时知晓，促进信息透明。",
        "收集活动反馈，整理成报告，提出改进建议，提升活动效果。",
        "分析销售数据，找出增长点，制定提升方案，推动业绩增长。",
        "制定下月KPI，明确考核标准，分解到个人，提升团队目标感。",
        "组织团队建设活动，增强成员凝聚力，提升团队协作能力。",
        "安排员工面谈，了解工作状态，收集建议，促进员工成长。",
        "准备项目PPT，内容简明扼要，突出重点，便于汇报展示。",
        "整理活动照片，分类存档，便于后续宣传和资料留存。",
        "编辑活动视频，剪辑精彩片段，制作成宣传片，提升活动影响力。",
        "上传作品至平台，完善描述和标签，提升曝光度和影响力。",
        "申请加班审批，说明原因和时长，等待领导批准，确保流程合规。",
        "审批员工请假申请，核对请假原因和时间，合理安排工作。",
        "安排轮休计划，确保工作正常运转，兼顾员工休息需求。",
        "检查仓库库存，补充短缺物料，避免断货影响生产。",
        "联系供应商补货，确认发货时间和数量，确保物资及时到位。",
        "核对本月账单，发现异常及时核实，确保账目清晰。",
        "催收未付款项，保持与客户沟通，确保资金及时回笼。",
        "安排发货事宜，确认收货地址和联系人，确保货物准时送达。",
        "确认客户收货情况，收集反馈，提升服务质量和客户满意度。",
        "处理客户投诉，耐心沟通，提出解决方案，维护公司形象。",
        "回访重点客户，了解需求，维护合作关系，促进业务发展。",
        "更新公司官网内容，发布最新动态，提升企业形象。",
        "维护公众号，定期推送优质内容，提升粉丝活跃度。",
        "发布产品推文，突出卖点，吸引潜在客户，促进销售转化。",
        "整理项目代码，优化结构，提升可读性和维护性。",
        "合并开发分支，解决冲突，确保主干代码稳定。",
        "进行代码评审，提出优化建议，提升代码质量。",
        "编写测试用例，覆盖主要功能，确保系统稳定可靠。",
        "运行自动化测试，记录结果，及时修复缺陷。",
        "生成测试报告，分析问题，制定改进措施，提升产品质量。",
        "优化系统性能，提升响应速度和并发能力，改善用户体验。",
        "升级依赖库，确保兼容性和安全性，减少潜在风险。",
        "修订项目计划，调整时间节点，合理分配资源，确保项目顺利推进。",
        "总结项目经验，整理成文档，便于团队学习和知识传承。",
        "制定下阶段目标，明确重点任务和负责人，提升执行力。",
        "安排项目复盘，总结得失，提出改进建议，持续优化流程。",
    ];

    let mut rng = rand::thread_rng();
    let name: String = Name().fake();
    let title_tpl = TITLES.choose(&mut rng).unwrap();
    let desc_tpl = DESCS.choose(&mut rng).unwrap();

    Todo {
        title: title_tpl.replace("{}", &name).into(),
        description: desc_tpl.replace("{}", &name).into(),
        ..Default::default()
    }
}

impl Focusable for TodoList {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodoList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_company))
            .on_action(cx.listener(Self::clone))
            .on_action(cx.listener(Self::open_todo))
            .on_action(cx.listener(Self::edit_todo))
            .size_full()
            .gap_4()
            .child(
                // 顶部工具栏
                h_flex()
                    .gap_2()
                    .flex_nowrap()
                    .child(
                        TabBar::new("todo-list-tabs")
                            .w_full()
                            .segmented()
                            .selected_index(self.active_tab_ix)
                            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                this.set_active_tab(*ix, window, cx);
                            }))
                            .children(vec!["All", "Planned", "Completed"]),
                    )
                    .child(
                        ButtonGroup::new("button-group")
                            .small()
                            .child(
                                Button::new("icon-button-top")
                                    .icon(IconName::ArrowUpToLine)
                                    .size(px(24.))
                                    .ghost(),
                            )
                            .child(
                                Button::new("icon-button-selected")
                                    .icon(IconName::MousePointerClick)
                                    .size(px(24.))
                                    .ghost(),
                            )
                            .child(
                                Button::new("icon-button-bottom")
                                    .icon(IconName::ArrowDownToLine)
                                    .size(px(24.))
                                    .ghost(),
                            )
                            .on_click(cx.listener(|this, clicked: &Vec<usize>, window, cx| {
                                if clicked.contains(&0) {
                                    this.set_scroll(0, window, cx);
                                } else if clicked.contains(&1) {
                                    this.set_scroll(1, window, cx);
                                } else if clicked.contains(&2) {
                                    this.set_scroll(2, window, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("icon-button-add")
                            .icon(IconName::Plus)
                            .size(px(24.))
                            .compact()
                            .ghost()
                            .on_click(cx.listener(|this, ev, widnow, cx| {
                                cx.activate(true);
                                let window_size = size(px(600.0), px(570.0));
                                let window_bounds = Bounds::centered(None, window_size, cx);
                                let options = WindowOptions {
                                    app_id: Some("x-todo-app".to_string()),
                                    window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                                    titlebar: Some(TitleBar::title_bar_options()),
                                    window_min_size: Some(gpui::Size {
                                        width: px(600.),
                                        height: px(570.),
                                    }),

                                    kind: WindowKind::PopUp,
                                    #[cfg(target_os = "linux")]
                                    window_background:
                                        gpui::WindowBackgroundAppearance::Transparent,
                                    #[cfg(target_os = "linux")]
                                    window_decorations: Some(gpui::WindowDecorations::Client),
                                    ..Default::default()
                                };
                                cx.create_normal_window("Add Todo", options, move |window, cx| {
                                    TodoThreadEdit::view(window, cx)
                                });
                            })),
                    ),
            )
            .child(
                // 待办事项列表
                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(self.company_list.clone()),
            )
    }
}
