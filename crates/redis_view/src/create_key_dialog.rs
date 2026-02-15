//! 创建键对话框视图

use gpui::{App, Context, Entity, Render, SharedString, Window, div, AppContext, IntoElement, Styled, ParentElement};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    ActiveTheme, IndexPath, Sizable, Size, h_flex, v_flex,
    button::{Button, ButtonVariants as _},
    input::{Input, InputState},
    select::{Select, SelectEvent, SelectItem, SelectState},
};

use crate::RedisKeyType;

/// 创建键表单数据
pub struct CreateKeyFormData {
    pub key: String,
    pub key_type: RedisKeyType,
    pub value: String,
    pub hash_field: String,
    pub zset_score: f64,
    pub ttl: Option<i64>,
}

/// 键类型选项（用于 Select 组件）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct KeyTypeOption(RedisKeyType);

impl SelectItem for KeyTypeOption {
    type Value = RedisKeyType;

    fn title(&self) -> SharedString {
        self.0.display_name().into()
    }

    fn value(&self) -> &Self::Value {
        &self.0
    }
}

/// 创建键对话框组件
pub struct CreateKeyDialog {
    db_index: u8,
    selected_type: RedisKeyType,
    key_input: Entity<InputState>,
    ttl_input: Entity<InputState>,
    value_input: Entity<InputState>,
    hash_field_input: Entity<InputState>,
    zset_score_input: Entity<InputState>,
    type_select: Entity<SelectState<Vec<KeyTypeOption>>>,
}

impl CreateKeyDialog {
    pub fn new(db_index: u8, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let key_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("输入键名")
        });
        let ttl_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("过期时间（秒）");
            state.set_value("-1", window, cx);
            state
        });
        let value_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入值")
                .multi_line(true)
                .auto_grow(3, 8)
        });
        let hash_field_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("字段名")
        });
        let zset_score_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("分数");
            state.set_value("0", window, cx);
            state
        });

        let type_options = vec![
            KeyTypeOption(RedisKeyType::String),
            KeyTypeOption(RedisKeyType::List),
            KeyTypeOption(RedisKeyType::Set),
            KeyTypeOption(RedisKeyType::ZSet),
            KeyTypeOption(RedisKeyType::Hash),
        ];
        let type_select = cx.new(|cx| {
            SelectState::new(type_options, Some(IndexPath::default().row(0)), window, cx)
        });

        cx.subscribe_in(&type_select, window, Self::on_type_changed)
            .detach();

        Self {
            db_index,
            selected_type: RedisKeyType::String,
            key_input,
            ttl_input,
            value_input,
            hash_field_input,
            zset_score_input,
            type_select,
        }
    }

    pub fn form_data(&self, cx: &App) -> CreateKeyFormData {
        let key = self.key_input.read(cx).text().to_string();
        let value = self.value_input.read(cx).text().to_string();
        let hash_field = self.hash_field_input.read(cx).text().to_string();
        let zset_score_str = self.zset_score_input.read(cx).text().to_string();
        let zset_score = zset_score_str.parse().unwrap_or(0.0);
        let ttl_str = self.ttl_input.read(cx).text().to_string();
        let ttl: Option<i64> = if ttl_str.is_empty() || ttl_str == "-1" {
            None
        } else {
            ttl_str.parse().ok()
        };
        let key_type = self
            .type_select
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or(RedisKeyType::String);

        CreateKeyFormData {
            key,
            key_type,
            value,
            hash_field,
            zset_score,
            ttl,
        }
    }

    fn on_type_changed(
        &mut self,
        _select: &Entity<SelectState<Vec<KeyTypeOption>>>,
        event: &SelectEvent<Vec<KeyTypeOption>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let SelectEvent::Confirm(Some(selected)) = event {
            self.selected_type = *selected;
            cx.notify();
        }
    }
}

impl Render for CreateKeyDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show_hash = self.selected_type == RedisKeyType::Hash;
        let show_zset = self.selected_type == RedisKeyType::ZSet;

        v_flex()
            .gap_3()
            // 键名
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_sm().child("键:"))
                    .child(Input::new(&self.key_input).w_full()),
            )
            // 数据库编号
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().text_sm().child("数据库编号:"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{}", self.db_index)),
                    ),
            )
            // 类型
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_sm().child("类型:"))
                    .child(
                        Select::new(&self.type_select)
                            .w_full()
                            .with_size(Size::Small),
                    ),
            )
            // TTL
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_sm().child("TTL:"))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Input::new(&self.ttl_input).flex_1())
                            .child(div().text_sm().child("秒"))
                            .child(
                                Button::new("ttl-permanent")
                                    .label("永久")
                                    .ghost()
                                    .xsmall()
                                    .on_click({
                                        let ttl_input = self.ttl_input.clone();
                                        move |_, window, cx| {
                                            ttl_input.update(cx, |state: &mut InputState, cx| {
                                                state.set_value("-1", window, cx);
                                            });
                                        }
                                    }),
                            ),
                    ),
            )
            // 值
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_sm().child("值:"))
                    .child(Input::new(&self.value_input).w_full()),
            )
            // Hash 字段名（Hash 类型专用）
            .when(show_hash, |this| {
                this.child(
                    v_flex()
                        .gap_1()
                        .child(div().text_sm().child("字段名 (Hash):"))
                        .child(Input::new(&self.hash_field_input).w_full()),
                )
            })
            // ZSet 分数（ZSet 类型专用）
            .when(show_zset, |this| {
                this.child(
                    v_flex()
                        .gap_1()
                        .child(div().text_sm().child("分数 (ZSet):"))
                        .child(Input::new(&self.zset_score_input).w_full()),
                )
            })
            .w_full()
    }
}
