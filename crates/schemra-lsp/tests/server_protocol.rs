use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

use serde_json::{Value, json};
use url::Url;

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestServer {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    next_request_id: i64,
}

impl TestServer {
    fn start(workspace: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_schemra-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("the LSP executable should start");
        let input = child.stdin.take().expect("the LSP should read stdin");
        let output = BufReader::new(child.stdout.take().expect("the LSP should write stdout"));
        let mut server = Self {
            child,
            input: Some(input),
            output,
            next_request_id: 3,
        };
        let root_uri = Url::from_directory_path(workspace)
            .expect("the test workspace should have a file URI")
            .to_string();
        server.send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": root_uri,
                "capabilities": {},
                "workspaceFolders": [{"uri": root_uri, "name": "test"}]
            }
        }));
        let response = server.receive();
        assert_eq!(response["id"], 1);
        assert_eq!(
            response["result"]["capabilities"]["textDocumentSync"]["openClose"],
            true
        );
        assert_eq!(
            response["result"]["capabilities"]["textDocumentSync"]["change"],
            1
        );
        server.send(json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));
        server
    }

    fn open_document(&mut self, workspace: &Path, name: &str, source: &str) -> (String, Value) {
        let source_path = workspace.join(name);
        fs::write(&source_path, source).expect("source should be written");
        let uri = Url::from_file_path(&source_path)
            .expect("the source path should have a file URI")
            .to_string();
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "schemra",
                    "version": 1,
                    "text": source
                }
            }
        }));
        (uri, self.receive())
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_request_id;
        self.next_request_id += 1;
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }));
        let response = self.receive();
        assert_eq!(response["id"], id);
        response
    }

    fn send(&mut self, message: Value) {
        let body = serde_json::to_vec(&message).expect("LSP messages should serialize");
        let input = self.input.as_mut().expect("the server input is open");
        write!(input, "Content-Length: {}\r\n\r\n", body.len())
            .expect("the LSP header should write");
        input.write_all(&body).expect("the LSP body should write");
        input.flush().expect("the LSP message should flush");
    }

    fn receive(&mut self) -> Value {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            self.output
                .read_line(&mut line)
                .expect("the LSP should return a message header");
            assert!(!line.is_empty(), "the LSP closed stdout unexpectedly");
            if line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(length) = line.strip_prefix("Content-Length:") {
                content_length = Some(
                    length
                        .trim()
                        .parse::<usize>()
                        .expect("the LSP content length should be numeric"),
                );
            }
        }
        let mut body = vec![0; content_length.expect("the LSP should set Content-Length")];
        self.output
            .read_exact(&mut body)
            .expect("the LSP message body should be complete");
        serde_json::from_slice(&body).expect("the LSP response should be JSON")
    }

    fn stop(mut self) {
        self.send(json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null}));
        let response = self.receive();
        assert_eq!(response["id"], 2);
        self.send(json!({"jsonrpc": "2.0", "method": "exit", "params": null}));
        drop(self.input.take());
        let status = self.child.wait().expect("the LSP process should exit");
        assert!(status.success(), "the LSP exited with {status}");
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn temporary_workspace() -> PathBuf {
    let id = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("schemra-lsp-{}-{id}", std::process::id()));
    fs::create_dir_all(&path).expect("the test workspace should be created");
    path
}

#[test]
fn server_publishes_diagnostics_and_refreshes_them_after_edits() {
    let workspace = temporary_workspace();
    let mut server = TestServer::start(&workspace);
    let (uri, invalid) =
        server.open_document(&workspace, "flow.schemra", "visual flow \"Flow\" {\n");
    assert_eq!(invalid["method"], "textDocument/publishDiagnostics");
    assert_eq!(invalid["params"]["version"], 1);
    assert!(
        !invalid["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics should be an array")
            .is_empty()
    );

    server.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": "visual flow \"Flow\" { node client \"Client\" {} }"}]
        }
    }));
    let valid = server.receive();
    assert_eq!(valid["method"], "textDocument/publishDiagnostics");
    assert_eq!(valid["params"]["version"], 2);
    assert!(
        valid["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics should be an array")
            .is_empty()
    );

    server.stop();
    fs::remove_dir_all(workspace).expect("the temporary workspace should be removed");
}

#[test]
fn completion_uses_node_definitions_from_the_open_project() {
    let workspace = temporary_workspace();
    let mut server = TestServer::start(&workspace);
    let source = "visual flow \"Flow\" {\n  node client \"Client\" {}\n  node server \"Server\" {}\n  client -> \n}\n";
    let (uri, _) = server.open_document(&workspace, "flow.schemra", source);

    let response = server.request(
        "textDocument/completion",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 3, "character": 12}
        }),
    );

    assert!(
        response["result"]
            .as_array()
            .expect("completion should return items")
            .iter()
            .any(|item| item["label"] == "server")
    );
    server.stop();
    fs::remove_dir_all(workspace).expect("the temporary workspace should be removed");
}

