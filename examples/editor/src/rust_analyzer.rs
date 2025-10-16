/// A minimal Rust Analyzer LSP client for code completions
///
/// Ref: https://github.com/Far-Beyond-Pulsar/Pulsar-Native/blob/3135b87564966c377e3a1716056dd912febeef1d/crates/ui/src/input/lsp/rust_analyzer.rs#L1
use anyhow::{Result, anyhow};
use gpui::{App, Context, Entity, SharedString, Task, Window};
use gpui_component::Rope;
use gpui_component::input::{CodeActionProvider, CompletionProvider};
use lsp_types::{
    CodeAction, CompletionContext, CompletionItem, CompletionParams, CompletionResponse,
    Diagnostic, TextDocumentIdentifier, TextDocumentPositionParams, Uri, WorkDoneProgressParams,
};
use std::ops::Range;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};

use crate::input::{InputState, RopeExt};

const RUST_ANALYZER_DEFAULT_PATHS: &[&str] = &[
    "rust-analyzer",
    "rust-analyzer.exe",
    "~/.cargo/bin/rust-analyzer",
    "~/.cargo/bin/rust-analyzer.exe",
];

fn find_binary() -> Result<PathBuf> {
    for candidate in RUST_ANALYZER_DEFAULT_PATHS {
        if let Some(ok) = Command::new(candidate)
            .arg("-V")
            .output()
            .map(|o| o.status.success())
            .ok()
        {
            if ok {
                return Ok(PathBuf::from(candidate));
            }
        }
    }

    Err(anyhow!("`rust-analyzer` not found in PATH."))
}

pub struct RustAnalyzerClient {
    binary_path: PathBuf,
    work_dir: PathBuf,
    document_uri: Option<Uri>,

    process: Arc<Mutex<Option<Child>>>,
    _initialized: Arc<Mutex<bool>>,
}

impl RustAnalyzerClient {
    fn new(work_dir: PathBuf) -> Result<Self> {
        let binary_path = find_binary()?;
        Ok(Self {
            binary_path,
            work_dir,
            document_uri: None,
            process: Arc::new(Mutex::new(None)),
            _initialized: Arc::new(Mutex::new(false)),
        })
    }

    pub fn initialize(&self) -> Result<()> {
        let mut lock = self
            .process
            .lock()
            .map_err(|e| anyhow!("Lock error: {}", e))?;

        if lock.is_some() {
            return Ok(());
        }

        // Spawn rust-analyzer as child process.
        let child = Command::new(&self.binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        *lock = Some(child);

        let mut init_lock = self
            ._initialized
            .lock()
            .map_err(|e| anyhow!("Lock error: {}", e))?;
        *init_lock = true;

        Ok(())
    }

    pub fn shutdown(&self) -> Result<()> {
        let mut process_lock = self
            .process
            .lock()
            .map_err(|e| anyhow!("Lock error: {}", e))?;

        if let Some(mut child) = process_lock.take() {
            child.kill()?;
            child.wait()?;
        }

        let mut init_lock = self
            ._initialized
            .lock()
            .map_err(|e| anyhow!("Lock error: {}", e))?;
        *init_lock = false;

        Ok(())
    }
}

pub struct RustAnalyzerLspProvider {
    client: Arc<Mutex<RustAnalyzerClient>>,

