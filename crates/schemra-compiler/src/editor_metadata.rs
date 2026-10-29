//! Source-facing declarations and symbol references for editor navigation.

use schemra_ir::{ActionIr, ModuleIr, SourceSpan};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeclarationKind {
    Visual,
    Scenario,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DeclarationInfo {
    pub kind: DeclarationKind,
    pub id: Option<String>,
    pub title: String,
    pub visual_id: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EditorSymbolKind {
    Visual,
    Node,
    Scenario,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EditorSymbolRole {
    Definition,
    Reference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EditorSymbol {
    pub name: String,
    pub kind: EditorSymbolKind,
    pub role: EditorSymbolRole,
    pub target_key: String,
    pub span: SourceSpan,
}

/// Collects declarations and reference spans from successfully compiled source.
pub fn editor_metadata(module: &ModuleIr) -> (Vec<DeclarationInfo>, Vec<EditorSymbol>) {
    let mut declarations = Vec::new();
    let mut symbols = Vec::new();

    for visual in &module.visuals {
        declarations.push(DeclarationInfo {
            kind: DeclarationKind::Visual,
            id: Some(visual.id.0.clone()),
            title: visual.title.clone(),
            visual_id: None,
            span: visual.span,
        });
        symbols.push(EditorSymbol {
            name: visual.id.0.clone(),
            kind: EditorSymbolKind::Visual,
            role: EditorSymbolRole::Definition,
            target_key: format!("visual:{}", visual.id.0),
            span: visual.id_span,
        });

        for node in &visual.nodes {
            symbols.push(EditorSymbol {
                name: node.id.0.clone(),
                kind: EditorSymbolKind::Node,
                role: EditorSymbolRole::Definition,
                target_key: node_target_key(&visual.id.0, &node.id.0),
                span: node.id_span,
            });
        }

        for edge in &visual.edges {
            symbols.push(node_reference(&visual.id.0, &edge.from.0, edge.from_span));
            symbols.push(node_reference(&visual.id.0, &edge.to.0, edge.to_span));
        }
    }

    for scenario in &module.scenarios {
        declarations.push(DeclarationInfo {
            kind: DeclarationKind::Scenario,
            id: Some(scenario.id.0.clone()),
            title: scenario.id.0.clone(),
            visual_id: Some(scenario.visual.0.clone()),
            span: scenario.span,
        });
        symbols.push(EditorSymbol {
            name: scenario.id.0.clone(),
            kind: EditorSymbolKind::Scenario,
            role: EditorSymbolRole::Definition,
            target_key: format!("scenario:{}", scenario.id.0),
            span: scenario.id_span,
        });
        symbols.push(EditorSymbol {
            name: scenario.visual.0.clone(),
            kind: EditorSymbolKind::Visual,
            role: EditorSymbolRole::Reference,
            target_key: format!("visual:{}", scenario.visual.0),
            span: scenario.visual_span,
        });
        collect_action_symbols(&scenario.actions, &scenario.visual.0, &mut symbols);
    }

    (declarations, symbols)
}

fn collect_action_symbols(actions: &[ActionIr], visual_id: &str, symbols: &mut Vec<EditorSymbol>) {
    for action in actions {
        match action {
            ActionIr::Sequence { actions, .. } | ActionIr::Parallel { actions, .. } => {
                collect_action_symbols(actions, visual_id, symbols);
            }
            ActionIr::Move {
                from,
                from_span,
                to,
                to_span,
                ..
            } => {
                symbols.push(node_reference(visual_id, &from.0, *from_span));
                symbols.push(node_reference(visual_id, &to.0, *to_span));
            }
            ActionIr::Highlight {
                node, node_span, ..
            } => symbols.push(node_reference(visual_id, &node.0, *node_span)),
            ActionIr::Repeat { body, .. } => collect_action_symbols(body, visual_id, symbols),
            ActionIr::Wait { .. } => {}
        }
    }
}

fn node_target_key(visual_id: &str, node_id: &str) -> String {
    format!("node:{visual_id}:{node_id}")
}

fn node_reference(visual_id: &str, name: &str, span: SourceSpan) -> EditorSymbol {
    EditorSymbol {
        name: name.to_owned(),
        kind: EditorSymbolKind::Node,
        role: EditorSymbolRole::Reference,
        target_key: node_target_key(visual_id, name),
        span,
    }
}
