use super::Constant;

impl Constant {
    /// Check existence without selecting or retaining a diagnostic origin.
    pub(super) fn has_change(&self) -> bool {
        match self {
            Self::Changed(_) => true,
            Self::Object(object) => object.values().any(Self::has_change),
            Self::Record(entries) => entries.iter().any(|(_, value)| value.has_change()),
            Self::Array(values) => values.iter().any(Self::has_change),
            Self::String(_)
            | Self::Number(_)
            | Self::Null
            | Self::Bool(_)
            | Self::Undefined
            | Self::Function
            | Self::Vars(_)
            | Self::Theme(_)
            | Self::Style(_)
            | Self::Failed(_)
            | Self::Unavailable(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ChainReason, Change, ChangeSite, Constant};
    use rustc_hash::FxHashMap;
    use std::rc::Rc;

    #[test]
    fn presence_is_absent_when_leaf_variants_hold_no_change() {
        // Given: every non-container variant without a retained change.
        let leaves = [
            Constant::String("value".into()),
            Constant::Number(1.0),
            Constant::Null,
            Constant::Bool(true),
            Constant::Undefined,
            Constant::Function,
            Constant::Vars(Rc::new(FxHashMap::default())),
            Constant::Theme("theme".into()),
            Constant::Style(None),
            Constant::Style(Some(Rc::new(vec![]))),
            Constant::Failed("failure".into()),
            Constant::Unavailable(ChainReason::Cycle),
            Constant::Unavailable(ChainReason::Module("module".into())),
            Constant::Unavailable(ChainReason::Export("export".into())),
            Constant::Unavailable(ChainReason::Constant),
            Constant::Unavailable(ChainReason::Static),
        ];
        for leaf in leaves {
            // When: asking only for change presence.
            let present = leaf.has_change();
            // Then: no leaf invents an origin.
            assert!(!present, "{leaf:?}");
            assert_eq!(present, leaf.change().is_some());
        }
    }

    #[test]
    fn presence_is_absent_when_nested_containers_have_no_changed_descendant() {
        // Given: empty and populated containers nested through every container kind.
        let empty = [
            Constant::Object(Rc::new(FxHashMap::default())),
            Constant::Record(Rc::new(vec![])),
            Constant::Array(Rc::new(vec![])),
        ];
        for value in empty {
            let nested = Constant::Object(Rc::new(FxHashMap::from_iter([(
                "outer".into(),
                Constant::Record(Rc::new(vec![(
                    "inner".into(),
                    Constant::Array(Rc::new(vec![Constant::Null, value])),
                )])),
            )])));
            // When: checking a fully unchanged tree.
            let present = nested.has_change();
            // Then: recursive presence agrees with absent diagnostic selection.
            assert!(!present);
            assert_eq!(present, nested.change().is_some());
        }
    }

    #[test]
    fn presence_is_true_when_any_container_holds_a_changed_descendant() {
        // Given: multiple origins, with a direct change and each container nesting.
        let alpha = Rc::new(Change {
            name: "zeta".into(),
            site: ChangeSite::Here(3),
            handed: false,
        });
        let beta = Rc::new(Change {
            name: "alpha".into(),
            site: ChangeSite::Here(4),
            handed: false,
        });
        let changed = Constant::Changed(alpha.clone());
        let cases = [
            changed.clone(),
            Constant::Object(Rc::new(FxHashMap::from_iter([
                ("beta".into(), Constant::Changed(beta)),
                (
                    "alpha".into(),
                    Constant::Array(Rc::new(vec![changed.clone()])),
                ),
            ]))),
            Constant::Record(Rc::new(vec![
                ("empty".into(), Constant::Null),
                ("changed".into(), changed.clone()),
            ])),
            Constant::Array(Rc::new(vec![
                Constant::Null,
                Constant::Record(Rc::new(vec![("nested".into(), changed)])),
            ])),
        ];
        for value in cases {
            let references = Rc::strong_count(&alpha);
            // When: checking presence without selecting or owning an origin.
            let present = value.has_change();
            // Then: existence agrees, ownership is untouched, lexical selection survives.
            assert!(present);
            assert_eq!(Rc::strong_count(&alpha), references);
            assert_eq!(present, value.change().is_some());
            assert!(
                value
                    .change()
                    .is_some_and(|origin| Rc::ptr_eq(&origin, &alpha))
            );
        }
    }
}
