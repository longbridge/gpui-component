use std::rc::Rc;

use anyhow::Result;
use db::ColumnInfo;
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, Render, Styled as _, Subscription,
    Task, Window,
};
use gpui_component::highlighter::Language;
use gpui_component::input::{CompletionProvider, Input, InputEvent, InputState};
use gpui_component::{ActiveTheme, Rope, RopeExt};
use lsp_types::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse, CompletionTextEdit,
    Documentation, InsertReplaceEdit, Range,
};

#[derive(Clone)]
pub struct TableSchema {
    pub columns: Vec<ColumnInfo>,
}

// Completion provider for WHERE clause
#[derive(Clone)]
pub struct WhereCompletionProvider {
    schema: TableSchema,
}

impl WhereCompletionProvider {
    pub fn new(schema: TableSchema) -> Self {
        Self { schema }
    }
}

/// 获取当前正在输入的 token
fn extract_current_word(rope: &Rope, offset: usize) -> (String, usize) {
    let mut start = offset;
    while start > 0 {
        let ch = rope.char(start - 1);
        if !(ch.is_alphanumeric() || ch == '_' || ch == '.') {
            break;
        }
        start -= 1;
    }
    (rope.slice(start..offset).to_string().to_uppercase(), start)
}

/// 检测光标是否在字符串内部（单引号或双引号）
fn is_inside_string(rope: &Rope, offset: usize) -> bool {
    let text = rope.slice(0..offset).to_string();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = '\0';

    for ch in text.chars() {
        // 跳过转义的引号
        if prev_char == '\\' {
            prev_char = ch;
            continue;
        }

        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            _ => {}
        }
        prev_char = ch;
    }

    in_single_quote || in_double_quote
}

/// 获取当前 token 前的最近有效 token（作为上下文）
fn get_last_token_before(rope: &Rope, offset: usize) -> Option<String> {
    if offset == 0 {
        return None;
    }

    let mut idx = offset;
    while idx > 0 && rope.char(idx - 1).is_whitespace() {
        idx -= 1;
    }
    if idx == 0 {
        return None;
    }

    let mut token_start = idx;
    while token_start > 0 {
        let ch = rope.char(token_start - 1);
        if !(ch.is_alphanumeric() || ch == '_' || ch == '.') {
            break;
        }
        token_start -= 1;
    }
    let token = rope.slice(token_start..idx).to_string();
    if token.is_empty() { None } else { Some(token) }
}

/// 智能建议生成
fn suggest_items(
    schema: &TableSchema,
    current_word: &str,
    last_token: Option<&str>,
    replace_range: Range,
    full_text: &str,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let after_column = last_token.and_then(|t| {
        schema
            .columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(t))
    });

    // 检测上下文：是否在操作符后面（需要值提示）
    let after_operator = last_token
        .map(|t| {
            let upper = t.to_uppercase();
            matches!(
                upper.as_str(),
                "=" | "!=" | ">" | "<" | ">=" | "<=" | "LIKE" | "IN"
            )
        })
        .unwrap_or(false);

    // 1️⃣ 如果在操作符后面，优先提示值的模板
    if after_operator {
        suggest_value_templates(current_word, replace_range, &mut items);
    }

    // 2️⃣ 如果上一个 token 是列名 → 推操作符
    if let Some(col) = after_column {
        suggest_operators(col, current_word, replace_range, &mut items);
    } else {
        // 3️⃣ 否则提示列名
        suggest_columns(schema, current_word, replace_range, &mut items);
    }

    // 4️⃣ 如果已有条件 → AND / OR 优先
    if has_complete_condition(full_text) {
        add_logic_keywords(current_word, replace_range, &mut items);
    }

    // 5️⃣ SQL 函数智能提示
    suggest_functions(current_word, replace_range, &mut items);

    // 全局排序（值模板 > 操作符 > 字段 > 逻辑关键词 > 函数）
    items.sort_by_key(|x| x.sort_text.clone().unwrap_or("9".into()));
    items
}

