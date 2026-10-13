use oxc_ast::ast::{BindingPattern, Expression};
use oxc_syntax::symbol::SymbolId;

pub(super) struct BindingPath<'e, 'a> {
    pub keys: Vec<String>,
    pub defaults: Vec<&'e Expression<'a>>,
}

pub(super) fn find<'e, 'a>(
    pattern: &'e BindingPattern<'a>,
    symbol: SymbolId,
) -> Option<BindingPath<'e, 'a>> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => {
            (identifier.symbol_id.get() == Some(symbol)).then_some(BindingPath {
                keys: Vec::new(),
                defaults: Vec::new(),
            })
        }
        BindingPattern::AssignmentPattern(pattern) => {
            let mut path = find(&pattern.left, symbol)?;
            if path.keys.is_empty() {
                path.defaults.push(&pattern.right);
            }
            Some(path)
        }
        BindingPattern::ObjectPattern(object) => object.properties.iter().find_map(|property| {
            let mut path = find(&property.value, symbol)?;
            path.keys
                .insert(0, property.key.static_name()?.into_owned());
            Some(path)
        }),
        BindingPattern::ArrayPattern(array) => {
            array
                .elements
                .iter()
                .enumerate()
                .find_map(|(index, element)| {
                    let mut path = find(element.as_ref()?, symbol)?;
                    path.keys.insert(0, index.to_string());
                    Some(path)
                })
        }
    }
}
