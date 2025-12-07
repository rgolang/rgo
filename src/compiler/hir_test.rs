use std::fs;
use std::io::Cursor;
use std::path::Path;

use super::{
    ast::{Block, Item, Lambda, Params, TypeRef},
    hir::{lower_function, Env, EnvEntry},
    lexer::Lexer,
    parser::Parser,
    symbol::SymbolRegistry,
};

#[test]
fn hir_test() {
    let source = include_bytes!("hir_test.rgo");
    let cursor = Cursor::new(&source[..]);
    let lexer = Lexer::new(cursor);
    let mut parser = Parser::new(lexer);
    let mut symbols = SymbolRegistry::new();

    let mut functions = Vec::new();
    let mut env = Env::new();

    while let Some(item) = parser
        .next(&mut symbols)
        .expect("parser should accept hir_test.rgo")
    {
        match item {
            Item::FunctionDef { .. } => {
                let (main_fn, nested) = lower_function(item, &mut symbols, &env)
                    .expect("lower_function should succeed");
                functions.push(main_fn);
                functions.extend(nested);
            }
            Item::Ident(ident) => {
                let span = ident.span;
                let term_item = Item::Ident(ident);
                // Create synthetic function for entry item
                let synthetic_entry = Item::FunctionDef {
                    name: "_start".into(),
                    lambda: Lambda {
                        params: Params {
                            items: Vec::new(),
                            span,
                        },
                        body: Block {
                            items: vec![term_item],
                            span,
                        },
                        args: Vec::new(),
                        span,
                    },
                    span,
                };
                let (main_fn, nested) = lower_function(synthetic_entry, &mut symbols, &env)
                    .expect("lower_function should succeed");
                functions.push(main_fn);
                functions.extend(nested);
            }
            Item::Lambda(lambda) => {
                let span = lambda.span;
                let term_item = Item::Lambda(lambda);
                // Create synthetic function for entry item
                let synthetic_entry = Item::FunctionDef {
                    name: "_start".into(),
                    lambda: Lambda {
                        params: Params {
                            items: Vec::new(),
                            span,
                        },
                        body: Block {
                            items: vec![term_item],
                            span,
                        },
                        args: Vec::new(),
                        span,
                    },
                    span,
                };
                let (main_fn, nested) = lower_function(synthetic_entry, &mut symbols, &env)
                    .expect("lower_function should succeed");
                functions.push(main_fn);
                functions.extend(nested);
            }
            Item::StrDef { name, span, .. } => {
                env.insert(
                    name.clone(),
                    EnvEntry {
                        ty: TypeRef::Str,
                        span,
                        constant: None,
                    },
                );
            }
            Item::IntDef { name, span, .. } => {
                env.insert(
                    name.clone(),
                    EnvEntry {
                        ty: TypeRef::Int,
                        span,
                        constant: None,
                    },
                );
            }
            Item::ScopeCapture { span, .. } => {
                panic!(
                    "scope capture not expected at top level in hir_test.rgo: {:?}",
                    span
                );
            }
            Item::IdentDef { .. } | Item::Import { .. } | Item::TypeDef { .. } => {}
        }
    }

    let pretty = format!("{:#?}", functions);
    let expected_pretty = include_str!("hir_test.expected");

    if pretty != expected_pretty {
        let actual_path = Path::new("src/compiler/hir_test.actual");
        fs::write(actual_path, &pretty).expect("failed to write actual HIR snapshot");
    }

    assert_eq!(pretty, expected_pretty, "hir should accept hir_test.rgo");
}
