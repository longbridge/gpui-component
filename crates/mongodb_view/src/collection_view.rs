//! MongoDB 集合文档视图

use std::collections::HashMap;

use gpui::{
    AnyElement, App, AppContext, AsyncApp, ClipboardItem, Context, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, UniformListScrollHandle, Window, div,
    prelude::FluentBuilder, px, uniform_list,
};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, Size, StyledExt, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dialog::DialogButtonProps,
    h_flex,
    input::{Input, InputEvent, InputState},
    notification::Notification,
    spinner::Spinner,
    tab::{Tab, TabBar},
    v_flex,
};
use mongodb::bson::{Bson, Document};
use mongodb::options::FindOptions;
use one_core::gpui_tokio::Tokio;
use one_core::tab_container::{TabContent, TabContentEvent};
use rust_i18n::t;
use tracing::{error, info, warn};

use crate::GlobalMongoState;
use crate::types::{MongoError, bson_to_string, document_to_pretty_json};

const DEFAULT_PAGE_SIZE: i64 = 25;
const DEFAULT_SKIP: i64 = 0;
const TAB_DOCUMENTS: usize = 0;
const TAB_AGGREGATIONS: usize = 1;
const TAB_SCHEMA: usize = 2;
const TAB_INDEXES: usize = 3;
const TAB_VALIDATION: usize = 4;

#[derive(Clone)]
struct DocumentItem {
    id: String,
    id_bson: Option<Bson>,
    summary: String,
    document: Document,
    pretty_json: String,
}

#[derive(Default, Clone)]
struct FieldStats {
    count: usize,
    types: HashMap<String, usize>,
    examples: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorMode {
    View,
    Create,
    Update,
}

#[derive(Clone)]
struct QueryInputs {
    filter: Option<Document>,
    sort: Option<Document>,
    projection: Option<Document>,
    page_size: i64,
    skip_base: i64,
}

impl QueryInputs {
    fn skip(&self, page_index: i64) -> i64 {
        self.skip_base + page_index * self.page_size
    }
}

fn truncate_summary(value: &str, max_len: usize) -> String {
    let mut summary = value.chars().take(max_len).collect::<String>();
    if value.chars().count() > max_len {
        summary.push_str("...");
    }
    summary
}

fn parse_optional_document(text: &str, label: &str) -> Result<Option<Document>, MongoError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
        MongoError::InvalidFilter(
            t!("MongoCollection.parse_failed", label = label, error = e).to_string(),
        )
    })?;
    let bson = mongodb::bson::to_bson(&value).map_err(|e| {
        MongoError::InvalidFilter(
            t!("MongoCollection.parse_failed", label = label, error = e).to_string(),
        )
    })?;
    match bson {
        Bson::Document(document) => Ok(Some(document)),
        _ => Err(MongoError::InvalidFilter(
            t!("MongoCollection.must_be_json_object", label = label).to_string(),
        )),
    }
}

fn parse_required_document(text: &str, label: &str) -> Result<Document, MongoError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(MongoError::InvalidFilter(
            t!("MongoCollection.required", label = label).to_string(),
        ));
    }

    let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
        MongoError::InvalidFilter(
            t!("MongoCollection.parse_failed", label = label, error = e).to_string(),
        )
    })?;
    let bson = mongodb::bson::to_bson(&value).map_err(|e| {
        MongoError::InvalidFilter(
            t!("MongoCollection.parse_failed", label = label, error = e).to_string(),
        )
    })?;
    match bson {
        Bson::Document(document) => Ok(document),
        _ => Err(MongoError::InvalidFilter(
            t!("MongoCollection.must_be_json_object", label = label).to_string(),
        )),
    }
}

fn parse_i64_input(text: &str, label: &str, default: i64, min: i64) -> Result<i64, MongoError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(default);
    }

    match trimmed.parse::<i64>() {
        Ok(value) if value >= min => Ok(value),
        _ => Err(MongoError::InvalidFilter(
            t!("MongoCollection.must_be_min_int", label = label, min = min).to_string(),
        )),
    }
}

fn parse_pipeline(text: &str) -> Result<Vec<Document>, MongoError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let value: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
        MongoError::InvalidFilter(
            t!("MongoCollection.pipeline_parse_failed", error = e).to_string(),
        )
    })?;
    let array = match value {
        serde_json::Value::Array(items) => items,
        _ => {
            return Err(MongoError::InvalidFilter(
                t!("MongoCollection.pipeline_must_be_array").to_string(),
            ));
        }
    };

    let mut pipeline = Vec::with_capacity(array.len());
    for (index, item) in array.into_iter().enumerate() {
        let bson = mongodb::bson::to_bson(&item).map_err(|e| {
            MongoError::InvalidFilter(
                t!(
                    "MongoCollection.pipeline_item_parse_failed",
                    index = index + 1,
                    error = e
                )
                .to_string(),
            )
        })?;
        match bson {
            Bson::Document(document) => pipeline.push(document),
            _ => {
                return Err(MongoError::InvalidFilter(
                    t!(
                        "MongoCollection.pipeline_item_must_be_object",
                        index = index + 1
                    )
                    .to_string(),
                ));
            }
        }
    }
    Ok(pipeline)
}

fn document_to_compact_json(document: &Document) -> Result<String, MongoError> {
    let bson =
        mongodb::bson::to_bson(document).map_err(|e| MongoError::Serialization(e.to_string()))?;
    serde_json::to_string(&bson).map_err(|e| MongoError::Serialization(e.to_string()))
}

fn documents_to_pretty_json(documents: &[Document]) -> Result<String, MongoError> {
    let mut array = Vec::with_capacity(documents.len());
    for document in documents {
        let bson = mongodb::bson::to_bson(document)
            .map_err(|e| MongoError::Serialization(e.to_string()))?;
        array.push(bson);
    }
    let bson = Bson::Array(array);
    serde_json::to_string_pretty(&bson).map_err(|e| MongoError::Serialization(e.to_string()))
}

fn bson_type_name(value: &Bson) -> &'static str {
    match value {
        Bson::String(_) => "string",
        Bson::Int32(_) => "int32",
        Bson::Int64(_) => "int64",
        Bson::Double(_) => "double",
        Bson::Boolean(_) => "bool",
        Bson::ObjectId(_) => "objectId",
        Bson::DateTime(_) => "date",
        Bson::Document(_) => "document",
        Bson::Array(_) => "array",
        Bson::Null => "null",
        Bson::Binary(_) => "binary",
        Bson::Decimal128(_) => "decimal128",
        Bson::RegularExpression(_) => "regex",
        Bson::JavaScriptCode(_) => "javascript",
        Bson::JavaScriptCodeWithScope(_) => "javascriptScope",
        Bson::Timestamp(_) => "timestamp",
        Bson::Symbol(_) => "symbol",
        Bson::Undefined => "undefined",
        Bson::MaxKey => "maxKey",
        Bson::MinKey => "minKey",
        Bson::DbPointer(_) => "dbPointer",
    }
}

fn bson_value_preview(value: &Bson) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("{:?}", value))
}

fn build_schema_payload(documents: &[Document]) -> Result<String, MongoError> {
    let mut fields: HashMap<String, FieldStats> = HashMap::new();
    for document in documents {
        for (key, value) in document.iter() {
            let stats = fields.entry(key.clone()).or_default();
            stats.count += 1;
            let type_name = bson_type_name(value).to_string();
            *stats.types.entry(type_name).or_insert(0) += 1;
            if stats.examples.len() < 3 {
                stats.examples.push(bson_value_preview(value));
            }
        }
    }

    let mut fields_json = serde_json::Map::new();
    for (key, stats) in fields {
        let mut types_json = serde_json::Map::new();
        for (type_name, count) in stats.types {
            types_json.insert(type_name, serde_json::json!(count));
        }
        fields_json.insert(
            key,
            serde_json::json!({
                "count": stats.count,
                "types": types_json,
                "examples": stats.examples,
            }),
        );
    }

    let payload = serde_json::json!({
        "samples": documents.len(),
        "fields": fields_json,
    });

    serde_json::to_string_pretty(&payload).map_err(|e| MongoError::Serialization(e.to_string()))
}

