use crate::compiler::ast;
use crate::compiler::hir;
use crate::compiler::hir_context as ctx;
use crate::compiler::span::Span;
use std::collections::{BTreeSet, HashMap, HashSet};

pub fn ast_signature_to_hir(signature: ast::Signature) -> hir::Signature {
    hir::Signature {
        items: signature
            .items
            .into_iter()
            .map(ast_sig_item_to_hir)
            .collect(),
        generics: signature.generics,
    }
}

pub fn hir_signature_to_ast(signature: hir::Signature) -> ast::Signature {
    ast::Signature {
        items: signature
            .items
            .into_iter()
            .map(hir_sig_item_to_ast)
            .collect(),
        span: Span::unknown(),
        generics: signature.generics,
    }
}

pub fn resolve_signature(signature: &hir::Signature, ctx: &mut ctx::Context) -> hir::Signature {
    hir::Signature {
        items: signature
            .items
            .iter()
            .map(|item| {
                let name = if item.name.is_empty() {
                    ctx.new_name()
                } else {
                    item.name.clone()
                };
                let ty = lower_sig_kind(&item.kind, ctx, item.has_bang);
                hir::SigItem {
                    name,
                    kind: ty,
                    has_bang: item.has_bang,
                }
            })
            .collect(),
        generics: signature.generics.clone(),
    }
}
/// Normalize a HIR SigKind by resolving any `Ident` that refers to an imported builtin (ctxentry.is_builtin=true),
/// converting e.g. `Ident("str") -> SigKind::Str`, `Ident("int") -> SigKind::Int`, etc.
/// Uses a single canonical folder to avoid match duplication.
pub fn normalize_signature(signature: &hir::Signature, ctx: &ctx::Context) -> hir::Signature {
    let items = signature
        .items
        .iter()
        .map(|item| {
            let mut normalized_item = item.clone();
            normalized_item.kind = normalize_sig_kind(&item.kind, ctx);
            normalized_item
        })
        .collect();
    hir::Signature {
        items,
        generics: signature.generics.clone(),
    }
}

pub fn normalize_sig_kind(kind: &hir::SigKind, ctx: &ctx::Context) -> hir::SigKind {
    let mut seen = HashSet::new();
    normalize_sig_kind_inner(kind, ctx, &mut seen)
}

fn normalize_sig_kind_inner(
    kind: &hir::SigKind,
    ctx: &ctx::Context,
    seen: &mut HashSet<String>,
) -> hir::SigKind {
    match kind {
        hir::SigKind::Ident(ident) => {
            if seen.contains(&ident.name) {
                return hir::SigKind::Ident(ident.clone());
            }
            if let Some(entry) = ctx.get(&ident.name) {
                seen.insert(ident.name.clone());
                let resolved = normalize_sig_kind_inner(&entry.kind, ctx, seen);
                seen.remove(&ident.name);
                return resolved;
            }
            hir::SigKind::Ident(ident.clone())
        }
        hir::SigKind::Sig(signature) => {
            let items = signature
                .items
                .iter()
                .map(|item| {
                    let mut normalized_item = item.clone();
                    normalized_item.kind = normalize_sig_kind_inner(&item.kind, ctx, seen);
                    normalized_item
                })
                .collect();
            hir::SigKind::Sig(hir::Signature {
                items,
                generics: signature.generics.clone(),
            })
        }
        other => other.clone(),
    }
}

pub fn resolve_target_signature(target: &str, ctx: &ctx::Context) -> Option<hir::Signature> {
    let mut visited = HashSet::new();
    ctx.get(target)
        .and_then(|entry| signature_from_kind(&entry.kind, ctx, &mut visited))
}

pub fn signature_from_kind(
    kind: &hir::SigKind,
    ctx: &ctx::Context,
    visited: &mut HashSet<String>,
) -> Option<hir::Signature> {
    match kind {
        hir::SigKind::Sig(signature) => Some(signature.clone()),
        hir::SigKind::Ident(ident) => {
            let name = &ident.name;
            if !visited.insert(name.clone()) {
                return None;
            }
            let out = ctx
                .get(name)
                .and_then(|entry| signature_from_kind(&entry.kind, ctx, visited));
            visited.remove(name);
            out
        }
        _ => None,
    }
}

