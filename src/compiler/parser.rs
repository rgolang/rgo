use std::collections::VecDeque;
use std::io::BufRead;

use crate::compiler::ast::{
    Block, Ident, IntLiteral, Item, Lambda, Param, Params, StrLiteral, Term, TypeRef,
};
use crate::compiler::builtins;
use crate::compiler::error::{CompileError, ParseError};
use crate::compiler::lexer::Lexer;
use crate::compiler::span::Span;
use crate::compiler::symbol::{FunctionSig, SymbolRegistry};
use crate::compiler::token::{Token, TokenKind};

pub struct Parser<R: BufRead> {
    lexer: Lexer<R>,
    peeked: VecDeque<Token>,
    allow_top_imports: bool,
    allow_partial_application: bool, //  TODO: what's this?
    token_recordings: Vec<Vec<Token>>,
}

#[derive(Copy, Clone)]
enum ParamContext {
    Params,
    Lambda,
}

impl<R: BufRead> Parser<R> {
    pub fn new(lexer: Lexer<R>) -> Self {
        Self {
            lexer,
            peeked: VecDeque::new(),
            allow_top_imports: true,
            allow_partial_application: false,
            token_recordings: Vec::new(),
        }
    }

    pub fn next(&mut self, symbols: &mut SymbolRegistry) -> Result<Option<Item>, CompileError> {
        loop {
            let token = self.peek_token()?.clone();
            let span: Span = token.span;
            match token.kind {
                TokenKind::Eof => return Ok(None),
                TokenKind::Import(name) => {
                    if !self.allow_top_imports {
                        return Err(ParseError::new(
                            "@ imports must appear before any other items",
                            token.span,
                        )
                        .into());
                    }
                    self.bump()?; // consume import token
                    builtins::register_import(&name, span, symbols)?;
                    return Ok(Some(Item::Import { name, span }));
                }
                _ => {
                    self.allow_top_imports = false;
                    let item = self.parse_block_item(symbols)?;
                    self.consume_optional_semicolons()?;
                    return Ok(Some(item));
                }
            }
        }
    }

    fn parse_block_item(&mut self, symbols: &mut SymbolRegistry) -> Result<Item, CompileError> {
        let token = self.peek_token()?.clone();
        let span: Span = token.span;
        match token.kind {
            TokenKind::Ident(name) => {
                let ident = self.bump()?; // Might be the name
                if matches!(self.peek_token()?.kind, TokenKind::Colon) {
                    // name: ... → declaration
                    self.bump()?; // consume colon
                    return self.parse_bind(name, span, symbols);
                }

                // Must be an invocation
                self.peeked.push_front(ident); // restore token to attempt invocation parse
            }
            TokenKind::LParen => {
                if let Some(scope_capture) = self.try_parse_scope_capture(symbols)? {
                    return Ok(scope_capture);
                }
            }
            TokenKind::LBrace => {
                // allow lambda invocation (possibly without args)
            }
            _ => return Err(ParseError::new("expected a top-level item", span).into()),
        }

        let term = self.parse_invocation_term(symbols)?;
        match term {
            Term::String(literal) => {
                return Err(
                    ParseError::new("string literals cannot be called yet", literal.span).into(),
                );
            }
            Term::Int(literal) => {
                return Err(
                    ParseError::new("int literals cannot be called yet", literal.span).into(),
                );
            }
            Term::Ident(ident) => Ok(Item::Ident(ident)),
            Term::Lambda(lambda) => Ok(Item::Lambda(lambda)),
        }
    }

    fn try_parse_scope_capture(
        &mut self,
        symbols: &mut SymbolRegistry,
    ) -> Result<Option<Item>, CompileError> {
        let (params, recorded_tokens) =
            self.with_token_recording(|parser| parser.parse_params(symbols, ParamContext::Lambda))?;
        if matches!(self.peek_token()?.kind, TokenKind::Equals) {
            self.bump()?; // consume '='
            let prev = self.allow_partial_application;
            self.allow_partial_application = true;
            let (term, term_tokens) =
                self.with_token_recording(|parser| parser.parse_term(symbols))?;
            self.allow_partial_application = prev;
            if matches!(self.peek_token()?.kind, TokenKind::Equals) {
                self.reinsert_trailing_empty_parens(&term_tokens)?;
            }
            let capture_span = params.span;
            return Ok(Some(Item::ScopeCapture {
                params,
                term,
                span: capture_span,
            }));
        }

        for token in recorded_tokens.into_iter().rev() {
            self.peeked.push_front(token);
        }
        Ok(None)
    }

