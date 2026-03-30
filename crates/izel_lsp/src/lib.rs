use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
pub struct Backend {
    client: Option<Client>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), "::".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                ..ServerCapabilities::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        if let Some(client) = &self.client {
            client
                .log_message(MessageType::INFO, "Izel Language Server is initialized!")
                .await;
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        if let Some(client) = &self.client {
            client
                .log_message(
                    MessageType::INFO,
                    format!("Opened file: {}", params.text_document.uri),
                )
                .await;
        }
        // Trigger generic diagnostics compiling
        self.validate_document(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.pop() {
            self.validate_document(params.text_document.uri, change.text)
                .await;
        }
    }

    async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        if let Some(client) = &self.client {
            client
                .log_message(MessageType::INFO, "Hover request received")
                .await;
        }
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("Izel symbol".to_string())),
            range: None,
        }))
    }
}

impl Backend {
    fn build_diagnostics(source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        let source_id = izel_span::SourceId(1); // Mock Source ID
        let mut lexer = izel_lexer::Lexer::new(source, source_id);

        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            let kind = token.kind;
            tokens.push(token);
            if kind == izel_lexer::TokenKind::Eof {
                break;
            }
        }

        let mut parser = izel_parser::Parser::new(tokens, source.to_string());
        let cst = parser.parse_source_file();

        let ast_lowerer = izel_ast_lower::Lowerer::new(source);
        let ast = ast_lowerer.lower_module(&cst);

        let mut typeck = izel_typeck::TypeChecker::new();
        typeck.check_ast(&ast);

        if !typeck.diagnostics.is_empty() {
            for diag in &typeck.diagnostics {
                // Approximate line mapping could happen here. Leaving as minimal for scaffolding.
                diagnostics.push(Diagnostic {
                    range: Range::default(), // Dummy range
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: diag.message.clone(),
                    ..Default::default()
                });
            }
        }

        diagnostics
    }

    async fn validate_document(&self, uri: Url, source: String) {
        let diagnostics = Self::build_diagnostics(&source);

        if let Some(client) = &self.client {
            client.publish_diagnostics(uri, diagnostics, None).await;
        }
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client: Some(client),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

pub fn run_server_sync() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run_server());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uri() -> Url {
        Url::parse("file:///tmp/test.iz").expect("valid test URL")
    }

    #[tokio::test]
    async fn initialize_reports_expected_capabilities() {
        let backend = Backend { client: None };
        let init = backend
            .initialize(InitializeParams::default())
            .await
            .expect("initialize should succeed");

        match init.capabilities.text_document_sync {
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)) => {}
            other => panic!("unexpected text document sync capability: {:?}", other),
        }

        assert_eq!(
            init.capabilities.hover_provider,
            Some(HoverProviderCapability::Simple(true))
        );

        let trigger_chars = init
            .capabilities
            .completion_provider
            .and_then(|c| c.trigger_characters)
            .unwrap_or_default();
        assert!(trigger_chars.contains(&".".to_string()));
        assert!(trigger_chars.contains(&"::".to_string()));
    }

    #[tokio::test]
    async fn hover_and_shutdown_are_callable_without_client() {
        let backend = Backend { client: None };

        let hover = backend
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: test_uri() },
                    position: Position::new(0, 0),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("hover should succeed");

        assert!(hover.is_some());
        backend.shutdown().await.expect("shutdown should succeed");
    }

    #[tokio::test]
    async fn did_open_and_did_change_trigger_validation_paths() {
        let backend = Backend { client: None };
        backend.initialized(InitializedParams {}).await;

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: test_uri(),
                    language_id: "izel".to_string(),
                    version: 1,
                    text: "echo { let x }".to_string(),
                },
            })
            .await;

        backend
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: test_uri(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "shape Packet {}".to_string(),
                }],
            })
            .await;
    }

    #[test]
    fn build_diagnostics_reports_expected_results() {
        let clean = Backend::build_diagnostics("shape Packet {}");
        assert!(clean.is_empty());

        let bad = Backend::build_diagnostics("echo { let x }");
        assert!(
            bad.iter()
                .any(|d| d.message.contains("requires an initializer")),
            "invalid echo should produce a diagnostic"
        );
    }

    #[tokio::test]
    async fn validate_document_noops_without_client() {
        let backend = Backend { client: None };
        backend
            .validate_document(test_uri(), "shape Ready {}".to_string())
            .await;
    }
}
