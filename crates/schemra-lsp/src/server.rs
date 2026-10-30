use std::error::Error;

use lsp_server::{Connection, Message, Response};
use lsp_types::{
    CodeActionParams, CompletionParams, DocumentFormattingParams, GotoDefinitionParams,
    HoverParams, InitializeParams, RenameParams, TextDocumentPositionParams,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::backend::Backend;

type ServerResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

pub(crate) fn run_stdio() -> ServerResult {
    let (connection, io_threads) = Connection::stdio();
    let result = run_connection(connection);
    io_threads.join()?;
    result
}

fn run_connection(connection: Connection) -> ServerResult {
    let initialize = connection.initialize(serde_json::to_value(Backend::capabilities())?)?;
    let params: InitializeParams = serde_json::from_value(initialize)?;
    let mut backend = Backend::new(&params);

    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    break;
                }
                let response = match decode_request(&request) {
                    Ok(RequestParams::Completion(params)) => Response::new_ok(
                        request.id,
                        serde_json::to_value(backend.completions(params))?,
                    ),
                    Ok(RequestParams::Definition(params)) => Response::new_ok(
                        request.id,
                        serde_json::to_value(backend.definition(params))?,
                    ),
                    Ok(RequestParams::Formatting(params)) => Response::new_ok(
                        request.id,
                        serde_json::to_value(backend.formatting(&params))?,
                    ),
                    Ok(RequestParams::Hover(params)) => {
                        Response::new_ok(request.id, serde_json::to_value(backend.hover(params))?)
                    }
                    Ok(RequestParams::PrepareRename(params)) => Response::new_ok(
                        request.id,
                        serde_json::to_value(backend.prepare_rename(params))?,
                    ),
                    Ok(RequestParams::Rename(params)) => match backend.rename(params) {
                        Ok(edit) => Response::new_ok(request.id, serde_json::to_value(edit)?),
                        Err(message) => Response::new_err(
                            request.id,
                            lsp_server::ErrorCode::InvalidParams as i32,
                            message,
                        ),
                    },
                    Ok(RequestParams::CodeAction(params)) => Response::new_ok(
                        request.id,
                        serde_json::to_value(backend.code_actions(params))?,
                    ),
                    Ok(RequestParams::Unsupported) => Response::new_err(
                        request.id,
                        lsp_server::ErrorCode::MethodNotFound as i32,
                        format!("unsupported request: {}", request.method),
                    ),
                    Err(error) => Response::new_err(
                        request.id,
                        lsp_server::ErrorCode::InvalidParams as i32,
                        format!("invalid parameters for {}: {error}", request.method),
                    ),
                };
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(notification) => {
                if notification.method == "exit" {
                    break;
                }
                backend.handle_notification(&connection, notification)?;
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

enum RequestParams {
    Completion(CompletionParams),
    Definition(GotoDefinitionParams),
    Formatting(DocumentFormattingParams),
    Hover(HoverParams),
    PrepareRename(TextDocumentPositionParams),
    Rename(RenameParams),
    CodeAction(CodeActionParams),
    Unsupported,
}

fn decode_request(request: &lsp_server::Request) -> Result<RequestParams, serde_json::Error> {
    match request.method.as_str() {
        "textDocument/completion" => decode(request.params.clone()).map(RequestParams::Completion),
        "textDocument/definition" => decode(request.params.clone()).map(RequestParams::Definition),
        "textDocument/formatting" => decode(request.params.clone()).map(RequestParams::Formatting),
        "textDocument/hover" => decode(request.params.clone()).map(RequestParams::Hover),
        "textDocument/prepareRename" => {
            decode(request.params.clone()).map(RequestParams::PrepareRename)
        }
        "textDocument/rename" => decode(request.params.clone()).map(RequestParams::Rename),
        "textDocument/codeAction" => decode(request.params.clone()).map(RequestParams::CodeAction),
        _ => Ok(RequestParams::Unsupported),
    }
}

fn decode<T: DeserializeOwned>(params: Value) -> Result<T, serde_json::Error> {
    serde_json::from_value(params)
}
