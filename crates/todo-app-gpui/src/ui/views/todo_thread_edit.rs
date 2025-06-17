use chrono::{ Utc};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    accordion::Accordion,
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset},
    dropdown::{Dropdown,  DropdownState},
    input::{InputEvent, InputState, TextInput},
    label::Label,
    switch::Switch,
    tooltip::Tooltip,
    *,
};
use crate::{models::{mcp_config::{McpProviderInfo, McpProviderManager, McpTool}, provider_config::{LlmProviderManager, ModelInfo}}, ui::{components::ViewKit, views::todo_thread::ProviderInfo, AppExt}};
use crate::models::todo_item::*;

actions!(todo_thread, [Tab, TabPrev, Save, Cancel, Delete]);

const CONTEXT: &str = "TodoThreadEdit";



pub struct TodoThreadEdit {
    focus_handle: FocusHandle,

    // 基本信息
    title_input: Entity<InputState>,
    description_input: Entity<InputState>,

    // // 状态和优先级
    // status_dropdown: Entity<DropdownState<Vec<SharedString>>>,
    // priority_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    // AI助手配置
    model_manager: LlmProviderManager,
    mcp_tool_manager: McpProviderManager,

    // 时间设置
    due_date_picker: Entity<DatePickerState>,
    reminder_date_picker: Entity<DatePickerState>,
    recurring_enabled: bool,
    recurring_dropdown: Entity<DropdownState<Vec<SharedString>>>,

    // 其他设置
    auto_execute: bool,
    enable_notifications: bool,

    // 手风琴展开状态
    expanded_providers: Vec<usize>,
    expanded_tool_providers: Vec<usize>,

    // // 添加上传文件列表
    // uploaded_files: Vec<UploadedFile>,

    _subscriptions: Vec<Subscription>,

    todoitem:Todo,
}
//实现业务操作
impl TodoThreadEdit {
fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
    }

    fn save(&mut self, _: &Save, _window: &mut Window, cx: &mut Context<Self>) {
        let selected_models = self.todoitem.selected_models.clone();
        let selected_tools = self.todoitem.selected_tools.clone(); // 改为获取选中的工具

        
        println!("保存Todo: {:?}", self.todoitem);
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

    fn toggle_recurring(&mut self, enabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.recurring_enabled = enabled;
        cx.notify();
    }

    fn toggle_auto_execute(&mut self, enabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.auto_execute = enabled;
        cx.notify();
    }

    fn toggle_notifications(&mut self, enabled: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.enable_notifications = enabled;
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
                self.save(&Save, window, cx);
            }
            _ => {}
        }
    }

    
    // 获取模型选择显示文本
    fn get_model_display_text(&self, _cx: &App) -> String {
        let selected_count = self.todoitem.selected_models.len();
        if selected_count == 0 {
            "选择模型".to_string()
        } else if selected_count <= 2 {
            self.todoitem.selected_models.iter().map(|item|item.model_name.as_str()).collect::<Vec<_>>().join(", ")
        } else {
            let first_two = self.todoitem.selected_models
                .iter()
                .take(2)
                .map(|item|item.model_name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} 等{}个模型", first_two, selected_count)
        }
    }

    // 获取工具选择显示文本
    fn get_tool_display_text(&self, _cx: &App) -> String {
        let selected_count = self.todoitem.selected_tools.len();

        if selected_count == 0 {
            "选择工具集".to_string()
        } else if selected_count <= 2 {
            self.todoitem.selected_tools.iter().map(|item|item.tool_name.as_str()).collect::<Vec<_>>().join(", ")
        } else {
            let first_two = self.todoitem.selected_tools
                .iter()
                .take(2)
                .map(|item|item.tool_name.as_str()).collect::<Vec<_>>()
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
        model: &ModelInfo,
        provider: &ProviderInfo,
        checked:bool,
        cx: &mut Context<Self>,
    ) {

    }

    fn toggle_tool_selection(&mut self, tool:&McpTool,provider:&McpProviderInfo, cx: &mut Context<Self>) {
        // 检查工具是否已被选中
        if let Some(index) = self.todoitem.selected_tools.iter().position(|t| t.tool_name == tool.name) {
            // 如果已选中，则移除
            self.todoitem.selected_tools.remove(index);
        } else {
            // 如果未选中，则添加
            if let Some((id,provider)) = self.mcp_tool_manager.providers.iter_mut().find(|(id,p)| p.tools.iter().any(|t| t.name == tool.name)) {
                if let Some(tool) = provider.tools.iter_mut().find(|t| t.name == tool.name) {
                    self.todoitem.selected_tools.push(crate::models::todo_item::SelectedTool {
                        provider_id: provider.id.clone(),
                        provider_name: provider.name.clone(),
                        description: tool.description.clone(),
                        tool_name: tool.name.clone(),
                    });
                }
            }
        }
          cx.notify(); // 通知主界面更新
    }

    fn uploaded_files(&self) -> Vec<TodoFile> {
        // 返回当前上传的文件列表
        vec![] // 这里可以根据实际情况返回已上传的文件
    }
}

