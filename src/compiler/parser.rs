use std::collections::{HashSet, VecDeque};
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
    generic_param_stack: Vec<Vec<String>>,
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
            generic_param_stack: Vec::new(),
        }
    }

    pub fn next(&mut self, symbols: &mut SymbolRegistry) -> Result<Option<Item>, CompileError> {
        loop {
            self.skip_newlines()?;
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
                    let is_libc = builtins::is_libc_import(&name);
                    return Ok(Some(Item::Import {
                        name,
                        span,
                        is_libc,
                    }));
                }
                _ => {
                    self.allow_top_imports = false;
                    let item = self.parse_block_item(symbols)?;
                    self.consume_block_item_separators()?;
                    return Ok(Some(item));
                }
            }
        }
    }

    fn skip_newlines(&mut self) -> Result<(), CompileError> {
        while self
            .consume_if(|k| matches!(k, TokenKind::Newline))?
            .is_some()
        {}
        Ok(())
    }

    fn parse_block_item(&mut self, symbols: &mut SymbolRegistry) -> Result<Item, CompileError> {
        self.skip_newlines()?;
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

                // Must be an exec
                self.peeked.push_front(ident); // restore token to attempt exec parse
            }
            TokenKind::LParen => {
                return self.parse_lambda_or_scope_capture(symbols);
            }
            TokenKind::LBrace => {} // allow lambda exec (possibly without args)
            TokenKind::Newline => {}
            _ => return Err(ParseError::new("expected a top-level item", span).into()),
        }

        let term = self.parse_term(symbols)?;
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

    fn parse_bind(
        &mut self,
        name: String,
        name_span: Span,
        symbols: &mut SymbolRegistry,
    ) -> Result<Item, CompileError> {
        let generics = self.parse_generic_params()?;
        let next_token = self.peek_token()?.clone();
        let params_span = next_token.span;
        let has_head = matches!(next_token.kind, TokenKind::LParen);
        let has_brace = matches!(next_token.kind, TokenKind::LBrace);

        if has_brace && !generics.is_empty() {
            return Err(ParseError::new(
                "generics are only supported on type aliases",
                next_token.span,
            )
            .into());
        }

        if has_head || has_brace {
            symbols.install_type(
                name.clone(),
                TypeRef::Alias(name.clone()),
                name_span,
                Vec::new(),
                generics.clone(),
            )?;
            let params = if has_head {
                self.with_generic_scope(&generics, |parser| {
                    parser.parse_params(symbols, ParamContext::Params)
                })?
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

        // Case 2: alias or literal (no params or body block)
        let term = self.parse_term(symbols)?;
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

    fn parse_lambda_or_scope_capture(
        &mut self,
        symbols: &mut SymbolRegistry,
    ) -> Result<Item, CompileError> {
        // 1. Parse params ALWAYS
        let params = self.parse_params(symbols, ParamContext::Lambda)?;

        // 2. Decide based on the next token
        match self.peek_token()?.kind {
            TokenKind::Equals => {
                self.bump()?; // consume '='
                let term = self.parse_term(symbols)?;
                Ok(Item::ScopeCapture {
                    params: params.clone(),
                    term,
                    span: params.span,
                })
            }
            TokenKind::LBrace => {
                // Parse lambda body as a term
                let mut term = self.parse_term(symbols)?;

                match &mut term {
                    Term::Lambda(lambda) => {
                        // attach params parsed earlier
                        lambda.params = params;

                        Ok(Item::Lambda(lambda.clone()))
                    }
                    _ => Err(ParseError::new(
                        "expected lambda body after parameter list",
                        params.span,
                    )
                    .into()),
                }
            }

            _ => {
                Err(ParseError::new("expected '=' or '{' after parameter list", params.span).into())
            }
        }
    }

    fn parse_term(&mut self, symbols: &mut SymbolRegistry) -> Result<Term, CompileError> {
        let mut term = self.parse_head(symbols)?;

        while matches!(self.peek_token()?.kind, TokenKind::LParen) {
            let lparen = self.bump()?; // consume '('
            let args = self.parse_argument_list(symbols)?;

            match &mut term {
                Term::Ident(ident) => {
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

    // parse_head parses primary terms: literals, variables, and lambdas before any curried args.
    fn parse_head(&mut self, symbols: &mut SymbolRegistry) -> Result<Term, CompileError> {
        self.skip_newlines()?;
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
            _ => Err(
                ParseError::new(&format!("unexpected token: {:?}", token.kind), token.span).into(),
            ),
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

    fn parse_generic_params(&mut self) -> Result<Vec<String>, CompileError> {
        if !matches!(self.peek_token()?.kind, TokenKind::AngleOpen) {
            return Ok(Vec::new());
        }
        let lt = self.expect_token("<", |kind| matches!(kind, TokenKind::AngleOpen))?;
        let mut params = Vec::new();
        let mut seen = HashSet::new();
        loop {
            let (name, span) = self.parse_identifier("generic parameter name")?;
            if !seen.insert(name.clone()) {
                return Err(ParseError::new(
                    format!("generic parameter '{}' already declared", name),
                    span,
                )
                .into());
            }
            params.push(name);
            if self
                .consume_if(|kind| matches!(kind, TokenKind::Comma))?
                .is_none()
            {
                break;
            }
        }
        self.expect_token(">", |kind| matches!(kind, TokenKind::AngleClose))?;
        if params.is_empty() {
            return Err(ParseError::new("expected at least one generic parameter", lt.span).into());
        }
        Ok(params)
    }

    fn with_generic_scope<F, Res>(&mut self, params: &[String], f: F) -> Result<Res, CompileError>
    where
        F: FnOnce(&mut Self) -> Result<Res, CompileError>,
    {
        self.generic_param_stack.push(params.to_vec());
        let result = f(self);
        self.generic_param_stack.pop();
        result
    }

    fn is_generic_param(&self, name: &str) -> bool {
        self.generic_param_stack
            .iter()
            .rev()
            .any(|scope| scope.iter().any(|param| param == name))
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

    fn parse_type_arguments(
        &mut self,
        symbols: &mut SymbolRegistry,
    ) -> Result<Vec<TypeRef>, CompileError> {
        self.expect_token("<", |kind| matches!(kind, TokenKind::AngleOpen))?;
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type_ref(symbols)?);
            if self
                .consume_if(|kind| matches!(kind, TokenKind::Comma))?
                .is_none()
            {
                break;
            }
        }
        self.expect_token(">", |kind| matches!(kind, TokenKind::AngleClose))?;
        Ok(args)
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
                if self.is_generic_param(&name) {
                    return Ok(TypeRef::Generic(name));
                }
                if matches!(self.peek_token()?.kind, TokenKind::Bang) {
                    if name == "str" {
                        if !symbols.builtin_imports().contains("str") {
                            return Err(ParseError::new(
                                "type 'str!' requires @/str import",
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
                                "type 'int!' requires @/int import",
                                token.span,
                            )
                            .into());
                        }
                        self.bump()?; // consume '!'
                        return Ok(TypeRef::CompileTimeInt);
                    }
                    return Err(ParseError::new("unexpected '!'", token.span).into());
                }
                let resolved_type = symbols.resolve_type(&name);
                if matches!(self.peek_token()?.kind, TokenKind::AngleOpen) {
                    let args = self.parse_type_arguments(symbols)?;
                    if let Some(info) = symbols.get_type_info(&name) {
                        if info.generics.len() != args.len() {
                            return Err(ParseError::new(
                                format!(
                                    "type '{}' expects {} generic arguments but got {}",
                                    name,
                                    info.generics.len(),
                                    args.len()
                                ),
                                token.span,
                            )
                            .into());
                        }
                    } else if resolved_type.is_some() {
                        return Err(ParseError::new(
                            format!("type '{}' is not generic", name),
                            token.span,
                        )
                        .into());
                    } else {
                        return Err(ParseError::new(
                            format!("unknown type '{}'", name),
                            token.span,
                        )
                        .into());
                    }
                    return Ok(TypeRef::AliasInstance { name, args });
                }
                if let Some(ty) = resolved_type {
                    if let TypeRef::Alias(alias_name) = &ty {
                        if let Some(info) = symbols.get_type_info(alias_name) {
                            if !info.generics.is_empty() {
                                return Err(ParseError::new(
                                    format!("generic type '{}' must be specialized", alias_name),
                                    token.span,
                                )
                                .into());
                            }
                        }
                    }
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
        if let Some(token) = self.peeked.pop_front() {
            return Ok(token);
        }
        self.lexer.next_token().map_err(CompileError::from)
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
                        if items.is_empty() {
                            return Err(ParseError::new(
                                "block must contain at least one item",
                                token.span,
                            )
                            .into());
                        }
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
                        .into());
                    }
                    _ => {
                        let item = self.parse_block_item(symbols)?;
                        self.consume_block_item_separators()?;
                        items.push(item);
                    }
                }
            }
        })()
    }

    fn consume_block_item_separators(&mut self) -> Result<(), CompileError> {
        while self
            .consume_if(|kind| matches!(kind, TokenKind::Semicolon | TokenKind::Newline))?
            .is_some()
        {}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::Lexer;
    use crate::compiler::symbol::SymbolRegistry;
    use std::io::Cursor;

    #[test]
    fn parse_body_rejects_empty_block() {
        let mut parser = Parser::new(Lexer::new(Cursor::new("{}")));
        let mut symbols = SymbolRegistry::new();
        let brace = parser
            .expect_token("{", |kind| matches!(kind, TokenKind::LBrace))
            .expect("expected opening brace");
        let err = parser
            .parse_body(&mut symbols, brace.span)
            .expect_err("empty block must fail");
        assert!(
            err.to_string()
                .contains("block must contain at least one item"),
            "unexpected error: {err}"
        );
    }
}
