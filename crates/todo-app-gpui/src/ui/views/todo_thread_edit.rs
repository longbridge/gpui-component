use crate::app::AppExt;
#[cfg(target_os = "windows")]
use crate::app::WindowExt;
use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use crate::backoffice::mcp::McpRegistry; // 新增导入
use crate::config::mcp_config::McpConfigManager;
use crate::config::llm_config::LlmProviderManager;
use crate::config::todo_item::*;
use crate::config::llm_config::ModelInfo;
use crate::ui::views::todo_thread::{Tab, TabPrev};
use crate::{app::AppState, config::llm_config::LlmProviderConfig};

// 从 rmcp 导入 MCP 类型
use rmcp::model::{Tool as McpTool};

use chrono::Utc;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    input::{InputEvent, InputState, TextInput},
    label::Label,
    notification::NotificationType,
    tooltip::Tooltip,
    *,
};

actions!(todo_thread, [ Save, Cancel, Delete]);

const CONTEXT: &str = "TodoThreadEdit";

pub struct TodoThreadEdit {
    focus_handle: FocusHandle,
    description_input: Entity<InputState>,
    // 时间设置
    due_date_picker: Entity<DatePickerState>,
    reminder_date_picker: Entity<DatePickerState>,
    recurring_input: Entity<InputState>,
    // 手风琴展开状态
    expanded_providers: Vec<usize>,
    expanded_tool_providers: Vec<usize>,
    // 缓存从 McpRegistry 获取的工具数据
    cached_server_tools: std::collections::HashMap<String, Vec<McpTool>>,
    _subscriptions: Vec<Subscription>,

    todoitem: Todo,
}

//实现业务操作
impl TodoThreadEdit {
    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
    }

    fn save(&mut self, save: &Save, window: &mut Window, cx: &mut Context<Self>) {
        self.todoitem.description = self.description_input.read(cx).value().to_string();
       
        match TodoManager::update_todo(self.todoitem.clone()) {
            Ok(_) => {
                // TODO: 处理保存成功的情况
                //_window.push_notification((NotificationType::Success, "Todo保存成功"), cx);
                cx.dispatch_global_action(save.boxed_clone());
            }
            Err(err) => {
                // TODO: 处理保存失败的情况
                window.push_notification(
                    (
                        NotificationType::Error,
                        SharedString::new(format!("Todo保存失败-{}", err)),
                    ),
                    cx,
                );
            }
        }
        cx.notify();
    }

    fn cancel(&mut self, _: &Cancel, _window: &mut Window, cx: &mut Context<Self>) {
        println!("取消编辑");
        cx.notify();
    }

    fn delete(&mut self, _: &Delete, _window: &mut Window, cx: &mut Context<Self>) {
        println!("删除Todo");
        cx.notify();
    }

    fn toggle_audio_recording(&mut self,  _: &mut Window, cx: &mut Context<Self>) {
        self.todoitem.toggle_needs_recording();
        cx.notify();
    }

    fn toggle_screen_recording(&mut self,  _: &mut Window, cx: &mut Context<Self>) {
        self.todoitem.toggle_needs_screen_recording();
        cx.notify();
    }

    fn on_input_event(
        &mut self,
        _entity: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => {
                window.dispatch_action(Box::new(Save), cx);
            }
            _ => {}
        }
    }

    // 获取模型选择显示文本
    fn get_model_display_text(&self, _cx: &App) -> String {
        if let Some(selected_model) = &self.todoitem.selected_model {
           selected_model.model_name.clone()
        }else{
            "选择模型".to_string()
        }
        
    }

    // 获取工具选择显示文本
    fn get_tool_display_text(&self, _cx: &App) -> String {
        let selected_count = self.todoitem.selected_tools.len();

        if selected_count == 0 {
            "选择工具".to_string()
        } else if selected_count <= 2 {
            self.todoitem
                .selected_tools
                .iter()
                .map(|item| item.tool_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            let first_two = self
                .todoitem
                .selected_tools
                .iter()
                .take(2)
                .map(|item| item.tool_name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} 等{}个工具", first_two, selected_count)
        }
    }

    fn toggle_accordion(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.expanded_providers = open_indices.to_vec();
        cx.notify();
    }

    fn toggle_tool_accordion(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.expanded_tool_providers = open_indices.to_vec();
        cx.notify();
    }

    fn toggle_model_selection(
        &mut self,
        checked: bool,
        model: &ModelInfo,
        provider: &LlmProviderConfig,
        cx: &mut Context<Self>,
    ) {
        if checked {
            // 如果选中，则添加
            self.todoitem
                .selected_model=Some(crate::config::todo_item::SelectedModel {
                    provider_id: provider.id.clone(),
                    provider_name: provider.name.clone(),
                    model_id: model.id.clone(),
                    model_name: model.display_name.clone(),
                });
        } else {
            // 如果未选中，则移除
            self.todoitem
                .selected_model=None;
        }
        cx.notify(); // 通知主界面更新
    }

    fn toggle_tool_selection(
        &mut self,
        checked: bool,
        tool: &McpTool,
        server: &crate::config::mcp_config::McpServerConfig, // 更新参数类型
        cx: &mut Context<Self>,
    ) {
        if checked {
            // 如果选中，则添加
            self.todoitem
                .selected_tools
                .push(crate::config::todo_item::SelectedTool {
                    provider_id: server.id.clone(),
                    provider_name: server.name.clone(),
                    description: tool.description.as_ref().map(|s| s.to_string()).unwrap_or_default(),
                    tool_name: tool.name.to_string(),
                });
        } else {
            // 如果未选中，则移除
            self.todoitem
                .selected_tools
                .retain(|t| t.tool_name != tool.name || t.provider_id != server.id);
        }
        cx.notify(); // 通知主界面更新
    }

    // 异步加载服务器工具
    fn load_server_tools_async(&mut self, server_id: String, cx: &mut Context<Self>) {
        let todo_edit_entity = cx.entity().clone();
        
        cx.spawn(async move |_this, cx| {
            if let Some(snapshot) = CrossRuntimeBridge::global().get_server_snapshot(&server_id).await {
                  let tools=snapshot.tools;
                todo_edit_entity.update(cx, |todo_edit, todo_cx| {
                            todo_edit.cached_server_tools.insert(server_id.clone(), tools);
                            todo_cx.notify();
                        }).ok();
            }
        }).detach();
    }

    // 获取缓存的工具数据
    fn get_server_tools(&self, server_id: &str) -> Vec<McpTool> {
        self.cached_server_tools.get(server_id).cloned().unwrap_or_default()
    }
}

