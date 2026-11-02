use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use lsp_server::{Connection, Message, Notification};
use lsp_types::{
    CompletionItemKind, Diagnostic as LspDiagnostic, DiagnosticSeverity, NumberOrString, Range,
    ServerCapabilities, TextDocumentSyncKind, TextDocumentSyncOptions,
};
use schemra_compiler::{
    CompletionKind, EditorSymbol, EditorSymbolKind, EditorSymbolRole, ProjectAnalysis,
    ProjectSource, SyntaxTokenKind, analyze_project, highlight_tokens,
};
use schemra_project::read_sources;
use schemra_syntax::{Diagnostic, Severity};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use url::Url;

use crate::positions::range_for_bytes;
use crate::validation::diagnostics_for_file;

#[derive(Clone)]
struct OpenDocument {
    path: Option<PathBuf>,
    root: Option<PathBuf>,
    version: i32,
    source: String,
}

struct CachedProject {
    sources: Vec<ProjectSource>,
    analysis: Arc<ProjectAnalysis>,
}

mod requests;

pub(crate) struct Backend {
    workspace_roots: Vec<PathBuf>,
    documents: HashMap<String, OpenDocument>,
    projects: HashMap<Option<PathBuf>, CachedProject>,
}

impl Backend {
    #[allow(deprecated)]
    pub(crate) fn new(params: &lsp_types::InitializeParams) -> Self {
        let mut roots = params
            .workspace_folders
            .as_ref()
            .into_iter()
            .flatten()
            .filter_map(|folder| uri_to_path(&folder.uri))
            .collect::<Vec<_>>();
        if roots.is_empty() {
            if let Some(root_uri) = &params.root_uri {
                roots.extend(uri_to_path(root_uri));
            }
        }
        if roots.is_empty() {
            roots.extend(params.root_path.as_ref().map(PathBuf::from));
        }

        roots = roots
            .into_iter()
            .map(|root| normalize_path(&root))
            .collect();
        roots.sort();
        roots.dedup();

        Self {
            workspace_roots: roots,
            documents: HashMap::new(),
            projects: HashMap::new(),
        }
    }

