use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
pub struct Backend {
    client: Client,
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
        self.client
            .log_message(MessageType::INFO, "Izel Language Server is initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("Opened file: {}", params.text_document.uri),
            )
            .await;
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
        self.client
            .log_message(MessageType::INFO, "Hover request received")
            .await;
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("Izel symbol".to_string())),
            range: None,
        }))
    }
}

impl Backend {
    async fn validate_document(&self, uri: Url, source: String) {
        let mut diagnostics = vec![];

        let source_id = izel_span::SourceId(1); // Mock Source ID
        let mut lexer = izel_lexer::Lexer::new(&source, source_id);

        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            let kind = token.kind;
            tokens.push(token);
            if kind == izel_lexer::TokenKind::Eof {
                break;
            }
        }

        let mut parser = izel_parser::Parser::new(tokens, source.clone());
        let cst = parser.parse_source_file();

        let ast_lowerer = izel_ast_lower::Lowerer::new(&source);
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

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}

pub fn run_server_sync() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run_server());
}
