use std::collections::{HashSet, VecDeque};
use std::io::BufRead;

use crate::compiler::ast;
use crate::compiler::ast::{
    Block, BlockItem, Ident, Lambda, Literal, SigIdent, SigItem, SigKind, Signature, Term,
};
use crate::compiler::error::{Code, Error};
use crate::compiler::lexer::Lexer;
use crate::compiler::span::Span;
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

    // TODO: iter
    // fn iter<'a>(
    //     &'a mut self,
    //     symbols: &'a mut SymbolRegistry,
    // ) -> impl Iterator<Item = Result<BlockItem, CompileError>> + 'a {
    //     std::iter::from_fn(move || match self.next(symbols) {
    //         Ok(Some(item)) => Some(Ok(item)),
    //         Ok(None) => None,
    //         Err(e) => Some(Err(e)),
    //     })
    // }

    pub fn next(&mut self) -> Result<Option<BlockItem>, Error> {
        loop {
            self.skip_newlines()?;
            let token = self.peek_token()?.clone();
            match token.kind {
                TokenKind::Eof => return Ok(None),
                TokenKind::Import(_) => {
                    return Err(Error::new(
                        Code::Parse,
                        "imports must have a label (e.g. `printf: @/printf`)",
                        token.span,
                    )
                    .into());
                }
                _ => {
                    let item = self.parse_block_item()?;
                    let item_span = item.span();
                    let is_import = matches!(&item, BlockItem::Import { .. });
                    if is_import {
                        if !self.allow_top_imports {
                            return Err(Error::new(
                                Code::Parse,
                                "@ imports must appear before any other items",
                                item_span,
                            )
                            .into());
                        }
                    } else {
                        self.allow_top_imports = false;
                    }
                    self.consume_block_item_separators()?;
                    return Ok(Some(item));
                }
            }
        }
    }

    fn skip_newlines(&mut self) -> Result<(), Error> {
        while self
            .consume_if(|k| matches!(k, TokenKind::Newline))?
            .is_some()
        {}
        Ok(())
    }

    fn parse_block_item(&mut self) -> Result<BlockItem, Error> {
        self.skip_newlines()?;
        let token = self.peek_token()?.clone();
        let span: Span = token.span;
        match token.kind {
            TokenKind::Ident(name) => {
                let ident = self.bump()?; // Might be the name
                if matches!(self.peek_token()?.kind, TokenKind::Colon) {
                    self.bump()?; // consume colon
                    let next = self.peek_token()?.clone();
                    if let TokenKind::Import(path) = next.kind {
                        self.bump()?; // consume import token
                        return Ok(BlockItem::Import {
                            label: name.clone(),
                            path,
                            span,
                        });
                    }
                    // name: ... → declaration
                    return self.parse_bind(name, span);
                }

                // Must be an exec
                self.peeked.push_front(ident); // restore token to attempt exec parse
            }
            TokenKind::LParen => {
                return self.parse_lambda_or_scope_capture();
            }
            TokenKind::LBrace => {} // allow lambda exec (possibly without args)
            TokenKind::Newline => {}
            _ => return Err(Error::new(Code::Parse, "expected a top-level item", span)),
        }

        let term = self.parse_term()?;
        match term {
            Term::Lit(literal) => {
                return Err(Error::new(
                    Code::Parse,
                    "literals cannot be called yet",
                    literal.span,
                ));
            }
            Term::Ident(ident) => Ok(BlockItem::Ident(ident)),
            Term::Lambda(lambda) => Ok(BlockItem::Lambda(lambda)),
        }
    }

    fn parse_bind(&mut self, name: String, name_span: Span) -> Result<BlockItem, Error> {
        let generics = self.parse_generic_params()?;
        let next_token = self.peek_token()?.clone();
        let params_span = next_token.span;
        let has_head = matches!(next_token.kind, TokenKind::LParen);
        let has_brace = matches!(next_token.kind, TokenKind::LBrace);

        if has_brace && !generics.is_empty() {
            return Err(Error::new(
                Code::Parse,
                "generics are only supported on type aliases",
                next_token.span,
            )
            .into());
        }

        if has_head || has_brace {
            let params = if has_head {
                self.with_generic_scope(&generics, |parser| {
                    parser.parse_params(ParamContext::Params)
                })?
            } else {
                Signature {
                    items: Vec::new(),
                    span: params_span,
                    generics: Vec::new(),
                }
            };

            if matches!(self.peek_token()?.kind, TokenKind::LBrace) {
                // FUNCTION CASE

                let brace = self.expect_token("{", |k| matches!(k, TokenKind::LBrace))?;
                let body = self.parse_body(brace.span)?;
                self.expect_token("}", |k| matches!(k, TokenKind::RBrace))?;
                let lambda = Lambda {
                    params,
                    body,
                    args: Vec::new(),
                    span: name_span,
                };

                return Ok(BlockItem::FunctionDef {
                    name,
                    lambda,
                    span: name_span,
                });
            }

            if has_head {
                let param_types = Self::collect_param_kinds(&params.items)?;
                let mut target = Signature::from_kinds(param_types, params_span);
                target.generics = generics.clone();
                return Ok(BlockItem::SigDef {
                    name,
                    sig: target,
                    span: name_span,
                });
            }
        }

        // Case 2: alias or literal (no params or body block)
        let term = self.parse_term()?;
        let term_span = term.span();
        return match term {
            Term::Lit(literal) => Ok(BlockItem::LitDef {
                name,
                literal,
                span: name_span,
            }),
            Term::Ident(ident) => Ok(BlockItem::IdentDef {
                name,
                ident,
                span: name_span,
            }),
            _ => Err(Error::new(
                Code::Parse,
                "expected a literal or identifier alias on the right-hand side",
                term_span,
            )
            .into()),
        };
    }

    fn parse_lambda_or_scope_capture(&mut self) -> Result<BlockItem, Error> {
        // 1. Parse params ALWAYS
        let params = self.parse_params(ParamContext::Lambda)?;

        // 2. Decide based on the next token
        match self.peek_token()?.kind {
            TokenKind::Equals => {
                self.bump()?; // consume '='
                let term = self.parse_term()?;
                let continuation = self.parse_body(params.span)?;
                Ok(BlockItem::ScopeCapture {
                    params: params.clone(),
                    continuation,
                    term,
                    span: params.span,
                })
            }
            TokenKind::LBrace => {
                // Parse lambda body as a term
                let mut term = self.parse_term()?;

                match &mut term {
                    Term::Lambda(lambda) => {
                        // attach params parsed earlier
                        lambda.params = params;

                        Ok(BlockItem::Lambda(lambda.clone()))
                    }
                    _ => Err(Error::new(
                        Code::Parse,
                        "expected lambda body after parameter list",
                        params.span,
                    )),
                }
            }

            _ => Err(Error::new(
                Code::Parse,
                "expected '=' or '{' after parameter list",
                params.span,
            )),
        }
    }

    fn parse_term(&mut self) -> Result<Term, Error> {
        let mut term = self.parse_head()?;

        while matches!(self.peek_token()?.kind, TokenKind::LParen) {
            let lparen = self.bump()?; // consume '('
            let args = self.parse_argument_list()?;

            match &mut term {
                Term::Ident(ident) => {
                    ident.args.extend(args);
                }
                Term::Lambda(lambda) => {
                    lambda.args.extend(args);
                }
                _ => {
                    return Err(Error::new(
                        Code::Parse,
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
    fn parse_head(&mut self) -> Result<Term, Error> {
        self.skip_newlines()?;
        let token = self.bump()?;
        match token.kind {
            TokenKind::IntLiteral(value) => Ok(Term::Lit(Literal {
                value: ast::Lit::Int(value as isize),
                span: token.span,
            })),
            TokenKind::StringLiteral(value) => Ok(Term::Lit(Literal {
                value: ast::Lit::Str(value),
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
                let params = self.parse_params(ParamContext::Lambda)?;
                let brace = self.expect_token("{", |kind| matches!(kind, TokenKind::LBrace))?;
                let body = self.parse_body(brace.span)?;
                self.expect_token("}", |k| matches!(k, TokenKind::RBrace))?;
                Ok(Term::Lambda(Lambda {
                    params,
                    body,
                    args: Vec::new(),
                    span: token.span,
                }))
            }
            TokenKind::LBrace => {
                // { ... } → lambda without explicit params
                let params = Signature {
                    items: Vec::new(),
                    span: token.span,
                    generics: Vec::new(),
                };
                let body = self.parse_body(token.span)?;
                self.expect_token("}", |k| matches!(k, TokenKind::RBrace))?;
                Ok(Term::Lambda(Lambda {
                    params,
                    body,
                    args: Vec::new(),
                    span: token.span,
                }))
            }
            _ => Err(Error::new(
                Code::Parse,
                format!("unexpected token: {:?}", token.kind),
                token.span,
            )),
        }
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Term>, Error> {
        let mut args = Vec::new();
        if matches!(self.peek_token()?.kind, TokenKind::RParen) {
            self.bump()?;
            return Ok(args);
        }
        loop {
            args.push(self.parse_term()?);
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
    fn parse_sig_item(&mut self, context: ParamContext) -> Result<ast::SigItem, Error> {
        let item_span = self.peek_token()?.span;
        let token = self.peek_token()?.clone();

        let (name, ty) = if matches!(token.kind, TokenKind::Ident(_)) {
            let (name, name_span) = self.parse_identifier("parameter name")?;

            // Case: name: Type
            if matches!(self.peek_token()?.kind, TokenKind::Colon) {
                self.expect_token(":", |kind| matches!(kind, TokenKind::Colon))?;
                (Some(name), self.parse_type_kind()?)
            } else {
                match context {
                    ParamContext::Params => {
                        // Put back IDENT so parse_type_ref sees it
                        self.peeked.push_front(token);
                        (None, self.parse_type_kind()?)
                    }
                    ParamContext::Lambda => {
                        return Err(Error::new(
                            Code::Parse,
                            "lambda parameters must have a type",
                            name_span,
                        ));
                    }
                }
            }
        } else {
            // Pure type-only parameter: `int`, `str`, `(a:int)`
            (None, self.parse_type_kind()?)
        };

        let has_bang = self
            .consume_if(|kind| matches!(kind, TokenKind::Bang))?
            .is_some();

        Ok(ast::SigItem {
            name: name.unwrap_or_default(),
            kind: ty,
            has_bang,
            span: item_span,
        })
    }

    fn parse_generic_params(&mut self) -> Result<Vec<String>, Error> {
        if !matches!(self.peek_token()?.kind, TokenKind::AngleOpen) {
            return Ok(Vec::new());
        }
        let lt = self.expect_token("<", |kind| matches!(kind, TokenKind::AngleOpen))?;
        let mut params = Vec::new();
        let mut seen = HashSet::new();
        loop {
            let (name, span) = self.parse_identifier("generic parameter name")?;
            if !seen.insert(name.clone()) {
                return Err(Error::new(
                    Code::Parse,
                    format!("generic parameter '{}' already declared", name),
                    span,
                ));
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
            return Err(Error::new(
                Code::Parse,
                "expected at least one generic parameter",
                lt.span,
            ));
        }
        Ok(params)
    }

    fn with_generic_scope<F, Res>(&mut self, params: &[String], f: F) -> Result<Res, Error>
    where
        F: FnOnce(&mut Self) -> Result<Res, Error>,
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

    fn collect_param_kinds(params: &[SigItem]) -> Result<Vec<ast::SigKind>, Error> {
        Ok(params.iter().map(|param| param.kind.clone()).collect())
    }

    fn parse_type_arguments(&mut self) -> Result<Vec<ast::SigKind>, Error> {
        self.expect_token("<", |kind| matches!(kind, TokenKind::AngleOpen))?;
        let mut args = Vec::new();
        loop {
            let ty = self.parse_type_kind()?;
            args.push(ty);
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
    fn parse_type_kind(&mut self) -> Result<ast::SigKind, Error> {
        let token = self.bump()?;
        let span = token.span;
        match token.kind {
            TokenKind::LParen => {
                let lparen = token;
                let mut args = Vec::new();
                if !matches!(self.peek_token()?.kind, TokenKind::RParen) {
                    loop {
                        args.push(self.parse_sig_item(ParamContext::Params)?);
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
                let kind = SigKind::Sig(Signature {
                    items: args,
                    span: lparen.span,
                    generics: Vec::new(),
                });
                return Ok(kind);
            }
            TokenKind::Ident(name) => {
                if self.is_generic_param(&name) {
                    return Ok(SigKind::Generic(name));
                }

                if matches!(self.peek_token()?.kind, TokenKind::AngleOpen) {
                    let args = self.parse_type_arguments()?;
                    // TODO: Not parsers job
                    // if let Some(info) = symbols.get_type_info(&name) {
                    //     if info.generics.len() != args.len() {
                    //         return Err(CompileError::new(CompileErrorCode::Parse,
                    //             format!(
                    //                 "type '{}' expects {} generic arguments but got {}",
                    //                 name,
                    //                 info.generics.len(),
                    //                 args.len()
                    //             ),
                    //             span,
                    //         )
                    //         .into());
                    //     }
                    // } else if resolved_type.is_some() {
                    //     return Err(CompileError::new(CompileErrorCode::Parse,
                    //         format!("type '{}' is not generic", name),
                    //         span,
                    //     )
                    //     .into());
                    // } else {
                    //     return Err(
                    //         CompileError::new(CompileErrorCode::Parse, format!("unknown type '{}'", name), span).into()
                    //     );
                    // }
                    return Ok(SigKind::GenericInst { name, args });
                }

                // TODO: Not parsers job
                // let resolved_type = symbols.resolve_type(&name);
                // if let Some(ty) = resolved_type {
                //     if let SigKind::Ident(ident) = &ty {
                //         let alias_name = &ident.name;
                //         if let Some(info) = symbols.get_type_info(alias_name) {
                //             if !info.generics.is_empty() {
                //                 return Err(CompileError::new(CompileErrorCode::Parse,
                //                     format!("generic type '{}' must be specialized", alias_name),
                //                     span,
                //                 )
                //                 .into());
                //             }
                //         }
                //     }
                //     return Ok(ast::SigKind { kind: ty, span });
                // }
                // return Err(CompileError::new(CompileErrorCode::Parse, format!("unknown type '{}'", name), span).into());
                Ok(SigKind::Ident(SigIdent { name, span }))
            }
            _ => return Err(Error::new(Code::Parse, "expected a type", span)),
        }
    }

    fn parse_params(&mut self, context: ParamContext) -> Result<Signature, Error> {
        let lparen = self.expect_token("(", |k| matches!(k, TokenKind::LParen))?;

        let mut params = Vec::new();
        loop {
            if matches!(self.peek_token()?.kind, TokenKind::RParen) {
                break;
            }

            params.push(self.parse_sig_item(context)?);

            if self
                .consume_if(|kind| matches!(kind, TokenKind::Comma))?
                .is_none()
            {
                break;
            }
        }

        self.expect_token(")", |k| matches!(k, TokenKind::RParen))?;
        Ok(Signature {
            items: params,
            span: lparen.span,
            generics: Vec::new(),
        })
    }

    fn parse_identifier(&mut self, expected: &str) -> Result<(String, Span), Error> {
        let token = self.bump()?;
        match token.kind {
            TokenKind::Ident(name) => Ok((name, token.span)),
            _ => Err(Error::new(
                Code::Parse,
                format!("expected {}", expected),
                token.span,
            )),
        }
    }

    fn expect_token<F>(&mut self, expected: &str, predicate: F) -> Result<Token, Error>
    where
        F: Fn(&TokenKind) -> bool,
    {
        let token = self.bump()?;
        if predicate(&token.kind) {
            Ok(token)
        } else {
            Err(Error::new(
                Code::Parse,
                format!("expected {}", expected),
                token.span,
            ))
        }
    }

    fn consume_if<F>(&mut self, predicate: F) -> Result<Option<Token>, Error>
    where
        F: Fn(&TokenKind) -> bool,
    {
        if predicate(&self.peek_token()?.kind) {
            Ok(Some(self.bump()?))
        } else {
            Ok(None)
        }
    }

    fn bump(&mut self) -> Result<Token, Error> {
        if let Some(token) = self.peeked.pop_front() {
            return Ok(token);
        }
        self.lexer.next_token().map_err(Error::from)
    }

    fn peek_token(&mut self) -> Result<&Token, Error> {
        if self.peeked.is_empty() {
            let token = self.lexer.next_token().map_err(Error::from)?;
            self.peeked.push_back(token);
        }
        Ok(self.peeked.front().expect("peeked token exists"))
    }

    fn parse_body(&mut self, start_span: Span) -> Result<Block, Error> {
        let mut items = Vec::new();
        loop {
            self.consume_block_item_separators()?;
            let token = self.peek_token()?;
            match token.kind {
                TokenKind::RBrace | TokenKind::Eof => break,
                _ => {
                    let item = self.parse_block_item()?;
                    items.push(item);
                }
            }
        }

        if items.is_empty() {
            let token = self.peek_token()?.clone();
            return Err(Error::new(
                Code::Parse,
                "block must contain at least one item",
                token.span,
            ));
        }

        Ok(Block {
            items,
            span: start_span,
        })
    }

    fn consume_block_item_separators(&mut self) -> Result<(), Error> {
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
    use std::io::Cursor;

    #[test]
    fn parse_body_rejects_empty_block() {
        let mut parser = Parser::new(Lexer::new(Cursor::new("{}")));
        let brace = parser
            .expect_token("{", |kind| matches!(kind, TokenKind::LBrace))
            .expect("expected opening brace");
        let err = parser
            .parse_body(brace.span)
            .expect_err("empty block must fail");
        parser
            .expect_token("}", |kind| matches!(kind, TokenKind::RBrace))
            .expect("expected closing brace");
        assert!(
            err.to_string()
                .contains("block must contain at least one item"),
            "unexpected error: {err}"
        );
    }
}
