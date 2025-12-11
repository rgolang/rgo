use crate::compiler::ast;
use crate::compiler::error::CompileError;
use crate::compiler::hir;
use crate::compiler::hir_scope::{ConstantValue, ScopeItem};
use crate::compiler::span::Span;
use crate::compiler::symbol::{FunctionSig, SymbolRegistry};

pub fn register_import_scope(
    import_path: &str,
    span: Span,
    scope: &mut hir::Scope,
) -> Result<Vec<String>, CompileError> {
    let name = extract_import_name(import_path);
    let spec = builtin_import_spec(name);
    for function in &spec.functions {
        register_function_in_scope(function, span, scope)?;
    }
    for value in &spec.values {
        scope.insert(
            value.name,
            ScopeItem::Value {
                ty: hir::ast_type_ref_to_hir_type_ref(&value.ty),
                span,
                constant: placeholder_constant(&value.ty),
            },
        )?;
    }
    Ok(spec.recorded_names.clone())
}

pub fn register_import_symbols(
    import_path: &str,
    span: Span,
    symbols: &mut SymbolRegistry,
) -> Result<(), CompileError> {
    let name = extract_import_name(import_path);
    let spec = builtin_import_spec(name);
    for recorded in &spec.recorded_names {
        symbols.record_builtin_import(recorded);
    }
    for function in &spec.functions {
        register_function_in_symbols(function, span, symbols)?;
    }
    for value in &spec.values {
        symbols.declare_value(value.name.to_string(), value.ty.clone(), span)?;
    }
    Ok(())
}

fn extract_import_name(import_path: &str) -> &str {
    if let Some(slash_pos) = import_path.rfind('/') {
        &import_path[slash_pos + 1..]
    } else {
        import_path
    }
}

struct BuiltinFunctionDef {
    name: &'static str,
    params: Vec<ast::SigKind>,
}

struct BuiltinValueDef {
    name: &'static str,
    ty: ast::SigKind,
}

struct BuiltinSpec {
    recorded_names: Vec<String>,
    functions: Vec<BuiltinFunctionDef>,
    values: Vec<BuiltinValueDef>,
}