const WIDTH: Pixels = px(700.0);
const HEIGHT: Pixels = px(650.0);
const SIZE: gpui::Size<Pixels> = size(WIDTH, HEIGHT);

// 实现 TodoThreadEdit界面的相关方法
impl TodoThreadEdit {
    pub fn edit(todo: Todo, parent: &mut Window, cx: &mut App)->WindowHandle<Root> {
        cx.activate(true);
        let window_bounds = Bounds::centered(None, SIZE, cx);
        let options = WindowOptions {
            app_id: Some("x-todo-app".to_string()),
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(SIZE),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };
        let parent_handle = parent.window_handle();
        cx.create_normal_window(
            format!("xTo-Do {}", todo.title),
            options,
            move |window, cx| cx.new(|cx| Self::new(todo, parent_handle, window, cx)),
        )
        // #[cfg(target_os = "windows")]
        // parent.enable_window(false);
    }

    pub fn add(parent: &mut Window, cx: &mut App) {
        cx.activate(true);
        let window_size = SIZE;
        let window_bounds = Bounds::centered(None, window_size, cx);
        let options = WindowOptions {
            app_id: Some("x-todo-app".to_string()),
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(SIZE),
            kind: WindowKind::PopUp,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };
        let parent_handle = parent.window_handle();
        cx.create_normal_window("xTodo-创建", options, move |window, cx| {
            cx.new(|cx| Self::new(Todo::default(), parent_handle, window, cx))
        });
        #[cfg(target_os = "windows")]
        parent.enable_window(false);
    }