/// MongoDB 集合文档视图
pub struct CollectionView {
    connection_id: Option<String>,
    database_name: Option<String>,
    collection_name: Option<String>,
    active_tab: usize,
    filter_input: Entity<InputState>,
    sort_input: Entity<InputState>,
    projection_input: Entity<InputState>,
    page_size_input: Entity<InputState>,
    skip_input: Entity<InputState>,
    editor_input: Entity<InputState>,
    explain_input: Entity<InputState>,
    aggregation_input: Entity<InputState>,
    aggregation_output: Entity<InputState>,
    schema_output: Entity<InputState>,
    index_name_input: Entity<InputState>,
    index_keys_input: Entity<InputState>,
    index_drop_input: Entity<InputState>,
    indexes_output: Entity<InputState>,
    validation_input: Entity<InputState>,
    documents: Vec<DocumentItem>,
    selected_index: Option<usize>,
    is_loading: bool,
    error_message: Option<String>,
    total_count: Option<i64>,
    page_index: i64,
    show_options: bool,
    show_explain: bool,
    editor_mode: EditorMode,
    editing_id: Option<Bson>,
    pending_editor_value: Option<String>,
    pending_explain_value: Option<String>,
    pending_aggregation_value: Option<String>,
    pending_schema_value: Option<String>,
    pending_indexes_value: Option<String>,
    pending_validation_value: Option<String>,
    pending_reload: bool,
    pending_select_id: Option<Bson>,
    aggregation_loading: bool,
    aggregation_error: Option<String>,
    aggregation_count: Option<usize>,
    schema_loading: bool,
    schema_error: Option<String>,
    schema_sample_count: Option<usize>,
    indexes_loading: bool,
    indexes_error: Option<String>,
    indexes_count: Option<usize>,
    validation_loading: bool,
    validation_error: Option<String>,
    list_scroll_handle: UniformListScrollHandle,
    _subscriptions: Vec<Subscription>,
    focus_handle: FocusHandle,
}