/// 提示值的模板（字符串、数字、NULL 等）
fn suggest_value_templates(current_word: &str, range: Range, items: &mut Vec<CompletionItem>) {
    let templates = [
        ("'...'", "''", "String value"),
        ("NULL", "NULL", "NULL value"),
        ("true", "true", "Boolean true"),
        ("false", "false", "Boolean false"),
    ];

    for (label, text, doc) in &templates {
        if label
            .to_uppercase()
            .starts_with(&current_word.to_uppercase())
            || current_word.is_empty()
        {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::VALUE),
                documentation: Some(Documentation::String(doc.to_string())),
                text_edit: Some(insert_replace(text, range)),
                sort_text: Some("0_VALUE".into()),
                ..Default::default()
            });
        }
    }
}

/// 检测是否已有完整条件（简单检测：包含操作符）
fn has_complete_condition(text: &str) -> bool {
    let upper = text.to_uppercase();
    upper.contains('=')
        || upper.contains('>')
        || upper.contains('<')
        || upper.contains("LIKE")
        || upper.contains("IN")
        || upper.contains("BETWEEN")
}

fn suggest_columns(
    schema: &TableSchema,
    current_word: &str,
    replace_range: Range,
    items: &mut Vec<CompletionItem>,
) {
    for col in &schema.columns {
        if col.name.to_uppercase().starts_with(current_word) || current_word.is_empty() {
            let detail = if col.is_nullable {
                format!("{} (nullable)", col.data_type)
            } else {
                format!("{} (not null)", col.data_type)
            };

            items.push(CompletionItem {
                label: col.name.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(detail),
                documentation: Some(Documentation::String(format!(
                    "Column: {}\nType: {}\nNullable: {}",
                    col.name, col.data_type, col.is_nullable
                ))),
                sort_text: Some("2_COLUMN".into()),
                text_edit: Some(insert_replace(&col.name, replace_range)),
                ..Default::default()
            });
        }
    }
}

/// 操作符智能提示（增强 LIKE、IN、BETWEEN 的结构补全）
fn suggest_operators(
    col: &ColumnInfo,
    current_word: &str,
    range: Range,
    items: &mut Vec<CompletionItem>,
) {
    let dt = col.data_type.to_uppercase();

    // 不使用 snippet 语法，直接插入简洁模板
    // 光标会定位在插入文本末尾，用户可以直接输入
    let ops: Vec<(&str, &str, &str)> =
        if dt.contains("CHAR") || dt.contains("TEXT") || dt.contains("VARCHAR") {
            vec![
                ("= ''", "= ''", "Equal to"),
                ("!= ''", "!= ''", "Not equal to"),
                ("LIKE '%%'", "LIKE '%%'", "Pattern match (contains)"),
                ("LIKE '%'", "LIKE '%'", "Pattern match (starts with)"),
                ("IN ()", "IN ()", "In list"),
                ("IS NULL", "IS NULL", "Is null"),
                ("IS NOT NULL", "IS NOT NULL", "Is not null"),
            ]
        } else if dt.contains("INT")
            || dt.contains("DECIMAL")
            || dt.contains("FLOAT")
            || dt.contains("DOUBLE")
            || dt.contains("NUMERIC")
        {
            vec![
                ("=", "= ", "Equal to"),
                ("!=", "!= ", "Not equal to"),
                ("<", "< ", "Less than"),
                (">", "> ", "Greater than"),
                ("<=", "<= ", "Less than or equal"),
                (">=", ">= ", "Greater than or equal"),
                ("BETWEEN", "BETWEEN  AND ", "Between range"),
                ("IN ()", "IN ()", "In list"),
                ("IS NULL", "IS NULL", "Is null"),
                ("IS NOT NULL", "IS NOT NULL", "Is not null"),
            ]
        } else if dt.contains("DATE") || dt.contains("TIME") {
            vec![
                ("= ''", "= ''", "Equal to"),
                ("!= ''", "!= ''", "Not equal to"),
                ("< ''", "< ''", "Before"),
                ("> ''", "> ''", "After"),
                ("BETWEEN '' AND ''", "BETWEEN '' AND ''", "Between dates"),
                ("IS NULL", "IS NULL", "Is null"),
                ("IS NOT NULL", "IS NOT NULL", "Is not null"),
            ]
        } else {
            vec![
                ("=", "= ", "Equal to"),
                ("!=", "!= ", "Not equal to"),
                ("IS NULL", "IS NULL", "Is null"),
                ("IS NOT NULL", "IS NOT NULL", "Is not null"),
            ]
        };

    for (label, text, doc) in ops {
        if !current_word.is_empty()
            && !label
                .to_uppercase()
                .starts_with(&current_word.to_uppercase())
        {
            continue;
        }

        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::OPERATOR),
            detail: Some(format!("{} ({})", doc, col.data_type)),
            documentation: Some(Documentation::String(format!(
                "{}\n\nColumn: {} ({})",
                doc, col.name, col.data_type
            ))),
            text_edit: Some(insert_replace(text, range)),
            sort_text: Some("1_OPERATOR".into()),
            ..Default::default()
        });
    }
}

