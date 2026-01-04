use crate::compiler::ast;
use crate::compiler::ast::{SigIdent, SigItem, SigKind, Signature};
use crate::compiler::hir_context as ctx;
use crate::compiler::span::Span;
use std::collections::{HashMap, HashSet};

pub fn resolve_ast_signature(signature: &ast::Signature, ctx: &mut ctx::Context) -> Signature {
    Signature {
        span: signature.span,
        items: signature
            .items
            .iter()
            .map(|item| {
                let name = if item.name.is_empty() {
                    ctx.new_name()
                } else {
                    item.name.clone()
                };
                let ty = lower_sig_kind(&item.ty, ctx, item.has_bang);
                SigItem {
                    name,
                    ty,
                    has_bang: item.has_bang,
                    span: item.span,
                }
            })
            .collect(),
        generics: signature.generics.clone(),
    }
}

/// Normalize a HIR SigKind by resolving any `Ident` that refers to an imported builtin (ctxentry.is_builtin=true),
/// converting e.g. `Ident("str") -> SigKind::Str`, `Ident("int") -> SigKind::Int`, etc.
/// Uses a single canonical folder to avoid match duplication.
pub fn normalize_signature(signature: &Signature, ctx: &ctx::Context) -> Signature {
    let items = signature
        .items
        .iter()
        .map(|item| {
            let mut normalized_item = item.clone();
            normalized_item.ty = normalize_sig_kind(&item.ty, ctx);
            normalized_item
        })
        .collect();
    Signature {
        items,
        span: signature.span,
        generics: signature.generics.clone(),
    }
}

pub fn normalize_sig_kind(kind: &SigKind, ctx: &ctx::Context) -> SigKind {
    let mut seen = HashSet::new();
    normalize_sig_kind_inner(kind, ctx, &mut seen)
}

fn normalize_sig_kind_inner(
    kind: &SigKind,
    ctx: &ctx::Context,
    seen: &mut HashSet<String>,
) -> SigKind {
    match kind {
        SigKind::Ident(ident) => {
            if seen.contains(&ident.name) {
                return SigKind::Ident(ident.clone());
            }
            if let Some(entry) = ctx.get(&ident.name) {
                seen.insert(ident.name.clone());
                let resolved = normalize_sig_kind_inner(&entry.kind, ctx, seen);
                seen.remove(&ident.name);
                return resolved;
            }
            SigKind::Ident(ident.clone())
        }
        SigKind::Sig(signature) => {
            let items = signature
                .items
                .iter()
                .map(|item| {
                    let mut normalized_item = item.clone();
                    normalized_item.ty = normalize_sig_kind_inner(&item.ty, ctx, seen);
                    normalized_item
                })
                .collect();
            SigKind::Sig(Signature {
                items,
                span: signature.span,
                generics: signature.generics.clone(),
            })
        }
        other => other.clone(),
    }
}

pub fn resolve_target_signature(target: &str, ctx: &ctx::Context) -> Option<Signature> {
    let mut visited = HashSet::new();
    ctx.get(target)
        .and_then(|entry| signature_from_kind(&entry.kind, ctx, &mut visited))
}