impl CollectionView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("MongoCollection.filter_placeholder").to_string())
        });
        let sort_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("MongoCollection.sort_placeholder").to_string())
        });
        let projection_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("MongoCollection.projection_placeholder").to_string())
        });
        let page_size_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("MongoCollection.page_size_placeholder").to_string());
            state.set_value(DEFAULT_PAGE_SIZE.to_string(), window, cx);
            state
        });
        let skip_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("MongoCollection.skip_placeholder").to_string());
            state.set_value(DEFAULT_SKIP.to_string(), window, cx);
            state
        });
        let editor_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
                .rows(16)
                .soft_wrap(false)
                .placeholder(t!("MongoCollection.document_placeholder").to_string())
        });
        let explain_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
                .rows(16)
                .soft_wrap(false)
                .placeholder(t!("MongoCollection.explain_placeholder").to_string())
        });
        let aggregation_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
                .rows(10)
                .soft_wrap(false)
                .placeholder(t!("MongoCollection.pipeline_placeholder").to_string());
            state.set_value("[]".to_string(), window, cx);
            state
        });
        let aggregation_output = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
                .rows(16)
                .soft_wrap(false)
                .placeholder(t!("MongoCollection.aggregation_output_placeholder").to_string())
        });
        let schema_output = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
                .rows(16)
                .soft_wrap(false)
                .placeholder(t!("MongoCollection.schema_placeholder").to_string())
        });
        let index_name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("MongoCollection.index_name_optional").to_string())
        });
        let index_keys_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
                .rows(6)
                .soft_wrap(false)
                .placeholder(t!("MongoCollection.index_keys_placeholder").to_string())
        });
        let index_drop_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("MongoCollection.index_name_placeholder").to_string())
        });
        let indexes_output = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
                .rows(16)
                .soft_wrap(false)
                .placeholder(t!("MongoCollection.indexes_placeholder").to_string())
        });
        let validation_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .line_number(false)
                .rows(16)
                .soft_wrap(false)
                .placeholder(t!("MongoCollection.validation_placeholder").to_string())
        });
        let mut subscriptions = Vec::new();
        let mut subscribe_enter = |subscriptions: &mut Vec<Subscription>,
                                   input: &Entity<InputState>| {
            subscriptions.push(cx.subscribe(input, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.page_index = 0;
                    this.reload(cx);
                }
            }));
        };
        subscribe_enter(&mut subscriptions, &filter_input);
        subscribe_enter(&mut subscriptions, &sort_input);
        subscribe_enter(&mut subscriptions, &projection_input);
        subscribe_enter(&mut subscriptions, &page_size_input);
        subscribe_enter(&mut subscriptions, &skip_input);

        Self {
            connection_id: None,
            database_name: None,
            collection_name: None,
            active_tab: TAB_DOCUMENTS,
            filter_input,
            sort_input,
            projection_input,
            page_size_input,
            skip_input,
            editor_input,
            explain_input,
            aggregation_input,
            aggregation_output,
            schema_output,
            index_name_input,
            index_keys_input,
            index_drop_input,
            indexes_output,
            validation_input,
            documents: Vec::new(),
            selected_index: None,
            is_loading: false,
            error_message: None,
            total_count: None,
            page_index: 0,
            show_options: false,
            show_explain: false,
            editor_mode: EditorMode::View,
            editing_id: None,
            pending_editor_value: None,
            pending_explain_value: None,
            pending_aggregation_value: None,
            pending_schema_value: None,
            pending_indexes_value: None,
            pending_validation_value: None,
            pending_reload: false,
            pending_select_id: None,
            aggregation_loading: false,
            aggregation_error: None,
            aggregation_count: None,
            schema_loading: false,
            schema_error: None,
            schema_sample_count: None,
            indexes_loading: false,
            indexes_error: None,
            indexes_count: None,
            validation_loading: false,
            validation_error: None,
            list_scroll_handle: UniformListScrollHandle::new(),
            _subscriptions: subscriptions,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn load_collection(
        &mut self,
        connection_id: String,
        database_name: String,
        collection_name: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_id = Some(connection_id);
        self.database_name = Some(database_name);
        self.collection_name = Some(collection_name);
        self.active_tab = TAB_DOCUMENTS;
        self.page_index = 0;
        self.selected_index = None;
        self.editor_mode = EditorMode::View;
        self.editing_id = None;
        self.show_explain = false;
        self.aggregation_loading = false;
        self.aggregation_error = None;
        self.aggregation_count = None;
        self.pending_aggregation_value = Some("[]".to_string());
        self.schema_loading = false;
        self.schema_error = None;
        self.schema_sample_count = None;
        self.pending_schema_value = Some("{}".to_string());
        self.indexes_loading = false;
        self.indexes_error = None;
        self.indexes_count = None;
        self.pending_indexes_value = Some("[]".to_string());
        self.validation_loading = false;
        self.validation_error = None;
        self.pending_validation_value = Some("{}".to_string());
        self.run_query(cx);
    }

    fn read_query_inputs(&self, cx: &mut Context<Self>) -> Result<QueryInputs, MongoError> {
        let filter_text = self.filter_input.read(cx).text().to_string();
        let filter =
            parse_optional_document(&filter_text, t!("MongoCollection.filter_label").as_ref())?;
        let sort_text = self.sort_input.read(cx).text().to_string();
        let sort = parse_optional_document(&sort_text, t!("MongoCollection.sort_label").as_ref())?;
        let projection_text = self.projection_input.read(cx).text().to_string();
        let projection = parse_optional_document(
            &projection_text,
            t!("MongoCollection.projection_label").as_ref(),
        )?;
        let page_size_text = self.page_size_input.read(cx).text().to_string();
        let page_size = parse_i64_input(
            &page_size_text,
            t!("MongoCollection.page_size_label").as_ref(),
            DEFAULT_PAGE_SIZE,
            1,
        )?;
        let skip_text = self.skip_input.read(cx).text().to_string();
        let skip_base = parse_i64_input(
            &skip_text,
            t!("MongoCollection.skip_label").as_ref(),
            DEFAULT_SKIP,
            0,
        )?;

        Ok(QueryInputs {
            filter,
            sort,
            projection,
            page_size,
            skip_base,
        })
    }

    fn read_query_inputs_or_default(&self, cx: &mut Context<Self>) -> QueryInputs {
        self.read_query_inputs(cx).unwrap_or(QueryInputs {
            filter: None,
            sort: None,
            projection: None,
            page_size: DEFAULT_PAGE_SIZE,
            skip_base: DEFAULT_SKIP,
        })
    }

    fn build_find_options(&self, inputs: &QueryInputs, skip: i64) -> FindOptions {
        let mut options = FindOptions::default();
        options.limit = Some(inputs.page_size);
        if skip > 0 {
            options.skip = Some(skip as u64);
        }
        if let Some(sort) = inputs.sort.clone() {
            options.sort = Some(sort);
        }
        if let Some(projection) = inputs.projection.clone() {
            options.projection = Some(projection);
        }
        options
    }

    fn set_error(&mut self, message: String, cx: &mut Context<Self>) {
        self.error_message = Some(message);
        self.is_loading = false;
        Self::notify_error(self.error_message.as_ref().unwrap(), cx);
        cx.notify();
    }

    fn notify_error(message: &str, cx: &mut Context<Self>) {
        if let Some(window) = cx.active_window() {
            let message = message.to_string();
            let _ = window.update(cx, |_, window, cx| {
                window.push_notification(Notification::error(message).autohide(true), cx);
            });
        }
    }

    fn notify_success(message: &str, cx: &mut Context<Self>) {
        if let Some(window) = cx.active_window() {
            let message = message.to_string();
            let _ = window.update(cx, |_, window, cx| {
                window.push_notification(Notification::success(message).autohide(true), cx);
            });
        }
    }

    fn set_input_value(
        input: &Entity<InputState>,
        value: impl Into<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = value.into();
        input.update(cx, |state, cx| {
            state.set_value(value, window, cx);
        });
    }

    fn set_editor_value(&mut self, value: String) {
        self.pending_editor_value = Some(value);
    }

    fn apply_pending_editor_value(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(value) = self.pending_editor_value.take() {
            self.editor_input.update(cx, |state, cx| {
                state.set_value(value, window, cx);
            });
        }
    }

    fn apply_pending_explain_value(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(value) = self.pending_explain_value.take() {
            self.explain_input.update(cx, |state, cx| {
                state.set_value(value, window, cx);
            });
        }
    }

    fn apply_pending_aggregation_value(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(value) = self.pending_aggregation_value.take() {
            self.aggregation_output.update(cx, |state, cx| {
                state.set_value(value, window, cx);
            });
        }
    }

    fn apply_pending_schema_value(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(value) = self.pending_schema_value.take() {
            self.schema_output.update(cx, |state, cx| {
                state.set_value(value, window, cx);
            });
        }
    }

    fn apply_pending_indexes_value(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(value) = self.pending_indexes_value.take() {
            self.indexes_output.update(cx, |state, cx| {
                state.set_value(value, window, cx);
            });
        }
    }

    fn apply_pending_validation_value(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(value) = self.pending_validation_value.take() {
            self.validation_input.update(cx, |state, cx| {
                state.set_value(value, window, cx);
            });
        }
    }

    fn apply_pending_reload(&mut self, cx: &mut Context<Self>) {
        if self.pending_reload {
            self.pending_reload = false;
            self.run_query(cx);
        }
    }

    fn set_active_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.active_tab == index {
            return;
        }
        self.active_tab = index;
        match index {
            TAB_SCHEMA => self.load_schema(cx),
            TAB_INDEXES => self.load_indexes(cx),
            TAB_VALIDATION => self.load_validation(cx),
            _ => {}
        }
        cx.notify();
    }

    fn select_document(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.documents.get(index).cloned() else {
            return;
        };
        self.selected_index = Some(index);
        self.editor_mode = EditorMode::View;
        self.editing_id = item.id_bson.clone();
        self.show_explain = false;
        self.set_editor_value(item.pretty_json);
        self.apply_pending_editor_value(window, cx);
        cx.notify();
    }

    fn start_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor_mode = EditorMode::Create;
        self.editing_id = None;
        self.show_explain = false;
        self.set_editor_value("{\n  \n}".to_string());
        self.apply_pending_editor_value(window, cx);
        cx.notify();
    }

    fn start_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.selected_index else {
            return;
        };
        let Some(item) = self.documents.get(index).cloned() else {
            return;
        };
        self.editor_mode = EditorMode::Update;
        self.editing_id = item.id_bson.clone();
        self.show_explain = false;
        self.set_editor_value(item.pretty_json);
        self.apply_pending_editor_value(window, cx);
        cx.notify();
    }

    fn start_clone(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.documents.get(index).cloned() else {
            return;
        };
        let mut cloned = item.document.clone();
        cloned.remove("_id");
        match document_to_pretty_json(&cloned) {
            Ok(content) => {
                self.editor_mode = EditorMode::Create;
                self.editing_id = None;
                self.show_explain = false;
                self.set_editor_value(content);
                self.apply_pending_editor_value(window, cx);
                cx.notify();
            }
            Err(error) => self.set_error(error.to_string(), cx),
        }
    }

    fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor_mode = EditorMode::View;
        self.show_explain = false;
        if let Some(index) = self.selected_index {
            if let Some(item) = self.documents.get(index) {
                self.set_editor_value(item.pretty_json.clone());
                self.apply_pending_editor_value(window, cx);
            }
        }
        cx.notify();
    }

    fn save_edit(&mut self, cx: &mut Context<Self>) {
        let content = self.editor_input.read(cx).text().to_string();
        let mut document = match parse_required_document(
            &content,
            t!("MongoCollection.document_label").as_ref(),
        ) {
            Ok(document) => document,
            Err(error) => {
                self.set_error(error.to_string(), cx);
                return;
            }
        };

        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        let editor_mode = self.editor_mode;
        let success_message = match editor_mode {
            EditorMode::Create => t!("MongoCollection.document_created").to_string(),
            EditorMode::Update => t!("MongoCollection.document_updated").to_string(),
            EditorMode::View => t!("MongoCollection.operation_done").to_string(),
        };
        let target_id = if editor_mode == EditorMode::Update {
            let target_id = self.editing_id.clone().or_else(|| {
                self.selected_index
                    .and_then(|index| self.documents.get(index))
                    .and_then(|item| item.id_bson.clone())
            });
            let Some(target_id) = target_id else {
                self.set_error(t!("MongoCollection.id_required_for_update").to_string(), cx);
                return;
            };
            document.insert("_id", target_id.clone());
            Some(target_id)
        } else {
            None
        };
        if editor_mode == EditorMode::Create {
            self.pending_select_id = document.get("_id").cloned();
        } else {
            self.pending_select_id = target_id.clone();
        }

        self.is_loading = true;
        self.error_message = None;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let target_id = target_id.clone();
            let success_message = success_message.to_string();
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                match editor_mode {
                    EditorMode::Create => guard
                        .insert_document(&database_name, &collection_name, document)
                        .await
                        .map(|_| ()),
                    EditorMode::Update => {
                        let target_id = target_id.ok_or_else(|| {
                            anyhow::anyhow!(t!("MongoCollection.id_missing").to_string())
                        })?;
                        guard
                            .replace_document(&database_name, &collection_name, target_id, document)
                            .await
                            .map(|_| ())
                    }
                    EditorMode::View => Ok(()),
                }
                .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.is_loading = false;
                match result {
                    Ok(_) => {
                        view.editor_mode = EditorMode::View;
                        view.editing_id = None;
                        view.show_explain = false;
                        view.pending_reload = true;
                        Self::notify_success(&success_message, cx);
                    }
                    Err(error) => {
                        view.error_message = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };
        let Some(index) = self.selected_index else {
            return;
        };
        let Some(item) = self.documents.get(index).cloned() else {
            return;
        };
        let Some(id_bson) = item.id_bson.clone() else {
            self.set_error(t!("MongoCollection.id_required_for_delete").to_string(), cx);
            return;
        };
        self.pending_select_id = self
            .documents
            .get(index + 1)
            .or_else(|| index.checked_sub(1).and_then(|idx| self.documents.get(idx)))
            .and_then(|item| item.id_bson.clone());

        self.is_loading = true;
        self.error_message = None;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                guard
                    .delete_document(&database_name, &collection_name, id_bson)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.is_loading = false;
                match result {
                    Ok(_) => {
                        view.editor_mode = EditorMode::View;
                        view.editing_id = None;
                        view.selected_index = None;
                        view.show_explain = false;
                        view.pending_reload = true;
                        Self::notify_success(t!("MongoCollection.document_deleted").as_ref(), cx);
                    }
                    Err(error) => {
                        view.error_message = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn copy_selected(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.selected_index else {
            return;
        };
        let Some(item) = self.documents.get(index) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(item.pretty_json.clone()));
        Self::notify_success(t!("MongoCollection.document_copied").as_ref(), cx);
    }

    fn export_data(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.build_export_data() {
            Ok(payload) => {
                self.show_export_dialog(
                    t!("MongoCollection.export_data_title").as_ref(),
                    payload,
                    "json",
                    window,
                    cx,
                );
            }
            Err(error) => self.set_error(error.to_string(), cx),
        }
    }

    fn export_code(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let inputs = match self.read_query_inputs(cx) {
            Ok(inputs) => inputs,
            Err(error) => {
                self.set_error(error.to_string(), cx);
                return;
            }
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };
        let skip = inputs.skip(self.page_index);
        match self.build_query_code(&collection_name, &inputs, skip) {
            Ok(code) => {
                self.show_export_dialog(
                    t!("MongoCollection.export_code_title").as_ref(),
                    code,
                    "javascript",
                    window,
                    cx,
                );
            }
            Err(error) => self.set_error(error.to_string(), cx),
        }
    }

    fn show_export_dialog(
        &self,
        title: &str,
        content: String,
        language: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let language = language.to_string();
        let editor = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor(language.clone())
                .line_number(false)
                .rows(18)
                .soft_wrap(false);
            state.set_value(content.clone(), window, cx);
            state
        });
        let title = title.to_string();
        let content_for_copy = content.clone();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let editor = editor.clone();
            let content = content_for_copy.clone();

            dialog
                .title(title.clone())
                .w(px(820.0))
                .h(px(600.0))
                .child(
                    v_flex()
                        .size_full()
                        .child(Input::new(&editor).size_full().disabled(true)),
                )
                .confirm()
                .button_props(
                    DialogButtonProps::default()
                        .ok_text(t!("Common.copy").to_string())
                        .cancel_text(t!("Common.close").to_string()),
                )
                .on_ok(move |_, window, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(content.clone()));
                    window.push_notification(
                        Notification::success(
                            t!("MongoCollection.copied_to_clipboard").to_string(),
                        )
                        .autohide(true),
                        cx,
                    );
                    window.close_dialog(cx);
                    false
                })
        });
    }

    fn build_export_data(&self) -> Result<String, MongoError> {
        let mut array = Vec::new();
        for item in &self.documents {
            let bson = mongodb::bson::to_bson(&item.document)
                .map_err(|e| MongoError::Serialization(e.to_string()))?;
            array.push(bson);
        }
        let bson = Bson::Array(array);
        serde_json::to_string_pretty(&bson).map_err(|e| MongoError::Serialization(e.to_string()))
    }

    fn build_query_code(
        &self,
        collection_name: &str,
        inputs: &QueryInputs,
        skip: i64,
    ) -> Result<String, MongoError> {
        let filter = match &inputs.filter {
            Some(filter) => document_to_compact_json(filter)?,
            None => "{}".to_string(),
        };
        let projection = match &inputs.projection {
            Some(projection) => Some(document_to_compact_json(projection)?),
            None => None,
        };

        let mut code = if let Some(projection) = projection {
            format!("db.{}.find({}, {})", collection_name, filter, projection)
        } else {
            format!("db.{}.find({})", collection_name, filter)
        };

        if let Some(sort) = &inputs.sort {
            let sort = document_to_compact_json(sort)?;
            code.push_str(&format!(".sort({})", sort));
        }
        if skip > 0 {
            code.push_str(&format!(".skip({})", skip));
        }
        code.push_str(&format!(".limit({})", inputs.page_size));
        Ok(code)
    }

    fn run_explain(&mut self, cx: &mut Context<Self>) {
        let inputs = match self.read_query_inputs(cx) {
            Ok(inputs) => inputs,
            Err(error) => {
                self.set_error(error.to_string(), cx);
                return;
            }
        };
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        let skip = inputs.skip(self.page_index);
        let options = self.build_find_options(&inputs, skip);
        self.is_loading = true;
        self.error_message = None;
        self.show_explain = true;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                guard
                    .explain_find(&database_name, &collection_name, inputs.filter, options)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.is_loading = false;
                match result {
                    Ok(document) => match document_to_pretty_json(&document) {
                        Ok(pretty) => {
                            view.pending_explain_value = Some(pretty);
                            view.show_explain = true;
                        }
                        Err(error) => {
                            view.error_message = Some(error.to_string());
                            view.show_explain = false;
                            Self::notify_error(&error.to_string(), cx);
                        }
                    },
                    Err(error) => {
                        view.error_message = Some(error.to_string());
                        view.show_explain = false;
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn run_aggregation(&mut self, cx: &mut Context<Self>) {
        let pipeline_text = self.aggregation_input.read(cx).text().to_string();
        let pipeline = match parse_pipeline(&pipeline_text) {
            Ok(pipeline) => pipeline,
            Err(error) => {
                self.aggregation_error = Some(error.to_string());
                Self::notify_error(&error.to_string(), cx);
                cx.notify();
                return;
            }
        };
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        self.aggregation_loading = true;
        self.aggregation_error = None;
        self.aggregation_count = None;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                guard
                    .aggregate_documents(&database_name, &collection_name, pipeline)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.aggregation_loading = false;
                match result {
                    Ok(documents) => match documents_to_pretty_json(&documents) {
                        Ok(payload) => {
                            view.pending_aggregation_value = Some(payload);
                            view.aggregation_count = Some(documents.len());
                        }
                        Err(error) => {
                            view.aggregation_error = Some(error.to_string());
                            Self::notify_error(&error.to_string(), cx);
                        }
                    },
                    Err(error) => {
                        view.aggregation_error = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn export_aggregation_result(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let content = self.aggregation_output.read(cx).text().to_string();
        if content.trim().is_empty() {
            Self::notify_error(t!("MongoCollection.no_aggregation_to_export").as_ref(), cx);
            return;
        }
        self.show_export_dialog(
            t!("MongoCollection.export_aggregation_title").as_ref(),
            content,
            "json",
            window,
            cx,
        );
    }

    fn load_schema(&mut self, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        self.schema_loading = true;
        self.schema_error = None;
        self.schema_sample_count = None;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                let mut options = FindOptions::default();
                options.limit = Some(100);
                guard
                    .find_documents(&database_name, &collection_name, None, options)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.schema_loading = false;
                match result {
                    Ok(documents) => match build_schema_payload(&documents) {
                        Ok(payload) => {
                            view.pending_schema_value = Some(payload);
                            view.schema_sample_count = Some(documents.len());
                        }
                        Err(error) => {
                            view.schema_error = Some(error.to_string());
                            Self::notify_error(&error.to_string(), cx);
                        }
                    },
                    Err(error) => {
                        view.schema_error = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn load_indexes(&mut self, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        self.indexes_loading = true;
        self.indexes_error = None;
        self.indexes_count = None;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                guard
                    .list_indexes(&database_name, &collection_name)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.indexes_loading = false;
                match result {
                    Ok(indexes) => match documents_to_pretty_json(&indexes) {
                        Ok(payload) => {
                            view.pending_indexes_value = Some(payload);
                            view.indexes_count = Some(indexes.len());
                        }
                        Err(error) => {
                            view.indexes_error = Some(error.to_string());
                            Self::notify_error(&error.to_string(), cx);
                        }
                    },
                    Err(error) => {
                        view.indexes_error = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn create_index(&mut self, cx: &mut Context<Self>) {
        let keys_text = self.index_keys_input.read(cx).text().to_string();
        let keys = match parse_required_document(
            &keys_text,
            t!("MongoCollection.index_keys_label").as_ref(),
        ) {
            Ok(keys) => keys,
            Err(error) => {
                self.indexes_error = Some(error.to_string());
                Self::notify_error(&error.to_string(), cx);
                cx.notify();
                return;
            }
        };
        let name = self.index_name_input.read(cx).text().to_string();
        let name = if name.trim().is_empty() {
            None
        } else {
            Some(name.trim().to_string())
        };
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        self.indexes_loading = true;
        self.indexes_error = None;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                guard
                    .create_index(&database_name, &collection_name, keys, name)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.indexes_loading = false;
                match result {
                    Ok(_) => {
                        Self::notify_success(t!("MongoCollection.index_created").as_ref(), cx);
                        view.load_indexes(cx);
                    }
                    Err(error) => {
                        view.indexes_error = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn drop_index(&mut self, cx: &mut Context<Self>) {
        let name = self.index_drop_input.read(cx).text().to_string();
        let name = name.trim().to_string();
        if name.is_empty() {
            self.indexes_error = Some(t!("MongoCollection.index_name_required").to_string());
            Self::notify_error(t!("MongoCollection.index_name_required").as_ref(), cx);
            cx.notify();
            return;
        }
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        self.indexes_loading = true;
        self.indexes_error = None;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                guard
                    .drop_index(&database_name, &collection_name, &name)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.indexes_loading = false;
                match result {
                    Ok(_) => {
                        Self::notify_success(t!("MongoCollection.index_deleted").as_ref(), cx);
                        view.load_indexes(cx);
                    }
                    Err(error) => {
                        view.indexes_error = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn load_validation(&mut self, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        self.validation_loading = true;
        self.validation_error = None;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                guard
                    .get_collection_validation(&database_name, &collection_name)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.validation_loading = false;
                match result {
                    Ok(validator) => {
                        let payload = match validator {
                            Some(doc) => {
                                document_to_pretty_json(&doc).unwrap_or_else(|_| "{}".to_string())
                            }
                            None => "{}".to_string(),
                        };
                        view.pending_validation_value = Some(payload);
                    }
                    Err(error) => {
                        view.validation_error = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn save_validation(&mut self, cx: &mut Context<Self>) {
        let text = self.validation_input.read(cx).text().to_string();
        let validator =
            match parse_optional_document(&text, t!("MongoCollection.validation_label").as_ref()) {
                Ok(validator) => validator,
                Err(error) => {
                    self.validation_error = Some(error.to_string());
                    Self::notify_error(&error.to_string(), cx);
                    cx.notify();
                    return;
                }
            };
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        self.validation_loading = true;
        self.validation_error = None;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                guard
                    .update_collection_validation(&database_name, &collection_name, validator)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.validation_loading = false;
                match result {
                    Ok(_) => {
                        Self::notify_success(t!("MongoCollection.validation_updated").as_ref(), cx);
                        view.load_validation(cx);
                    }
                    Err(error) => {
                        view.validation_error = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn clear_validation(&mut self, cx: &mut Context<Self>) {
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        self.validation_loading = true;
        self.validation_error = None;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;
                guard
                    .update_collection_validation(&database_name, &collection_name, None)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                view.validation_loading = false;
                match result {
                    Ok(_) => {
                        view.pending_validation_value = Some("{}".to_string());
                        Self::notify_success(t!("MongoCollection.validation_cleared").as_ref(), cx);
                        view.load_validation(cx);
                    }
                    Err(error) => {
                        view.validation_error = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn reset_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        Self::set_input_value(&self.filter_input, "", window, cx);
        Self::set_input_value(&self.sort_input, "", window, cx);
        Self::set_input_value(&self.projection_input, "", window, cx);
        Self::set_input_value(
            &self.page_size_input,
            DEFAULT_PAGE_SIZE.to_string(),
            window,
            cx,
        );
        Self::set_input_value(&self.skip_input, DEFAULT_SKIP.to_string(), window, cx);
        self.page_index = 0;
        self.show_explain = false;
        cx.notify();
    }

    fn run_query(&mut self, cx: &mut Context<Self>) {
        let inputs = match self.read_query_inputs(cx) {
            Ok(inputs) => inputs,
            Err(error) => {
                self.set_error(error.to_string(), cx);
                return;
            }
        };
        let Some(connection_id) = self.connection_id.clone() else {
            return;
        };
        let Some(database_name) = self.database_name.clone() else {
            return;
        };
        let Some(collection_name) = self.collection_name.clone() else {
            return;
        };

        let skip = inputs.skip(self.page_index);
        let options = self.build_find_options(&inputs, skip);
        if self.pending_select_id.is_none() {
            self.pending_select_id = self
                .selected_index
                .and_then(|index| self.documents.get(index))
                .and_then(|item| item.id_bson.clone());
        }

        self.is_loading = true;
        self.error_message = None;
        self.documents.clear();
        self.total_count = None;
        self.selected_index = None;
        self.editor_mode = EditorMode::View;
        self.editing_id = None;
        self.show_explain = false;
        cx.notify();

        let global_state = cx.global::<GlobalMongoState>().clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = Tokio::spawn_result(cx, async move {
                info!(
                    "[run_query] 开始查询 db={}, collection={}, filter={:?}",
                    database_name, collection_name, inputs.filter
                );
                let connection = global_state.get_connection(&connection_id).ok_or_else(|| {
                    anyhow::anyhow!(t!("MongoCollection.connection_missing").to_string())
                })?;
                let guard = connection.read().await;

                info!("[run_query] 开始 find_documents");
                let documents = guard
                    .find_documents(
                        &database_name,
                        &collection_name,
                        inputs.filter.clone(),
                        options,
                    )
                    .await;
                match &documents {
                    Ok(docs) => info!("[run_query] find_documents 成功，返回 {} 条文档", docs.len()),
                    Err(e) => error!("[run_query] find_documents 失败: {e}"),
                }
                let documents = documents.map_err(|e| anyhow::anyhow!("{}", e))?;

                info!("[run_query] 开始 count_documents");
                let total = guard
                    .count_documents(&database_name, &collection_name, inputs.filter)
                    .await;
                match &total {
                    Ok(count) => info!("[run_query] count_documents 成功，total={count}"),
                    Err(e) => warn!("[run_query] count_documents 失败（将忽略）: {e}"),
                }
                // count_documents 在某些系统集合（如 system.sessions）上可能失败，
                // 失败时不阻断文档展示，总数显示为未知
                let total = total.ok();
                Ok((documents, total))
            })
            .await;

            _ = this.update(cx, |view, cx| {
                match result {
                    Ok((documents, total)) => {
                        info!("[run_query] 异步结果成功，文档数={}, total={:?}", documents.len(), total);
                        let items_result: Result<Vec<DocumentItem>, MongoError> = documents
                            .into_iter()
                            .enumerate()
                            .map(|(index, document)| {
                                let id_bson = document.get("_id").cloned();
                                let id = id_bson
                                    .as_ref()
                                    .map(bson_to_string)
                                    .unwrap_or_else(|| format!("#{}", index + 1));
                                let json = document_to_pretty_json(&document)?;
                                let summary =
                                    truncate_summary(json.lines().next().unwrap_or(""), 120);
                                Ok(DocumentItem {
                                    id,
                                    id_bson,
                                    summary,
                                    document,
                                    pretty_json: json,
                                })
                            })
                            .collect();
                        match items_result {
                            Ok(items) => {
                                view.documents = items;
                                view.total_count = total;
                                view.is_loading = false;
                                view.error_message = None;

                                if let Some(target_id) = view.pending_select_id.take() {
                                    if let Some((index, item)) =
                                        view.documents.iter().enumerate().find(|(_, item)| {
                                            item.id_bson.as_ref() == Some(&target_id)
                                        })
                                    {
                                        view.selected_index = Some(index);
                                        view.editor_mode = EditorMode::View;
                                        view.editing_id = item.id_bson.clone();
                                        view.set_editor_value(item.pretty_json.clone());
                                    } else {
                                        view.selected_index = None;
                                        view.editing_id = None;
                                    }
                                }
                            }
                            Err(error) => {
                                error!("[run_query] 文档序列化失败: {error}");
                                view.is_loading = false;
                                view.error_message = Some(error.to_string());
                                Self::notify_error(&error.to_string(), cx);
                            }
                        }
                    }
                    Err(error) => {
                        error!("[run_query] Tokio任务失败: {error}");
                        view.is_loading = false;
                        view.error_message = Some(error.to_string());
                        Self::notify_error(&error.to_string(), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        if self.connection_id.is_none()
            || self.database_name.is_none()
            || self.collection_name.is_none()
        {
            return;
        }

        self.run_query(cx);
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let title = match (&self.database_name, &self.collection_name) {
            (Some(database), Some(collection)) => format!("{} / {}", database, collection),
            _ => t!("MongoCollection.select_collection").to_string(),
        };

        h_flex()
            .items_center()
            .gap_2()
            .child(
                Icon::new(IconName::MongoDB)
                    .color()
                    .with_size(Size::Small)
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(title),
            )
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let document_label = t!("MongoCollection.tab_documents").to_string();
        let aggregation_label = t!("MongoCollection.tab_aggregations").to_string();
        let schema_label = t!("MongoCollection.tab_schema").to_string();
        let index_label = t!("MongoCollection.tab_indexes").to_string();
        let validation_label = t!("MongoCollection.tab_validation").to_string();

        TabBar::new("mongo-collection-tabs")
            .with_size(Size::Large)
            .underline()
            .selected_index(self.active_tab)
            .on_click(cx.listener(|this, ix: &usize, _, cx| {
                this.set_active_tab(*ix, cx);
            }))
            .child(Tab::new().label(document_label))
            .child(Tab::new().label(aggregation_label))
            .child(Tab::new().label(schema_label))
            .child(Tab::new().label(index_label))
            .child(Tab::new().label(validation_label))
    }

    fn render_query_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_loading = self.is_loading;
        h_flex()
            .gap_2()
            .items_center()
            .child(Input::new(&self.filter_input).w(px(320.0)))
            .child(
                Button::new("mongo-explain")
                    .small()
                    .outline()
                    .label(t!("MongoCollection.explain").to_string())
                    .disabled(is_loading)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        if this.show_explain {
                            this.show_explain = false;
                            cx.notify();
                            return;
                        }
                        this.run_explain(cx);
                    })),
            )
            .child(
                Button::new("mongo-reset")
                    .small()
                    .outline()
                    .label(t!("MongoCollection.reset").to_string())
                    .disabled(is_loading)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.reset_inputs(window, cx);
                    })),
            )
            .child(
                Button::new("mongo-find")
                    .small()
                    .primary()
                    .label(t!("MongoCollection.query").to_string())
                    .disabled(is_loading)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.page_index = 0;
                        this.reload(cx);
                    })),
            )
            .child(
                Button::new("mongo-options")
                    .small()
                    .outline()
                    .icon(if self.show_options {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .label(t!("MongoCollection.options").to_string())
                    .disabled(is_loading)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_options = !this.show_options;
                        cx.notify();
                    })),
            )
    }

    fn render_options_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        if !self.show_options {
            return div().into_any_element();
        }

        v_flex()
            .gap_2()
            .p_2()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(6.0))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("MongoCollection.sort_label").to_string()),
                    )
                    .child(Input::new(&self.sort_input).w(px(220.0)))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("MongoCollection.projection_label").to_string()),
                    )
                    .child(Input::new(&self.projection_input).w(px(220.0)))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("MongoCollection.skip_label").to_string()),
                    )
                    .child(Input::new(&self.skip_input).w(px(100.0)))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!("MongoCollection.page_size_label").to_string()),
                    )
                    .child(Input::new(&self.page_size_input).w(px(100.0))),
            )
            .into_any_element()
    }

    fn render_action_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_selection = self.selected_index.is_some();
        let is_loading = self.is_loading;
        h_flex()
            .gap_2()
            .items_center()
            .child(
                Button::new("mongo-add")
                    .small()
                    .outline()
                    .icon(IconName::Plus)
                    .label(t!("MongoCollection.add").to_string())
                    .disabled(is_loading)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.start_create(window, cx);
                    })),
            )
            .child(
                Button::new("mongo-update")
                    .small()
                    .outline()
                    .icon(IconName::Edit)
                    .label(t!("MongoCollection.update").to_string())
                    .disabled(is_loading || !has_selection)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.start_update(window, cx);
                    })),
            )
            .child(
                Button::new("mongo-delete")
                    .small()
                    .outline()
                    .icon(IconName::Remove)
                    .label(t!("Common.delete").to_string())
                    .disabled(is_loading || !has_selection)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.delete_selected(cx);
                    })),
            )
            .child(
                Button::new("mongo-export-data")
                    .small()
                    .outline()
                    .icon(IconName::ExternalLink)
                    .label(t!("MongoCollection.export_data").to_string())
                    .disabled(is_loading)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.export_data(window, cx);
                    })),
            )
            .child(
                Button::new("mongo-export-code")
                    .small()
                    .outline()
                    .icon(IconName::Query)
                    .label(t!("MongoCollection.export_code").to_string())
                    .disabled(is_loading)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.export_code(window, cx);
                    })),
            )
    }

    fn render_documents_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(self.render_query_bar(cx))
            .child(self.render_options_panel(cx))
            .child(self.render_action_bar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .h_full()
                    .child(self.render_document_list(cx))
                    .child(self.render_detail_panel(cx)),
            )
            .child(self.render_pagination_bar(cx))
    }

    fn render_aggregation_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let result_summary = self
            .aggregation_count
            .map(|count| t!("MongoCollection.result_count", count = count).to_string());

        let result_body = if self.aggregation_loading {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new())
                .into_any_element()
        } else if let Some(error) = &self.aggregation_error {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().danger)
                .child(error.clone())
                .into_any_element()
        } else {
            div()
                .size_full()
                .child(Input::new(&self.aggregation_output).h_full().disabled(true))
                .into_any_element()
        };

        v_flex()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("mongo-aggregation-run")
                            .small()
                            .primary()
                            .label(t!("MongoCollection.run").to_string())
                            .disabled(self.aggregation_loading)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.run_aggregation(cx);
                            })),
                    )
                    .child(
                        Button::new("mongo-aggregation-export")
                            .small()
                            .outline()
                            .label(t!("MongoCollection.export_result").to_string())
                            .disabled(self.aggregation_loading)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.export_aggregation_result(window, cx);
                            })),
                    )
                    .when_some(result_summary, |this, summary| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(summary),
                        )
                    }),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .gap_2()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_h_0()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(px(6.0))
                            .p_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("MongoCollection.pipeline_label").to_string()),
                            )
                            .child(Input::new(&self.aggregation_input).h_full()),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_h_0()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(px(6.0))
                            .p_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("MongoCollection.result_preview_label").to_string()),
                            )
                            .child(result_body),
                    ),
            )
    }

    fn render_schema_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let summary = self
            .schema_sample_count
            .map(|count| t!("MongoCollection.sample_count", count = count).to_string());

        let body = if self.schema_loading {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new())
                .into_any_element()
        } else if let Some(error) = &self.schema_error {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().danger)
                .child(error.clone())
                .into_any_element()
        } else {
            div()
                .size_full()
                .child(Input::new(&self.schema_output).h_full().disabled(true))
                .into_any_element()
        };

        v_flex()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("mongo-schema-refresh")
                            .small()
                            .outline()
                            .label(t!("MongoCollection.resample").to_string())
                            .disabled(self.schema_loading)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.load_schema(cx);
                            })),
                    )
                    .when_some(summary, |this, text| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(text),
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(6.0))
                    .p_2()
                    .child(body),
            )
    }

    fn render_indexes_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let summary = self
            .indexes_count
            .map(|count| t!("MongoCollection.index_count", count = count).to_string());

        let list_body = if self.indexes_loading {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new())
                .into_any_element()
        } else if let Some(error) = &self.indexes_error {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().danger)
                .child(error.clone())
                .into_any_element()
        } else {
            div()
                .size_full()
                .child(Input::new(&self.indexes_output).h_full().disabled(true))
                .into_any_element()
        };

        v_flex()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Button::new("mongo-index-refresh")
                                    .small()
                                    .outline()
                                    .label(t!("MongoCollection.refresh_indexes").to_string())
                                    .disabled(self.indexes_loading)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.load_indexes(cx);
                                    })),
                            )
                            .when_some(summary, |this, text| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(text),
                                )
                            }),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("MongoCollection.create_index_label").to_string()),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(Input::new(&self.index_name_input).w(px(220.0)))
                                    .child(
                                        Button::new("mongo-index-create")
                                            .small()
                                            .primary()
                                            .label(t!("Common.create").to_string())
                                            .disabled(self.indexes_loading)
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.create_index(cx);
                                            })),
                                    ),
                            )
                            .child(Input::new(&self.index_keys_input).w_full()),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("MongoCollection.drop_index_label").to_string()),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(Input::new(&self.index_drop_input).w(px(220.0)))
                                    .child(
                                        Button::new("mongo-index-drop")
                                            .small()
                                            .outline()
                                            .label(t!("Common.delete").to_string())
                                            .disabled(self.indexes_loading)
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.drop_index(cx);
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(6.0))
                    .p_2()
                    .child(list_body),
            )
    }

    fn render_validation_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.validation_loading {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new())
                .into_any_element()
        } else if let Some(error) = &self.validation_error {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().danger)
                .child(error.clone())
                .into_any_element()
        } else {
            div()
                .size_full()
                .child(Input::new(&self.validation_input).h_full())
                .into_any_element()
        };

        v_flex()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("mongo-validation-refresh")
                            .small()
                            .outline()
                            .label(t!("MongoCollection.refresh_validation").to_string())
                            .disabled(self.validation_loading)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.load_validation(cx);
                            })),
                    )
                    .child(
                        Button::new("mongo-validation-save")
                            .small()
                            .primary()
                            .label(t!("MongoCollection.save_validation").to_string())
                            .disabled(self.validation_loading)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.save_validation(cx);
                            })),
                    )
                    .child(
                        Button::new("mongo-validation-clear")
                            .small()
                            .outline()
                            .label(t!("MongoCollection.clear_validation").to_string())
                            .disabled(self.validation_loading)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.clear_validation(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(6.0))
                    .p_2()
                    .child(body),
            )
    }

    fn render_empty_state(&self, message: &str, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(cx.theme().muted_foreground)
            .child(message.to_string())
            .into_any_element()
    }

    fn render_document_row(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(item) = self.documents.get(index).cloned() else {
            return div().into_any_element();
        };
        let is_selected = Some(index) == self.selected_index;
        let label = format!("{} {}", item.id, item.summary);

        h_flex()
            .id(SharedString::from(format!("mongo-doc-{}", index)))
            .w_full()
            .px_2()
            .py_1()
            .rounded(px(6.0))
            .group("mongo-doc-row")
            .cursor_pointer()
            .when(is_selected, |this| this.bg(cx.theme().list_active))
            .when(!is_selected, |this| this.text_color(cx.theme().foreground))
            .hover(|style| style.bg(cx.theme().list_active))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.select_document(index, window, cx);
            }))
            .child(div().flex_1().text_sm().truncate().child(label))
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .invisible()
                    .group_hover("mongo-doc-row", |style| style.visible())
                    .child(
                        Button::new(SharedString::from(format!("mongo-doc-edit-{}", index)))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Edit)
                            .tooltip(t!("Common.edit").to_string())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.select_document(index, window, cx);
                                this.start_update(window, cx);
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("mongo-doc-delete-{}", index)))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Remove)
                            .tooltip(t!("Common.delete").to_string())
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                cx.stop_propagation();
                                this.selected_index = Some(index);
                                this.delete_selected(cx);
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("mongo-doc-copy-{}", index)))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Copy)
                            .tooltip(t!("Common.copy").to_string())
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                cx.stop_propagation();
                                this.selected_index = Some(index);
                                this.copy_selected(cx);
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("mongo-doc-clone-{}", index)))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Plus)
                            .tooltip(t!("MongoCollection.clone").to_string())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.start_clone(index, window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    fn render_list_body(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.is_loading {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new())
                .into_any_element();
        }

        if let Some(error) = &self.error_message {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().danger)
                .child(error.clone())
                .into_any_element();
        }

        let item_count = self.documents.len();
        if item_count == 0 {
            return self.render_empty_state(t!("MongoCollection.no_documents").as_ref(), cx);
        }

        uniform_list(
            "mongo-documents-list",
            item_count,
            cx.processor(
                move |view: &mut Self, visible_range: std::ops::Range<usize>, window, cx| {
                    visible_range
                        .map(|index| view.render_document_row(index, window, cx))
                        .collect()
                },
            ),
        )
        .size_full()
        .track_scroll(&self.list_scroll_handle)
        .into_any_element()
    }

    fn render_document_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let total = self.total_count.unwrap_or(self.documents.len() as i64);
        let title = t!("MongoCollection.documents_title", total = total).to_string();

        v_flex()
            .flex_1()
            .h_full()
            .min_h_0()
            .min_w(px(280.0))
            .max_w(px(420.0))
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                h_flex().items_center().px_2().py_1().child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(cx.theme().foreground)
                        .child(title),
                ),
            )
            .child(
                div()
                    .id("mongo-document-list-scroll")
                    .flex_1()
                    .min_h_0()
                    .child(self.render_list_body(cx)),
            )
    }

    fn render_detail_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_editing = matches!(self.editor_mode, EditorMode::Create | EditorMode::Update);
        let header_title = if self.show_explain {
            t!("MongoCollection.explain_result_title").to_string()
        } else if is_editing {
            match self.editor_mode {
                EditorMode::Create => t!("MongoCollection.new_document").to_string(),
                EditorMode::Update => t!("MongoCollection.edit_document").to_string(),
                EditorMode::View => t!("MongoCollection.document_detail").to_string(),
            }
        } else {
            self.selected_index
                .and_then(|index| self.documents.get(index))
                .map(|item| t!("MongoCollection.document_with_id", id = item.id).to_string())
                .unwrap_or_else(|| t!("MongoCollection.select_document").to_string())
        };

        let body = if self.show_explain {
            div()
                .flex_1()
                .min_h_0()
                .child(Input::new(&self.explain_input).h_full().disabled(true))
                .into_any_element()
        } else if is_editing || self.selected_index.is_some() {
            div()
                .flex_1()
                .min_h_0()
                .child(
                    Input::new(&self.editor_input)
                        .h_full()
                        .disabled(!is_editing),
                )
                .into_any_element()
        } else {
            self.render_empty_state(t!("MongoCollection.select_document").as_ref(), cx)
        };

        v_flex()
            .flex_1()
            .h_full()
            .min_h_0()
            .min_w(px(320.0))
            .px_2()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(header_title),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .when(is_editing, |this| {
                                this.child(
                                    Button::new("mongo-save")
                                        .small()
                                        .primary()
                                        .label(t!("Common.save").to_string())
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            this.save_edit(cx);
                                        })),
                                )
                                .child(
                                    Button::new("mongo-cancel")
                                        .small()
                                        .outline()
                                        .label(t!("Common.cancel").to_string())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.cancel_edit(window, cx);
                                        })),
                                )
                            })
                            .when(!is_editing, |this| {
                                this.child(
                                    Button::new("mongo-copy")
                                        .small()
                                        .outline()
                                        .icon(IconName::Copy)
                                        .label(t!("Common.copy").to_string())
                                        .disabled(self.selected_index.is_none())
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            this.copy_selected(cx);
                                        })),
                                )
                            }),
                    ),
            )
            .child(body)
    }

    fn render_pagination_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let inputs = self.read_query_inputs_or_default(cx);
        let page_size = inputs.page_size;
        let skip_base = inputs.skip_base;
        let total_count = self.total_count.unwrap_or(0);
        let current_offset = inputs.skip(self.page_index);
        let current_count = self.documents.len() as i64;
        let start_index = if current_count == 0 {
            0
        } else {
            current_offset + 1
        };
        let end_index = current_offset + current_count;
        let remaining_total = total_count.saturating_sub(skip_base);
        let total_pages = if remaining_total == 0 {
            0
        } else {
            (remaining_total.saturating_sub(1) / page_size) + 1
        };
        let current_page_display = if total_pages == 0 {
            0
        } else {
            self.page_index + 1
        };

        let can_prev = self.page_index > 0;
        let can_next = if self.total_count.is_some() {
            total_pages > 0 && self.page_index + 1 < total_pages
        } else {
            current_count == page_size
        };

        h_flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "{}",
                        t!(
                            "MongoCollection.pagination_summary",
                            start = start_index,
                            end = end_index,
                            total = if self.total_count.is_some() {
                                total_count.to_string()
                            } else {
                                "?".to_string()
                            }
                        )
                    )),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("mongo-prev")
                            .xsmall()
                            .outline()
                            .icon(IconName::ChevronLeft)
                            .disabled(!can_prev)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                if this.page_index > 0 {
                                    this.page_index -= 1;
                                    this.reload(cx);
                                }
                            })),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                t!(
                                    "MongoCollection.page_summary",
                                    current = current_page_display,
                                    total = if total_pages == 0 {
                                        "?".to_string()
                                    } else {
                                        total_pages.to_string()
                                    }
                                )
                                .to_string(),
                            ),
                    )
                    .child(
                        Button::new("mongo-next")
                            .xsmall()
                            .outline()
                            .icon(IconName::ChevronRight)
                            .disabled(!can_next)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.page_index += 1;
                                this.reload(cx);
                            })),
                    ),
            )
    }
}