/// 逻辑关键词
fn add_logic_keywords(current_word: &str, range: Range, items: &mut Vec<CompletionItem>) {
    let keywords = [
        ("AND", "AND ", "Logical AND - both conditions must be true"),
        (
            "OR",
            "OR ",
            "Logical OR - at least one condition must be true",
        ),
    ];

    for (label, snippet, doc) in &keywords {
        if label.starts_with(&current_word.to_uppercase()) || current_word.is_empty() {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                documentation: Some(Documentation::String(doc.to_string())),
                text_edit: Some(insert_replace(snippet, range)),
                sort_text: Some("3_LOGIC".into()),
                ..Default::default()
            });
        }
    }
}

/// SQL 函数智能提示
fn suggest_functions(current_word: &str, range: Range, items: &mut Vec<CompletionItem>) {
    // 不使用 snippet 语法，光标定位在括号内
    let fns = [
        ("UPPER()", "UPPER()", "Convert to uppercase", "String"),
        ("LOWER()", "LOWER()", "Convert to lowercase", "String"),
        ("LENGTH()", "LENGTH()", "Get string length", "String"),
        ("TRIM()", "TRIM()", "Remove spaces", "String"),
        ("CONCAT()", "CONCAT(, )", "Concatenate strings", "String"),
        (
            "SUBSTRING()",
            "SUBSTRING(, , )",
            "Extract substring",
            "String",
        ),
        ("DATE()", "DATE()", "Extract date part", "Date"),
        ("YEAR()", "YEAR()", "Extract year", "Date"),
        ("MONTH()", "MONTH()", "Extract month", "Date"),
        ("DAY()", "DAY()", "Extract day", "Date"),
        ("NOW()", "NOW()", "Current timestamp", "Date"),
        ("COALESCE()", "COALESCE(, )", "First non-null", "Utility"),
        ("CAST()", "CAST( AS )", "Convert type", "Utility"),
    ];

    for (label, text, doc, category) in &fns {
        if label
            .to_uppercase()
            .starts_with(&current_word.to_uppercase())
            || current_word.is_empty()
        {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(category.to_string()),
                documentation: Some(Documentation::String(doc.to_string())),
                text_edit: Some(insert_replace(text, range)),
                sort_text: Some("4_FUNCTION".into()),
                ..Default::default()
            });
        }
    }
}

/// 工具方法
fn insert_replace(text: &str, range: Range) -> CompletionTextEdit {
    CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
        new_text: text.into(),
        insert: range,
        replace: range,
    })
}