    fn new(
        todo: Todo,
        parent: AnyWindowHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let description_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder("详细描述任务内容和要求...")
                .auto_grow(10, 10);
            state.set_value(todo.description.clone(), window, cx);
            state
        });
        window.on_window_should_close(cx, move |window, cx| {
            window.clear_notifications(cx);
            parent
                .update(cx, |_, window, cx| {
                    #[cfg(target_os = "windows")]
                    window.enable_window(true);
                    window.activate_window();
                })
                .ok();
            true
        });
        // 时间选择器
        let due_date_picker = cx.new(|cx| DatePickerState::new(window, cx));
        let reminder_date_picker = cx.new(|cx| DatePickerState::new(window, cx));
        let recurring_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("cron表达式，通常由系统自动生成"));
        let _subscriptions = vec![
            cx.subscribe_in(&description_input, window, Self::on_input_event),
            cx.subscribe(&due_date_picker, |_, _, ev, cx| match ev {
                DatePickerEvent::Change(_) => {
                    cx.notify();
                }
            }),
            cx.subscribe(&reminder_date_picker, |_, _, ev, cx| match ev {
                DatePickerEvent::Change(_) => {
                    cx.notify();
                }
            }),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            description_input,
            due_date_picker,
            reminder_date_picker,
            recurring_input,
            expanded_providers: Vec::new(),
            expanded_tool_providers: Vec::new(),
            cached_server_tools: std::collections::HashMap::new(), // 新增缓存
            _subscriptions,
            todoitem: todo,
        }
    }

    fn section_title(title: &'static str) -> impl IntoElement {
        div()
            .text_lg()
            .font_semibold()
            .text_color(gpui::rgb(0x374151))
            .pb_2()
            .child(title)
    }
    
    fn open_drawer_at(
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 使用 Entity 来共享状态
        let todo_edit_entity = cx.entity().clone();
        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
            // 从 entity 中读取当前的模型数据
            let providers =  LlmProviderManager::get_enabled_providers().clone();
            let expanded_providers = todo_edit_entity.read(drawer_cx).expanded_providers.clone();
            let todoitem = todo_edit_entity.read(drawer_cx).todoitem.clone();

            // 创建手风琴组件并添加切换监听器
            let mut accordion = Accordion::new("model-providers")
                .on_toggle_click({
                    let todo_edit_entity_for_toggle = todo_edit_entity.clone();
                    move |open_indices, _window, cx| {
                        todo_edit_entity_for_toggle.update(cx, |todo_edit, cx| {
                            todo_edit.toggle_accordion(open_indices, cx);
                        });
                    }
                });

            for (provider_index, provider) in providers.into_iter().enumerate() {
                let provider_name = provider.name.clone();
                let provider_models = provider.models.clone();

                // 检查该供应商是否有被选中的模型
                let has_selected_models = provider_models.iter().any(|model| {
                    todoitem.selected_model.iter().any(|selected| selected.model_id == model.id && selected.provider_id == provider.id)
                });

                // 检查当前供应商是否应该展开
                let is_expanded = has_selected_models || expanded_providers.contains(&provider_index);

                accordion = accordion.item(|item| {
                    item.open(is_expanded) // 根据选中状态和展开状态决定是否展开
                        .icon(IconName::Bot)
                        .title(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .font_medium()
                                                .text_color(gpui::rgb(0x374151))
                                                .child(provider_name.clone()),
                                        )
                                        .when(has_selected_models, |this| {
                                            this.child(
                                                Icon::new(IconName::Check)
                                                    .xsmall()
                                                    .text_color(gpui::rgb(0x10B981)),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(if has_selected_models {
                                            gpui::rgb(0xDCFCE7) // 有选中模型时使用绿色背景
                                        } else {
                                            gpui::rgb(0xEFF6FF) // 无选中模型时使用蓝色背景
                                        })
                                        .text_color(if has_selected_models {
                                            gpui::rgb(0x166534) // 绿色文字
                                        } else {
                                            gpui::rgb(0x1D4ED8) // 蓝色文字
                                        })
                                        .rounded_md()
                                        .text_xs()
                                        .child(format!("{} 个模型", provider_models.len())),
                                ),
                        )
                        .content(
                            v_flex()
                                .gap_2()
                                .p_2()
                                .children(provider_models.iter().enumerate().map(
                                    |(model_index, model)| {
                                        let model_id = model.id.clone();
                                        let model_name_for_event = model.display_name.clone();
                                        let checkbox_id = SharedString::new(format!("model-{}-{}",provider_index, model_index));
                                        let todo_edit_entity_for_event = todo_edit_entity.clone();

                                        div()
                                            .p_1()
                                            .bg(gpui::rgb(0xFAFAFA))
                                            .rounded_md()
                                            .hover(|style| style.bg(gpui::rgb(0xF3F4F6)))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        h_flex()
                                                            .items_center()
                                                            .gap_3()
                                                            .child(
                                                                Checkbox::new(checkbox_id)
                                                                    .checked(
                                                                        todoitem.selected_model.iter().any(|selected|
                                                                            selected.model_id == model.id && selected.provider_id == provider.id
                                                                        )
                                                                    )
                                                                    .label(model.display_name.clone()).tooltip(move |window, cx| {
                                                                        Tooltip::new(model_id.clone()).build(window, cx)
                                                                    })
                                                                    .on_click({
                                                                        let model_clone = model.clone();
                                                                                let provider_clone = provider.clone();
                                                                        move |checked, window, cx| {
                                                                            let model_name_to_toggle =
                                                                                model_name_for_event.clone();

                                                                            // 更新原始数据
                                                                                    todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                        todo_edit.toggle_model_selection(*checked,&model_clone, &provider_clone, todo_cx);
                                                                                        todo_edit.save(&Save, window, todo_cx);
                                                                                        todo_cx.notify();
                                                                                    });
                                                                                println!("切换模型选择: {}",model_name_to_toggle);

                                                                            }}),
                                                            )
                                                            .child(
                                                                h_flex().gap_1().items_center().children(
                                                                    model.capabilities.iter().enumerate().map(
                                                                        |(cap_index, cap)| {
                                                                            let capability_unique_id = provider_index * 10000
                                                                                + model_index * 1000
                                                                                + cap_index;

                                                                            div()
                                                                                .id(("capability", capability_unique_id))
                                                                                .p_1()
                                                                                .rounded_md()
                                                                                .bg(gpui::rgb(0xF3F4F6))
                                                                                .child(
                                                                                    Icon::new(cap.icon())
                                                                                        .xsmall()
                                                                                        .text_color(gpui::rgb(0x6B7280)),
                                                                                )
                                                                        },
                                                                    ),
                                                                ),
                                                            ),
                                                    ),
                                            )
                                    },
                                ))
                                .when(provider_models.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .p_4()
                                            .text_center()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x9CA3AF))
                                            .child("该服务商暂无可用模型"),
                                    )
                                }),
                        )
                });
            }

            let todo_edit_entity_for_clear = todo_edit_entity.clone();

            drawer
                .overlay(true)
                .size(px(280.))
                .title("选择模型")
                .child(accordion)
                .footer(
                    h_flex()
                        .justify_center()
                        .items_center()
                        .p_2()
                        .bg(gpui::rgb(0xFAFAFA))
                        .child(
                            Button::new("clear-all-models")
                                .label("清空选择")
                                .on_click(move |_, window, cx| {
                                    // 清空所有模型选择
                                    todo_edit_entity_for_clear.update(cx, |todo_edit, todo_cx| {
                                        todo_edit.todoitem.selected_model=None;
                                        todo_edit.save(&Save, window, todo_cx);
                                        todo_cx.notify(); // 通知主界面更新
                                    });
                                    println!("清空所有模型选择");
                                }),
                        ),
                )
        });
    }

    fn open_tool_drawer_at(
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 使用 Entity 来共享状态
        let todo_edit_entity = cx.entity().clone();

        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
            // 从 entity 中读取当前的工具数据
            let servers = McpConfigManager::load_servers().unwrap_or_default()
                .into_iter()
                .filter(|s| s.enabled)
                .collect::<Vec<_>>();
            
            let expanded_providers = todo_edit_entity.read(drawer_cx).expanded_tool_providers.clone();
            let todoitem = todo_edit_entity.read(drawer_cx).todoitem.clone();
            
            // 创建手风琴组件并添加切换监听器
            let mut accordion = Accordion::new("tool-providers")
                .on_toggle_click({
                    let todo_edit_entity_for_toggle = todo_edit_entity.clone();
                    move |open_indices, _window, cx| {
                        todo_edit_entity_for_toggle.update(cx, |todo_edit, todo_cx| {
                            todo_edit.toggle_tool_accordion(open_indices, todo_cx);
                        });
                    }
                });

            for (provider_index, server) in servers.into_iter().enumerate() {
                let server_name = server.name.clone();
                let server_id = server.id.clone();
                
                // 从缓存获取工具列表
                let server_tools = todo_edit_entity.read(drawer_cx)
                    .get_server_tools(&server.id);

                // 检查该服务器是否有被选中的工具
                let has_selected_tools = server_tools.iter().any(|tool| {
                    todoitem.selected_tools.iter().any(|selected| {
                        selected.tool_name == tool.name && selected.provider_id == server.id
                    })
                });
                let provider_tool_len = server_tools.len();
                // 检查当前供应商是否应该展开
                let is_expanded = has_selected_tools || expanded_providers.contains(&provider_index);
                // 如果还没有加载工具数据，异步加载
                if server_tools.is_empty() {
                    let server_id_for_load = server_id.clone();
                    let todo_edit_entity_for_load = todo_edit_entity.clone();
                    drawer_cx.spawn(async move | cx| {
                        if let Some(snapshot) = CrossRuntimeBridge::global().get_server_snapshot(&server_id_for_load).await {
                            let tools= snapshot.tools.clone();
                            todo_edit_entity_for_load.update(cx, |todo_edit, todo_cx| {
                                todo_edit.cached_server_tools.insert(server_id_for_load.clone(), tools);
                                todo_cx.notify();
                            }).ok();
                        }
                    }).detach();
                }

                accordion = accordion.item(|item| {
                    item.open(is_expanded)
                        .icon(IconName::Wrench)
                        .title(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .font_medium()
                                                .text_color(gpui::rgb(0x374151))
                                                .child(server_name.clone()),
                                        )
                                        .when(has_selected_tools, |this| {
                                            this.child(
                                                Icon::new(IconName::Check)
                                                    .xsmall()
                                                    .text_color(gpui::rgb(0x10B981)),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(if has_selected_tools {
                                            gpui::rgb(0xDCFCE7) // 有选中工具时使用绿色背景
                                        } else {
                                            gpui::rgb(0xFFF7ED) // 无选中工具时使用橙色背景
                                        })
                                        .text_color(if has_selected_tools {
                                            gpui::rgb(0x166534) // 绿色文字
                                        } else {
                                            gpui::rgb(0xEA580C) // 橙色文字
                                        })
                                        .rounded_md()
                                        .text_xs()
                                        .child(if provider_tool_len == 0 {
                                            "加载中...".to_string()
                                        } else {
                                            format!("{} 个工具", provider_tool_len)
                                        }),
                                ),
                        )
                        .content(
                            v_flex()
                                .gap_2()
                                .p_2()
                                .when(server_tools.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .p_4()
                                            .text_center()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x9CA3AF))
                                            .child("正在加载工具列表..."),
                                    )
                                })
                                .when(!server_tools.is_empty(), |this| {
                                    this.children(server_tools.iter().enumerate().map(
                                        |(tool_index, tool)| {
                                            let tool_name_for_event = tool.name.clone();
                                            let checkbox_id = SharedString::new(format!(
                                                "tool-{}-{}",
                                                provider_index, tool_index
                                            ));
                                            let todo_edit_entity_for_event = todo_edit_entity.clone();

                                            div()
                                                .p_1()
                                                .bg(gpui::rgb(0xFAFAFA))
                                                .rounded_md()
                                                .hover(|style| style.bg(gpui::rgb(0xF3F4F6)))
                                                .child(
                                                    v_flex()
                                                        .gap_1()
                                                        .child(
                                                            h_flex()
                                                                .items_center()
                                                                .justify_between()
                                                                .child(
                                                                    h_flex()
                                                                        .items_center()
                                                                        .gap_3()
                                                                        .child(
                                                                            Checkbox::new(checkbox_id)
                                                                                .checked(todoitem.selected_tools.iter().any(|selected|
                                                                                selected.tool_name == tool.name && selected.provider_id == server.id
                                                                            ))
                                                                                .label(tool.name.to_string())
                                                                                .on_click({
                                                                                    let tool_clone = tool.clone();
                                                                                    let server_clone = server.clone();
                                                                                    move |checked, window, cx| {
                                                                                        let tool_name_to_toggle =
                                                                                            tool_name_for_event.clone();

                                                                                        // 更新原始数据
                                                                                        todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                            todo_edit.toggle_tool_selection(*checked,&tool_clone, &server_clone, todo_cx);
                                                                                            todo_edit.save(&Save, window, todo_cx);
                                                                                            todo_cx.notify();
                                                                                        });
                                                                                        println!(
                                                                                            "切换工具选择: {}",
                                                                                            tool_name_to_toggle
                                                                                        );
                                                                                    }
                                                                                }),
                                                                        )
                                                                ),
                                                        )
                                                        // .child(
                                                        //     div()
                                                        //         .pl_6()
                                                        //         .text_xs()
                                                        //         .text_color(gpui::rgb(0x6B7280))
                                                        //         .child(tool.description.as_ref().map(|desc|desc.to_string()).unwrap_or_default()),
                                                        // ),
                                                )
                                        },
                                    ))
                                }),
                        )
                });
            }

            let todo_edit_entity_for_clear = todo_edit_entity.clone();

            drawer
                .overlay(true)
                .size(px(280.))
                .title("选择工具集")
                .child(accordion)
                .footer(
                    h_flex()
                        .justify_center()
                        .items_center()
                        .p_2()
                        .bg(gpui::rgb(0xFAFAFA))
                        .child(
                            Button::new("clear-all-tools")
                                .label("清空选择")
                                .on_click(move |_, window, cx| {
                                    // 清空所有工具选择
                                    todo_edit_entity_for_clear.update(cx, |todo_edit, todo_cx| {
                                        todo_edit.todoitem.selected_tools.clear();
                                        todo_edit.save(&Save, window, todo_cx);
                                        todo_cx.notify();
                                    });
                                    println!("清空所有工具选择");
                                }),
                        ),
                )
        });
    }

    // 添加移除文件的方法
    fn remove_file(&mut self, file_path: &str, _window: &mut Window, cx: &mut Context<Self>) {
        self.todoitem.files.retain(|file| file.path != file_path);
        cx.notify();
    }

    // 添加处理文件拖拽的方法
    fn handle_file_drop(
        &mut self,
        external_paths: &ExternalPaths,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for path in external_paths.paths() {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                let path_str = path.to_string_lossy().to_string();
                // 检查文件是否已经存在
                if !self.todoitem.files.iter().any(|f| f.path == path_str) {
                    // 获取文件大小（可选）
                    let file_size = std::fs::metadata(&path).ok().map(|metadata| metadata.len());

                    let uploaded_file = TodoFile {
                        name: file_name.to_string(),
                        path: path_str,
                        size: file_size,
                        mime_type: None,
                        uploaded_at: Utc::now().to_utc(),
                    };

                    self.todoitem.files.push(uploaded_file);
                }
            }
        }
        cx.notify();
    }

    // 格式化文件大小的辅助方法
    fn format_file_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size_f = size as f64;
        let mut unit_index = 0;

        while size_f >= 1024.0 && unit_index < UNITS.len() - 1 {
            size_f /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size_f, UNITS[unit_index])
        }
    }
}

