use std::collections::HashSet;

use crate::compiler::ast;
use crate::compiler::span::Span;
use crate::compiler::symbol::SymbolRegistry;

// TODO: This is already handled by the resolution, duplicate logic here.
pub fn expand_alias_chain(
    kind: &ast::SigKind, // TODO: should be the type here
    symbols: &SymbolRegistry,
    visited: &mut HashSet<String>,
) -> ast::SigKind {
    let span = Span::unknown(); // TODO: because not passing SigType, can't get span
    match kind {
        // -------- IDENT --------
        ast::SigKind::Ident(ident) => {
            let name = &ident.name;

            if visited.contains(name) {
                return kind.clone();
            }

            if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());

                // info.target must be a *SigKind*, so expand it
                let expanded = expand_alias_chain(&info.target, symbols, visited);

                visited.remove(name);
                expanded
            } else {
                kind.clone()
            }
        }

        // -------- GENERIC INST --------
        ast::SigKind::GenericInst { name, args } => {
            let expanded_args: Vec<_> = args
                .iter()
                .map(|arg| expand_alias_chain(&arg.kind, symbols, visited))
                .collect();

            if visited.contains(name) {
                return ast::SigKind::GenericInst {
                    name: name.clone(),
                    args: expanded_args
                        .into_iter()
                        .map(|kind| ast::SigType { kind, span: span })
                        .collect(),
                };
            }

            if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());

                // substitute_generics takes SigKind target + args
                let substituted = substitute_generics(&info.target, &info.generics, &expanded_args);

                let expanded = expand_alias_chain(&substituted, symbols, visited);

                visited.remove(name);
                expanded
            } else {
                ast::SigKind::GenericInst {
                    name: name.clone(),
                    args: expanded_args
                        .into_iter()
                        .map(|kind| ast::SigType { kind, span: span })
                        .collect(),
                }
            }
        }

        // -------- TUPLE SIGNATURE --------
        ast::SigKind::Tuple(sig) => {
            let mut new_items = Vec::new();

            for item in &sig.items {
                // Expand inside the SigItem.ty.kind
                let new_kind = expand_alias_chain(&item.ty.kind, symbols, visited);

                new_items.push(ast::SigItem {
                    name: item.name.clone(),
                    ty: ast::SigType {
                        kind: new_kind,
                        span: item.ty.span,
                    },
                    is_variadic: item.is_variadic,
                    span: item.span,
                });
            }

            ast::SigKind::Tuple(ast::Signature {
                items: new_items,
                span: sig.span,
            })
        }

        // -------- GENERIC, ETC. --------
        other => other.clone(),
    }
}

fn substitute_generics(
    ty: &ast::SigKind,
    generics: &[String],
    values: &[ast::SigKind],
) -> ast::SigKind {
    match ty {
        // --- Generic parameter ---
        ast::SigKind::Generic(name) => {
            if let Some(idx) = generics.iter().position(|g| g == name) {
                return values[idx].clone();
            }
            ty.clone()
        }

        // --- Tuple Signature: (items...) ---
        ast::SigKind::Tuple(sig) => {
            let new_items = sig
                .items
                .iter()
                .map(|item| {
                    // Substitute inside the type of each SigItem
                    let new_kind = substitute_generics(&item.ty.kind, generics, values);

                    ast::SigItem {
                        name: item.name.clone(),
                        ty: ast::SigType {
                            kind: new_kind,
                            span: item.ty.span,
                        },
                        is_variadic: item.is_variadic,
                        span: item.span,
                    }
                })
                .collect();

            ast::SigKind::Tuple(ast::Signature {
                items: new_items,
                span: sig.span,
            })
        }

        // --- Generic Instantiation: Foo<T, X> ---
        ast::SigKind::GenericInst { name, args } => {
            let new_args = args
                .iter()
                .map(|arg| {
                    let new_kind = substitute_generics(&arg.kind, generics, values);
                    ast::SigType {
                        kind: new_kind,
                        span: arg.span,
                    }
                })
                .collect();

            ast::SigKind::GenericInst {
                name: name.clone(),
                args: new_args,
            }
        }

        // --- All other kinds ---
        other => other.clone(),
    }
}