impl Focusable for CollectionView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<TabContentEvent> for CollectionView {}

impl TabContent for CollectionView {
    fn content_key(&self) -> &'static str {
        "MongoDocuments"
    }

    fn title(&self, _cx: &App) -> SharedString {
        t!("MongoCollection.tab_documents").to_string().into()
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::Table).color().with_size(Size::Medium))
    }

    fn closeable(&self, _cx: &App) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct CollectionTabConfig {
    pub connection_id: String,
    pub database_name: String,
    pub collection_name: String,
}

pub struct CollectionTabView {
    config: CollectionTabConfig,
    collection_view: Entity<CollectionView>,
}

impl CollectionTabView {
    pub fn new(config: CollectionTabConfig, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let collection_view = cx.new(|cx| CollectionView::new(window, cx));
        collection_view.update(cx, |view, cx| {
            view.load_collection(
                config.connection_id.clone(),
                config.database_name.clone(),
                config.collection_name.clone(),
                cx,
            );
        });

        Self {
            config,
            collection_view,
        }
    }
}

impl Focusable for CollectionTabView {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.collection_view.read(cx).focus_handle(cx)
    }
}

impl EventEmitter<TabContentEvent> for CollectionTabView {}

impl TabContent for CollectionTabView {
    fn content_key(&self) -> &'static str {
        "MongoCollection"
    }

    fn title(&self, _cx: &App) -> SharedString {
        format!(
            "{}/{}",
            self.config.database_name, self.config.collection_name
        )
        .into()
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::Table).color().with_size(Size::Medium))
    }

    fn closeable(&self, _cx: &App) -> bool {
        true
    }
}