impl FocusableCycle for TodoThreadEdit {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        vec![
            //self.title_input.focus_handle(cx),
            self.description_input.focus_handle(cx),
            self.due_date_picker.focus_handle(cx),
            self.reminder_date_picker.focus_handle(cx),
            self.recurring_input.focus_handle(cx),
        ]
    }
}

impl Focusable for TodoThreadEdit {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodoThreadEdit {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .id("todo-thread-view")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::delete))
            .size_full()
            .p_2()
            .gap_2()
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        v_flex()
                            .gap_3()
                            .pt_1()
                            .px_2()
                            .pb_2()
                            .bg(gpui::rgb(0xF9FAFB))
                            .rounded_lg()
                            .child(
                                v_flex()
                                    .child(TextInput::new(&self.description_input).cleanable()),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_3()
                            .pt_1()
                            .px_2()
                            .pb_2()
                            .bg(gpui::rgb(0xF9FAFB))
                            .rounded_lg()
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .id("file-drop-zone")
                                            .min_h_10() // 改为最小高度，而不是固定高度
                                            .w_full()
                                            .border_2()
                                            .border_color(gpui::rgb(0xD1D5DB))
                                            .border_dashed()
                                            .rounded_lg()
                                            .bg(gpui::rgb(0xFAFAFA))
                                            .flex()
                                            .flex_col() // 改为垂直布局
                                            .cursor_pointer()
                                            .hover(|style| {
                                                style
                                                    .border_color(gpui::rgb(0x3B82F6))
                                                    .bg(gpui::rgb(0xF0F9FF))
                                            })
                                            .active(|style| {
                                                style
                                                    .border_color(gpui::rgb(0x1D4ED8))
                                                    .bg(gpui::rgb(0xE0F2FE))
                                            }).when(self.todoitem.files.is_empty(),|this|{
                                                this.child(
                                                // 拖拽提示区域
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .p_1()
                                                    .child(
                                                        v_flex()
                                                            .items_center()
                                                            .gap_1()
                                                            .child(
                                                                Icon::new(IconName::Upload)
                                                                    .size_4()
                                                                    .text_color(gpui::rgb(0x6B7280)),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(gpui::rgb(0xB91C1C))
                                                                    .child("支持 PDF、DOC、TXT、图片等格式"),
                                                            ),
                                                    ),
                                            )
                                            })
                                            
                                            // 文件列表区域（集成到拖拽区域内）
                                            .when(!self.todoitem.files.is_empty(), |this| {
                                                this.child(
                                                    div()
                                                        // .border_t_1()
                                                        // .border_color(gpui::rgb(0xE5E7EB))
                                                        .p_1()
                                                       // .bg(gpui::rgb(0xF8F9FA))
                                                        .child(
                                                            v_flex()
                                                                .gap_1()
                                                                .child(
                                                                    h_flex()
                                                                        .gap_1()
                                                                        .flex_wrap()
                                                                        .children(
                                                                            self.todoitem.files.iter().enumerate().map(|(index, file)| {
                                                                                let file_path_for_remove = file.path.clone();
                                                                                let file_name = file.name.clone();

                                                                                // 截断文件名，最大显示15个字符
                                                                                let display_name = if file.name.chars().count() > 10 {
                                                                                    format!("{}...", file.name.chars().take(10).collect::<String>())
                                                                                } else {
                                                                                    file.name.clone()
                                                                                };

                                                                                div()
                                                                                    .id(("uploaded-file", index))
                                                                                    .flex()
                                                                                    .items_center()
                                                                                    .gap_0()
                                                                                    .px_0()
                                                                                    .py_0()
                                                                                    .max_w_40() // 限制最大宽度
                                                                                    .overflow_hidden()
                                                                                    .bg(gpui::rgb(0xF3F4F6))
                                                                                    // .border_1()
                                                                                    // .border_color(gpui::rgb(0xE5E7EB))
                                                                                    .rounded_md()
                                                                                    .on_click(|_,_,cx|{
                                                                                        cx.stop_propagation();
                                                                                    })
                                                                                    .hover(|style| style.bg(gpui::rgb(0xE5E7EB)))
                                                                                    .tooltip({
                                                                                        let file_name = file_name.clone();
                                                                                        move |window, cx| {
                                                                                            let file_name = file_name.clone();
                                                                                            Tooltip::element( move|_, _| {
                                                                                                Label::new(file_name.clone())
                                                                                            })
                                                                                            .build(window, cx)
                                                                                        }
                                                                                    })
                                                                                    .child(

                                                                                        div()
                                                                                            .text_xs()
                                                                                            .font_medium()
                                                                                            .text_color(gpui::rgb(0x374151))
                                                                                            .flex_1()
                                                                                            .overflow_hidden()
                                                                                            .child(display_name),
                                                                                    )
                                                                                    .child(
                                                                                        Button::new(SharedString::new(format!("remove-file-{}", index)))
                                                                                            .ghost()
                                                                                            .xsmall()
                                                                                            .icon(IconName::X)
                                                                                            .text_color(gpui::rgb(0x9CA3AF))
                                                                                            .p_0()
                                                                                            .min_w_4()
                                                                                            //.h_4()
                                                                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                                                                this.remove_file(&file_path_for_remove, window, cx);
                                                                                            })),
                                                                                    )
                                                                            })
                                                                        ),
                                                                ),
                                                        )
                                                )
                                            })
                                            .drag_over(|style, _path: &ExternalPaths, _window, _cx| {
                                                style
                                                    .border_color(gpui::rgb(0x3B82F6))
                                                    .bg(gpui::rgb(0xF0F9FF))
                                            })
                                            .on_drop(cx.listener(|this, external_paths: &ExternalPaths, window, cx| {
                                                this.handle_file_drop(external_paths, window, cx);
                                                cx.stop_propagation();
                                            }))
                                            .on_click(cx.listener(|_, _, _, cx| {
                                                println!("点击上传文件");
                                                cx.notify();
                                            })),
                                    )
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .pt_1()
                            .px_2()
                            .pb_2()
                            .bg(gpui::rgb(0xF9FAFB))
                            .rounded_lg()
                            .child(Self::section_title("助手配置"))
                            .child(
                                h_flex()
                                    .gap_2().justify_start()
                                    .items_center()
                                    .child(
                                       Button::new("show-drawer-left")
                                                .label({
                                                    let display_text =
                                                        self.get_model_display_text(cx);
                                                    if display_text == "选择模型" {
                                                        display_text
                                                    } else {
                                                        display_text
                                                    }
                                                }).ghost().xsmall()
                                                .justify_center()
                                                .text_color(
                                                    if self.get_model_display_text(cx)
                                                        == "选择模型"
                                                    {
                                                        gpui::rgb(0x9CA3AF)
                                                    } else {
                                                        gpui::rgb(0x374151)
                                                    },
                                                )
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    Self::open_drawer_at(Placement::Left, window, cx)
                                                })),
                                    ).child(
                                       Button::new("show-tool-drawer-left")
                                                .label({
                                                    let display_text = self.get_tool_display_text(cx);
                                                    if display_text == "选择工具集" {
                                                        display_text
                                                    } else {
                                                        display_text
                                                    }
                                                })
                                                .ghost()
                                                .xsmall()
                                                .justify_center()
                                                .text_color(
                                                    if self.get_tool_display_text(cx) == "选择工具集" {
                                                        gpui::rgb(0x9CA3AF)
                                                    } else {
                                                        gpui::rgb(0x374151)
                                                    },
                                                )
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    Self::open_tool_drawer_at(Placement::Left, window, cx)
                                                }))
                                    ),
                            ).child(
                                h_flex()
                                    .gap_2()
                                    .items_center().justify_between()
                                    .child(

                                        h_flex()
                                    .gap_2()
                                    .items_center().justify_start()
                                    .child(
                                        Checkbox::new("push-feishu-button")
                                                    .label("推送到飞书")
                                                    .checked(self.todoitem.push_to_feishu)
                                                    .on_click(cx.listener(|view, _, _, cx| {
                                                        view.todoitem.push_to_feishu = !view.todoitem.push_to_feishu;
                                                        cx.notify();
                                                    }))
                                    )
                                    .child(
                                        Checkbox::new("needs-audio-recoding")
                                                    .label("开启录音")
                                                    .checked(self.todoitem.needs_recording)
                                                    .on_click(cx.listener(|this, _, win, cx| {
                                                       this.toggle_audio_recording( win, cx);
                                                        cx.notify();
                                                    }))
                                    )
                                    .child(
                                        Checkbox::new("needs-screen-recoding")
                                                    .label("开启录屏")
                                                    .checked(self.todoitem.needs_screen_recording)
                                                    .on_click(cx.listener(|this, _, win, cx| {
                                                       this.toggle_screen_recording( win, cx);
                                                        cx.notify();
                                                    }))
                                    )
                                    
                                    ).child(

                                        h_flex()
                                    .gap_2()
                                    .items_center().justify_end()
                                    .child(
                                        DatePicker::new(&self.due_date_picker).number_of_months(1)
                                        .placeholder("截止日期")
                                        .cleanable()
                                        .small()
                                    )
                                    
                                    )
                            )

                    )

            )
            .child(
                h_flex().items_center().justify_center().pt_2().child(
                    h_flex().gap_1().child(
                        Button::new("save-btn")
                            .with_variant(ButtonVariant::Primary)
                            .label("保存&关闭")
                            .icon(IconName::Check)
                            .on_click(
                                cx.listener(|this,ev, window, cx| {
                                    this.save(&Save, window, cx);
                                    window.remove_window();
                                }
                            )),
                    ),
                ),
            )
    }
}