pub fn expected_params_for_args<'a>(
    params: &'a [hir::SigItem],
    args_len: usize,
) -> Vec<Option<&'a hir::SigItem>> {
    let mut expected = Vec::with_capacity(args_len);
    if args_len == 0 {
        return expected;
    }

    let mut prefix_assigned = 0;
    let mut suffix_start = args_len;
    let variadic_index = params
        .iter()
        .position(|item| matches!(item.kind, hir::SigKind::Variadic));

    if let Some(var_idx) = variadic_index {
        let prefix_count = var_idx;
        prefix_assigned = prefix_count.min(args_len);
        let remaining_after_prefix = args_len.saturating_sub(prefix_assigned);
        let suffix_params = params.len().saturating_sub(var_idx + 1);
        let suffix_assigned = remaining_after_prefix.min(suffix_params);
        suffix_start = args_len.saturating_sub(suffix_assigned);
    }

    for idx in 0..args_len {
        let expected_param = if let Some(var_idx) = variadic_index {
            if idx < prefix_assigned {
                params.get(idx)
            } else if idx >= suffix_start {
                let suffix_param_idx = var_idx + 1 + (idx - suffix_start);
                params.get(suffix_param_idx)
            } else {
                params.get(var_idx)
            }
        } else {
            params.get(idx)
        };
        expected.push(expected_param);
    }

    expected
}

fn resolve_ident(ident: &hir::SigIdent, ctx: &ctx::Context) -> hir::SigKind {
    if let Some(entry) = ctx.get(&ident.name) {
        if entry.is_builtin || matches!(entry.kind, hir::SigKind::Generic(_)) {
            return entry.kind.clone();
        }
    }
    hir::SigKind::Ident(ident.clone())
}

fn ast_sig_item_to_hir(item: ast::SigItem) -> hir::SigItem {
    hir::SigItem {
        name: item.name,
        kind: ast_sig_kind_to_hir(item.kind),
        has_bang: item.has_bang,
    }
}

fn ast_sig_kind_to_hir(kind: ast::SigKind) -> hir::SigKind {
    match kind {
        ast::SigKind::Int => hir::SigKind::Int,
        ast::SigKind::Str => hir::SigKind::Str,
        ast::SigKind::F64 => hir::SigKind::F64,
        ast::SigKind::Variadic => hir::SigKind::Variadic,
        ast::SigKind::CompileTimeInt => hir::SigKind::CompileTimeInt,
        ast::SigKind::CompileTimeStr => hir::SigKind::CompileTimeStr,
        ast::SigKind::Ident(ident) => hir::SigKind::Ident(hir::SigIdent { name: ident.name }),
        ast::SigKind::Sig(signature) => hir::SigKind::Sig(ast_signature_to_hir(signature)),
        ast::SigKind::GenericInst { name, args } => hir::SigKind::GenericInst {
            name,
            args: args.into_iter().map(ast_sig_kind_to_hir).collect(),
        },
        ast::SigKind::Generic(name) => hir::SigKind::Generic(name),
    }
}

fn hir_sig_item_to_ast(item: hir::SigItem) -> ast::SigItem {
    ast::SigItem {
        name: item.name,
        kind: hir_sig_kind_to_ast(item.kind),
        has_bang: item.has_bang,
        span: Span::unknown(),
    }
}

fn hir_sig_kind_to_ast(kind: hir::SigKind) -> ast::SigKind {
    match kind {
        hir::SigKind::Int => ast::SigKind::Int,
        hir::SigKind::Str => ast::SigKind::Str,
        hir::SigKind::F64 => ast::SigKind::F64,
        hir::SigKind::Variadic => ast::SigKind::Variadic,
        hir::SigKind::CompileTimeInt => ast::SigKind::CompileTimeInt,
        hir::SigKind::CompileTimeStr => ast::SigKind::CompileTimeStr,
        hir::SigKind::Ident(ident) => ast::SigKind::Ident(ast::SigIdent {
            name: ident.name,
            span: Span::unknown(),
        }),
        hir::SigKind::Sig(signature) => ast::SigKind::Sig(hir_signature_to_ast(signature)),
        hir::SigKind::GenericInst { name, args } => ast::SigKind::GenericInst {
            name,
            args: args.into_iter().map(hir_sig_kind_to_ast).collect(),
        },
        hir::SigKind::Generic(name) => ast::SigKind::Generic(name),
    }
}

