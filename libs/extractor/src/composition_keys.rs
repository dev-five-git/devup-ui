use super::CascadeKey;
use crate::{ExtractStyleProp, ExtractStyleValue};

/// The keys `props` set, `None` when a style chosen at runtime has none;
/// classes only the runtime gives compose at runtime, so they set none here.
pub(super) fn keys(props: &[ExtractStyleProp<'_>]) -> Option<Vec<CascadeKey>> {
    let mut keys = Vec::new();
    for prop in props {
        match prop {
            ExtractStyleProp::Static(ExtractStyleValue::LayerOrder(_))
            | ExtractStyleProp::Expression { .. }
            | ExtractStyleProp::Unreadable { .. } => {}
            ExtractStyleProp::Static(value) => keys.push(CascadeKey::of(value)),
            ExtractStyleProp::StaticArray(props) => keys.extend(self::keys(props)?),
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => {
                for side in [consequent, alternate].into_iter().flatten() {
                    keys.extend(self::keys(std::slice::from_ref(side.as_ref()))?);
                }
            }
            ExtractStyleProp::Enum { .. } | ExtractStyleProp::MemberExpression { .. } => {
                return None;
            }
        }
    }
    Some(keys)
}