// 实现 TodoThreadEdit界面的相关方法
impl TodoThreadEdit {

    pub fn edit(todo:Todo,
        cx: &mut App,
    )  {
        println!("编辑Todo: {:?}", todo);
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
                format!("xTodo-{}", todo.title),
                options,
                move |window, cx| cx.new(|cx| Self::new(todo,window, cx)),
            );
    }

    pub fn add(
        cx: &mut App,
    )  {
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
                "xTodo-Add",
                options,
                move |window, cx|  cx.new(|cx| Self::new(Todo::default(),window, cx)),
            );
    }
    
    fn new(todo:Todo,window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 基本信息输入框
        let title_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入任务标题..."));

        let description_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("详细描述任务内容和要求...")
                .auto_grow(10, 10)
        });

        // 模型管理器和工具管理器
        let model_manager = LlmProviderManager::load();
        let mcp_tool_manager = McpProviderManager::load();

        // 时间选择器
        let due_date_picker = cx.new(|cx| DatePickerState::new(window, cx));
        let reminder_date_picker = cx.new(|cx| DatePickerState::new(window, cx));

        let recurring_options = vec!["每日".into(), "每周".into(), "每月".into(), "每年".into()];
        let recurring_dropdown =
            cx.new(|cx| DropdownState::new(recurring_options, Some(1), window, cx));

        let _subscriptions = vec![
            cx.subscribe_in(&title_input, window, Self::on_input_event),
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
            title_input,
            description_input,
            model_manager,
            mcp_tool_manager,
            due_date_picker,
            reminder_date_picker,
            recurring_enabled: false,
            recurring_dropdown,
            auto_execute: false,
            enable_notifications: true,
            expanded_providers: Vec::new(),
            expanded_tool_providers: Vec::new(),
            _subscriptions,
            todoitem:todo,
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

    fn form_row(label: &'static str, content: impl IntoElement) -> impl IntoElement {
        h_flex()
            .gap_4()
            .items_center()
            .child(
                div()
                    .text_sm()
                    .text_color(gpui::rgb(0x6B7280))
                    .min_w_24()
                    .child(label),
            )
            .child(div().flex_1().max_w_80().child(content))
    }


    fn open_drawer_at(
        // &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 使用 Entity 来共享状态
        let todo_edit_entity = cx.entity().clone();
        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
            // 从 entity 中读取当前的模型数据
            let providers = todo_edit_entity.read(drawer_cx).model_manager.providers.clone();
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

            for (provider_index, (_id,provider)) in providers.into_iter().enumerate() {
                let provider_name = provider.name.clone();
                let provider_models = provider.models.clone();
                
                // 检查该供应商是否有被选中的模型
                let has_selected_models = provider_models.iter().any(|model| {
                    todoitem.selected_models.iter().any(|selected| selected.model_name == model.display_name)
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
                                        let model_name_for_event = model.display_name.clone();
                                        let checkbox_id = SharedString::new(format!("model-{}-{}",provider_index, model_index));
                                        let todo_edit_entity_for_event = todo_edit_entity.clone();

                                        div()
                                            .p_1()
                                            .bg(gpui::rgb(0xFAFAFA))
                                            .rounded_md()
                                            // .border_1()
                                            // .border_color(gpui::rgb(0xE5E7EB))
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
                                                                        todoitem.selected_models.iter().any(|selected| 
                                                                            selected.model_name == model.display_name
                                                                        )
                                                                    )
                                                                    .label(model.display_name.clone())
                                                                    .on_click({
                                                                        let provider_id = provider.id.clone();
                                                                        let provider_name = provider.name.clone();
                                                                        move |_checked, _window, cx| {
                                                                            let model_name_to_toggle =
                                                                                model_name_for_event.clone();
                                                                            
                                                                            // 更新原始数据
                                                                            todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                // First, update the method call in the selection placeholder:
                                                                                let model_to_toggle = todo_edit.model_manager.providers
                                                                                    .iter()
                                                                                    .flat_map(|(_, provider)| &provider.models)
                                                                                    .find(|model| model.display_name == model_name_to_toggle)
                                                                                    .cloned();

                                                                                if let Some(model_info) = model_to_toggle {
                                                                                    // Check if model is already selected
                                                                                    if let Some(index) = todo_edit.todoitem.selected_models
                                                                                        .iter()
                                                                                        .position(|selected| selected.model_name == model_name_to_toggle) 
                                                                                    {
                                                                                        // Remove if already selected
                                                                                        todo_edit.todoitem.selected_models.remove(index);
                                                                                    } else {
                                                                                        // Add if not selected
                                                                                        todo_edit.todoitem.selected_models.push(crate::models::todo_item::SelectedModel {
                                                                                            provider_id: provider_id.clone(),
                                                                                            provider_name: provider_name.clone(),
                                                                                            model_id: model_info.id.clone(),
                                                                                            model_name: model_info.display_name.clone(),
                                                                                        });
                                                                                    }
                                                                                }
                                                                                println!("切换模型选择: {}",model_name_to_toggle);
                                                                                todo_cx.notify(); // 通知主界面更新
                                                                            });
                                                                        }
                                                                    }),
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
                .size(px(380.))
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
                                        todo_edit.todoitem.selected_models.clear();
                                        todo_cx.notify(); // 通知主界面更新
                                    });
                                    println!("清空所有模型选择");
                                    // 关闭抽屉
                                    window.close_drawer(cx);
                                }),
                        ),
                )
        });
    }

    fn open_tool_drawer_at(
        // &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 使用 Entity 来共享状态
        let todo_edit_entity = cx.entity().clone();

        window.open_drawer_at(placement, cx, move |drawer, _window, drawer_cx| {
            // 从 entity 中读取当前的工具数据
            let providers = todo_edit_entity.read(drawer_cx).mcp_tool_manager.providers.clone();
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

            for (provider_index, (_id,provider)) in providers.into_iter().enumerate() {
                let provider_name = provider.name.clone();
                let provider_tools = provider.tools.clone();
                
                // 检查该供应商是否有被选中的工具
                let has_selected_tools = provider_tools.iter().any(|tool|  todoitem.selected_tools.iter().any(|selected| selected.tool_name == tool.name));
                let provider_tool_len = provider_tools.len();
                // 检查当前供应商是否应该展开
                let is_expanded = has_selected_tools || expanded_providers.contains(&provider_index);

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
                                                .child(provider_name.clone()),
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
                                        .child(format!("{} 个工具", provider_tool_len)),
                                ),
                        )
                        .content(
                            v_flex()
                                .gap_2()
                                .p_2()
                                .children(provider_tools.iter().enumerate().map(
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
                                                                            selected.tool_name == tool.name
                                                                        ))
                                                                            .label(tool.name.clone())
                                                                            .on_click({
                                                                                let tool_clone = tool.clone();
                                                                                let provider_clone = provider.clone();
                                                                                move |_checked, _window, cx| {
                                                                                    let tool_name_to_toggle =
                                                                                        tool_name_for_event.clone();
                                                                                    
                                                                                    // 更新原始数据
                                                                                    todo_edit_entity_for_event.update(cx, |todo_edit, todo_cx| {
                                                                                        todo_edit.toggle_tool_selection(&tool_clone, &provider_clone, todo_cx);
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
                                                    .child(
                                                        div()
                                                            .pl_6()
                                                            .text_xs()
                                                            .text_color(gpui::rgb(0x6B7280))
                                                            .child(tool.description.clone()),
                                                    ),
                                            )
                                    },
                                ))
                                .when(provider_tools.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .p_4()
                                            .text_center()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x9CA3AF))
                                            .child("该类别暂无可用工具"),
                                    )
                                }),
                        )
                });
            }

            let todo_edit_entity_for_clear = todo_edit_entity.clone();

            drawer
                .overlay(true)
                .size(px(380.))
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
                                        todo_cx.notify();
                                    });
                                    println!("清空所有工具选择");
                                    // 关闭抽屉
                                   // window.close_drawer(cx);
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
                        mime_type:None,
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

