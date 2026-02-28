use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, Render, Styled as _, Window,
};
use gpui_component::highlighter::Language;
use gpui_component::input::{Input, InputEvent, InputState, TabSize};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::v_flex;
use tracing::error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorTab {
    Text,
    Json,
}

impl EditorTab {
    pub fn language(&self) -> Language {
        match self {
            EditorTab::Text => Language::Plain,
            EditorTab::Json => Language::Json,
        }
    }
}

impl EventEmitter<InputEvent> for MultiTextEditor {}

pub struct MultiTextEditor {
    active_tab: EditorTab,
    text_editor: Entity<InputState>,
    json_editor: Entity<InputState>,
}

impl MultiTextEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text_editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(EditorTab::Text.language())
                .line_number(true)
                .searchable(true)
                .indent_guides(true)
                .tab_size(TabSize {
                    tab_size: 2,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .placeholder("Enter your text here...")
        });

        let json_editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(EditorTab::Json.language())
                .line_number(true)
                .searchable(true)
                .indent_guides(true)
                .tab_size(TabSize {
                    tab_size: 2,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .placeholder("Enter JSON here...")
        });

        Self {
            active_tab: EditorTab::Text,
            text_editor,
            json_editor,
        }
    }

    pub fn switch_tab(&mut self, tab: EditorTab, cx: &mut Context<Self>) {
        self.active_tab = tab;
        cx.notify();
    }

    fn get_active_editor(&self) -> &Entity<InputState> {
        match self.active_tab {
            EditorTab::Text => &self.text_editor,
            EditorTab::Json => &self.json_editor,
        }
    }

    pub fn get_active_text(&self, cx: &App) -> Result<String, json5::Error> {
        let value = self.get_active_editor().read(cx).text().to_string();
        if self.active_tab == EditorTab::Json {
            return match json5::from_str::<serde_json::Value>(&value) {
                Ok(v) => Ok(v.to_string()),
                Err(e) => Err(e),
            };
        }
        Ok(value)
    }

    pub fn set_active_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        // Set text editor
        self.text_editor.update(cx, |s, cx| {
            s.set_value(text.clone(), window, cx);
        });

        // Try to parse and format as JSON for json editor
        let json_text = match json5::from_str::<serde_json::Value>(&text) {
            Ok(value) => serde_json::to_string_pretty(&value).unwrap_or(text.clone()),
            Err(_) => text.clone(),
        };

        self.json_editor.update(cx, |s, cx| {
            s.set_value(json_text, window, cx);
        });
    }

    pub fn format_json(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.json_editor.read(cx).text().to_string();
        match json5::from_str::<serde_json::Value>(&text) {
            Ok(value) => {
                if let Ok(formatted) = serde_json::to_string_pretty(&value) {
                    self.json_editor.update(cx, |s, cx| {
                        s.set_value(formatted, window, cx);
                    });
                }
            }
            Err(e) => {
                error!("JSON解析错误: {:?}", e)
            }
        }
    }

    pub fn minify_json(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.json_editor.read(cx).text().to_string();
        match json5::from_str::<serde_json::Value>(&text) {
            Ok(value) => {
                if let Ok(minified) = serde_json::to_string(&value) {
                    self.json_editor.update(cx, |s, cx| {
                        s.set_value(minified, window, cx);
                    });
                }
            }
            Err(e) => {
                error!("JSON压缩错误: {:?}", e)
            }
        }
    }
}

impl Render for MultiTextEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui::ParentElement;
        use gpui::prelude::FluentBuilder;
        use gpui_component::{IconName, Sizable, Size, button::Button, h_flex};

        let active_tab = self.active_tab;
        let is_json_tab = active_tab == EditorTab::Json;
        let active_index = if active_tab == EditorTab::Text { 0 } else { 1 };

        v_flex()
            .size_full()
            .child(
                TabBar::new("editor-tabs")
                    .with_size(Size::Small)
                    .selected_index(active_index)
                    .child(Tab::new().label("Text"))
                    .child(Tab::new().label("JSON"))
                    .on_click(cx.listener(|this, ix: &usize, _, cx| {
                        let tab = if *ix == 0 {
                            EditorTab::Text
                        } else {
                            EditorTab::Json
                        };
                        this.switch_tab(tab, cx);
                    }))
                    .suffix(h_flex().gap_2().when(is_json_tab, |this| {
                        this.child(
                            Button::new("format-json")
                                .with_size(Size::Small)
                                .label("Format")
                                .icon(IconName::Star)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.format_json(window, cx);
                                })),
                        )
                        .child(
                            Button::new("minify-json")
                                .with_size(Size::Small)
                                .label("Minify")
                                .icon(IconName::File)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.minify_json(window, cx);
                                })),
                        )
                    })),
            )
            .child(v_flex().flex_1().child(match active_tab {
                EditorTab::Text => Input::new(&self.text_editor).size_full(),
                EditorTab::Json => Input::new(&self.json_editor).size_full(),
            }))
    }
}

pub fn create_multi_text_editor_with_content(
    initial_content: Option<String>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<MultiTextEditor> {
    cx.new(|cx| {
        let mut editor = MultiTextEditor::new(window, cx);
        if let Some(content) = initial_content {
            editor.set_active_text(content, window, cx);
        }
        editor
    })
}
