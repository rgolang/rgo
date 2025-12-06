use std::collections::HashSet;

use crate::compiler::ast::TypeRef;
use crate::compiler::symbol::SymbolRegistry;

pub fn expand_alias_chain(
    ty: &TypeRef,
    symbols: &SymbolRegistry,
    visited: &mut HashSet<String>,
) -> TypeRef {
    match ty {
        TypeRef::Alias(name) => {
            if visited.contains(name) {
                return ty.clone();
            }
            if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());
                let expanded = expand_alias_chain(&info.target, symbols, visited);
                visited.remove(name);
                expanded
            } else {
                ty.clone()
            }
        }
        TypeRef::AliasInstance { name, args } => {
            if visited.contains(name) {
                return TypeRef::AliasInstance {
                    name: name.clone(),
                    args: args
                        .iter()
                        .map(|arg| expand_alias_chain(arg, symbols, visited))
                        .collect(),
                };
            }
            if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());
                let expanded_args = args
                    .iter()
                    .map(|arg| expand_alias_chain(arg, symbols, visited))
                    .collect::<Vec<_>>();
                let substituted = substitute_generics(&info.target, &info.generics, &expanded_args);
                let expanded = expand_alias_chain(&substituted, symbols, visited);
                visited.remove(name);
                expanded
            } else {
                TypeRef::AliasInstance {
                    name: name.clone(),
                    args: args
                        .iter()
                        .map(|arg| expand_alias_chain(arg, symbols, visited))
                        .collect(),
                }
            }
        }
        TypeRef::Type(params) => TypeRef::Type(
            params
                .iter()
                .map(|param| expand_alias_chain(param, symbols, visited))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn substitute_generics(ty: &TypeRef, generics: &[String], values: &[TypeRef]) -> TypeRef {
    match ty {
        TypeRef::Generic(name) => {
            if let Some(idx) = generics.iter().position(|param| param == name) {
                return values[idx].clone();
            }
            ty.clone()
        }
        TypeRef::Type(params) => TypeRef::Type(
            params
                .iter()
                .map(|param| substitute_generics(param, generics, values))
                .collect(),
        ),
        TypeRef::AliasInstance { name, args } => TypeRef::AliasInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_generics(arg, generics, values))
                .collect(),
        },
        other => other.clone(),
    }
}
