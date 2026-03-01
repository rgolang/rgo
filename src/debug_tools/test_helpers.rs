use std::collections::HashMap;

use crate::compiler::{
    air,
    error::Error,
    hir,
    symbol::{self, SymbolRegistry},
};

pub fn generate_air_functions(items: &[hir::BlockItem]) -> Result<Vec<air::AirFunction>, Error> {
    let mut symbols = SymbolRegistry::new();
    let mut functions = Vec::new();
    let mut entry_items = Vec::new();
    let mut hir_functions = HashMap::new();

    for item in items {
        match item {
            hir::BlockItem::Import { label, path } => {
                symbol::register_builtin_import(label, path, &mut symbols)?;
            }
            hir::BlockItem::SigDef { name, sig } => {
                symbols.install_type(name.to_string(), air::SigKind::Sig(sig.clone()))?;
            }
            hir::BlockItem::FunctionDef(function) => {
                symbols.declare_function(air::function_sig_from_hir(function))?;
                hir_functions.insert(function.name.clone(), function.clone());
            }
            _ => entry_items.push(item.clone()),
        }
    }

    if !entry_items.is_empty() {
        let mut function_lowerer = air::FunctionLowerer::new(hir_functions);
        let entry_funcs = air::entry_function(entry_items, &mut symbols, &mut function_lowerer)?;
        let mut generated = function_lowerer.take_generated_functions();
        generated.extend(entry_funcs);
        functions.extend(generated);
    }

    Ok(functions)
}
