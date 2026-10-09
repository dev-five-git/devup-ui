use crate::extract_style::extract_dynamic_style::ExtractDynamicStyle;
use crate::{ExtractStyleProp, ExtractStyleValue};

pub(super) fn reference(style: &ExtractDynamicStyle, responsive: bool) -> String {
    if responsive {
        format!("__devupValue?.[{}]", style.level())
    } else {
        "__devupValue".to_string()
    }
}

pub(super) fn rewrite(
    props: &mut [ExtractStyleProp<'_>],
    values: &[ExtractStyleValue],
    responsive: bool,
) {
    for prop in props {
        match prop {
            ExtractStyleProp::Static(value) => rewrite_value(value, values, responsive),
            ExtractStyleProp::StaticArray(props)
            | ExtractStyleProp::Evaluated { styles: props, .. } => {
                rewrite(props, values, responsive);
            }
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => {
                for branch in consequent.iter_mut().chain(alternate.iter_mut()) {
                    rewrite(std::slice::from_mut(branch.as_mut()), values, responsive);
                }
            }
            ExtractStyleProp::Enum { map, .. } => {
                for props in map.values_mut() {
                    rewrite(props, values, responsive);
                }
            }
            ExtractStyleProp::MemberExpression { map, .. } => {
                for prop in map.values_mut() {
                    rewrite(std::slice::from_mut(prop.as_mut()), values, responsive);
                }
            }
            ExtractStyleProp::Expression { styles, .. } => {
                for value in styles {
                    rewrite_value(value, values, responsive);
                }
            }
            ExtractStyleProp::Unreadable { .. } => {}
        }
    }
}

fn rewrite_value(value: &mut ExtractStyleValue, values: &[ExtractStyleValue], responsive: bool) {
    if let ExtractStyleValue::Dynamic(style) = value
        && style.presence()
        && values
            .iter()
            .any(|value| matches!(value, ExtractStyleValue::Dynamic(original) if original == style))
    {
        style.replace_identifier(&reference(style, responsive));
    }
}
