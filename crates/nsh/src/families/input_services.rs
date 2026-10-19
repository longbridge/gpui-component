//! Editor language requests stay on the owning NSH runtime. Pending receivers
//! cancel when their native owner unmounts; providers never retain that owner.
use super::EditorEvents;
use anyhow::{Result, anyhow};
use futures::channel::oneshot;
use gpui_kit::component::input::{
    CodeActionProvider, CompletionProvider, DefinitionProvider, DocumentColorProvider,
    DocumentRangeSemanticTokensProvider, EditorState, HoverProvider, Rope,
};
use gpui_kit::{App, Entity, SharedString, Task, Window};
use lsp_types::{
    CodeAction, ColorInformation, CompletionContext, CompletionResponse, Hover,
    InlineCompletionContext, InlineCompletionResponse, LocationLink, SemanticTokens,
    SemanticTokensLegend,
};
use nsh_core::ui::{JsHost, JsIn, JsObject, JsOwned, JsValue, PropEnum, PropError, TsTypes};
use nsh_platform_gpui::TypedComponentContext;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    ops::Range,
    rc::{Rc, Weak},
};

type Reply = oneshot::Sender<Result<Value>>;
#[derive(Default)]
pub(super) struct Requests {
    next: u64,
    pending: HashMap<u64, Reply>,
}
impl Requests {
    pub(super) fn resolve(&mut self, args: &Value) {
        let Some(id) = args.get("id").and_then(Value::as_u64) else {
            return;
        };
        if let Some(sender) = self.pending.remove(&id) {
            let result = match args.get("error").and_then(Value::as_str) {
                Some(error) => Err(anyhow!(error.to_owned())),
                None => Ok(args.get("result").cloned().unwrap_or(Value::Null)),
            };
            let _ = sender.send(result);
        }
    }
}
#[derive(Clone, Default, PartialEq, JsObject)]
pub(super) struct EditorLanguageServices {
    #[prop(default)]
    completion: bool,
    #[prop(default)]
    hover: bool,
    #[prop(default)]
    definition: bool,
    #[prop(default)]
    code_action: bool,
    #[prop(default)]
    document_color: bool,
    semantic_tokens: Option<EditorSemanticTokensLegend>,
    #[prop(default)]
    trigger_characters: Vec<String>,
    #[prop(default)]
    show_document: bool,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct EditorSemanticTokensLegend {
    token_types: Vec<String>,
    token_modifiers: Vec<String>,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum EditorRequestMethod {
    #[prop(rename = "textDocument/completion")]
    Completion,
    #[prop(rename = "textDocument/inlineCompletion")]
    InlineCompletion,
    #[prop(rename = "textDocument/hover")]
    Hover,
    #[prop(rename = "textDocument/definition")]
    Definition,
    #[prop(rename = "textDocument/documentColor")]
    DocumentColor,
    #[prop(rename = "textDocument/semanticTokens/range")]
    SemanticTokens,
    #[prop(rename = "textDocument/codeAction")]
    CodeAction,
    #[prop(rename = "codeAction/perform")]
    PerformCodeAction,
}
#[derive(JsObject)]
pub(crate) struct EditorRequest {
    id: u64,
    method: EditorRequestMethod,
    params: EditorRequestParams,
}
#[derive(Default, JsObject)]
pub(crate) struct EditorRequestParams {
    text: String,
    offset: Option<usize>,
    range: Option<EditorByteRange>,
    context: Option<EditorCompletionContext>,
    action: Option<EditorCodeActionValue>,
    push_to_history: Option<bool>,
}
#[derive(JsObject)]
pub(crate) struct EditorByteRange {
    start: usize,
    end: usize,
}
#[derive(JsObject, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditorCompletionContext {
    trigger_kind: u32,
    trigger_character: Option<String>,
    selected_completion_info: Option<EditorSelectedCompletion>,
}
#[derive(JsObject, Deserialize)]
pub(crate) struct EditorSelectedCompletion {
    range: super::EditorRange,
    text: String,
}
#[derive(JsObject, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditorShowDocument {
    uri: String,
    external: Option<bool>,
    take_focus: Option<bool>,
    selection: Option<super::EditorRange>,
}
// LSP owns these protocol types. The codec forwards the original CodeAction
// losslessly; the JS schema keeps every standardized field precise. Only data
// and command arguments are arbitrary JSON, as required by the LSP protocol.
#[derive(JsObject)]
pub(crate) struct EditorCodeAction {
    title: String,
    kind: Option<String>,
    diagnostics: Option<Vec<EditorLspDiagnostic>>,
    is_preferred: Option<bool>,
    disabled: Option<EditorActionDisabled>,
    edit: Option<EditorWorkspaceEdit>,
    command: Option<EditorCommand>,
    data: Option<Value>,
}
#[derive(JsObject)]
pub(crate) struct EditorActionDisabled {
    reason: String,
}
#[derive(JsObject)]
pub(crate) struct EditorCommand {
    title: String,
    command: String,
    arguments: Option<Vec<Value>>,
}
#[derive(JsObject)]
pub(crate) struct EditorLspDiagnostic {
    range: super::EditorRange,
    severity: Option<u32>,
    code: Option<EditorDiagnosticCode>,
    code_description: Option<EditorCodeDescription>,
    source: Option<String>,
    message: String,
    tags: Option<Vec<u32>>,
    related_information: Option<Vec<EditorDiagnosticRelated>>,
    data: Option<Value>,
}
#[derive(JsObject)]
pub(crate) struct EditorCodeDescription {
    href: String,
}
#[derive(JsObject)]
pub(crate) struct EditorDiagnosticRelated {
    location: EditorLspLocation,
    message: String,
}
#[derive(JsObject)]
pub(crate) struct EditorLspLocation {
    uri: String,
    range: super::EditorRange,
}
#[derive(JsObject)]
pub(crate) struct EditorWorkspaceEdit {
    changes: Option<BTreeMap<String, Vec<EditorTextEdit>>>,
    document_changes: Option<Vec<EditorDocumentChange>>,
    change_annotations: Option<BTreeMap<String, EditorChangeAnnotation>>,
}
#[derive(JsObject)]
pub(crate) struct EditorTextEdit {
    range: super::EditorRange,
    new_text: String,
    annotation_id: Option<String>,
}
#[derive(JsObject)]
pub(crate) struct EditorChangeAnnotation {
    label: String,
    needs_confirmation: Option<bool>,
    description: Option<String>,
}
#[derive(JsObject)]
pub(crate) struct EditorDocumentChange {
    text_document: Option<EditorDocumentIdentifier>,
    edits: Option<Vec<EditorTextEdit>>,
    kind: Option<String>,
    uri: Option<String>,
    old_uri: Option<String>,
    new_uri: Option<String>,
    options: Option<EditorResourceOptions>,
    annotation_id: Option<String>,
}
#[derive(JsObject)]
pub(crate) struct EditorDocumentIdentifier {
    uri: String,
    version: EditorDocumentVersion,
}
#[derive(JsObject)]
pub(crate) struct EditorResourceOptions {
    overwrite: Option<bool>,
    ignore_if_exists: Option<bool>,
    recursive: Option<bool>,
    ignore_if_not_exists: Option<bool>,
}
struct EditorCodeActionValue(CodeAction);
impl JsValue for EditorCodeActionValue {
    fn ts_type(types: &mut TsTypes) -> String {
        EditorCodeAction::ts_type(types)
    }
    fn from_js(value: JsIn<'_>) -> std::result::Result<Self, PropError> {
        serde_json::from_value(Value::from_js(value)?)
            .map(Self)
            .map_err(|error| PropError(error.to_string()))
    }
    fn to_js<'a>(&self, host: &'a dyn JsHost) -> std::result::Result<JsOwned<'a>, PropError> {
        serde_json::to_value(&self.0)
            .map_err(|error| PropError(error.to_string()))?
            .to_js(host)
    }
}
enum EditorDiagnosticCode {
    Number(i32),
    Text(String),
}
impl JsValue for EditorDiagnosticCode {
    fn ts_type(types: &mut TsTypes) -> String {
        format!("{} | {}", i32::ts_type(types), String::ts_type(types))
    }
    fn from_js(value: JsIn<'_>) -> std::result::Result<Self, PropError> {
        if value.as_number().is_some() {
            i32::from_js(value).map(Self::Number)
        } else {
            String::from_js(value).map(Self::Text)
        }
    }
    fn to_js<'a>(&self, host: &'a dyn JsHost) -> std::result::Result<JsOwned<'a>, PropError> {
        match self {
            Self::Number(value) => value.to_js(host),
            Self::Text(value) => value.to_js(host),
        }
    }
}
struct EditorDocumentVersion(Option<i32>);
impl JsValue for EditorDocumentVersion {
    fn ts_type(types: &mut TsTypes) -> String {
        format!("{} | null", i32::ts_type(types))
    }
    fn from_js(value: JsIn<'_>) -> std::result::Result<Self, PropError> {
        Option::<i32>::from_js(value).map(Self)
    }
    fn to_js<'a>(&self, host: &'a dyn JsHost) -> std::result::Result<JsOwned<'a>, PropError> {
        match self.0 {
            Some(value) => value.to_js(host),
            None => Ok(JsOwned::new(host, host.null())),
        }
    }
}