impl CompletionProvider for WhereCompletionProvider {
    fn completions(
        &self,
        rope: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let rope = rope.clone();
        let schema = self.schema.clone();

        cx.background_spawn(async move {
            // 如果在字符串内部，不提供自动完成
            if is_inside_string(&rope, offset) {
                return Ok(CompletionResponse::Array(vec![]));
            }

            // 获取当前 token
            let (current_word, start_offset) = extract_current_word(&rope, offset);

            let start_pos = rope.offset_to_position(start_offset);
            let end_pos = rope.offset_to_position(offset);
            let replace_range = Range::new(start_pos, end_pos);

            let last_token = get_last_token_before(&rope, start_offset);
            let full_text = rope.to_string();

            let items = suggest_items(
                &schema,
                current_word.as_str(),
                last_token.as_deref(),
                replace_range,
                &full_text,
            );

            Ok(CompletionResponse::Array(items))
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        // 触发自动完成的条件：
        // 1. 空格、点、操作符后
        // 2. 输入字母、数字、下划线时
        // 注意：不在引号输入时触发，因为字符串内部不需要自动完成
        matches!(new_text, " " | "." | "=" | ">" | "<" | "!" | "(")
            || new_text
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
    }
}
// Completion provider for ORDER BY clause
#[derive(Clone)]
pub struct OrderByCompletionProvider {
    schema: TableSchema,
}

impl OrderByCompletionProvider {
    pub fn new(schema: TableSchema) -> Self {
        Self { schema }
    }
}

impl CompletionProvider for OrderByCompletionProvider {
    fn completions(
        &self,
        rope: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let rope = rope.clone();
        let schema = self.schema.clone();

        cx.background_spawn(async move {
            // 如果在字符串内部，不提供自动完成
            if is_inside_string(&rope, offset) {
                return Ok(CompletionResponse::Array(vec![]));
            }

            // 获取当前 token
            let (current_word, start_offset) = extract_current_word(&rope, offset);

            let start_pos = rope.offset_to_position(start_offset);
            let end_pos = rope.offset_to_position(offset);
            let replace_range = Range::new(start_pos, end_pos);
            let mut items = Vec::new();
            let last_token = get_last_token_before(&rope, start_offset);
            let after_column = last_token.clone().and_then(|t| {
                schema
                    .columns
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(&t))
            });

            // 如果 ORDER BY 后面已有字段，就优先提示 ASC / DESC
            if after_column.is_some() {
                // 补 ASC / DESC
                for (kw, doc) in &[("ASC", "Ascending"), ("DESC", "Descending")] {
                    items.push(CompletionItem {
                        label: kw.to_string(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        text_edit: Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
                            new_text: kw.to_string(),
                            insert: replace_range,
                            replace: replace_range,
                        })),
                        documentation: Some(Documentation::String(doc.to_string())),
                        sort_text: Some("0_ORDER_DIR".into()),
                        ..Default::default()
                    });
                }
            }

            let is_sort = last_token
                .as_ref()
                .map(|t| {
                    let upper = t.to_uppercase();
                    upper == "ASC" || upper == "DESC"
                })
                .unwrap_or(false);

            if is_sort {
                // 提示继续排序，补 ", <column>"
                for col in &schema.columns {
                    let text = format!(", {}", col.name);
                    items.push(CompletionItem {
                        label: text.clone(),
                        kind: Some(CompletionItemKind::FIELD),
                        text_edit: Some(insert_replace(&text, replace_range)),
                        sort_text: Some("1_ORDER_NEXT".into()),
                        detail: Some("Next ordering field".into()),
                        ..Default::default()
                    });
                }
            } else {
                suggest_columns(&schema, &current_word, replace_range, &mut items);
            }

            Ok(CompletionResponse::Array(items))
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        // Trigger completion on space, dot, comma, or when typing letters/numbers/underscore
        matches!(new_text, " " | "." | ",")
            || new_text
                .chars()
                .next()
                .map_or(false, |c| c.is_alphabetic() || c == '_')
    }
}

pub enum FilterEditorEvent {
    QueryApply,
}

pub struct SimpleCodeEditor {
    editor: Entity<InputState>,
    _sub: Subscription,
}

impl SimpleCodeEditor {
    pub fn new(editor: Entity<InputState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _sub = cx.subscribe_in(&editor, window, |_, _, event: &InputEvent, _, cx| {
            if let InputEvent::PressEnter { .. } = event {
                cx.emit(FilterEditorEvent::QueryApply);
            }
        });
        Self { editor, _sub }
    }

