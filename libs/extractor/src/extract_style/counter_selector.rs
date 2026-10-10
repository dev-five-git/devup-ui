use std::cmp::Ordering;

use css::style_selector::StyleSelector;

#[derive(PartialEq, Eq, Hash)]
pub(super) struct CounterSelector<'a>(pub(super) &'a Option<StyleSelector>);

impl CounterSelector<'_> {
    fn global_file(&self) -> Option<&str> {
        self.0.as_ref().and_then(|selector| match selector {
            StyleSelector::Global(_, file) => Some(file.as_str()),
            StyleSelector::At { .. } | StyleSelector::Selector(_) => None,
        })
    }
}

impl PartialOrd for CounterSelector<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CounterSelector<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .cmp(other.0)
            .then_with(|| self.global_file().cmp(&other.global_file()))
    }
}

#[cfg(test)]
mod tests {
    use css::style_selector::AtRuleKind;

    use super::*;

    #[test]
    fn partial_order_preserves_full_equality_when_global_cleanup_owners_differ() {
        // Given: absent, scoped, at-rule and otherwise identical global selectors.
        let selectors = [
            None,
            Some(StyleSelector::Selector(":hover".into())),
            Some(StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "screen".into(),
                selector: None,
                outer: vec![],
                file: None,
            }),
            Some(StyleSelector::Global("body".into(), "a.tsx".into())),
            Some(StyleSelector::Global("body".into(), "b.tsx".into())),
        ];
        // When: each pair is compared through both ordering interfaces.
        for left in &selectors {
            for right in &selectors {
                let left = CounterSelector(left);
                let right = CounterSelector(right);
                let ordering = left.cmp(&right);
                // Then: partial ordering has no owner-losing equality tie.
                assert_eq!(left.partial_cmp(&right), Some(ordering));
                assert_eq!(left == right, ordering == Ordering::Equal);
            }
        }
    }
}
