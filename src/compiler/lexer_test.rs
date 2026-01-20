use std::io::Cursor;

use super::lexer::Lexer;
use super::token::TokenKind;

#[test]
fn lexer_test() {
    let source = include_bytes!("lexer_test.rgo");
    let cursor = Cursor::new(&source[..]);
    let mut lexer = Lexer::new(cursor);
    let ident = |name: &str| TokenKind::Ident(name.to_string());
    let import = |name: &str| TokenKind::Import(name.to_string());
    let str_lit = |value: &str| TokenKind::StringLiteral(value.to_string());

    let expected_tokens = vec![
        TokenKind::Newline,
        ident("str"),
        TokenKind::Colon,
        import("/str"),
        TokenKind::Newline,
        ident("printf"),
        TokenKind::Colon,
        import("/printf"),
        TokenKind::Newline,
        ident("foo"),
        TokenKind::Colon,
        TokenKind::LParen,
        ident("ok"),
        TokenKind::Colon,
        TokenKind::LParen,
        ident("str"),
        TokenKind::RParen,
        TokenKind::RParen,
        TokenKind::LBrace,
        TokenKind::Newline,
        ident("ok"),
        TokenKind::LParen,
        str_lit("charlie"),
        TokenKind::RParen,
        TokenKind::Newline,
        TokenKind::RBrace,
        TokenKind::Newline,
        ident("baz"),
        TokenKind::Colon,
        TokenKind::LParen,
        ident("ok"),
        TokenKind::Colon,
        TokenKind::LParen,
        ident("str"),
        TokenKind::Comma,
        ident("str"),
        TokenKind::RParen,
        TokenKind::RParen,
        TokenKind::LBrace,
        TokenKind::Newline,
        ident("foo"),
        TokenKind::LParen,
        ident("ok"),
        TokenKind::LParen,
        str_lit("bob"),
        TokenKind::RParen,
        TokenKind::RParen,
        TokenKind::Newline,
        TokenKind::RBrace,
        TokenKind::Newline,
        ident("bar"),
        TokenKind::Colon,
        TokenKind::LParen,
        ident("name0"),
        TokenKind::Colon,
        ident("str"),
        TokenKind::Comma,
        ident("name1"),
        TokenKind::Colon,
        ident("str"),
        TokenKind::Comma,
        ident("name2"),
        TokenKind::Colon,
        ident("str"),
        TokenKind::RParen,
        TokenKind::LBrace,
        TokenKind::Newline,
        ident("printf"),
        TokenKind::LParen,
        str_lit("hello %s, %s and %s\n"),
        TokenKind::Comma,
        ident("name0"),
        TokenKind::Comma,
        ident("name1"),
        TokenKind::Comma,
        ident("name2"),
        TokenKind::RParen,
        TokenKind::Newline,
        TokenKind::RBrace,
        TokenKind::Newline,
        ident("baz"),
        TokenKind::LParen,
        ident("bar"),
        TokenKind::LParen,
        str_lit("alice"),
        TokenKind::RParen,
        TokenKind::RParen,
        TokenKind::Newline,
        TokenKind::Eof,
    ];

    let mut actual_tokens = Vec::new();

    loop {
        let token = lexer.next_token().expect("lexer should succeed");
        let is_eof = matches!(token.kind, TokenKind::Eof);
        actual_tokens.push(token.kind);
        if is_eof {
            break;
        }
    }

    assert_eq!(
        actual_tokens, expected_tokens,
        "lexer should produce the exact token stream for lexer_test.rgo"
    );
}

fn lex_single_string(source: &[u8]) -> String {
    let cursor = Cursor::new(source);
    let mut lexer = Lexer::new(cursor);
    let token = lexer.next_token().expect("lexer should produce a token");
    match token.kind {
        TokenKind::StringLiteral(value) => value,
        other => panic!("expected string literal, got {:?}", other),
    }
}

#[test]
fn single_quote_strings_preserve_backslashes() {
    let literal = lex_single_string(b"'raw\\n'");
    assert_eq!(literal, "raw\\n");
}

#[test]
fn double_quote_strings_support_unicode_and_escapes() {
    let literal = lex_single_string(b"\"\\u{1F600}\\n\"");
    assert_eq!(literal, "\u{1F600}\n");
}

#[test]
fn invalid_double_quote_escape_is_rejected() {
    let cursor = Cursor::new(b"\"bad\\x\"");
    let mut lexer = Lexer::new(cursor);
    assert!(lexer.next_token().is_err());
}

#[test]
fn ellipsis_is_single_token() {
    let cursor = Cursor::new(b"... foo");
    let mut lexer = Lexer::new(cursor);

    let ellipsis = lexer
        .next_token()
        .expect("lexer should produce ellipsis token");
    assert_eq!(ellipsis.kind, TokenKind::Ellipsis);

    let ident = lexer
        .next_token()
        .expect("lexer should produce identifier token");
    assert_eq!(ident.kind, TokenKind::Ident("foo".to_string()));

    let eof = lexer.next_token().expect("should reach EOF");
    assert!(matches!(eof.kind, TokenKind::Eof));
}
