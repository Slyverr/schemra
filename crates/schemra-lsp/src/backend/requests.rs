use std::{collections::HashMap, fs, path::PathBuf};

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CompletionItem,
    CompletionParams, CompletionResponse, CompletionTextEdit, DocumentFormattingParams,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, MarkedString, Position,
    PrepareRenameResponse, Range, RenameParams, TextDocumentPositionParams, TextEdit,
    WorkspaceEdit,
};
use schemra_compiler::{
    EditorSymbolKind, EditorSymbolRole, ProjectSource, analyze_project, complete_at_with_nodes,
    prepare_project_rename, rename_project,
};
use schemra_syntax::format_source;
use url::Url;

use crate::positions::{byte_offset_at_position, range_for_bytes};

use super::{
    Backend, OpenDocument, completion_kind, convert_diagnostics, edit_distance, error_counts,
    find_definition, has_errors, normalize_path, symbol_at, symbol_kind_name, uri_to_path,
    valid_identifier, visual_target_from_source,
};

impl Backend {
    pub(crate) fn formatting(&self, params: &DocumentFormattingParams) -> Option<Vec<TextEdit>> {
        let uri = params.text_document.uri.as_str();
        let source = self
            .documents
            .get(uri)
            .map(|document| document.source.clone())
            .or_else(|| {
                uri_to_path(&params.text_document.uri)
                    .and_then(|path| fs::read_to_string(path).ok())
            })?;
        let formatted = format_source(&source);
        if formatted == source {
            return Some(Vec::new());
        }
        Some(vec![TextEdit {
            range: Range {
                start: Position::new(0, 0),
                end: range_for_bytes(&source, source.len(), source.len()).end,
            },
            new_text: formatted,
        }])
    }

    pub(crate) fn completions(&mut self, params: CompletionParams) -> Option<CompletionResponse> {
        let uri = params.text_document_position.text_document.uri.as_str();
        let document = self.documents.get(uri)?.clone();
        let path = self.document_project_path(&document, document.root.as_deref())?;
        self.ensure_project(document.root.as_deref()).ok()?;
        let cached = self.projects.get(&document.root)?;
        let file = cached.analysis.files.iter().find(|file| file.path == path);
        let visual_id = file
            .and_then(|file| file.visual_id.as_ref().or(file.resolved_visual_id.as_ref()))
            .cloned()
            .or_else(|| visual_target_from_source(&document.source))?;
        let node_ids = cached
            .analysis
            .files
            .iter()
            .find(|candidate| candidate.visual_id.as_deref() == Some(&visual_id))
            .and_then(|visual| visual.visual.as_ref())
            .map(|visual| {
                visual
                    .nodes
                    .iter()
                    .map(|node| node.id.0.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let offset =
            byte_offset_at_position(&document.source, params.text_document_position.position);
        let suggestions = complete_at_with_nodes(&document.source, offset, true, &node_ids)?;
        let replacement = range_for_bytes(&document.source, suggestions.start, suggestions.end);
        let items = suggestions
            .items
            .into_iter()
            .map(|item| CompletionItem {
                label: item.label,
                kind: Some(completion_kind(item.kind)),
                detail: Some(item.detail),
                insert_text: Some(item.insert_text.clone()),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: replacement,
                    new_text: item.insert_text,
                })),
                ..Default::default()
            })
            .collect();
        Some(CompletionResponse::Array(items))
    }

