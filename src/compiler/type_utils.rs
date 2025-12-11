use std::collections::HashSet;

use crate::compiler::hir;
use crate::compiler::symbol::SymbolRegistry;

// TODO: This is already handled by the resolution, duplicate logic here.
pub fn expand_alias_chain(
    kind: &hir::SigKind, // TODO: should be the type here
    symbols: &SymbolRegistry,
    visited: &mut HashSet<String>,
) -> hir::SigKind {
    match kind {
        // -------- IDENT --------
        hir::SigKind::Ident(ident) => {
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
        hir::SigKind::GenericInst { name, args } => {
            let expanded_args: Vec<_> = args
                .iter()
                .map(|arg| expand_alias_chain(arg, symbols, visited))
                .collect();

            if visited.contains(name) {
                return hir::SigKind::GenericInst {
                    name: name.clone(),
                    args: expanded_args,
                };
            }

            if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());

                // substitute_generics takes SigKind target + args
                let expanded = expand_alias_chain(&info.target, symbols, visited);

                visited.remove(name);
                expanded
            } else {
                hir::SigKind::GenericInst {
                    name: name.clone(),
                    args: expanded_args,
                }
            }
        }

        // -------- TUPLE SIGNATURE --------
        hir::SigKind::Tuple(sig) => {
            let new_items: Vec<hir::SigItem> = sig
                .items
                .iter()
                .map(|item| {
                    let new_kind = expand_alias_chain(&item.ty.kind, symbols, visited);
                    hir::SigItem {
                        name: item.name.clone(),
                        ty: hir::SigType {
                            kind: new_kind,
                            span: item.ty.span,
                        },
                        is_variadic: item.is_variadic,
                        span: item.span,
                    }
                })
                .collect();

            hir::SigKind::Tuple(hir::Signature {
                items: new_items,
                span: sig.span,
            })
        }

        // -------- GENERIC, ETC. --------
        other => other.clone(),
    }
}