    pub(crate) fn capabilities() -> ServerCapabilities {
        ServerCapabilities {
            text_document_sync: Some(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::FULL),
                    ..Default::default()
                }
                .into(),
            ),
            completion_provider: Some(Default::default()),
            definition_provider: Some(lsp_types::OneOf::Left(true)),
            document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
            hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
            rename_provider: Some(lsp_types::OneOf::Right(lsp_types::RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: Default::default(),
            })),
            code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
            ..Default::default()
        }
    }

    pub(crate) fn handle_notification(
        &mut self,
        connection: &Connection,
        notification: Notification,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match notification.method.as_str() {
            "textDocument/didOpen" => {
                let Some(params) = decode_notification::<lsp_types::DidOpenTextDocumentParams>(
                    notification.params,
                    &notification.method,
                ) else {
                    return Ok(());
                };
                let document = params.text_document;
                let uri = document.uri.as_str().to_owned();
                let path = uri_to_path(&document.uri).map(|path| normalize_path(&path));
                let root = path.as_deref().and_then(|path| self.root_for_path(path));
                self.documents.insert(
                    uri,
                    OpenDocument {
                        path,
                        root: root.clone(),
                        version: document.version,
                        source: document.text,
                    },
                );
                self.publish_diagnostics(connection, root.as_deref())?;
            }
            "textDocument/didChange" => {
                let Some(params) = decode_notification::<lsp_types::DidChangeTextDocumentParams>(
                    notification.params,
                    &notification.method,
                ) else {
                    return Ok(());
                };
                let uri = params.text_document.uri.as_str();
                let root = if let Some(document) = self.documents.get_mut(uri) {
                    if let Some(change) = params.content_changes.into_iter().last() {
                        document.source = change.text;
                        document.version = params.text_document.version;
                    }
                    document.root.clone()
                } else {
                    None
                };
                self.publish_diagnostics(connection, root.as_deref())?;
            }
            "textDocument/didClose" => {
                let Some(params) = decode_notification::<lsp_types::DidCloseTextDocumentParams>(
                    notification.params,
                    &notification.method,
                ) else {
                    return Ok(());
                };
                let uri = params.text_document.uri.as_str().to_owned();
                let root = self
                    .documents
                    .remove(&uri)
                    .and_then(|document| document.root);
                connection
                    .sender
                    .send(Message::Notification(Notification::new(
                        "textDocument/publishDiagnostics".into(),
                        json!({"uri": uri, "diagnostics": []}),
                    )))?;
                self.publish_diagnostics(connection, root.as_deref())?;
            }
            _ => {}
        }
        Ok(())
    }

    fn publish_diagnostics(
        &mut self,
        connection: &Connection,
        root: Option<&Path>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let analysis = self.ensure_project(root);
        for (uri, document) in self
            .documents
            .iter()
            .filter(|(_, document)| document.root.as_deref() == root)
        {
            let diagnostics = match &analysis {
                Ok(()) => self
                    .projects
                    .get(&root.map(Path::to_path_buf))
                    .and_then(|project| {
                        let path = self.document_project_path(document, root)?;
                        let file = project
                            .analysis
                            .files
                            .iter()
                            .find(|file| file.path == path)?;
                        let diagnostics = diagnostics_for_file(&project.analysis, &file.path);
                        Some(convert_diagnostics(&document.source, &diagnostics))
                    })
                    .unwrap_or_default(),
                Err(error) => vec![project_resolution_diagnostic(error)],
            };
            connection
                .sender
                .send(Message::Notification(Notification::new(
                    "textDocument/publishDiagnostics".into(),
                    json!({
                        "uri": uri,
                        "version": document.version,
                        "diagnostics": diagnostics,
                    }),
                )))?;
        }
        Ok(())
    }

    fn ensure_project(&mut self, root: Option<&Path>) -> Result<(), String> {
        let key = root.map(Path::to_path_buf);
        let mut sources = BTreeMap::new();
        if let Some(root) = root {
            for source in read_sources(root).map_err(|error| error.to_string())? {
                sources.insert(source.path.clone(), source);
            }
        }
        for document in self
            .documents
            .values()
            .filter(|document| document.root.as_deref() == root)
        {
            let Some(path) = self.document_project_path(document, root) else {
                continue;
            };
            if path.to_ascii_lowercase().ends_with(".schemra") {
                sources.insert(
                    path.clone(),
                    ProjectSource {
                        id: path.clone(),
                        path,
                        source: document.source.clone(),
                    },
                );
            }
        }
        let sources = sources.into_values().collect::<Vec<_>>();
        if self
            .projects
            .get(&key)
            .is_some_and(|cached| same_sources(&cached.sources, &sources))
        {
            return Ok(());
        }
        let analysis = Arc::new(analyze_project(&sources));
        self.projects
            .insert(key, CachedProject { sources, analysis });
        Ok(())
    }

    fn document_project_path(
        &self,
        document: &OpenDocument,
        root: Option<&Path>,
    ) -> Option<String> {
        let path = document.path.as_deref()?;
        let relative = if let Some(root) = root {
            path.strip_prefix(root).ok()?
        } else {
            Path::new(path.file_name()?.to_str()?)
        };
        relative.to_str().map(|path| path.replace('\\', "/"))
    }

    fn root_for_path(&self, path: &Path) -> Option<PathBuf> {
        self.workspace_roots
            .iter()
            .filter(|root| path.starts_with(root))
            .max_by_key(|root| root.components().count())
            .cloned()
            .or_else(|| path.parent().map(Path::to_path_buf))
    }
}

fn decode_notification<T: DeserializeOwned>(params: Value, method: &str) -> Option<T> {
    serde_json::from_value(params).map_or_else(
        |error| {
            eprintln!("Schemra LSP ignored malformed {method} notification: {error}");
            None
        },
        Some,
    )
}

fn same_sources(left: &[ProjectSource], right: &[ProjectSource]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.path == right.path && left.source == right.source)
}

fn has_errors(analysis: &ProjectAnalysis) -> bool {
    analysis.files.iter().any(|file| {
        file.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    })
}

fn error_counts(analysis: &ProjectAnalysis) -> HashMap<(String, String, String), usize> {
    let mut counts = HashMap::new();
    for file in &analysis.files {
        for diagnostic in file
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
        {
            *counts
                .entry((
                    file.path.clone(),
                    diagnostic.code.clone(),
                    diagnostic.message.clone(),
                ))
                .or_insert(0) += 1;
        }
    }
    counts
}

fn edit_distance(left: &str, right: &str) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    for (index, left_byte) in left.bytes().enumerate() {
        let mut current = vec![index + 1; right.len() + 1];
        for (other_index, right_byte) in right.bytes().enumerate() {
            current[other_index + 1] = (previous[other_index + 1] + 1)
                .min(current[other_index] + 1)
                .min(previous[other_index] + usize::from(left_byte != right_byte));
        }
        previous = current;
    }
    previous[right.len()]
}