    pub fn get_text_from_app(&self, app_cx: &App) -> String {
        self.editor.read(app_cx).text().to_string()
    }
}

impl EventEmitter<FilterEditorEvent> for SimpleCodeEditor {}

impl Render for SimpleCodeEditor {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.editor).cleanable(true).size_full()
    }
}

pub fn create_simple_editor(
    window: &mut Window,
    cx: &mut Context<SimpleCodeEditor>,
) -> SimpleCodeEditor {
    let editor = cx.new(|cx| {
        let editor = InputState::new(window, cx)
            .code_editor(Language::from_str("sql"))
            .multi_line(false)
            .clean_on_escape();

        editor
    });

    SimpleCodeEditor::new(editor, window, cx)
}

// A combined component for table filtering that includes both WHERE and ORDER BY editors
pub struct TableFilterEditor {
    where_editor: Entity<SimpleCodeEditor>,
    order_by_editor: Entity<SimpleCodeEditor>,
    _subs: Vec<Subscription>,
}

impl TableFilterEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let where_editor = cx.new(|cx| create_simple_editor(window, cx));
        let order_by_editor = cx.new(|cx| create_simple_editor(window, cx));
        let where_sub = cx.subscribe_in(
            &where_editor,
            window,
            |_, _, evt: &FilterEditorEvent, _, cx| match evt {
                FilterEditorEvent::QueryApply => {
                    cx.emit(FilterEditorEvent::QueryApply);
                }
            },
        );
        let order_by_sub = cx.subscribe_in(
            &order_by_editor,
            window,
            |_, _, evt: &FilterEditorEvent, _, cx| match evt {
                FilterEditorEvent::QueryApply => {
                    cx.emit(FilterEditorEvent::QueryApply);
                }
            },
        );

        Self {
            where_editor,
            order_by_editor,
            _subs: vec![where_sub, order_by_sub],
        }
    }

    pub fn get_where_clause(&self, cx: &App) -> String {
        self.where_editor.read(cx).get_text_from_app(cx)
    }

    pub fn get_order_by_clause(&self, cx: &App) -> String {
        self.order_by_editor.read(cx).get_text_from_app(cx)
    }

    pub fn set_order_by_clause(
        &mut self,
        clause: impl Into<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let clause = clause.into();
        self.order_by_editor.update(cx, |editor, cx| {
            editor.editor.update(cx, |input_state, cx| {
                input_state.set_value(clause.clone(), window, cx);
            });
        });
    }

    pub fn set_schema(&mut self, schema: TableSchema, cx: &mut Context<Self>) {
        let schema_clone = schema.clone();

        self.where_editor.update(cx, |editor, cx| {
            editor.editor.update(cx, |input_state, _cx| {
                input_state.lsp.completion_provider =
                    Some(Rc::new(WhereCompletionProvider::new(schema.clone())));
            });
        });

        self.order_by_editor.update(cx, |editor, cx| {
            editor.editor.update(cx, |input_state, _cx| {
                input_state.lsp.completion_provider =
                    Some(Rc::new(OrderByCompletionProvider::new(schema_clone)));
            });
        });
    }
}

impl Render for TableFilterEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui::{ParentElement, Styled, div};
        use gpui_component::h_flex;

        h_flex()
            .size_full()
            .gap_3()
            .child(
                h_flex()
                    .flex_1()
                    .items_center()
                    .gap_2()
                    .child({
                        div()
                            .py_1()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(cx.theme().primary)
                            .child("WHERE")
                    })
                    .child(div().flex_1().child(self.where_editor.clone())),
            )
            .child(
                h_flex()
                    .flex_1()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .py_1()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(cx.theme().primary)
                            .child("ORDER BY"),
                    )
                    .child(div().flex_1().child(self.order_by_editor.clone())),
            )
    }
}

impl EventEmitter<FilterEditorEvent> for TableFilterEditor {}