    pub(crate) fn hover(&mut self, params: lsp_types::HoverParams) -> Option<Hover> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let document = self.document_for_request(&uri)?;
        let path = self.document_project_path(&document, document.root.as_deref())?;
        self.ensure_project(document.root.as_deref()).ok()?;
        let cached = self.projects.get(&document.root)?;
        let offset = byte_offset_at_position(&document.source, position);
        let symbol = cached
            .analysis
            .files
            .iter()
            .find(|file| file.path == path)
            .and_then(|file| symbol_at(&file.editor_symbols, offset))?;

        Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(format!(
                "{}: {}",
                symbol_kind_name(symbol.kind),
                symbol.name
            ))),
            range: Some(range_for_bytes(
                &document.source,
                symbol.span.start.offset,
                symbol.span.end.offset,
            )),
        })
    }

    pub(crate) fn definition(
        &mut self,
        params: GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let document = self.document_for_request(&uri)?;
        let path = self.document_project_path(&document, document.root.as_deref())?;
        self.ensure_project(document.root.as_deref()).ok()?;
        let cached = self.projects.get(&document.root)?;
        let offset = byte_offset_at_position(&document.source, position);
        let symbol = cached
            .analysis
            .files
            .iter()
            .find(|file| file.path == path)
            .and_then(|file| symbol_at(&file.editor_symbols, offset))?;
        let (target_file, target_symbol) = find_definition(&cached.analysis, &path, symbol)?;
        let source = cached
            .sources
            .iter()
            .find(|source| source.path == target_file.path)?;
        let target_path = document
            .root
            .as_deref()
            .map(|root| root.join(&target_file.path))
            .unwrap_or_else(|| PathBuf::from(&target_file.path));
        let uri = Url::from_file_path(target_path)
            .ok()?
            .as_str()
            .parse()
            .ok()?;
        Some(GotoDefinitionResponse::Scalar(lsp_types::Location {
            uri,
            range: range_for_bytes(
                &source.source,
                target_symbol.span.start.offset,
                target_symbol.span.end.offset,
            ),
        }))
    }

    pub(crate) fn prepare_rename(
        &mut self,
        params: TextDocumentPositionParams,
    ) -> Option<PrepareRenameResponse> {
        let (document, _, path, sources) = self.rename_context(&params).ok()?;
        let offset = byte_offset_at_position(&document.source, params.position);
        let selection = prepare_project_rename(&sources, &path, offset).ok()?;
        Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: range_for_bytes(
                &document.source,
                selection.span.start.offset,
                selection.span.end.offset,
            ),
            placeholder: selection.name,
        })
    }

    #[allow(clippy::mutable_key_type)] // URI equality and hashing use the immutable URI string.
    pub(crate) fn rename(&mut self, params: RenameParams) -> Result<WorkspaceEdit, String> {
        let (document, root, path, sources) =
            self.rename_context(&params.text_document_position)?;
        let offset =
            byte_offset_at_position(&document.source, params.text_document_position.position);
        let result = rename_project(&sources, &path, offset, &params.new_name)?;
        let mut changes = HashMap::new();
        for edit in result.edits {
            let source = sources
                .iter()
                .find(|source| source.path == edit.path)
                .ok_or_else(|| "A referenced project source is unavailable.".to_owned())?;
            let file_path = root.join(&edit.path);
            let uri = Url::from_file_path(file_path)
                .ok()
                .and_then(|url| url.as_str().parse().ok())
                .ok_or_else(|| "Cannot construct a file URI for this edit.".to_owned())?;
            changes.entry(uri).or_insert_with(Vec::new).push(TextEdit {
                range: range_for_bytes(
                    &source.source,
                    edit.span.start.offset,
                    edit.span.end.offset,
                ),
                new_text: edit.replacement,
            });
        }
        Ok(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    #[allow(clippy::mutable_key_type)] // WorkspaceEdit maps are keyed by the LSP URI type.
    pub(crate) fn code_actions(&mut self, params: CodeActionParams) -> Vec<CodeActionOrCommand> {
        if params.context.only.as_ref().is_some_and(|kinds| {
            !kinds.iter().any(|kind| {
                kind.as_str() == CodeActionKind::QUICKFIX.as_str() || kind.as_str().is_empty()
            })
        }) {
            return Vec::new();
        }
        let Some(document) = self.document_for_request(&params.text_document.uri) else {
            return Vec::new();
        };
        let Some(root) = document.root.as_deref() else {
            return Vec::new();
        };
        let Some(path) = self.document_project_path(&document, Some(root)) else {
            return Vec::new();
        };
        if self.ensure_project(Some(root)).is_err() {
            return Vec::new();
        }
        let Some(project) = self.projects.get(&Some(root.to_path_buf())) else {
            return Vec::new();
        };
        let Some(file) = project.analysis.files.iter().find(|file| file.path == path) else {
            return Vec::new();
        };

        let requested_start = byte_offset_at_position(&document.source, params.range.start);
        let requested_end = byte_offset_at_position(&document.source, params.range.end);
        let baseline = error_counts(&project.analysis);
        let definitions = file
            .editor_symbols
            .iter()
            .filter(|symbol| {
                symbol.kind == EditorSymbolKind::Node && symbol.role == EditorSymbolRole::Definition
            })
            .map(|symbol| symbol.name.as_str())
            .collect::<std::collections::HashSet<_>>();
        let mut actions = Vec::new();
        let mut validations = 0usize;

        'diagnostics: for diagnostic in &file.diagnostics {
            if diagnostic.code != "E_UNKNOWN_EDGE_NODE"
                || diagnostic.span.end.offset < requested_start
                || diagnostic.span.start.offset > requested_end
            {
                continue;
            }
            let mut references = file
                .editor_symbols
                .iter()
                .filter(|symbol| {
                    symbol.kind == EditorSymbolKind::Node
                        && symbol.role == EditorSymbolRole::Reference
                        && symbol.span.start.offset >= diagnostic.span.start.offset
                        && symbol.span.end.offset <= diagnostic.span.end.offset
                        && !definitions.contains(symbol.name.as_str())
                        && valid_identifier(&symbol.name)
                        && document
                            .source
                            .get(symbol.span.start.offset..symbol.span.end.offset)
                            == Some(symbol.name.as_str())
                })
                .collect::<Vec<_>>();
            references.sort_by_key(|symbol| symbol.span.start.offset);

            for reference in references {
                let mut candidates = file
                    .editor_symbols
                    .iter()
                    .filter(|symbol| {
                        symbol.kind == EditorSymbolKind::Node
                            && symbol.role == EditorSymbolRole::Definition
                            && symbol.name != reference.name
                            && valid_identifier(&symbol.name)
                    })
                    .map(|symbol| {
                        (
                            edit_distance(&reference.name, &symbol.name),
                            symbol.name.clone(),
                        )
                    })
                    .filter(|(distance, _)| *distance <= 2)
                    .collect::<Vec<_>>();
                candidates.sort();
                candidates.dedup();
                let Some(best_distance) = candidates.first().map(|candidate| candidate.0) else {
                    continue;
                };

                for (_, candidate) in candidates
                    .into_iter()
                    .filter(|item| item.0 == best_distance)
                    .take(3)
                {
                    if validations >= 8 {
                        break 'diagnostics;
                    }
                    validations += 1;
                    let mut sources = project.sources.clone();
                    let Some(source) = sources.iter_mut().find(|source| source.path == path) else {
                        continue;
                    };
                    source.source.replace_range(
                        reference.span.start.offset..reference.span.end.offset,
                        &candidate,
                    );
                    let checked = analyze_project(&sources);
                    let after = error_counts(&checked);
                    let key = (
                        path.clone(),
                        diagnostic.code.clone(),
                        diagnostic.message.clone(),
                    );
                    if after
                        .iter()
                        .any(|(key, count)| *count > baseline.get(key).copied().unwrap_or(0))
                        || after.get(&key).copied().unwrap_or(0)
                            >= baseline.get(&key).copied().unwrap_or(0)
                    {
                        continue;
                    }

                    let mut changes = HashMap::new();
                    changes.insert(
                        params.text_document.uri.clone(),
                        vec![TextEdit {
                            range: range_for_bytes(
                                &document.source,
                                reference.span.start.offset,
                                reference.span.end.offset,
                            ),
                            new_text: candidate.clone(),
                        }],
                    );
                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: format!("Replace {:?} with {candidate:?}", reference.name),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(convert_diagnostics(
                            &document.source,
                            std::slice::from_ref(diagnostic),
                        )),
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            document_changes: None,
                            change_annotations: None,
                        }),
                        command: None,
                        is_preferred: None,
                        disabled: None,
                        data: None,
                    }));
                }
            }
        }
        if actions.len() == 1 {
            if let CodeActionOrCommand::CodeAction(action) = &mut actions[0] {
                action.is_preferred = Some(true);
            }
        }
        actions
    }

    fn rename_context(
        &mut self,
        params: &TextDocumentPositionParams,
    ) -> Result<(OpenDocument, PathBuf, String, Vec<ProjectSource>), String> {
        let document = self
            .document_for_request(&params.text_document.uri)
            .ok_or_else(|| "Open a project source file to rename a symbol.".to_owned())?;
        let root = document
            .root
            .clone()
            .ok_or_else(|| "The project root is unavailable.".to_owned())?;
        let path = self
            .document_project_path(&document, Some(&root))
            .ok_or_else(|| "The source file is outside the project.".to_owned())?;
        self.ensure_project(Some(&root))?;
        let cached = self
            .projects
            .get(&Some(root.clone()))
            .ok_or_else(|| "The project sources are unavailable.".to_owned())?;
        if has_errors(&cached.analysis) {
            return Err(
                "Resolve project errors before renaming so all references can be identified safely."
                    .into(),
            );
        }
        Ok((document, root, path, cached.sources.clone()))
    }

    fn document_for_request(&self, uri: &lsp_types::Uri) -> Option<OpenDocument> {
        if let Some(document) = self.documents.get(uri.as_str()) {
            return Some(document.clone());
        }
        let path = uri_to_path(uri).map(|path| normalize_path(&path))?;
        let root = self.root_for_path(&path);
        let source = fs::read_to_string(&path).ok()?;
        Some(OpenDocument {
            path: Some(path),
            root,
            version: 0,
            source,
        })
    }
}