fn valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    name.len() <= 32
        && chars
            .next()
            .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn project_resolution_diagnostic(message: &str) -> LspDiagnostic {
    LspDiagnostic {
        range: Range::default(),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("E_PROJECT_RESOLUTION".into())),
        code_description: None,
        source: Some("schemra".into()),
        message: message.to_owned(),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn convert_diagnostics(source: &str, diagnostics: &[Diagnostic]) -> Vec<LspDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| LspDiagnostic {
            range: range_for_bytes(
                source,
                diagnostic.span.start.offset,
                diagnostic.span.end.offset,
            ),
            severity: Some(match diagnostic.severity {
                Severity::Error => DiagnosticSeverity::ERROR,
                Severity::Warning => DiagnosticSeverity::WARNING,
                Severity::Information => DiagnosticSeverity::INFORMATION,
            }),
            code: Some(NumberOrString::String(diagnostic.code.clone())),
            code_description: None,
            source: Some("schemra".into()),
            message: diagnostic.hint.as_ref().map_or_else(
                || diagnostic.message.clone(),
                |hint| format!("{}\nHint: {hint}", diagnostic.message),
            ),
            related_information: None,
            tags: None,
            data: None,
        })
        .collect()
}

fn symbol_at(symbols: &[EditorSymbol], offset: usize) -> Option<&EditorSymbol> {
    symbols
        .iter()
        .filter(|symbol| symbol.span.start.offset <= offset && offset < symbol.span.end.offset)
        .min_by_key(|symbol| symbol.span.end.offset - symbol.span.start.offset)
}

fn find_definition<'a>(
    analysis: &'a ProjectAnalysis,
    source_path: &str,
    symbol: &EditorSymbol,
) -> Option<(&'a schemra_compiler::ProjectFileAnalysis, &'a EditorSymbol)> {
    if symbol.role == EditorSymbolRole::Definition {
        if let Some(file) = analysis.files.iter().find(|file| file.path == source_path) {
            if let Some(definition) = file.editor_symbols.iter().find(|candidate| {
                candidate.role == EditorSymbolRole::Definition
                    && candidate.kind == symbol.kind
                    && candidate.target_key == symbol.target_key
            }) {
                return Some((file, definition));
            }
        }
    }
    analysis.files.iter().find_map(|file| {
        file.editor_symbols
            .iter()
            .find(|candidate| {
                candidate.role == EditorSymbolRole::Definition
                    && candidate.kind == symbol.kind
                    && candidate.target_key == symbol.target_key
            })
            .map(|definition| (file, definition))
    })
}

fn symbol_kind_name(kind: EditorSymbolKind) -> &'static str {
    match kind {
        EditorSymbolKind::Visual => "Visual",
        EditorSymbolKind::Node => "Node",
        EditorSymbolKind::Scenario => "Scenario",
    }
}

fn completion_kind(kind: CompletionKind) -> CompletionItemKind {
    match kind {
        CompletionKind::Keyword => CompletionItemKind::KEYWORD,
        CompletionKind::Value => CompletionItemKind::VALUE,
        CompletionKind::Identifier => CompletionItemKind::VARIABLE,
    }
}

fn visual_target_from_source(source: &str) -> Option<String> {
    let tokens = highlight_tokens(source);
    let token_text = |token: &schemra_compiler::SyntaxToken| {
        source.get(token.span.start.offset..token.span.end.offset)
    };

    if tokens
        .first()
        .is_some_and(|token| token_text(token) == Some("visual"))
        && tokens
            .get(1)
            .is_some_and(|token| token.kind == SyntaxTokenKind::Identifier)
    {
        return token_text(&tokens[1]).map(str::to_owned);
    }
    tokens.windows(3).find_map(|window| {
        (window[0].kind == SyntaxTokenKind::Keyword
            && token_text(&window[0]) == Some("for")
            && window[1].kind == SyntaxTokenKind::Keyword
            && token_text(&window[1]) == Some("visual")
            && window[2].kind == SyntaxTokenKind::Identifier)
            .then(|| token_text(&window[2]).map(str::to_owned))
            .flatten()
    })
}

fn uri_to_path(uri: &lsp_types::Uri) -> Option<PathBuf> {
    let url = Url::parse(uri.as_str()).ok()?;
    (url.scheme() == "file")
        .then(|| url.to_file_path().ok())
        .flatten()
}

fn normalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| {
        path.parent()
            .and_then(|parent| fs::canonicalize(parent).ok())
            .and_then(|parent| path.file_name().map(|name| parent.join(name)))
            .unwrap_or_else(|| path.to_path_buf())
    })
}