impl ViewKit for TodoThreadEdit {
    fn title() -> &'static str {
        "任务编辑"
    }

    fn description() -> &'static str {
        "创建和编辑任务，配置AI助手和时间安排"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        cx.new(|cx| Self::new(Todo::default(),window, cx))
    }
}

impl FocusableCycle for TodoThreadEdit {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        vec![
            self.title_input.focus_handle(cx),
            self.description_input.focus_handle(cx),
            self.due_date_picker.focus_handle(cx),
            self.reminder_date_picker.focus_handle(cx),
            self.recurring_dropdown.focus_handle(cx),
        ]
    }
}

impl Focusable for TodoThreadEdit {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodoThreadEdit {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let due_date_presets = vec![
            DateRangePreset::single("今天", Utc::now().naive_local().date()),
            DateRangePreset::single(
                "明天",
                (Utc::now() + chrono::Duration::days(1))
                    .naive_local()
                    .date(),
            ),
            DateRangePreset::single(
                "下周",
                (Utc::now() + chrono::Duration::weeks(1))
                    .naive_local()
                    .date(),
            ),
            DateRangePreset::single(
                "下个月",
                (Utc::now() + chrono::Duration::days(30))
                    .naive_local()
                    .date(),
            ),
        ];

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
                                    .gap_1()
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
                                            .min_h_24() // 改为最小高度，而不是固定高度
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
                                            })
                                            .child(
                                                // 拖拽提示区域
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .p_4()
                                                    .child(
                                                        v_flex()
                                                            .items_center()
                                                            .gap_2()
                                                            .child(
                                                                Icon::new(IconName::Upload)
                                                                    .size_6()
                                                                    .text_color(gpui::rgb(0x6B7280)),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(gpui::rgb(0x9CA3AF))
                                                                    .child("拖拽文件到此处上传或点击选择文件"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(gpui::rgb(0xB91C1C))
                                                                    .child("支持 PDF、DOC、TXT、图片等格式"),
                                                            ),
                                                    ),
                                            )
                                            // 文件列表区域（集成到拖拽区域内）
                                            .when(!self.todoitem.files.is_empty(), |this| {
                                                this.child(
                                                    div()
                                                        .border_t_1()
                                                        .border_color(gpui::rgb(0xE5E7EB))
                                                        .p_3()
                                                        .bg(gpui::rgb(0xF8F9FA))
                                                        .child(
                                                            v_flex()
                                                                .gap_2()
                                                                
                                                                .child(
                                                                    h_flex()
                                                                        .gap_2()
                                                                        .flex_wrap()
                                                                        .children(
                                                                            self.todoitem.files.iter().enumerate().map(|(index, file)| {
                                                                                let file_path_for_remove = file.path.clone();
                                                                                let file_name = file.name.clone();
                                                                                
                                                                                // 截断文件名，最大显示15个字符
                                                                                let display_name = if file.name.len() > 15 {
                                                                                    format!("{}...", &file.name[..12])
                                                                                } else {
                                                                                    file.name.clone()
                                                                                };
                                                                                
                                                                                div()
                                                                                    .id(("uploaded-file", index))
                                                                                    .flex()
                                                                                    .items_center()
                                                                                    .gap_1()
                                                                                    .px_2()
                                                                                    .py_1()
                                                                                    .max_w_32() // 限制最大宽度
                                                                                    .bg(gpui::rgb(0xF3F4F6))
                                                                                    .border_1()
                                                                                    .border_color(gpui::rgb(0xE5E7EB))
                                                                                    .rounded_md()
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
                                                                                            .h_4()
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
                            .child(h_flex()
                                            .justify_between()
                                            .items_center()
                                            .child(Self::section_title("助手配置"))
                                            .child(
                                                Checkbox::new("push-feishu-button")
                                                    .label("推送到飞书")
                                                    .checked(true)
                                                    .on_click(cx.listener(|view, _, _, cx| {
                                                        // view.disabled = !view.disabled;
                                                        cx.notify();
                                                    })),
                                            ),)
                            .child(
                                h_flex()
                                    .gap_2().justify_start()
                                    .items_center()
                                    // .child(
                                    //     div()
                                    //         .text_sm()
                                    //         .text_color(gpui::rgb(0x6B7280))
                                    //         .min_w_24()
                                    //         .child("模型选择"),
                                    // )
                                    .child(
                                        div().justify_start().child(
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
                                        ),
                                    ).child(
                                        h_flex()
                                        .max_w_32()
                                        .child( DatePicker::new(&self.due_date_picker)
                                        .placeholder("截止日期")
                                        .cleanable()
                                        .presets(due_date_presets.clone())
                                        .small())
                            ),
                            ).child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    // .child(
                                    //     div()
                                    //         .text_sm()
                                    //         .text_color(gpui::rgb(0x6B7280))
                                    //         .min_w_24()
                                    //         .child("工具集"),
                                    // )
                                    .child(
                                        div().justify_start().child(
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
                                                })),
                                        ),
                                    ).child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x6B7280))
                                            .min_w_24()
                                            .child("周期重复"),
                                    )
                                    .child(
                                        Switch::new("recurring")
                                            .checked(self.recurring_enabled)
                                            .on_click(cx.listener(
                                                move |this, checked, window, cx| {
                                                    this.toggle_recurring(*checked, window, cx);
                                                },
                                            )),
                                    )
                                    .when(self.recurring_enabled, |this| {
                                        this.child(
                                            div().ml_4().child(
                                                Dropdown::new(&self.recurring_dropdown)
                                                    .placeholder("选择周期")
                                                    .small(),
                                            ),
                                        )
                                    }),
                            ),
                            )
                            
                    )
                    
            )
            .child(
                h_flex().items_center().justify_center().pt_2().child(
                    h_flex().gap_1().child(
                        Button::new("save-btn")
                            .with_variant(ButtonVariant::Primary)
                            .label("保存任务")
                            .icon(IconName::Check)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.save(&Save, window, cx)),
                            ),
                    ),
                ),
            )
    }
}