use crate::compiler::error::CompileError;
use crate::compiler::hir;
use crate::compiler::hir_context::{ConstantValue, ContextEntry};
use crate::compiler::span::Span;
use crate::compiler::symbol::{FunctionSig, SymbolRegistry};

pub fn register_import(
    import_path: &str,
    span: Span,
    ctx: &mut hir::Context,
) -> Result<Vec<String>, CompileError> {
    let name = extract_import_name(import_path);
    let spec = builtin_import_spec(name, span);
    for ty in &spec.types {
        ctx.insert_type(&ty.name, ty.ty.kind.clone(), span, false)?;
    }
    for function in &spec.functions {
        register_function(function, span, ctx)?;
    }
    for value in &spec.values {
        ctx.insert(
            value.name,
            ContextEntry::Value {
                ty: value.ty.clone(),
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
    let spec = builtin_import_spec(name, span);
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
    params: Vec<hir::SigKind>,
}

struct BuiltinValueDef {
    name: &'static str,
    ty: hir::SigKind,
}

struct BuiltinSpec {
    recorded_names: Vec<String>,
    functions: Vec<BuiltinFunctionDef>,
    values: Vec<BuiltinValueDef>,
    types: Vec<hir::SigItem>,
}

fn builtin_import_spec(name: &str, span: Span) -> BuiltinSpec {
    let mut recorded_names = vec![name.to_string()];
    if name == "puts" {
        recorded_names.push("rgo_puts".to_string());
    }
    let mut functions = Vec::new();
    let mut values = Vec::new();
    let mut types = Vec::new();

    match name {
        "int" => {
            types.push(hir::SigItem {
                name: "int".into(),
                ty: hir::SigType {
                    kind: hir::SigKind::Int,
                    span,
                },
                span,
                is_variadic: false,
            });
        }
        "str" => {
            types.push(hir::SigItem {
                name: "str".into(),
                ty: hir::SigType {
                    kind: hir::SigKind::Str,
                    span,
                },
                span,
                is_variadic: false,
            });
        }
        "add" => {
            functions.push(BuiltinFunctionDef {
                name: "add",
                params: vec![
                    hir::SigKind::Int,
                    hir::SigKind::Int,
                    hir::SigKind::tuple([hir::SigKind::Int]),
                ],
            });
        }
        "sub" => {
            functions.push(BuiltinFunctionDef {
                name: "sub",
                params: vec![
                    hir::SigKind::Int,
                    hir::SigKind::Int,
                    hir::SigKind::tuple([hir::SigKind::Int]),
                ],
            });
        }
        "mul" => {
            functions.push(BuiltinFunctionDef {
                name: "mul",
                params: vec![
                    hir::SigKind::Int,
                    hir::SigKind::Int,
                    hir::SigKind::tuple([hir::SigKind::Int]),
                ],
            });
        }
        "div" => {
            functions.push(BuiltinFunctionDef {
                name: "div",
                params: vec![
                    hir::SigKind::Int,
                    hir::SigKind::Int,
                    hir::SigKind::tuple([hir::SigKind::Int]),
                ],
            });
        }
        "eq" => {
            functions.push(BuiltinFunctionDef {
                name: "eq",
                params: vec![
                    hir::SigKind::Int,
                    hir::SigKind::Int,
                    hir::SigKind::tuple([]),
                    hir::SigKind::tuple([]),
                ],
            });
        }
        "fmt" => {
            functions.push(BuiltinFunctionDef {
                name: "fmt",
                params: vec![
                    hir::SigKind::Str,
                    hir::SigKind::Int,
                    hir::SigKind::tuple([hir::SigKind::Str]),
                ],
            });
            functions.push(BuiltinFunctionDef {
                name: "write",
                params: vec![hir::SigKind::Str, hir::SigKind::tuple([])],
            });
        }
        "eqi" => {
            functions.push(BuiltinFunctionDef {
                name: "eqi",
                params: vec![
                    hir::SigKind::Int,
                    hir::SigKind::Int,
                    hir::SigKind::tuple([]),
                    hir::SigKind::tuple([]),
                ],
            });
        }
        "lt" => {
            functions.push(BuiltinFunctionDef {
                name: "lt",
                params: vec![
                    hir::SigKind::Int,
                    hir::SigKind::Int,
                    hir::SigKind::tuple([]),
                    hir::SigKind::tuple([]),
                ],
            });
        }
        "gt" => {
            functions.push(BuiltinFunctionDef {
                name: "gt",
                params: vec![
                    hir::SigKind::Int,
                    hir::SigKind::Int,
                    hir::SigKind::tuple([]),
                    hir::SigKind::tuple([]),
                ],
            });
        }
        "eqs" => {
            functions.push(BuiltinFunctionDef {
                name: "eqs",
                params: vec![
                    hir::SigKind::Str,
                    hir::SigKind::Str,
                    hir::SigKind::tuple([]),
                    hir::SigKind::tuple([]),
                ],
            });
        }
        "itoa" => {
            functions.push(BuiltinFunctionDef {
                name: "itoa",
                params: vec![hir::SigKind::Int, hir::SigKind::tuple([hir::SigKind::Str])],
            });
        }
        "stdout" => {
            values.push(BuiltinValueDef {
                name: "stdout",
                ty: hir::SigKind::Str,
            });
        }
        "write" => {
            functions.push(BuiltinFunctionDef {
                name: "write",
                params: vec![hir::SigKind::Str, hir::SigKind::tuple([])],
            });
        }
        "puts" => {
            functions.push(BuiltinFunctionDef {
                name: "rgo_puts",
                params: vec![hir::SigKind::Str, hir::SigKind::tuple([])],
            });
        }
        "rgo_write" => {
            functions.push(BuiltinFunctionDef {
                name: "rgo_write",
                params: vec![hir::SigKind::Str, hir::SigKind::tuple([])],
            });
        }
        "exit" => {
            functions.push(BuiltinFunctionDef {
                name: "exit",
                params: vec![hir::SigKind::Int],
            });
        }
        "printf" | "sprintf" => {}
        _ => {}
    }

    BuiltinSpec {
        recorded_names,
        functions,
        values,
        types,
    }
}

fn register_function(
    def: &BuiltinFunctionDef,
    span: Span,
    ctx: &mut hir::Context,
) -> Result<(), CompileError> {
    let hir_sig_items = def
        .params
        .iter()
        .enumerate()
        .map(|(idx, kind)| hir::SigItem {
            name: format!("_{}", idx),
            ty: hir::SigType {
                kind: kind.clone(),
                span: Span::unknown(),
            },
            is_variadic: false,
            span: Span::unknown(),
        })
        .collect();
    let hir_sig = hir::Signature {
        items: hir_sig_items,
        span,
    };
    ctx.insert_func(def.name, hir_sig, span, true)?;
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
    symbols.declare_function(FunctionSig {
        name: def.name.to_string(),
        params: builtin_sig_items(&def.params),
        span,
    })?;
    Ok(())
}

fn builtin_sig_items(params: &[hir::SigKind]) -> Vec<hir::SigItem> {
    params
        .iter()
        .map(|kind| hir::SigItem {
            name: String::new(),
            ty: hir::SigType {
                kind: kind.clone(),
                span: Span::unknown(),
            },
            is_variadic: false,
            span: Span::unknown(),
        })
        .collect()
}

fn placeholder_constant(kind: &hir::SigKind) -> ConstantValue {
    match kind {
        hir::SigKind::Str | hir::SigKind::CompileTimeStr => ConstantValue::Str(String::new()),
        _ => ConstantValue::Int(0),
    }
}
