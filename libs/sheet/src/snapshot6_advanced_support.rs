pub(super) use super::test_support::*;
use css::style_selector::{AtRule, AtRuleKind, StyleSelector};

pub(super) fn advanced_sheet(mode: u8) -> StyleSheet {
    css::debug::set_debug(mode == 1);
    css::atom_hoist::set_atom_hoist((mode == 2).then_some(2));
    css::atom_hoist::restore_atom_plan((mode == 2).then(|| BTreeSet::from(["a".into()])));
    let mut sheet = StyleSheet::default();
    sheet.set_theme(crate::theme::Theme {
        typography: BTreeMap::from([(
            "heading".into(),
            crate::theme::Typographies(vec![
                Some(frame("14px", Some("400"))),
                None,
                Some(frame("24px", None)),
            ]),
        )]),
        ..Default::default()
    });
    let outer = AtRule {
        kind: AtRuleKind::Supports,
        query: "(display:grid)".into(),
    };
    let selector = StyleSelector::At {
        kind: AtRuleKind::Container,
        query: "(min-width:1px)".into(),
        selector: Some("body".into()),
        outer: vec![outer.clone(), outer],
        file: Some("structured-owner".into()),
    };
    let items = fixture("a", || {
        styles([
            ExtractStyleValue::Static(ExtractStaticStyle::new("width", "20px", 0, Some(selector))),
            ExtractStyleValue::Static(ExtractStaticStyle::new(
                "typography",
                "heading|font-weight:2,font-weight:2",
                0,
                None,
            )),
            ExtractStyleValue::Dynamic(
                ExtractDynamicStyle::new(
                    "color",
                    0,
                    "tone",
                    Some(StyleSelector::Global("body".into(), "a".into())),
                )
                .at_role(2, 1),
            ),
        ])
    });
    update(&mut sheet, &items).required("advanced genuine update");
    assert!(sheet.rm_global_css("a", false));
    for owner in ["left-owner", "right-owner"] {
        sheet.add_property(
            "owner-equal",
            "color",
            0,
            "red",
            Some(&StyleSelector::Global("body".into(), owner.into())),
            None,
            Some("a"),
        );
    }
    sheet
}

pub(super) fn paths(value: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    super::schema_tests::object_paths(value, "", &mut paths);
    paths
}

pub(super) fn at_record_path(value: &Value) -> String {
    paths(value)
        .into_iter()
        .find(|path| {
            value
                .pointer(path)
                .is_some_and(|node| node.get("c").is_some() && node["s"].get("At").is_some())
        })
        .required("structured At record")
}
