use super::{Composition, into_prop};
use crate::{ExtractStyleProp, ExtractStyleValue};
use css::style_selector::StyleSelector;

#[derive(Clone)]
pub(crate) struct Coverage(Option<Vec<CascadeKey>>);

pub(crate) fn coverage(props: &[ExtractStyleProp<'_>]) -> Coverage {
    Coverage(super::key_query::keys(props))
}

impl Coverage {
    pub(crate) fn overlaps(&self, props: &[ExtractStyleProp<'_>]) -> bool {
        match (&self.0, super::key_query::keys(props)) {
            (Some(earlier), Some(later)) => later.iter().any(|key| earlier.contains(key)),
            (None, _) | (_, None) => true,
        }
    }

    pub(crate) fn union(&self, own: &Self) -> Self {
        Self(match (&self.0, &own.0) {
            (Some(earlier), Some(later)) => Some(earlier.iter().chain(later).cloned().collect()),
            (None, _) | (_, None) => None,
        })
    }
}

/// What a declaration competes on: property, selector, breakpoint and layer.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CascadeKey {
    property: String,
    selector: Option<StyleSelector>,
    level: u8,
    layer: Option<String>,
}

impl CascadeKey {
    pub(super) fn of(value: &ExtractStyleValue) -> Self {
        let (property, selector, level, layer) = match value {
            ExtractStyleValue::Static(style) => (
                style.property(),
                style.selector(),
                style.level(),
                style.layer(),
            ),
            ExtractStyleValue::Dynamic(style) => (
                style.property(),
                style.selector(),
                style.level(),
                style.layer(),
            ),
            _ => ("typography", None, 0, None),
        };
        Self {
            property: property.to_string(),
            selector: selector.cloned(),
            level,
            layer: layer.map(ToString::to_string),
        }
    }
}

impl<'a> Composition<'a> {
    pub(crate) fn into_residual(self) -> Vec<(Option<CascadeKey>, Vec<ExtractStyleProp<'a>>)> {
        let mut entries: Vec<_> = self
            .entries
            .into_iter()
            .map(|(key, choice)| (Some(key), into_prop(choice).into_iter().collect()))
            .collect();
        if !self.unkeyed.is_empty() {
            entries.push((None, self.unkeyed));
        }
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_style::{
        extract_dynamic_style::ExtractDynamicStyle, extract_static_style::ExtractStaticStyle,
    };

    #[test]
    fn overlap_when_dynamic_atoms_differ_only_in_value_uses_the_cascade_key() {
        // Given
        let atom = |identifier: &str, layer: &str| {
            let mut style = ExtractDynamicStyle::new("width", 1, identifier, None);
            style.layer = Some(layer.to_string());
            ExtractStyleProp::Static(ExtractStyleValue::Dynamic(style))
        };
        let earlier = coverage(&[atom("first", "base")]);
        // When
        let same = earlier.overlaps(&[atom("second", "base")]);
        let separate = earlier.overlaps(&[atom("second", "theme")]);
        // Then
        assert!(same);
        assert!(!separate);
    }

    #[test]
    fn union_when_either_side_has_unknown_keys_keeps_overlap_conservative() {
        // Given
        let arena = oxc_allocator::Allocator::default();
        let condition = oxc_parser::Parser::new(&arena, "variant", oxc_span::SourceType::tsx())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}"));
        let atom = || {
            ExtractStyleProp::Static(ExtractStyleValue::Static(ExtractStaticStyle::new(
                "color", "red", 0, None,
            )))
        };
        let known = coverage(&[atom()]);
        let unknown = coverage(&[ExtractStyleProp::Enum {
            condition,
            map: std::collections::BTreeMap::from([("red".into(), vec![atom()])]),
        }]);
        let disjoint = [ExtractStyleProp::Static(ExtractStyleValue::Static(
            ExtractStaticStyle::new("margin", "4px", 0, None),
        ))];
        // When
        let known_then_unknown = known.union(&unknown);
        let unknown_then_known = unknown.union(&known);
        // Then
        assert!(!known.overlaps(&disjoint));
        assert!(known_then_unknown.overlaps(&disjoint));
        assert!(unknown_then_known.overlaps(&disjoint));
    }
}