fn builtin_import_spec(name: &str) -> BuiltinSpec {
    let mut recorded_names = vec![name.to_string()];
    if name == "puts" {
        recorded_names.push("rgo_puts".to_string());
    }
    let mut functions = Vec::new();
    let mut values = Vec::new();

    match name {
        "add" => {
            functions.push(BuiltinFunctionDef {
                name: "add",
                params: vec![
                    ast::SigKind::Int,
                    ast::SigKind::Int,
                    ast::SigKind::tuple([ast::SigKind::Int]),
                ],
            });
        }
        "sub" => {
            functions.push(BuiltinFunctionDef {
                name: "sub",
                params: vec![
                    ast::SigKind::Int,
                    ast::SigKind::Int,
                    ast::SigKind::tuple([ast::SigKind::Int]),
                ],
            });
        }
        "mul" => {
            functions.push(BuiltinFunctionDef {
                name: "mul",
                params: vec![
                    ast::SigKind::Int,
                    ast::SigKind::Int,
                    ast::SigKind::tuple([ast::SigKind::Int]),
                ],
            });
        }
        "div" => {
            functions.push(BuiltinFunctionDef {
                name: "div",
                params: vec![
                    ast::SigKind::Int,
                    ast::SigKind::Int,
                    ast::SigKind::tuple([ast::SigKind::Int]),
                ],
            });
        }
        "eq" => {
            functions.push(BuiltinFunctionDef {
                name: "eq",
                params: vec![
                    ast::SigKind::Int,
                    ast::SigKind::Int,
                    ast::SigKind::tuple([]),
                    ast::SigKind::tuple([]),
                ],
            });
        }
        "fmt" => {
            functions.push(BuiltinFunctionDef {
                name: "fmt",
                params: vec![
                    ast::SigKind::Str,
                    ast::SigKind::Int,
                    ast::SigKind::tuple([ast::SigKind::Str]),
                ],
            });
            functions.push(BuiltinFunctionDef {
                name: "write",
                params: vec![ast::SigKind::Str, ast::SigKind::tuple([])],
            });
        }
        "eqi" => {
            functions.push(BuiltinFunctionDef {
                name: "eqi",
                params: vec![
                    ast::SigKind::Int,
                    ast::SigKind::Int,
                    ast::SigKind::tuple([]),
                    ast::SigKind::tuple([]),
                ],
            });
        }
        "lt" => {
            functions.push(BuiltinFunctionDef {
                name: "lt",
                params: vec![
                    ast::SigKind::Int,
                    ast::SigKind::Int,
                    ast::SigKind::tuple([]),
                    ast::SigKind::tuple([]),
                ],
            });
        }
        "gt" => {
            functions.push(BuiltinFunctionDef {
                name: "gt",
                params: vec![
                    ast::SigKind::Int,
                    ast::SigKind::Int,
                    ast::SigKind::tuple([]),
                    ast::SigKind::tuple([]),
                ],
            });
        }
        "eqs" => {
            functions.push(BuiltinFunctionDef {
                name: "eqs",
                params: vec![
                    ast::SigKind::Str,
                    ast::SigKind::Str,
                    ast::SigKind::tuple([]),
                    ast::SigKind::tuple([]),
                ],
            });
        }
        "itoa" => {
            functions.push(BuiltinFunctionDef {
                name: "itoa",
                params: vec![ast::SigKind::Int, ast::SigKind::tuple([ast::SigKind::Str])],
            });
        }
        "stdout" => {
            values.push(BuiltinValueDef {
                name: "stdout",
                ty: ast::SigKind::Str,
            });
        }
        "write" => {
            functions.push(BuiltinFunctionDef {
                name: "write",
                params: vec![ast::SigKind::Str, ast::SigKind::tuple([])],
            });
        }
        "puts" => {
            functions.push(BuiltinFunctionDef {
                name: "rgo_puts",
                params: vec![ast::SigKind::Str, ast::SigKind::tuple([])],
            });
        }
        "rgo_write" => {
            functions.push(BuiltinFunctionDef {
                name: "rgo_write",
                params: vec![ast::SigKind::Str, ast::SigKind::tuple([])],
            });
        }
        "exit" => {
            functions.push(BuiltinFunctionDef {
                name: "exit",
                params: vec![ast::SigKind::Int],
            });
        }
        "printf" | "sprintf" => {}
        _ => {}
    }

    BuiltinSpec {
        recorded_names,
        functions,
        values,
    }
}

fn register_function_in_scope(
    def: &BuiltinFunctionDef,
    span: Span,
    scope: &mut hir::Scope,
) -> Result<(), CompileError> {
    let ast_sig_items = build_ast_sig_items(&def.params);
    let hir_sig_items = ast_sig_items
        .iter()
        .enumerate()
        .map(|(idx, item)| hir::SigItem {
            name: item.name.clone().unwrap_or_else(|| format!("_{}", idx)),
            ty: hir::SigType {
                kind: hir::ast_type_ref_to_hir_type_ref(&item.ty.kind),
                span: item.ty.span,
            },
            is_variadic: item.is_variadic,
            span: item.span,
        })
        .collect();
    let hir_sig = hir::Signature {
        items: hir_sig_items,
        span,
    };
    scope.insert_func(def.name, hir_sig, span, true)?;
    Ok(())
}

fn register_function_in_symbols(
    def: &BuiltinFunctionDef,
    span: Span,
    symbols: &mut SymbolRegistry,
) -> Result<(), CompileError> {
    if symbols.get_function(def.name).is_some() {
        return Ok(());
    }
    let ast_sig_items = build_ast_sig_items(&def.params);
    symbols.declare_function(FunctionSig {
        name: def.name.to_string(),
        params: ast_sig_items,
        span,
    })?;
    Ok(())
}

fn build_ast_sig_items(params: &[ast::SigKind]) -> Vec<ast::SigItem> {
    params
        .iter()
        .map(|kind| ast::SigItem {
            name: None,
            ty: ast::SigType {
                kind: kind.clone(),
                span: Span::unknown(),
            },
            is_variadic: false,
            span: Span::unknown(),
        })
        .collect()
}

fn placeholder_constant(kind: &ast::SigKind) -> ConstantValue {
    match kind {
        ast::SigKind::Str | ast::SigKind::CompileTimeStr => ConstantValue::Str(String::new()),
        _ => ConstantValue::Int(0),
    }
}