    code_actions: Arc<RwLock<Vec<(Range<usize>, CodeAction)>>>,
    diagnostics: Arc<RwLock<Vec<Diagnostic>>>,
}

impl RustAnalyzerLspProvider {
    /// Create a new Rust Analyzer completion provider
    pub fn new(work_dir: PathBuf) -> Result<Self> {
        let client = RustAnalyzerClient::new(work_dir)?;
        client.initialize()?;

        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            code_actions: Arc::new(RwLock::new(vec![])),
            diagnostics: Arc::new(RwLock::new(vec![])),
        })
    }

    /// Set the file being edited
    pub fn open_file(&self, file: PathBuf) -> Result<()> {
        let mut client = self
            .client
            .lock()
            .map_err(|e| anyhow!("Lock error: {}", e))?;

        let uri = Uri::from_str(&format!("file://{}", file.to_string_lossy()))?;
        client.document_uri = Some(uri);
        Ok(())
    }

    /// Get completions from rust-analyzer
    fn _completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
    ) -> Result<CompletionResponse> {
        let client = self
            .client
            .lock()
            .map_err(|e| anyhow!("Lock error: {}", e))?;

        // Check if initialized
        let initialized = client
            ._initialized
            .lock()
            .map_err(|e| anyhow!("Lock error: {}", e))?;
        if !*initialized {
            return Ok(CompletionResponse::Array(vec![]));
        }

        let Some(uri) = client.document_uri.clone() else {
            return Ok(CompletionResponse::Array(vec![]));
        };

        // Convert byte offset to LSP position
        let position = text.offset_to_position(offset);

        // Create completion params
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: Default::default(),
            context: None,
        };

        // Send LSP request to rust-analyzer
        let mut lock = client
            .process
            .lock()
            .map_err(|e| anyhow!("Lock error: {}", e))?;

        if let Some(child) = lock.as_mut() {
            let payload = serde_json::to_value(params)?;
            match lsp_client::request(child, "textDocument/completion", payload) {
                Ok(response) => {
                    if let Ok(items) =
                        serde_json::from_value::<Vec<CompletionItem>>(response["result"].clone())
                    {
                        return Ok(CompletionResponse::Array(items));
                    } else if let Ok(list) = serde_json::from_value::<lsp_types::CompletionList>(
                        response["result"].clone(),
                    ) {
                        return Ok(CompletionResponse::List(list));
                    }
                }
                Err(e) => {
                    eprintln!("rust-analyzer completion error: {}", e);
                }
            }
        }

        Ok(CompletionResponse::Array(vec![]))
    }
}

impl CompletionProvider for RustAnalyzerLspProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let text = text.clone();
        let client = self.client.clone();
        let offset_copy = offset; // Copy primitive to move
        let trigger_copy = trigger.clone(); // Clone to move

        cx.spawn_in(window, async move |_, _cx| {
            let provider = Self {
                client,
                code_actions: Arc::new(RwLock::new(vec![])),
                diagnostics: Arc::new(RwLock::new(vec![])),
            };
            provider._completions(&text, offset_copy, trigger_copy)
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        // Trigger on:
        // 1. Dot (method completion)
        // 2. Double colon (path completion)
        // 3. Alphanumeric (word completion)
        new_text.contains('.')
            || new_text.contains("::")
            || new_text.chars().any(|c| c.is_alphanumeric())
    }
}

impl CodeActionProvider for RustAnalyzerLspProvider {
    fn id(&self) -> SharedString {
        "rust-analyzer".into()
    }

    fn code_actions(
        &self,
        _state: Entity<InputState>,
        range: Range<usize>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>> {
        let mut actions = vec![];
        let code_actions = self.code_actions.read().unwrap();
        for (node_range, code_action) in code_actions.iter() {
            if !(range.start >= node_range.start && range.end <= node_range.end) {
                continue;
            }

            actions.push(code_action.clone());
        }

        Task::ready(Ok(actions))
    }

    fn perform_code_action(
        &self,
        state: Entity<InputState>,
        action: CodeAction,
        _push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>> {
        let Some(edit) = action.edit else {
            return Task::ready(Ok(()));
        };

        let changes = if let Some(changes) = edit.changes {
            changes
        } else {
            return Task::ready(Ok(()));
        };

        let Some((_, text_edits)) = changes.into_iter().next() else {
            return Task::ready(Ok(()));
        };

        let state = state.downgrade();
        window.spawn(cx, async move |cx| {
            state.update_in(cx, |state, window, cx| {
                state.apply_lsp_edits(&text_edits, window, cx);
            })
        })
    }
}

impl Drop for RustAnalyzerClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Minimal LSP client implementation for communicating with rust-analyzer
mod lsp_client {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    pub fn request(
        child: &mut Child,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("Failed to get stdin"))?;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let request_body = serde_json::to_string(&request)?;
        let content_length = request_body.len();

        write!(stdin, "Content-Length: {}\r\n\r\n", content_length)?;
        write!(stdin, "{}", request_body)?;
        stdin.flush()?;

        let stdout = child
            .stdout
            .as_mut()
            .ok_or_else(|| anyhow!("Failed to get stdout"))?;
        let mut reader = BufReader::new(stdout);

        let mut headers = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            if line == "\r\n" {
                break;
            }
            headers.push_str(&line);
        }

        let content_length: usize = headers
            .lines()
            .find(|line| line.starts_with("Content-Length:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|len| len.trim().parse().ok())
            .ok_or_else(|| anyhow!("Missing Content-Length header"))?;

        let mut content = vec![0u8; content_length];
        std::io::Read::read_exact(&mut reader, &mut content)?;

        let response: serde_json::Value = serde_json::from_slice(&content)?;
        Ok(response)
    }
}
