use oxc_ast::ast::ObjectPropertyKind;

use crate::utils::get_string_by_property_key;

/// Whether later explicit keys leave this assignment as the object's final value.
pub(crate) fn is_final_assignment(name: &str, later: &[ObjectPropertyKind<'_>]) -> bool {
    !later.iter().any(|property| match property {
        ObjectPropertyKind::ObjectProperty(property) => {
            get_string_by_property_key(&property.key).is_some_and(|key| key == name)
        }
        ObjectPropertyKind::SpreadProperty(_) => false,
    })
}
