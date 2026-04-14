use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use izel_fmt::format_source;
use izel_lexer::{Lexer, Token, TokenKind};
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeclKind {
    Function,
    Type,
    Module,
    Variable,
    Macro,
    Alias,
}

#[derive(Debug, Clone)]
struct SymbolOccurrence {
    name: String,
    range: Range,
    is_definition: bool,
    decl_kind: Option<DeclKind>,
}

#[derive(Debug)]
pub struct Backend {
    client: Option<Client>,
    documents: Arc<tokio::sync::RwLock<HashMap<Url, String>>>,
}

impl Backend {
    pub fn new(client: Option<Client>) -> Self {
        Self {
            client,
            documents: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    fn semantic_legend() -> SemanticTokensLegend {
        SemanticTokensLegend {
            token_types: vec![
                SemanticTokenType::KEYWORD,
                SemanticTokenType::VARIABLE,
                SemanticTokenType::FUNCTION,
                SemanticTokenType::TYPE,
                SemanticTokenType::STRING,
                SemanticTokenType::NUMBER,
                SemanticTokenType::COMMENT,
                SemanticTokenType::OPERATOR,
            ],
            token_modifiers: vec![],
        }
    }

    async fn upsert_document(&self, uri: Url, source: String) {
        self.documents.write().await.insert(uri, source);
    }

    async fn remove_document(&self, uri: &Url) {
        self.documents.write().await.remove(uri);
    }

    async fn get_document(&self, uri: &Url) -> Option<String> {
        self.documents.read().await.get(uri).cloned()
    }

    fn byte_to_position(source: &str, byte_index: usize) -> Position {
        let mut line = 0u32;
        let mut col = 0u32;

        for (idx, ch) in source.char_indices() {
            if idx >= byte_index {
                break;
            }

            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        Position::new(line, col)
    }

    fn range_from_bytes(source: &str, start: usize, end: usize) -> Range {
        let capped_start = start.min(source.len());
        let capped_end = end.min(source.len()).max(capped_start);
        Range::new(
            Self::byte_to_position(source, capped_start),
            Self::byte_to_position(source, capped_end),
        )
    }

    fn fallback_range(source: &str) -> Range {
        if source.is_empty() {
            return Range::default();
        }

        Range::new(
            Position::new(0, 0),
            Self::byte_to_position(source, source.len()),
        )
    }

    fn full_document_range(source: &str) -> Range {
        Self::fallback_range(source)
    }

    fn position_lt(a: Position, b: Position) -> bool {
        (a.line, a.character) < (b.line, b.character)
    }

    fn position_leq(a: Position, b: Position) -> bool {
        (a.line, a.character) <= (b.line, b.character)
    }

    fn range_contains_position(range: Range, pos: Position) -> bool {
        Self::position_leq(range.start, pos) && Self::position_lt(pos, range.end)
    }

    fn ranges_overlap(a: Range, b: Range) -> bool {
        Self::position_lt(a.start, b.end) && Self::position_lt(b.start, a.end)
    }

    fn source_slice<'a>(source: &'a str, token: &Token) -> &'a str {
        let lo = (token.span.lo.0 as usize).min(source.len());
        let hi = (token.span.hi.0 as usize).min(source.len());
        if lo >= hi {
            return "";
        }
        &source[lo..hi]
    }

    fn lex_tokens(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source, izel_span::SourceId(1));
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            let kind = token.kind;
            tokens.push(token);
            if kind == TokenKind::Eof {
                break;
            }
        }
        tokens
    }

    fn is_keyword_token(kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Forge
                | TokenKind::Shape
                | TokenKind::Scroll
                | TokenKind::Weave
                | TokenKind::Ward
                | TokenKind::Macro
                | TokenKind::Echo
                | TokenKind::Branch
                | TokenKind::Given
                | TokenKind::Else
                | TokenKind::Loop
                | TokenKind::Each
                | TokenKind::While
                | TokenKind::Break
                | TokenKind::Next
                | TokenKind::Give
                | TokenKind::Let
                | TokenKind::Raw
                | TokenKind::Bridge
                | TokenKind::Flow
                | TokenKind::Tide
                | TokenKind::Zone
                | TokenKind::Dual
                | TokenKind::Seek
                | TokenKind::Catch
                | TokenKind::Draw
                | TokenKind::Open
                | TokenKind::Hidden
                | TokenKind::Pkg
                | TokenKind::Pure
                | TokenKind::Sole
                | TokenKind::SelfKw
                | TokenKind::SelfType
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Nil
                | TokenKind::As
                | TokenKind::In
                | TokenKind::Of
                | TokenKind::Is
                | TokenKind::Not
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Comptime
                | TokenKind::Static
                | TokenKind::Extern
                | TokenKind::Type
                | TokenKind::Alias
                | TokenKind::Impl
                | TokenKind::For
                | TokenKind::Bind
        )
    }

    fn is_operator_token(kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Tilde
                | TokenKind::Bang
                | TokenKind::At
                | TokenKind::Pipe
                | TokenKind::Bar
                | TokenKind::DoubleColon
                | TokenKind::Arrow
                | TokenKind::FatArrow
                | TokenKind::DotDot
                | TokenKind::DotDotEq
                | TokenKind::Dot
                | TokenKind::Question
                | TokenKind::QuestionQuestion
                | TokenKind::Pound
                | TokenKind::Equal
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Caret
                | TokenKind::Ampersand
                | TokenKind::AmpersandTilde
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::Le
                | TokenKind::Ge
                | TokenKind::EqEq
                | TokenKind::NotEq
                | TokenKind::OpenParen
                | TokenKind::CloseParen
                | TokenKind::OpenBrace
                | TokenKind::CloseBrace
                | TokenKind::OpenBracket
                | TokenKind::CloseBracket
                | TokenKind::Comma
                | TokenKind::Semicolon
                | TokenKind::Colon
        )
    }

    fn is_significant_token(kind: TokenKind) -> bool {
        !matches!(kind, TokenKind::Whitespace | TokenKind::Comment)
    }

    fn previous_significant_kind(tokens: &[Token], idx: usize) -> Option<TokenKind> {
        for i in (0..idx).rev() {
            let kind = tokens[i].kind;
            if Self::is_significant_token(kind) {
                return Some(kind);
            }
        }
        None
    }

    fn declaration_kind_from_prev(prev_kind: Option<TokenKind>) -> Option<DeclKind> {
        match prev_kind {
            Some(TokenKind::Forge) => Some(DeclKind::Function),
            Some(TokenKind::Shape)
            | Some(TokenKind::Scroll)
            | Some(TokenKind::Weave)
            | Some(TokenKind::Type)
            | Some(TokenKind::Impl)
            | Some(TokenKind::For) => Some(DeclKind::Type),
            Some(TokenKind::Ward) => Some(DeclKind::Module),
            Some(TokenKind::Let) => Some(DeclKind::Variable),
            Some(TokenKind::Macro) => Some(DeclKind::Macro),
            Some(TokenKind::Alias) => Some(DeclKind::Alias),
            _ => None,
        }
    }

    fn completion_kind_for_decl(decl_kind: DeclKind) -> CompletionItemKind {
        match decl_kind {
            DeclKind::Function => CompletionItemKind::FUNCTION,
            DeclKind::Type => CompletionItemKind::CLASS,
            DeclKind::Module => CompletionItemKind::MODULE,
            DeclKind::Variable => CompletionItemKind::VARIABLE,
            DeclKind::Macro => CompletionItemKind::SNIPPET,
            DeclKind::Alias => CompletionItemKind::TYPE_PARAMETER,
        }
    }

    fn symbol_occurrences(source: &str) -> Vec<SymbolOccurrence> {
        let tokens = Self::lex_tokens(source);
        let mut out = Vec::new();

        for (idx, token) in tokens.iter().enumerate() {
            if token.kind != TokenKind::Ident {
                continue;
            }

            let name = Self::source_slice(source, token).to_string();
            if name.is_empty() {
                continue;
            }

            let prev_kind = Self::previous_significant_kind(&tokens, idx);
            let decl_kind = Self::declaration_kind_from_prev(prev_kind);
            let is_definition = decl_kind.is_some();
            let range =
                Self::range_from_bytes(source, token.span.lo.0 as usize, token.span.hi.0 as usize);

            out.push(SymbolOccurrence {
                name,
                range,
                is_definition,
                decl_kind,
            });
        }

        out
    }

    fn symbol_at_position(source: &str, position: Position) -> Option<SymbolOccurrence> {
        let occurrences = Self::symbol_occurrences(source);

        if let Some(found) = occurrences
            .iter()
            .find(|occ| Self::range_contains_position(occ.range, position))
        {
            return Some(found.clone());
        }

        if position.character == 0 {
            return None;
        }

        let mut previous_position = position;
        previous_position.character -= 1;
        occurrences
            .into_iter()
            .find(|occ| Self::range_contains_position(occ.range, previous_position))
    }

    fn find_definitions(source: &str, name: &str) -> Vec<SymbolOccurrence> {
        let occurrences = Self::symbol_occurrences(source);
        let defs = occurrences
            .iter()
            .filter(|occ| occ.name == name && occ.is_definition)
            .cloned()
            .collect::<Vec<_>>();

        if !defs.is_empty() {
            return defs;
        }

        occurrences
            .into_iter()
            .filter(|occ| occ.name == name)
            .take(1)
            .collect()
    }

    fn find_references(
        source: &str,
        name: &str,
        include_declaration: bool,
    ) -> Vec<SymbolOccurrence> {
        Self::symbol_occurrences(source)
            .into_iter()
            .filter(|occ| occ.name == name && (include_declaration || !occ.is_definition))
            .collect()
    }

    fn semantic_token_type_index(kind: TokenKind, prev_kind: Option<TokenKind>) -> Option<u32> {
        if Self::is_keyword_token(kind) {
            return Some(0);
        }

        match kind {
            TokenKind::Ident => match prev_kind {
                Some(TokenKind::Forge) => Some(2),
                Some(TokenKind::Shape)
                | Some(TokenKind::Scroll)
                | Some(TokenKind::Weave)
                | Some(TokenKind::Type)
                | Some(TokenKind::Alias)
                | Some(TokenKind::Impl)
                | Some(TokenKind::For) => Some(3),
                _ => Some(1),
            },
            TokenKind::Str { .. }
            | TokenKind::InterpolatedStr { .. }
            | TokenKind::ByteStr { .. }
            | TokenKind::Char { .. }
            | TokenKind::Byte { .. } => Some(4),
            TokenKind::Int { .. } | TokenKind::Float => Some(5),
            TokenKind::Comment => Some(6),
            _ if Self::is_operator_token(kind) => Some(7),
            _ => None,
        }
    }

    fn build_semantic_tokens(source: &str, requested_range: Option<Range>) -> Vec<SemanticToken> {
        let tokens = Self::lex_tokens(source);
        let mut absolute_tokens = Vec::<(u32, u32, u32, u32)>::new();

        for (idx, token) in tokens.iter().enumerate() {
            if token.kind == TokenKind::Whitespace
                || token.kind == TokenKind::Eof
                || token.kind == TokenKind::Unknown
            {
                continue;
            }

            let prev_kind = Self::previous_significant_kind(&tokens, idx);
            let Some(token_type) = Self::semantic_token_type_index(token.kind, prev_kind) else {
                continue;
            };

            let token_range =
                Self::range_from_bytes(source, token.span.lo.0 as usize, token.span.hi.0 as usize);

            if let Some(range) = requested_range {
                if !Self::ranges_overlap(token_range, range) {
                    continue;
                }
            }

            if token_range.start.line != token_range.end.line {
                continue;
            }

            let length = token_range
                .end
                .character
                .saturating_sub(token_range.start.character);
            if length == 0 {
                continue;
            }

            absolute_tokens.push((
                token_range.start.line,
                token_range.start.character,
                length,
                token_type,
            ));
        }

        absolute_tokens.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

        let mut out = Vec::with_capacity(absolute_tokens.len());
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;
        let mut first = true;

        for (line, start, length, token_type) in absolute_tokens {
            let delta_line = if first { line } else { line - prev_line };
            let delta_start = if first {
                start
            } else if delta_line == 0 {
                start - prev_start
            } else {
                start
            };

            out.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type,
                token_modifiers_bitset: 0,
            });

            prev_line = line;
            prev_start = start;
            first = false;
        }

        out
    }

    fn build_completion_items(source: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen = HashSet::new();

        let keyword_items = [
            ("forge", CompletionItemKind::KEYWORD),
            ("shape", CompletionItemKind::KEYWORD),
            ("scroll", CompletionItemKind::KEYWORD),
            ("weave", CompletionItemKind::KEYWORD),
            ("ward", CompletionItemKind::KEYWORD),
            ("draw", CompletionItemKind::KEYWORD),
            ("let", CompletionItemKind::KEYWORD),
            ("given", CompletionItemKind::KEYWORD),
            ("else", CompletionItemKind::KEYWORD),
            ("while", CompletionItemKind::KEYWORD),
            ("flow", CompletionItemKind::KEYWORD),
            ("tide", CompletionItemKind::KEYWORD),
            ("pure", CompletionItemKind::KEYWORD),
            ("std", CompletionItemKind::MODULE),
        ];

        for (label, kind) in keyword_items {
            if seen.insert(label.to_string()) {
                items.push(CompletionItem {
                    label: label.to_string(),
                    kind: Some(kind),
                    detail: Some("Izel keyword/module".to_string()),
                    ..Default::default()
                });
            }
        }

        for occ in Self::symbol_occurrences(source)
            .into_iter()
            .filter(|occ| occ.is_definition)
        {
            if !seen.insert(occ.name.clone()) {
                continue;
            }

            let kind = occ
                .decl_kind
                .map(Self::completion_kind_for_decl)
                .or(Some(CompletionItemKind::VARIABLE));

            items.push(CompletionItem {
                label: occ.name,
                kind,
                detail: Some("Symbol from current document".to_string()),
                ..Default::default()
            });
        }

        items.sort_by(|a, b| a.label.cmp(&b.label));
        items
    }

    fn build_inlay_hints(source: &str, requested_range: Range) -> Vec<InlayHint> {
        let tokens = Self::lex_tokens(source);
        let mut hints = Vec::new();

        for idx in 0..tokens.len() {
            if tokens[idx].kind != TokenKind::Let {
                continue;
            }

            let ident_idx = ((idx + 1)..tokens.len()).find(|i| {
                let kind = tokens[*i].kind;
                Self::is_significant_token(kind) && kind == TokenKind::Ident
            });

            let Some(ident_idx) = ident_idx else {
                continue;
            };

            let next_kind = ((ident_idx + 1)..tokens.len())
                .map(|i| tokens[i].kind)
                .find(|k| Self::is_significant_token(*k));
            if next_kind == Some(TokenKind::Colon) {
                continue;
            }

            let ident_range = Self::range_from_bytes(
                source,
                tokens[ident_idx].span.lo.0 as usize,
                tokens[ident_idx].span.hi.0 as usize,
            );
            if !Self::range_contains_position(requested_range, ident_range.start) {
                continue;
            }

            hints.push(InlayHint {
                position: ident_range.end,
                label: InlayHintLabel::String(": ?".to_string()),
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: Some(InlayHintTooltip::String(
                    "Type hint placeholder until full inlay inference is implemented".to_string(),
                )),
                padding_left: Some(true),
                padding_right: None,
                data: None,
            });
        }

        hints
    }

    fn is_valid_identifier_name(name: &str) -> bool {
        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            return false;
        };

        if !(first == '_' || first.is_ascii_alphabetic()) {
            return false;
        }

        chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    }

    fn action_kind_allowed(kind: &CodeActionKind, allowed: &Option<Vec<CodeActionKind>>) -> bool {
        match allowed {
            None => true,
            Some(allowed) => allowed
                .iter()
                .any(|requested| kind.as_str().starts_with(requested.as_str())),
        }
    }

    fn build_code_actions_for_document(
        uri: &Url,
        source: &str,
        params: &CodeActionParams,
    ) -> CodeActionResponse {
        let mut actions = Vec::new();

        let formatted = format_source(source);
        if formatted != source
            && Self::action_kind_allowed(&CodeActionKind::SOURCE_FIX_ALL, &params.context.only)
        {
            let mut edits = HashMap::new();
            edits.insert(
                uri.clone(),
                vec![TextEdit {
                    range: Self::full_document_range(source),
                    new_text: formatted,
                }],
            );

            let action = CodeAction {
                title: "Format document".to_string(),
                kind: Some(CodeActionKind::SOURCE_FIX_ALL),
                edit: Some(WorkspaceEdit {
                    changes: Some(edits),
                    ..Default::default()
                }),
                ..Default::default()
            };
            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        for diagnostic in &params.context.diagnostics {
            if !diagnostic.message.contains("requires an initializer") {
                continue;
            }

            if !Self::action_kind_allowed(&CodeActionKind::QUICKFIX, &params.context.only) {
                continue;
            }

            let mut edits = HashMap::new();
            edits.insert(
                uri.clone(),
                vec![TextEdit {
                    range: Range::new(diagnostic.range.end, diagnostic.range.end),
                    new_text: " = 0".to_string(),
                }],
            );

            let action = CodeAction {
                title: "Insert placeholder initializer".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(edits),
                    ..Default::default()
                }),
                ..Default::default()
            };
            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        actions
    }

    fn build_diagnostics(source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        let mut parser = izel_parser::Parser::new(Self::lex_tokens(source), source.to_string());
        let cst = parser.parse_source_file();

        let ast_lowerer = izel_ast_lower::Lowerer::new(source);
        let ast = ast_lowerer.lower_module(&cst);

        let mut typeck = izel_typeck::TypeChecker::new();
        typeck.check_ast(&ast);

        if !typeck.diagnostics.is_empty() {
            for diag in &typeck.diagnostics {
                let mut range = diag
                    .labels
                    .first()
                    .map(|label| Self::range_from_bytes(source, label.range.start, label.range.end))
                    .unwrap_or_else(|| Self::fallback_range(source));

                if range == Range::default() && !source.is_empty() {
                    range = Self::fallback_range(source);
                }

                diagnostics.push(Diagnostic {
                    range,
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

    async fn formatting_edits_for_uri(&self, uri: &Url) -> Option<Vec<TextEdit>> {
        let source = self.get_document(uri).await?;
        let formatted = format_source(&source);

        if formatted == source {
            return None;
        }

        Some(vec![TextEdit {
            range: Self::full_document_range(&source),
            new_text: formatted,
        }])
    }
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
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), "::".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::QUICKFIX,
                            CodeActionKind::SOURCE_FIX_ALL,
                        ]),
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        resolve_provider: Some(false),
                    },
                )),
                inlay_hint_provider: Some(OneOf::Right(InlayHintServerCapabilities::Options(
                    InlayHintOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        resolve_provider: Some(false),
                    },
                ))),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                            legend: Self::semantic_legend(),
                            range: Some(true),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                    ),
                ),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
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

        let uri = params.text_document.uri;
        let source = params.text_document.text;
        self.upsert_document(uri.clone(), source.clone()).await;
        self.validate_document(uri, source).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.pop() {
            let uri = params.text_document.uri;
            self.upsert_document(uri.clone(), change.text.clone()).await;
            self.validate_document(uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.remove_document(&params.text_document.uri).await;

        if let Some(client) = &self.client {
            client
                .publish_diagnostics(params.text_document.uri, vec![], None)
                .await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let response = if let Some(source) = self.get_document(uri).await {
            let symbol = Self::symbol_at_position(&source, position)
                .map(|occ| occ.name)
                .unwrap_or_else(|| "Izel symbol".to_string());

            Hover {
                contents: HoverContents::Scalar(MarkedString::String(format!(
                    "{symbol}\n\nHover endpoint is currently in placeholder mode"
                ))),
                range: None,
            }
        } else {
            Hover {
                contents: HoverContents::Scalar(MarkedString::String("Izel symbol".to_string())),
                range: None,
            }
        };

        Ok(Some(response))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        let Some(symbol) = Self::symbol_at_position(&source, position) else {
            return Ok(None);
        };

        let locations = Self::find_definitions(&source, &symbol.name)
            .into_iter()
            .map(|occ| Location {
                uri: uri.clone(),
                range: occ.range,
            })
            .collect::<Vec<_>>();

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(GotoDefinitionResponse::Array(locations)))
        }
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        let Some(symbol) = Self::symbol_at_position(&source, position) else {
            return Ok(None);
        };

        let locations =
            Self::find_references(&source, &symbol.name, params.context.include_declaration)
                .into_iter()
                .map(|occ| Location {
                    uri: uri.clone(),
                    range: occ.range,
                })
                .collect::<Vec<_>>();

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        let Some(symbol) = Self::symbol_at_position(&source, params.position) else {
            return Ok(None);
        };

        Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: symbol.range,
            placeholder: symbol.name,
        }))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        if !Self::is_valid_identifier_name(&params.new_name) {
            return Err(Error::invalid_params(
                "new_name must be a valid ASCII identifier",
            ));
        }

        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        let Some(symbol) = Self::symbol_at_position(&source, position) else {
            return Ok(None);
        };

        let edits = Self::find_references(&source, &symbol.name, true)
            .into_iter()
            .map(|occ| TextEdit {
                range: occ.range,
                new_text: params.new_name.clone(),
            })
            .collect::<Vec<_>>();

        if edits.is_empty() {
            return Ok(None);
        }

        let mut changes = HashMap::new();
        changes.insert(uri, edits);

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let source = self.get_document(&uri).await.unwrap_or_default();
        let items = Self::build_completion_items(&source);

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri.clone();
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        let actions = Self::build_code_actions_for_document(&uri, &source, &params);
        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        let hints = Self::build_inlay_hints(&source, params.range);
        if hints.is_empty() {
            Ok(None)
        } else {
            Ok(Some(hints))
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: Self::build_semantic_tokens(&source, None),
        })))
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        let uri = params.text_document.uri;
        let Some(source) = self.get_document(&uri).await else {
            return Ok(None);
        };

        Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: None,
            data: Self::build_semantic_tokens(&source, Some(params.range)),
        })))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        Ok(self
            .formatting_edits_for_uri(&params.text_document.uri)
            .await)
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        Ok(self
            .formatting_edits_for_uri(&params.text_document.uri)
            .await)
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(Some(client)));
    Server::new(stdin, stdout, socket).serve(service).await;
}

