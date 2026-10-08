use oxc_ast::ast::{BindingPattern, FormalParameters};
use oxc_syntax::symbol::SymbolId;

/// The `css`, `cx` and theme bindings a `<ClassNames>` child function takes
#[derive(Default, Clone, Copy)]
pub struct ClassNamesSymbols {
    pub css: Option<SymbolId>,
    pub cx: Option<SymbolId>,
    pub theme: Option<SymbolId>,
}

impl ClassNamesSymbols {
    /// The bindings `params`, already read as `{ css, cx, theme }`, declare
    pub fn of(params: &FormalParameters<'_>) -> Self {
        let mut symbols = Self::default();
        if let Some(BindingPattern::ObjectPattern(object)) =
            params.items.first().map(|param| &param.pattern)
        {
            for property in &object.properties {
                let symbol = property
                    .value
                    .get_binding_identifier()
                    .and_then(|local| local.symbol_id.get());
                let slot = match property.key.static_name().as_deref() {
                    Some("css") => &mut symbols.css,
                    Some("cx") => &mut symbols.cx,
                    _ => &mut symbols.theme,
                };
                *slot = symbol;
            }
        }
        symbols
    }

    /// Whether `symbol` is the binding `slot` holds
    pub fn is(slot: Option<SymbolId>, symbol: Option<SymbolId>) -> bool {
        slot.is_some() && slot == symbol
    }
}