struct Services {
    context: TypedComponentContext<EditorEvents>,
    requests: Weak<RefCell<Requests>>,
    capabilities: EditorLanguageServices,
}
impl Services {
    fn request<T: DeserializeOwned + 'static>(
        &self,
        method: EditorRequestMethod,
        params: EditorRequestParams,
        cx: &mut App,
    ) -> Task<Result<T>> {
        let Some(requests) = self.requests.upgrade() else {
            return Task::ready(Err(anyhow!("editor has unmounted")));
        };
        let (sender, receiver) = oneshot::channel();
        let id = {
            let mut requests = requests.borrow_mut();
            requests.next = requests
                .next
                .checked_add(1)
                .expect("editor request identifier exhausted");
            let id = requests.next;
            requests.pending.insert(id, sender);
            id
        };
        self.context
            .emit(EditorEvents::Request(Box::new(EditorRequest {
                id,
                method,
                params,
            })));
        // Guard removes cancelled/completed requests, including when GPUI drops
        // a superseded completion or hover task before JS responds.
        let guard = Pending {
            requests: Rc::downgrade(&requests),
            id,
        };
        cx.spawn(async move |_| {
            let _guard = guard;
            let value = receiver
                .await
                .map_err(|_| anyhow!("editor language request cancelled"))??;
            serde_json::from_value(value).map_err(Into::into)
        })
    }
}
struct Pending {
    requests: Weak<RefCell<Requests>>,
    id: u64,
}
impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(requests) = self.requests.upgrade() {
            requests.borrow_mut().pending.remove(&self.id);
        }
    }
}
pub(super) fn configure(
    state: &mut EditorState,
    value: Option<&EditorLanguageServices>,
    context: &TypedComponentContext<EditorEvents>,
    requests: &Rc<RefCell<Requests>>,
    cx: &mut gpui_kit::Context<EditorState>,
) {
    let capabilities = value.cloned().unwrap_or_default();
    requests.borrow_mut().pending.clear();
    let services = Rc::new(Services {
        context: context.clone(),
        requests: Rc::downgrade(requests),
        capabilities,
    });
    let lsp = state.lsp_mut();
    lsp.show_document = if services.capabilities.show_document {
        let context = context.clone();
        Some(Rc::new(move |params, _, _| {
            match serde_json::from_value(
                serde_json::to_value(params).expect("serializable LSP showDocument"),
            ) {
                Ok(params) => context.emit(EditorEvents::ShowDocument(params)),
                Err(error) => context.emit(EditorEvents::Error(error.to_string())),
            }
            true
        }))
    } else {
        None
    };
    lsp.completion_provider = services
        .capabilities
        .completion
        .then(|| services.clone() as Rc<dyn CompletionProvider>);
    lsp.hover_provider = services
        .capabilities
        .hover
        .then(|| services.clone() as Rc<dyn HoverProvider>);
    lsp.definition_provider = services
        .capabilities
        .definition
        .then(|| services.clone() as Rc<dyn DefinitionProvider>);
    lsp.document_color_provider = services
        .capabilities
        .document_color
        .then(|| services.clone() as Rc<dyn DocumentColorProvider>);
    lsp.semantic_tokens_provider = services
        .capabilities
        .semantic_tokens
        .is_some()
        .then(|| services.clone() as Rc<dyn DocumentRangeSemanticTokensProvider>);
    lsp.code_action_providers = if services.capabilities.code_action {
        vec![services]
    } else {
        vec![]
    };
    cx.notify();
}
impl CompletionProvider for Services {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<CompletionResponse>> {
        self.request(
            EditorRequestMethod::Completion,
            EditorRequestParams {
                text: text.to_string(),
                offset: Some(offset),
                context: Some(
                    serde_json::from_value(
                        serde_json::to_value(trigger).expect("LSP completion context"),
                    )
                    .expect("typed completion context"),
                ),
                ..Default::default()
            },
            cx,
        )
    }
    fn inline_completion(
        &self,
        text: &Rope,
        offset: usize,
        trigger: InlineCompletionContext,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<InlineCompletionResponse>> {
        self.request(
            EditorRequestMethod::InlineCompletion,
            EditorRequestParams {
                text: text.to_string(),
                offset: Some(offset),
                context: Some(
                    serde_json::from_value(
                        serde_json::to_value(trigger).expect("LSP inline context"),
                    )
                    .expect("typed inline context"),
                ),
                ..Default::default()
            },
            cx,
        )
    }
    fn is_completion_trigger(&self, _: usize, text: &str, _: &mut App) -> bool {
        self.capabilities
            .trigger_characters
            .iter()
            .any(|trigger| text.ends_with(trigger))
            || text
                .chars()
                .last()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
    }
}
impl HoverProvider for Services {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Option<Hover>>> {
        self.request(
            EditorRequestMethod::Hover,
            EditorRequestParams {
                text: text.to_string(),
                offset: Some(offset),
                ..Default::default()
            },
            cx,
        )
    }
}
impl DefinitionProvider for Services {
    fn definitions(
        &self,
        text: &Rope,
        offset: usize,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<LocationLink>>> {
        self.request(
            EditorRequestMethod::Definition,
            EditorRequestParams {
                text: text.to_string(),
                offset: Some(offset),
                ..Default::default()
            },
            cx,
        )
    }
}
impl DocumentColorProvider for Services {
    fn document_colors(
        &self,
        text: &Rope,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<ColorInformation>>> {
        self.request(
            EditorRequestMethod::DocumentColor,
            EditorRequestParams {
                text: text.to_string(),
                ..Default::default()
            },
            cx,
        )
    }
}
impl DocumentRangeSemanticTokensProvider for Services {
    fn legend(&self) -> SemanticTokensLegend {
        self.capabilities
            .semantic_tokens
            .as_ref()
            .map(|legend| SemanticTokensLegend {
                token_types: legend.token_types.iter().cloned().map(Into::into).collect(),
                token_modifiers: legend
                    .token_modifiers
                    .iter()
                    .cloned()
                    .map(Into::into)
                    .collect(),
            })
            .unwrap_or_default()
    }
    fn semantic_tokens(
        &self,
        text: &Rope,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<SemanticTokens>> {
        self.request(
            EditorRequestMethod::SemanticTokens,
            EditorRequestParams {
                text: text.to_string(),
                range: Some(EditorByteRange {
                    start: range.start,
                    end: range.end,
                }),
                ..Default::default()
            },
            cx,
        )
    }
}
impl CodeActionProvider for Services {
    fn id(&self) -> SharedString {
        "nsh".into()
    }
    fn code_actions(
        &self,
        state: Entity<EditorState>,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>> {
        self.request(
            EditorRequestMethod::CodeAction,
            EditorRequestParams {
                text: state.read(cx).value().to_string(),
                range: Some(EditorByteRange {
                    start: range.start,
                    end: range.end,
                }),
                ..Default::default()
            },
            cx,
        )
    }
    fn perform_code_action(
        &self,
        state: Entity<EditorState>,
        action: CodeAction,
        push_to_history: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>> {
        self.request(
            EditorRequestMethod::PerformCodeAction,
            EditorRequestParams {
                text: state.read(cx).value().to_string(),
                action: Some(EditorCodeActionValue(action)),
                push_to_history: Some(push_to_history),
                ..Default::default()
            },
            cx,
        )
    }
}
