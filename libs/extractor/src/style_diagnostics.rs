use crate::ExtractStyleProp;

pub(crate) fn collect(styles: &[ExtractStyleProp<'_>], errors: &mut Vec<(u32, String)>) {
    for style in styles {
        match style {
            ExtractStyleProp::Diagnostic { offset, message } => {
                errors.push((*offset, message.clone()));
            }
            ExtractStyleProp::StaticArray(styles) => collect(styles, errors),
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => {
                for branch in [consequent, alternate].into_iter().flatten() {
                    collect(std::slice::from_ref(branch.as_ref()), errors);
                }
            }
            ExtractStyleProp::Enum { map, .. } => {
                for styles in map.values() {
                    collect(styles, errors);
                }
            }
            ExtractStyleProp::MemberExpression { map, .. } => {
                for style in map.values() {
                    collect(std::slice::from_ref(style.as_ref()), errors);
                }
            }
            ExtractStyleProp::Static(_)
            | ExtractStyleProp::Expression { .. }
            | ExtractStyleProp::Unreadable { .. } => {}
        }
    }
}
