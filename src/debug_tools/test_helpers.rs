use crate::compiler::{
    ast,
    error::Error,
    hir, mir,
    symbol::{self, SymbolRegistry},
};

pub fn generate_mir_functions(items: &[hir::BlockItem]) -> Result<Vec<mir::MirFunction>, Error> {
    let mut symbols = SymbolRegistry::new();
    let mut functions = Vec::new();
    let mut entry_items = Vec::new();
    for item in items {
        match item {
            hir::BlockItem::Import { name, span } => {
                symbol::register_builtin_import(name, *span, &mut symbols)?;
            }
            hir::BlockItem::SigDef { name, sig, span } => {
                symbols.install_type(name.to_string(), ast::SigKind::Sig(sig.clone()), *span)?;
            }
            hir::BlockItem::FunctionDef(function) => {
                symbols.declare_function(mir::FunctionSig {
                    name: function.name.clone(),
                    params: function.sig.items.clone(),
                    span: function.span,
                    builtin: None,
                })?;
                let lowered_functions = mir::lower_function(function, &mut symbols)?;
                functions.extend(lowered_functions);
            }
            _ => entry_items.push(item.clone()),
        }
    }

    if !entry_items.is_empty() {
        let mir_funcs = mir::entry_function(entry_items, &mut symbols)?;
        functions.extend(mir_funcs);
    }

    Ok(functions)
}