pub fn run_server_sync() {
    match tokio::runtime::Runtime::new() {
        Ok(rt) => rt.block_on(run_server()),
        Err(err) => eprintln!("failed to start izel_lsp runtime: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn test_uri() -> Url {
        Url::parse("file:///tmp/test.iz").expect("valid test URL")
    }

    fn test_backend() -> Backend {
        Backend::new(None)
    }

    fn formatting_options() -> FormattingOptions {
        FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            properties: HashMap::new(),
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: None,
        }
    }

    #[tokio::test]
    async fn initialize_reports_expected_capabilities() {
        let backend = test_backend();
        let init = backend
            .initialize(InitializeParams::default())
            .await
            .expect("initialize should succeed");

        assert!(matches!(
            init.capabilities.text_document_sync,
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL))
        ));
        assert_eq!(
            init.capabilities.hover_provider,
            Some(HoverProviderCapability::Simple(true))
        );
        assert!(init.capabilities.definition_provider.is_some());
        assert!(init.capabilities.references_provider.is_some());
        assert!(init.capabilities.rename_provider.is_some());
        assert!(init.capabilities.code_action_provider.is_some());
        assert!(init.capabilities.inlay_hint_provider.is_some());
        assert!(init.capabilities.semantic_tokens_provider.is_some());
        assert!(init.capabilities.document_formatting_provider.is_some());
        assert!(init
            .capabilities
            .document_range_formatting_provider
            .is_some());

        let trigger_chars = init
            .capabilities
            .completion_provider
            .and_then(|c| c.trigger_characters)
            .unwrap_or_default();
        assert!(trigger_chars.contains(&".".to_string()));
        assert!(trigger_chars.contains(&"::".to_string()));
    }

    #[tokio::test]
    async fn did_open_change_and_close_manage_document_state() {
        let backend = test_backend();

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

        assert!(backend.get_document(&test_uri()).await.is_some());

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

        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: test_uri() },
            })
            .await;

        assert!(backend.get_document(&test_uri()).await.is_none());
    }

    #[tokio::test]
    async fn hover_definition_references_prepare_rename_and_rename_are_callable() {
        let backend = test_backend();
        let source = "forge add(a: i32) -> i32 { give a }\nforge main() -> i32 { let value = add(1); give value }";
        let uri = test_uri();

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "izel".to_string(),
                    version: 1,
                    text: source.to_string(),
                },
            })
            .await;

        let call_offset = source.find("add(1)").expect("call site should exist");
        let call_position = Backend::byte_to_position(source, call_offset + 1);

        let hover = backend
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: call_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("hover should succeed");
        assert!(hover.is_some());

        let definition = backend
            .goto_definition(GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: call_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("definition should succeed");
        assert!(definition.is_some());

        let refs = backend
            .references(ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: call_position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: ReferenceContext {
                    include_declaration: true,
                },
            })
            .await
            .expect("references should succeed")
            .expect("references should be present");
        assert!(refs.len() >= 2);

        let prepared = backend
            .prepare_rename(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: call_position,
            })
            .await
            .expect("prepare rename should succeed");
        assert!(prepared.is_some());

        let rename_edit = backend
            .rename(RenameParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: call_position,
                },
                new_name: "sum".to_string(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("rename should succeed")
            .expect("rename edits should exist");

        let edits = rename_edit
            .changes
            .and_then(|c| c.get(&uri).cloned())
            .unwrap_or_default();
        assert!(edits.len() >= 2);
    }

    #[tokio::test]
    async fn completion_actions_semantic_tokens_inlay_and_formatting_are_callable() {
        let backend = test_backend();
        let source = "forge main() -> i32 { let x = 1+2; give x }";
        let uri = test_uri();

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "izel".to_string(),
                    version: 1,
                    text: source.to_string(),
                },
            })
            .await;

        let completion = backend
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(0, 0),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            })
            .await
            .expect("completion should succeed");
        assert!(completion.is_some());

        let code_actions = backend
            .code_action(CodeActionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                context: CodeActionContext {
                    diagnostics: vec![Diagnostic {
                        range: Range::new(Position::new(0, 10), Position::new(0, 11)),
                        message: "binding requires an initializer".to_string(),
                        ..Default::default()
                    }],
                    only: None,
                    trigger_kind: None,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("code action should succeed");
        assert!(code_actions.is_some());

        let semantic_full = backend
            .semantic_tokens_full(SemanticTokensParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await
            .expect("semantic full should succeed");
        assert!(semantic_full.is_some());

        let semantic_range = backend
            .semantic_tokens_range(SemanticTokensRangeParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range: Range::new(Position::new(0, 0), Position::new(0, 40)),
            })
            .await
            .expect("semantic range should succeed");
        assert!(semantic_range.is_some());

        let hints = backend
            .inlay_hint(InlayHintParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range: Range::new(Position::new(0, 0), Position::new(0, 60)),
            })
            .await
            .expect("inlay hints should succeed");
        assert!(hints.is_some());

        let formatted = backend
            .formatting(DocumentFormattingParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                options: formatting_options(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("formatting should succeed");
        assert!(formatted.is_some());

        let range_formatted = backend
            .range_formatting(DocumentRangeFormattingParams {
                text_document: TextDocumentIdentifier { uri },
                range: Range::new(Position::new(0, 0), Position::new(0, 20)),
                options: formatting_options(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("range formatting should succeed");
        assert!(range_formatted.is_some());
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
        assert!(
            bad.iter().any(|d| d.range != Range::default()),
            "invalid echo should produce mapped source ranges"
        );
    }

    #[tokio::test]
    async fn validate_document_noops_without_client() {
        let backend = test_backend();
        backend
            .validate_document(test_uri(), "shape Ready {}".to_string())
            .await;
    }

    #[tokio::test]
    async fn validate_document_publishes_with_client() {
        let (service, _socket) = LspService::new(|client| Backend::new(Some(client)));

        service
            .inner()
            .validate_document(test_uri(), "echo { let x }".to_string())
            .await;
    }
}