#[test]
fn definition_resolves_a_node_reference_to_its_declaration() {
    let workspace = temporary_workspace();
    let mut server = TestServer::start(&workspace);
    let source = "visual flow \"Flow\" {\n  node client \"Client\" {}\n  node server \"Server\" {}\n  client -> server\n}\n";
    let (uri, _) = server.open_document(&workspace, "flow.schemra", source);

    let response = server.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 3, "character": 14}
        }),
    );

    assert_eq!(response["result"]["range"]["start"]["line"], 2);
    assert_eq!(response["result"]["range"]["start"]["character"], 7);
    server.stop();
    fs::remove_dir_all(workspace).expect("the temporary workspace should be removed");
}

#[test]
fn definition_positions_are_line_relative() {
    let workspace = temporary_workspace();
    let mut server = TestServer::start(&workspace);
    let source = "visual flow \"🌐\" {\n  node client \"Client\" {}\n  node server \"Server\" {}\n  client -> server\n}";
    let (uri, diagnostics) = server.open_document(&workspace, "flow.schemra", source);
    assert!(
        diagnostics["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let declaration_start = source.find("server").unwrap();
    let reference_start = source.rfind("server").unwrap();
    let line_start = source[..reference_start]
        .rfind('\n')
        .map_or(0, |offset| offset + 1);
    let declaration_line_start = source[..declaration_start]
        .rfind('\n')
        .map_or(0, |offset| offset + 1);

    let response = server.request(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": uri},
            "position": {
                "line": source[..reference_start].matches('\n').count(),
                "character": reference_start - line_start
            }
        }),
    );

    assert_eq!(
        response["result"]["range"]["start"]["character"],
        source[declaration_line_start..declaration_start]
            .encode_utf16()
            .count()
    );
    server.stop();
    fs::remove_dir_all(workspace).expect("the temporary workspace should be removed");
}

#[test]
fn hover_describes_a_node_reference() {
    let workspace = temporary_workspace();
    let mut server = TestServer::start(&workspace);
    let source = "visual flow \"Flow\" {\n  node client \"Client\" {}\n  node server \"Server\" {}\n  client -> server\n}\n";
    let (uri, _) = server.open_document(&workspace, "flow.schemra", source);

    let response = server.request(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 3, "character": 14}
        }),
    );

    assert!(
        response["result"]["contents"]
            .to_string()
            .contains("Node: server")
    );
    assert_eq!(response["result"]["range"]["start"]["character"], 12);
    server.stop();
    fs::remove_dir_all(workspace).expect("the temporary workspace should be removed");
}

#[test]
fn formatting_returns_an_edit_for_the_open_document() {
    let workspace = temporary_workspace();
    let mut server = TestServer::start(&workspace);
    let source = "visual flow \"Flow\" {\nnode client \"Client\" {}\n}\n";
    let (uri, _) = server.open_document(&workspace, "flow.schemra", source);

    let response = server.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": uri}, "options": {"tabSize": 2, "insertSpaces": true}}),
    );

    assert_eq!(
        response["result"][0]["newText"],
        "visual flow \"Flow\" {\n  node client \"Client\" {}\n}\n"
    );
    server.stop();
    fs::remove_dir_all(workspace).expect("the temporary workspace should be removed");
}

#[test]
fn rename_returns_workspace_edits_for_a_node_symbol() {
    let workspace = temporary_workspace();
    let mut server = TestServer::start(&workspace);
    let source = "visual flow \"Flow\" {\n  node client \"Client\" {}\n  node server \"Server\" {}\n  client -> server\n}\n";
    let (uri, _) = server.open_document(&workspace, "flow.schemra", source);

    let prepared = server.request(
        "textDocument/prepareRename",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 9}
        }),
    );
    assert_eq!(prepared["result"]["placeholder"], "client");

    let response = server.request(
        "textDocument/rename",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 9},
            "newName": "frontend"
        }),
    );

    assert!(
        response["result"]["changes"]
            .as_object()
            .expect("rename should return file changes")
            .values()
            .flat_map(|edits| edits.as_array().into_iter().flatten())
            .any(|edit| edit["newText"] == "frontend")
    );
    server.stop();
    fs::remove_dir_all(workspace).expect("the temporary workspace should be removed");
}

#[test]
fn code_action_suggests_a_nearby_node_for_an_unknown_edge_target() {
    let workspace = temporary_workspace();
    let mut server = TestServer::start(&workspace);
    let source = "visual flow \"Flow\" {\n  node client \"Client\" {}\n  node server \"Server\" {}\n  client -> servr\n}\n";
    let (uri, diagnostics) = server.open_document(&workspace, "flow.schemra", source);
    assert!(
        !diagnostics["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let response = server.request(
        "textDocument/codeAction",
        json!({
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 3, "character": 12},
                "end": {"line": 3, "character": 17}
            },
            "context": {"diagnostics": diagnostics["params"]["diagnostics"]}
        }),
    );

    assert!(
        response["result"]
            .as_array()
            .expect("code actions should be returned")
            .iter()
            .any(|action| action["edit"]["changes"]
                .as_object()
                .is_some_and(|files| files
                    .values()
                    .flat_map(|edits| edits.as_array().into_iter().flatten())
                    .any(|edit| edit["newText"] == "server")))
    );
    server.stop();
    fs::remove_dir_all(workspace).expect("the temporary workspace should be removed");
}