    fn reinsert_trailing_empty_parens(&mut self, tokens: &[Token]) -> Result<(), CompileError> {
        let mut len = tokens.len();
        while len >= 2 {
            if matches!(tokens[len - 2].kind, TokenKind::LParen)
                && matches!(tokens[len - 1].kind, TokenKind::RParen)
            {
                self.peeked.push_front(tokens[len - 1].clone());
                self.peeked.push_front(tokens[len - 2].clone());
                len -= 2;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn parse_invocation_term(
        &mut self,
        symbols: &mut SymbolRegistry,
    ) -> Result<Term, CompileError> {
        let prev = self.allow_partial_application;
        self.allow_partial_application = true;
        let result = self.parse_term(symbols);
        self.allow_partial_application = prev;
        result
    }

    fn parse_term(&mut self, symbols: &mut SymbolRegistry) -> Result<Term, CompileError> {
        let mut term = self.parse_head(symbols)?;

        while matches!(self.peek_token()?.kind, TokenKind::LParen) {
            if self.next_tokens_indicate_scope_capture()? {
                break;
            }
            let lparen = self.bump()?;
            let args = self.parse_argument_list(symbols)?;

            match &mut term {
                Term::Ident(ident) => {
                    if let Some(sig) = symbols.get_function(&ident.name) {
                        let expected = sig.params.len();
                        let got = ident.args.len() + args.len();

                        if !self.allow_partial_application && expected != got {
                            return Err(ParseError::new(
                                format!(
                                    "function '{}' expects {} arguments but got {}",
                                    ident.name, expected, got
                                ),
                                lparen.span,
                            )
                            .into());
                        }
                    }
                    ident.args.extend(args);
                }
                Term::Lambda(lambda) => {
                    lambda.args.extend(args);
                }
                _ => {
                    return Err(ParseError::new(
                        "expected identifier or lambda before argument list",
                        lparen.span,
                    )
                    .into());
                }
            }
        }

        Ok(term)
    }

    fn next_tokens_indicate_scope_capture(&mut self) -> Result<bool, CompileError> {
        if !matches!(self.peek_token()?.kind, TokenKind::LParen) {
            return Ok(false);
        }
        if let TokenKind::Ident(_) = &self.peek_nth_token(1)?.kind {
            if matches!(self.peek_nth_token(2)?.kind, TokenKind::Colon) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn peek_nth_token(&mut self, index: usize) -> Result<&Token, CompileError> {
        while self.peeked.len() <= index {
            let token = self.lexer.next_token().map_err(CompileError::from)?;
            self.peeked.push_back(token);
        }
        Ok(&self.peeked[index])
    }

    // parse_head parses primary terms: literals, variables, and lambdas before any curried args.
    fn parse_head(&mut self, symbols: &mut SymbolRegistry) -> Result<Term, CompileError> {
        let token = self.bump()?;
        match token.kind {
            TokenKind::IntLiteral(value) => Ok(Term::Int(IntLiteral {
                value,
                span: token.span,
            })),
            TokenKind::StringLiteral(value) => Ok(Term::String(StrLiteral {
                value,
                span: token.span,
            })),
            TokenKind::Ident(name) => Ok(Term::Ident(Ident {
                name,
                args: Vec::new(),
                span: token.span,
            })),
            TokenKind::LParen => {
                // ( ... ) { ... } → lambda with params
                self.peeked.push_front(token.clone());
                let params = self.parse_params(symbols, ParamContext::Lambda)?;
                let brace = self.expect_token("{", |kind| matches!(kind, TokenKind::LBrace))?;
                let body = self.parse_body(symbols, brace.span)?;
                Ok(Term::Lambda(Lambda {
                    params,
                    body,
                    args: Vec::new(),
                    span: token.span,
                }))
            }
            TokenKind::LBrace => {
                // { ... } → lambda without explicit params
                let params = Params {
                    items: Vec::new(),
                    span: token.span,
                };
                let body = self.parse_body(symbols, token.span)?;
                Ok(Term::Lambda(Lambda {
                    params,
                    body,
                    args: Vec::new(),
                    span: token.span,
                }))
            }
            _ => Err(ParseError::new("unexpected token", token.span).into()),
        }
    }

    fn parse_argument_list(
        &mut self,
        symbols: &mut SymbolRegistry,
    ) -> Result<Vec<Term>, CompileError> {
        let mut args = Vec::new();
        if matches!(self.peek_token()?.kind, TokenKind::RParen) {
            self.bump()?;
            return Ok(args);
        }
        loop {
            args.push(self.parse_term(symbols)?);
            if self
                .consume_if(|kind| matches!(kind, TokenKind::Comma))?
                .is_some()
            {
                continue;
            }
            break;
        }
        self.expect_token(")", |kind| matches!(kind, TokenKind::RParen))?;
        Ok(args)
    }

    fn parse_param(
        &mut self,
        symbols: &mut SymbolRegistry,
        context: ParamContext,
    ) -> Result<Param, CompileError> {
        let is_variadic = self.consume_variadic_marker()?;
        let token = self.peek_token()?.clone();

        if matches!(token.kind, TokenKind::Ident(_)) {
            let (name, name_span) = self.parse_identifier("parameter name")?;
            if matches!(self.peek_token()?.kind, TokenKind::Colon) {
                self.expect_token(":", |kind| matches!(kind, TokenKind::Colon))?;
                let ty = self.parse_type_ref(symbols)?;
                return Ok(Param::NameAndType {
                    name,
                    ty,
                    span: name_span,
                    is_variadic,
                });
            }

            match context {
                ParamContext::Params => {
                    self.peeked.push_front(token.clone());
                    let ty = self.parse_type_ref(symbols)?;
                    return Ok(Param::TypeOnly {
                        ty,
                        span: name_span,
                        is_variadic,
                    });
                }
                ParamContext::Lambda => {
                    return Ok(Param::NameOnly {
                        name,
                        span: name_span,
                        is_variadic,
                    });
                }
            }
        }

        let span = token.span;
        let ty = self.parse_type_ref(symbols)?;
        Ok(Param::TypeOnly {
            ty,
            span,
            is_variadic,
        })
    }

    fn consume_variadic_marker(&mut self) -> Result<bool, CompileError> {
        if matches!(self.peek_token()?.kind, TokenKind::Ellipsis) {
            self.bump()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn collect_param_types(params: &[Param]) -> Result<Vec<TypeRef>, CompileError> {
        params
            .iter()
            .map(|param| {
                param
                    .ty()
                    .cloned()
                    .ok_or_else(|| ParseError::new("expected parameter type", param.span()).into())
            })
            .collect()
    }

    fn parse_bind(
        &mut self,
        name: String,
        name_span: Span,
        symbols: &mut SymbolRegistry,
    ) -> Result<Item, CompileError> {
        let next_token = self.peek_token()?.clone();
        let params_span = next_token.span;
        let has_head = matches!(next_token.kind, TokenKind::LParen);
        let has_brace = matches!(next_token.kind, TokenKind::LBrace);

        if has_head || has_brace {
            symbols.install_type(
                name.clone(),
                TypeRef::Alias(name.clone()),
                name_span,
                Vec::new(),
            )?;
            let params = if has_head {
                self.parse_params(symbols, ParamContext::Params)?
            } else {
                Params {
                    items: Vec::new(),
                    span: params_span,
                }
            };
            let param_variadic = params
                .items
                .iter()
                .map(|param| param.is_variadic())
                .collect::<Vec<bool>>();

            if matches!(self.peek_token()?.kind, TokenKind::LBrace) {
                // FUNCTION CASE
                symbols.remove_type(&name);
                let param_types = Self::collect_param_types(&params.items)?;
                let sig = FunctionSig {
                    name: name.clone(),
                    params: param_types.clone(),
                    is_variadic: param_variadic.clone(),
                    span: name_span,
                };
                symbols.declare_function(sig)?;

                let brace = self.expect_token("{", |k| matches!(k, TokenKind::LBrace))?;
                let body = self.parse_body(symbols, brace.span)?;

                let lambda = Lambda {
                    params,
                    body,
                    args: Vec::new(),
                    span: name_span,
                };

                return Ok(Item::FunctionDef {
                    name,
                    lambda,
                    span: name_span,
                });
            }

            if has_head {
                let param_types = Self::collect_param_types(&params.items)?;
                let target = TypeRef::Type(param_types);
                symbols.update_type(&name, target.clone(), param_variadic.clone())?;
                return Ok(Item::TypeDef {
                    name,
                    term: target,
                    span: name_span,
                });
            }
        }

        // CASE 2 — alias or literal (no params or body block)
        let term = {
            let prev = self.allow_partial_application;
            self.allow_partial_application = true;
            let result = self.parse_term(symbols);
            self.allow_partial_application = prev;
            result
        }?;
        let term_span = term.span();
        return match term {
            Term::String(literal) => Ok(Item::StrDef {
                name,
                literal,
                span: name_span,
            }),
            Term::Int(literal) => Ok(Item::IntDef {
                name,
                literal,
                span: name_span,
            }),
            Term::Ident(ident) => Ok(Item::IdentDef {
                name,
                ident,
                span: name_span,
            }),
            _ => Err(ParseError::new(
                "expected a literal or identifier alias on the right-hand side",
                term_span,
            )
            .into()),
        };
    }

    fn parse_type_ref(&mut self, symbols: &mut SymbolRegistry) -> Result<TypeRef, CompileError> {
        let token = self.bump()?;
        match token.kind {
            TokenKind::LParen => {
                let mut args = Vec::new();
                let mut variadics = Vec::new();
                if !matches!(self.peek_token()?.kind, TokenKind::RParen) {
                    loop {
                        let is_variadic = self.consume_variadic_marker()?;
                        if matches!(self.peek_token()?.kind, TokenKind::Ident(_)) {
                            let ident_token = self.bump()?;
                            if self
                                .consume_if(|kind| matches!(kind, TokenKind::Colon))?
                                .is_some()
                            {
                                args.push(self.parse_type_ref(symbols)?);
                            } else {
                                self.peeked.push_front(ident_token);
                                args.push(self.parse_type_ref(symbols)?);
                            }
                            variadics.push(is_variadic);
                        } else {
                            args.push(self.parse_type_ref(symbols)?);
                            variadics.push(is_variadic);
                        }
                        if self
                            .consume_if(|kind| matches!(kind, TokenKind::Comma))?
                            .is_some()
                        {
                            continue;
                        }
                        break;
                    }
                }
                self.expect_token(")", |kind| matches!(kind, TokenKind::RParen))?;
                let ty = TypeRef::Type(args);
                symbols.record_type_variadic(ty.clone(), variadics);
                Ok(ty)
            }
            TokenKind::Ident(name) => {
                if matches!(self.peek_token()?.kind, TokenKind::Bang) {
                    if name == "str" {
                        if !symbols.builtin_imports().contains("str") {
                            return Err(ParseError::new(
                                "type 'str!' requires @str import",
                                token.span,
                            )
                            .into());
                        }
                        self.bump()?; // consume '!'
                        return Ok(TypeRef::CompileTimeStr);
                    }
                    if name == "int" {
                        if !symbols.builtin_imports().contains("int") {
                            return Err(ParseError::new(
                                "type 'int!' requires @int import",
                                token.span,
                            )
                            .into());
                        }
                        self.bump()?; // consume '!'
                        return Ok(TypeRef::CompileTimeInt);
                    }
                    return Err(ParseError::new("unexpected '!'", token.span).into());
                }
                if let Some(ty) = symbols.resolve_type(&name) {
                    Ok(ty)
                } else {
                    Err(ParseError::new(format!("unknown type '{}'", name), token.span).into())
                }
            }
            _ => Err(ParseError::new("expected a type", token.span).into()),
        }
    }

    fn parse_params(
        &mut self,
        symbols: &mut SymbolRegistry,
        context: ParamContext,
    ) -> Result<Params, CompileError> {
        let lparen = self.expect_token("(", |k| matches!(k, TokenKind::LParen))?;

        let mut params = Vec::new();
        loop {
            if matches!(self.peek_token()?.kind, TokenKind::RParen) {
                break;
            }

            params.push(self.parse_param(symbols, context)?);

            if self
                .consume_if(|kind| matches!(kind, TokenKind::Comma))?
                .is_none()
            {
                break;
            }
        }

        self.expect_token(")", |k| matches!(k, TokenKind::RParen))?;
        Ok(Params {
            items: params,
            span: lparen.span,
        })
    }

    fn parse_identifier(&mut self, expected: &str) -> Result<(String, Span), CompileError> {
        let token = self.bump()?;
        match token.kind {
            TokenKind::Ident(name) => Ok((name, token.span)),
            _ => Err(ParseError::new(format!("expected {}", expected), token.span).into()),
        }
    }

    fn expect_token<F>(&mut self, expected: &str, predicate: F) -> Result<Token, CompileError>
    where
        F: Fn(&TokenKind) -> bool,
    {
        let token = self.bump()?;
        if predicate(&token.kind) {
            Ok(token)
        } else {
            Err(ParseError::new(format!("expected {}", expected), token.span).into())
        }
    }

    fn consume_if<F>(&mut self, predicate: F) -> Result<Option<Token>, CompileError>
    where
        F: Fn(&TokenKind) -> bool,
    {
        if predicate(&self.peek_token()?.kind) {
            Ok(Some(self.bump()?))
        } else {
            Ok(None)
        }
    }

    fn bump(&mut self) -> Result<Token, CompileError> {
        let token = if let Some(token) = self.peeked.pop_front() {
            token
        } else {
            self.lexer.next_token().map_err(CompileError::from)?
        };
        if let Some(recordings) = self.token_recordings.last_mut() {
            recordings.push(token.clone());
        }
        Ok(token)
    }

    fn with_token_recording<F, Res>(&mut self, f: F) -> Result<(Res, Vec<Token>), CompileError>
    where
        F: FnOnce(&mut Self) -> Result<Res, CompileError>,
    {
        self.token_recordings.push(Vec::new());
        let result = f(self);
        let recorded = self
            .token_recordings
            .pop()
            .expect("recording stack should not be empty");
        result.map(|value| (value, recorded))
    }

    fn peek_token(&mut self) -> Result<&Token, CompileError> {
        if self.peeked.is_empty() {
            let token = self.lexer.next_token().map_err(CompileError::from)?;
            self.peeked.push_back(token);
        }
        Ok(self.peeked.front().expect("peeked token exists"))
    }

    fn parse_body(
        &mut self,
        symbols: &mut SymbolRegistry,
        start_span: Span,
    ) -> Result<Block, CompileError> {
        (|| {
            let mut items = Vec::new();
            loop {
                let token = self.peek_token()?;
                match token.kind {
                    TokenKind::RBrace => {
                        self.bump()?;
                        return Ok(Block {
                            items,
                            span: start_span,
                        });
                    }
                    TokenKind::Eof => {
                        return Err(ParseError::new(
                            "unexpected EOF inside continuation body",
                            token.span,
                        )
                        .into())
                    }
                    _ => {
                        let item = self.parse_block_item(symbols)?;
                        self.consume_optional_semicolons()?;
                        items.push(item);
                    }
                }
            }
        })()
    }

    fn consume_optional_semicolons(&mut self) -> Result<(), CompileError> {
        while self
            .consume_if(|kind| matches!(kind, TokenKind::Semicolon))?
            .is_some()
        {}
        Ok(())
    }
}
