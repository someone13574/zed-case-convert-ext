use std::{collections::HashMap, error::Error};

use lsp_server::{Connection, ErrorCode, Message, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
};
use lsp_types::request::{CodeActionRequest, Request};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, Position, Range, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextEdit, WorkspaceEdit,
};

use crate::case::Case;

mod case;
mod ident;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (connection, io_threads) = Connection::stdio();
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![CodeActionKind::REFACTOR_REWRITE]),
            ..Default::default()
        })),
        ..Default::default()
    };
    connection.initialize(serde_json::to_value(capabilities)?)?;

    let mut documents = HashMap::new();
    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    break;
                }
                let response = if request.method != CodeActionRequest::METHOD {
                    Response::new_err(
                        request.id,
                        ErrorCode::MethodNotFound as i32,
                        format!("unhandled method `{}`", request.method),
                    )
                } else {
                    match serde_json::from_value(request.params) {
                        Ok(params) => {
                            Response::new_ok(request.id, code_actions(&documents, params))
                        }
                        Err(err) => Response::new_err(
                            request.id,
                            ErrorCode::InvalidParams as i32,
                            err.to_string(),
                        ),
                    }
                };
                connection.sender.send(response.into())?;
            }
            Message::Notification(notification) => match notification.method.as_str() {
                DidOpenTextDocument::METHOD => {
                    let params: DidOpenTextDocumentParams =
                        notification.extract(DidOpenTextDocument::METHOD)?;
                    documents.insert(
                        params.text_document.uri.to_string(),
                        params.text_document.text,
                    );
                }
                DidChangeTextDocument::METHOD => {
                    let params: DidChangeTextDocumentParams =
                        notification.extract(DidChangeTextDocument::METHOD)?;
                    if let Some(change) = params.content_changes.into_iter().last() {
                        documents.insert(params.text_document.uri.to_string(), change.text);
                    }
                }
                DidCloseTextDocument::METHOD => {
                    let params: DidCloseTextDocumentParams =
                        notification.extract(DidCloseTextDocument::METHOD)?;
                    documents.remove(&params.text_document.uri.to_string());
                }
                _ => {}
            },
            Message::Response(_) => {}
        }
    }

    drop(connection);
    io_threads.join()?;
    Ok(())
}

fn code_actions(
    documents: &HashMap<String, String>,
    params: CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    let kind = CodeActionKind::REFACTOR_REWRITE;
    let wanted = params.context.only.is_none_or(|only| {
        only.iter().any(|only| {
            kind.as_str() == only.as_str()
                || kind.as_str().starts_with(&format!("{}.", only.as_str()))
        })
    });
    let uri = params.text_document.uri;
    let Some(text) = documents.get(&uri.to_string()).filter(|_| wanted) else {
        return Vec::new();
    };

    let lines = LineIndex::new(text);
    let byte_start = lines.byte_offset(params.range.start);
    let byte_end = lines.byte_offset(params.range.end);
    let mut idents = ident::in_range(text, byte_start..byte_end);
    if idents.len() > 1 {
        idents.retain(|(_, name)| !ident::is_reserved(name));
    }

    Case::ALL
        .into_iter()
        .filter_map(|case| {
            let edits: Vec<TextEdit> = idents
                .iter()
                .filter_map(|(byte_range, name)| {
                    let converted = case.convert(name);
                    (converted != *name).then(|| TextEdit {
                        range: Range::new(
                            lines.position(byte_range.start),
                            lines.position(byte_range.end),
                        ),
                        new_text: converted,
                    })
                })
                .collect();
            if edits.is_empty() {
                return None;
            }

            let title = match idents.as_slice() {
                [(_, name)] => format!("Convert `{name}` to {}", case.name()),
                _ => format!("Convert selection to {}", case.name()),
            };
            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title,
                kind: Some(kind.clone()),
                edit: Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(uri.clone(), edits)])),
                    ..Default::default()
                }),
                ..Default::default()
            }))
        })
        .collect()
}

struct LineIndex<'a> {
    text: &'a str,
    line_byte_starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    fn new(text: &'a str) -> Self {
        let line_byte_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(byte_idx, _)| byte_idx + 1))
            .collect();
        Self {
            text,
            line_byte_starts,
        }
    }

    fn byte_offset(&self, position: Position) -> usize {
        let line_idx = position.line as usize;
        let Some(&line_byte_start) = self.line_byte_starts.get(line_idx) else {
            return self.text.len();
        };
        let line_byte_end = self
            .line_byte_starts
            .get(line_idx + 1)
            .map_or(self.text.len(), |&next_line_byte_start| {
                next_line_byte_start - 1
            });

        let mut utf16_offset = 0;
        for (byte_idx, ch) in self.text[line_byte_start..line_byte_end].char_indices() {
            if utf16_offset >= position.character as usize {
                return line_byte_start + byte_idx;
            }
            utf16_offset += ch.len_utf16();
        }
        line_byte_end
    }

    fn position(&self, byte_offset: usize) -> Position {
        let line_idx = self
            .line_byte_starts
            .partition_point(|&line_byte_start| line_byte_start <= byte_offset)
            - 1;
        let utf16_offset = self.text[self.line_byte_starts[line_idx]..byte_offset]
            .encode_utf16()
            .count();
        Position::new(line_idx as u32, utf16_offset as u32)
    }
}