fn lower_sig_kind(kind: &hir::SigKind, ctx: &mut ctx::Context, has_bang: bool) -> hir::SigKind {
    match kind {
        hir::SigKind::Ident(ident) => {
            let resolved = resolve_ident(ident, ctx);
            if has_bang {
                match resolved {
                    hir::SigKind::Int => hir::SigKind::CompileTimeInt,
                    hir::SigKind::Str => hir::SigKind::CompileTimeStr,
                    hir::SigKind::CompileTimeInt => hir::SigKind::CompileTimeInt,
                    hir::SigKind::CompileTimeStr => hir::SigKind::CompileTimeStr,
                    other => other,
                }
            } else {
                resolved
            }
        }
        hir::SigKind::Sig(signature) => hir::SigKind::Sig(resolve_signature(signature, ctx)),
        hir::SigKind::GenericInst { name, args } => {
            let resolved_args = args
                .iter()
                .map(|arg| lower_sig_kind(arg, ctx, false))
                .collect::<Vec<_>>();

            instantiate_generic_inst(name, &resolved_args, ctx)
                .unwrap_or_else(|| hir::SigKind::Ident(hir::SigIdent { name: name.clone() }))
        }
        hir::SigKind::Generic(name) => hir::SigKind::Generic(name.clone()),
        hir::SigKind::Int => {
            if has_bang {
                hir::SigKind::CompileTimeInt
            } else {
                hir::SigKind::Int
            }
        }
        hir::SigKind::Str => {
            if has_bang {
                hir::SigKind::CompileTimeStr
            } else {
                hir::SigKind::Str
            }
        }
        hir::SigKind::F64 => hir::SigKind::F64,
        hir::SigKind::Variadic => hir::SigKind::Variadic,
        hir::SigKind::CompileTimeInt => hir::SigKind::CompileTimeInt,
        hir::SigKind::CompileTimeStr => hir::SigKind::CompileTimeStr,
    }
}

fn instantiate_generic_inst(
    name: &str,
    args: &[hir::SigKind],
    ctx: &ctx::Context,
) -> Option<hir::SigKind> {
    let entry = ctx.get(name)?;
    let signature = if let hir::SigKind::Sig(signature) = &entry.kind {
        signature
    } else {
        return None;
    };

    if signature.generics.len() != args.len() {
        return None;
    }

    let mapping: HashMap<String, hir::SigKind> = signature
        .generics
        .iter()
        .cloned()
        .zip(args.iter().cloned())
        .collect();

    Some(hir::SigKind::Sig(substitute_signature(signature, &mapping)))
}

fn substitute_signature(
    signature: &hir::Signature,
    mapping: &HashMap<String, hir::SigKind>,
) -> hir::Signature {
    let items = signature
        .items
        .iter()
        .map(|item| {
            let mut out = item.clone();
            out.kind = substitute_kind(&item.kind, mapping);
            out
        })
        .collect();

    hir::Signature {
        items,
        generics: BTreeSet::new(),
    }
}

// TODO: Remove this
fn substitute_kind(kind: &hir::SigKind, mapping: &HashMap<String, hir::SigKind>) -> hir::SigKind {
    match kind {
        hir::SigKind::Sig(signature) => hir::SigKind::Sig(substitute_signature(signature, mapping)),
        hir::SigKind::Ident(ident) => {
            if let Some(mapped) = mapping.get(&ident.name) {
                mapped.clone()
            } else {
                hir::SigKind::Ident(ident.clone())
            }
        }
        hir::SigKind::Generic(name) => {
            if let Some(mapped) = mapping.get(name) {
                mapped.clone()
            } else {
                hir::SigKind::Generic(name.clone())
            }
        }
        _ => kind.clone(),
    }
}