// With cycle detection
pub fn signature_from_kind(
    kind: &SigKind,
    ctx: &ctx::Context,
    visited: &mut HashSet<String>,
) -> Option<Signature> {
    match kind {
        SigKind::Sig(signature) => Some(signature.clone()),

        SigKind::Ident(ident) => {
            let name = &ident.name;
            if !visited.insert(name.clone()) {
                return None; // Cycle detected
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
    params: &'a [SigItem],
    args_len: usize,
) -> Vec<Option<&'a SigItem>> {
    let mut expected = Vec::with_capacity(args_len);
    if args_len == 0 {
        return expected;
    }

    let mut prefix_assigned = 0;
    let mut suffix_start = args_len;
    let variadic_index = params
        .iter()
        .position(|item| matches!(item.ty, SigKind::Variadic));

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

/// Core logic for resolving identifiers.
/// If ctx has an entry for `ident.name` and it is_builtin=true, we convert to the builtin kind:
/// `str -> SigKind::Str`, `int -> SigKind::Int`, etc.
/// Otherwise it remains a user-defined SigKind::Ident.
fn resolve_ident(ident: &SigIdent, ctx: &ctx::Context) -> (SigKind, bool) {
    if let Some(entry) = ctx.get(&ident.name) {
        if entry.is_builtin {
            return (entry.kind.clone(), true);
        }
    }
    (SigKind::Ident(ident.clone()), false)
}

fn lower_sig_kind(ast_kind: &ast::SigKind, ctx: &mut ctx::Context, has_bang: bool) -> SigKind {
    match ast_kind {
        ast::SigKind::Ident(ast_ident) => {
            let ident = SigIdent {
                name: ast_ident.name.clone(),
                span: ast_ident.span,
            };
            let kind = resolve_ident(&ident, ctx).0;
            if has_bang {
                match kind {
                    SigKind::Int => SigKind::CompileTimeInt,
                    SigKind::Str => SigKind::CompileTimeStr,
                    SigKind::CompileTimeInt => SigKind::CompileTimeInt,
                    SigKind::CompileTimeStr => SigKind::CompileTimeStr,
                    other => other,
                }
            } else {
                kind
            }
        }
        ast::SigKind::Sig(signature) => SigKind::Sig(resolve_ast_signature(signature, ctx)),
        ast::SigKind::GenericInst { name, args } => {
            let resolved_args = args
                .iter()
                .map(|arg| lower_sig_kind(&arg, ctx, false)) // TODO: handle has_bang?
                .collect::<Vec<_>>();

            instantiate_generic_inst(name, &resolved_args, ctx).unwrap_or_else(|| {
                SigKind::Ident(SigIdent {
                    name: name.clone(),
                    span: Span::unknown(),
                })
            })
        }
        ast::SigKind::Generic(name) => SigKind::Ident(SigIdent {
            name: name.clone(),
            span: Span::unknown(),
        }),
        ast::SigKind::Int => {
            if has_bang {
                SigKind::CompileTimeInt
            } else {
                SigKind::Int
            }
        }
        ast::SigKind::Str => {
            if has_bang {
                SigKind::CompileTimeStr
            } else {
                SigKind::Str
            }
        }
        ast::SigKind::Variadic => SigKind::Variadic,
        ast::SigKind::CompileTimeInt => SigKind::CompileTimeInt,
        ast::SigKind::CompileTimeStr => SigKind::CompileTimeStr,
    }
}

fn instantiate_generic_inst(name: &str, args: &[SigKind], ctx: &ctx::Context) -> Option<SigKind> {
    let entry = ctx.get(name)?;
    let signature = if let SigKind::Sig(signature) = &entry.kind {
        signature
    } else {
        return None;
    };

    if signature.generics.len() != args.len() {
        return None;
    }

    let mapping: HashMap<String, SigKind> = signature
        .generics
        .iter()
        .cloned()
        .zip(args.iter().cloned())
        .collect();

    Some(SigKind::Sig(substitute_signature(signature, &mapping)))
}

fn substitute_signature(signature: &Signature, mapping: &HashMap<String, SigKind>) -> Signature {
    let items = signature
        .items
        .iter()
        .map(|item| {
            let mut out = item.clone();
            out.ty = substitute_kind(&item.ty, mapping);
            out
        })
        .collect();

    Signature {
        items,
        span: signature.span,
        generics: Vec::new(),
    }
}

// TODO: Remove this
fn substitute_kind(kind: &SigKind, mapping: &HashMap<String, SigKind>) -> SigKind {
    match kind {
        SigKind::Sig(signature) => SigKind::Sig(substitute_signature(signature, mapping)),
        SigKind::Ident(ident) => {
            if let Some(mapped) = mapping.get(&ident.name) {
                mapped.clone()
            } else {
                SigKind::Ident(ident.clone())
            }
        }
        _ => kind.clone(),
    }
}
