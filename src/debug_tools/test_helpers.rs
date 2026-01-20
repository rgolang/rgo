use crate::compiler::{
    air, ast,
    error::Error,
    hir,
    symbol::{self, SymbolRegistry},
};

pub fn generate_air_functions(items: &[hir::BlockItem]) -> Result<Vec<air::AirFunction>, Error> {
    let mut symbols = SymbolRegistry::new();
    let mut functions = Vec::new();
    let mut entry_items = Vec::new();
    for item in items {
        match item {
            hir::BlockItem::Import { label, path, span } => {
                symbol::register_builtin_import(label, path, *span, &mut symbols)?;
            }
            hir::BlockItem::SigDef { name, sig, .. } => {
                symbols.install_type(name.to_string(), ast::SigKind::Sig(sig.clone()))?;
            }
            hir::BlockItem::FunctionDef(function) => {
                symbols.declare_function(air::FunctionSig {
                    name: function.name.clone(),
                    params: function.sig.items.clone(),
                    span: function.span,
                    builtin: None,
                })?;
                let lowered_functions = air::lower_function(&function, &mut symbols)?;
                functions.extend(lowered_functions);
            }
            _ => entry_items.push(item.clone()),
        }
    }

    if !entry_items.is_empty() {
        let air_funcs = air::entry_function(entry_items, &mut symbols)?;
        functions.extend(air_funcs);
    }

    Ok(functions)
}
