use std::io::Cursor;

use super::lexer::Lexer;
use super::parser::Parser;
use super::symbol::SymbolRegistry;

#[test]
fn parser_test() {
    let source = include_bytes!("parser_test.rgo");
    let cursor = Cursor::new(&source[..]);
    let lexer = Lexer::new(cursor);
    let mut parser = Parser::new(lexer);
    let mut symbols = SymbolRegistry::new();

    let mut items = Vec::new();
    while let Some(item) = parser
        .next(&mut symbols)
        .expect("parser should accept parser_test.rgo")
    {
        items.push(item);
    }

    let pretty = format!("{:#?}", items);
    let expected_pretty = r#"[
    Import {
        name: "str",
        span: Span {
            line: 2,
            column: 1,
            offset: 31,
        },
    },
    Import {
        name: "printf",
        span: Span {
            line: 3,
            column: 1,
            offset: 36,
        },
    },
    TypeDef {
        name: "Pair",
        term: Type(
            [
                Int,
                Int,
            ],
        ),
        span: Span {
            line: 5,
            column: 1,
            offset: 45,
        },
    },
    FunctionDef {
        name: "foo",
        lambda: Lambda {
            params: Params {
                items: [
                    NameAndType {
                        name: "ok",
                        ty: Type(
                            [
                                Str,
                            ],
                        ),
                        span: Span {
                            line: 7,
                            column: 7,
                            offset: 82,
                        },
                        is_variadic: false,
                    },
                ],
                span: Span {
                    line: 7,
                    column: 6,
                    offset: 81,
                },
            },
            body: Block {
                items: [
                    StrDef {
                        name: "greeting",
                        literal: StrLiteral {
                            value: "charlie",
                            span: Span {
                                line: 8,
                                column: 15,
                                offset: 107,
                            },
                        },
                        span: Span {
                            line: 8,
                            column: 5,
                            offset: 97,
                        },
                    },
                    Ident(
                        Ident {
                            name: "ok",
                            args: [
                                Ident(
                                    Ident {
                                        name: "greeting",
                                        args: [],
                                        span: Span {
                                            line: 9,
                                            column: 8,
                                            offset: 124,
                                        },
                                    },
                                ),
                            ],
                            span: Span {
                                line: 9,
                                column: 5,
                                offset: 121,
                            },
                        },
                    ),
                ],
                span: Span {
                    line: 7,
                    column: 16,
                    offset: 91,
                },
            },
            args: [],
            span: Span {
                line: 7,
                column: 1,
                offset: 76,
            },
        },
        span: Span {
            line: 7,
            column: 1,
            offset: 76,
        },
    },
    FunctionDef {
        name: "baz",
        lambda: Lambda {
            params: Params {
                items: [
                    NameAndType {
                        name: "ok",
                        ty: Type(
                            [
                                Str,
                                Str,
                            ],
                        ),
                        span: Span {
                            line: 11,
                            column: 7,
                            offset: 142,
                        },
                        is_variadic: false,
                    },
                ],
                span: Span {
                    line: 11,
                    column: 6,
                    offset: 141,
                },
            },
            body: Block {
                items: [
                    Ident(
                        Ident {
                            name: "foo",
                            args: [
                                Ident(
                                    Ident {
                                        name: "ok",
                                        args: [
                                            String(
                                                StrLiteral {
                                                    value: "bob",
                                                    span: Span {
                                                        line: 12,
                                                        column: 12,
                                                        offset: 169,
                                                    },
                                                },
                                            ),
                                        ],
                                        span: Span {
                                            line: 12,
                                            column: 9,
                                            offset: 166,
                                        },
                                    },
                                ),
                            ],
                            span: Span {
                                line: 12,
                                column: 5,
                                offset: 162,
                            },
                        },
                    ),
                ],
                span: Span {
                    line: 11,
                    column: 21,
                    offset: 156,
                },
            },
            args: [],
            span: Span {
                line: 11,
                column: 1,
                offset: 136,
            },
        },
        span: Span {
            line: 11,
            column: 1,
            offset: 136,
        },
    },
    FunctionDef {
        name: "bar",
        lambda: Lambda {
            params: Params {
                items: [
                    NameAndType {
                        name: "name0",
                        ty: Str,
                        span: Span {
                            line: 14,
                            column: 7,
                            offset: 185,
                        },
                        is_variadic: false,
                    },
                    NameAndType {
                        name: "name1",
                        ty: Str,
                        span: Span {
                            line: 14,
                            column: 19,
                            offset: 197,
                        },
                        is_variadic: false,
                    },
                    NameAndType {
                        name: "name2",
                        ty: Str,
                        span: Span {
                            line: 14,
                            column: 31,
                            offset: 209,
                        },
                        is_variadic: false,
                    },
                ],
                span: Span {
                    line: 14,
                    column: 6,
                    offset: 184,
                },
            },
            body: Block {
                items: [
                    Ident(
                        Ident {
                            name: "printf",
                            args: [
                                String(
                                    StrLiteral {
                                        value: "hello %s, %s and %s\n",
                                        span: Span {
                                            line: 15,
                                            column: 12,
                                            offset: 233,
                                        },
                                    },
                                ),
                                Ident(
                                    Ident {
                                        name: "name0",
                                        args: [],
                                        span: Span {
                                            line: 15,
                                            column: 37,
                                            offset: 258,
                                        },
                                    },
                                ),
                                Ident(
                                    Ident {
                                        name: "name1",
                                        args: [],
                                        span: Span {
                                            line: 15,
                                            column: 44,
                                            offset: 265,
                                        },
                                    },
                                ),
                                Ident(
                                    Ident {
                                        name: "name2",
                                        args: [],
                                        span: Span {
                                            line: 15,
                                            column: 51,
                                            offset: 272,
                                        },
                                    },
                                ),
                            ],
                            span: Span {
                                line: 15,
                                column: 5,
                                offset: 226,
                            },
                        },
                    ),
                ],
                span: Span {
                    line: 14,
                    column: 42,
                    offset: 220,
                },
            },
            args: [],
            span: Span {
                line: 14,
                column: 1,
                offset: 179,
            },
        },
        span: Span {
            line: 14,
            column: 1,
            offset: 179,
        },
    },
    Ident(
        Ident {
            name: "baz",
            args: [
                Ident(
                    Ident {
                        name: "bar",
                        args: [
                            String(
                                StrLiteral {
                                    value: "alice",
                                    span: Span {
                                        line: 17,
                                        column: 9,
                                        offset: 289,
                                    },
                                },
                            ),
                        ],
                        span: Span {
                            line: 17,
                            column: 5,
                            offset: 285,
                        },
                    },
                ),
            ],
            span: Span {
                line: 17,
                column: 1,
                offset: 281,
            },
        },
    ),
]"#;

    if pretty != expected_pretty {
        // Print a raw-string literal you can copy-paste into `expected_pretty`
        println!("expected_pretty = r#\"{}\"#;", pretty);
        panic!("parsed continuation-style program should match expected CPS shape");
    }
}