impl Render for CollectionTabView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.collection_view.clone()
    }
}

impl Render for CollectionView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_pending_editor_value(window, cx);
        self.apply_pending_explain_value(window, cx);
        self.apply_pending_aggregation_value(window, cx);
        self.apply_pending_schema_value(window, cx);
        self.apply_pending_indexes_value(window, cx);
        self.apply_pending_validation_value(window, cx);
        self.apply_pending_reload(cx);

        let body = if self.collection_name.is_none() {
            self.render_empty_state(t!("MongoCollection.select_collection_prompt").as_ref(), cx)
                .into_any_element()
        } else {
            match self.active_tab {
                TAB_DOCUMENTS => self.render_documents_tab(cx).into_any_element(),
                TAB_AGGREGATIONS => self.render_aggregation_tab(cx).into_any_element(),
                TAB_SCHEMA => self.render_schema_tab(cx).into_any_element(),
                TAB_INDEXES => self.render_indexes_tab(cx).into_any_element(),
                TAB_VALIDATION => self.render_validation_tab(cx).into_any_element(),
                _ => self.render_documents_tab(cx).into_any_element(),
            }
        };

        let container = v_flex().size_full().p_2().gap_2();
        if self.collection_name.is_none() {
            container.child(body)
        } else {
            container
                .child(self.render_header(cx))
                .child(self.render_tab_bar(cx))
                .child(body)
        }
    }
}
