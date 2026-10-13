use crate::ExtractStyleProp;

pub(crate) fn collect(
    styles: &[ExtractStyleProp<'_>],
    errors: &mut Vec<(u32, String)>,
) -> crate::ErrorDisposition {
    let mut disposition = crate::ErrorDisposition::NeedsEvaluation;
    for style in styles {
        match style {
            ExtractStyleProp::Diagnostic {
                offset,
                message,
                disposition: kind,
            } => {
                errors.push((*offset, message.clone()));
                disposition.include(*kind);
            }
            ExtractStyleProp::StaticArray(styles) => disposition.include(collect(styles, errors)),
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => {
                for branch in [consequent, alternate].into_iter().flatten() {
                    disposition.include(collect(std::slice::from_ref(branch.as_ref()), errors));
                }
            }
            ExtractStyleProp::Enum { map, .. } => {
                for styles in map.values() {
                    disposition.include(collect(styles, errors));
                }
            }
            ExtractStyleProp::MemberExpression { map, .. } => {
                for style in map.values() {
                    disposition.include(collect(std::slice::from_ref(style.as_ref()), errors));
                }
            }
            ExtractStyleProp::Static(_)
            | ExtractStyleProp::Expression { .. }
            | ExtractStyleProp::Unreadable { .. } => {}
        }
    }
    disposition
}
