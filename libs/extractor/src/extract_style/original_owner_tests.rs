use css::{CounterOwner, Naming};
use rustc_hash::FxHashSet;
use serial_test::serial;

use super::{
    ExtractStyleProperty, extract_dynamic_style::ExtractDynamicStyle,
    extract_static_style::ExtractStaticStyle, extract_style_value::ExtractStyleValue,
};
use crate::sparse_sites::SiteScope;

#[test]
#[serial]
fn nested_raw_scopes_restore_owners_when_construction_is_deferred() {
    // Given
    css::file_map::reset_file_map();
    css::file_map::seed_file_numbers(&["a.tsx".into(), "child.tsx".into()]);
    let outer = SiteScope::enter("a.tsx", "outer", &[]);
    let first = ExtractStaticStyle::new("color", "red", 0, None);
    // When
    let nested = {
        let _inner = SiteScope::enter("child.tsx", "inner", &[]);
        ExtractStaticStyle::new_basic("color", "red", 0, None)
    };
    let restored = ExtractStaticStyle::new("color", "red", 0, None);
    drop(outer);
    // Then
    assert_eq!(first.counter_owner, CounterOwner::D9(0));
    assert_eq!(nested.counter_owner, CounterOwner::D9(1));
    assert_eq!(first, restored);
    assert_eq!(crate::sparse_sites::counter_owner(), CounterOwner::Inactive);
    css::file_map::reset_file_map();
}

#[test]
#[serial]
fn direct_order_and_naming_transitions_keep_only_counter_bearing_owners_distinct() {
    // Given
    css::file_map::reset_file_map();
    css::file_map::seed_file_numbers(&["a.tsx".into(), "child.tsx".into()]);
    let mut styles: Vec<_> = ["a.tsx", "child.tsx"]
        .into_iter()
        .map(|file| {
            let _scope = SiteScope::enter(file, "source", &[]);
            ExtractStaticStyle::new_basic("color", "red", 0, None)
        })
        .collect();
    assert_eq!(styles[0], styles[1]);
    // When
    for style in &mut styles {
        style.style_order = Some(255);
    }
    // Then
    assert_ne!(styles[0], styles[1]);
    assert_eq!(styles.iter().cloned().collect::<FxHashSet<_>>().len(), 2);
    assert_eq!(
        styles
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
    let risky: Vec<_> = styles
        .into_iter()
        .map(|style| style.with_naming(Naming::Risky))
        .collect();
    assert_eq!(risky[0], risky[1]);
    assert_eq!(risky.iter().cloned().collect::<FxHashSet<_>>().len(), 1);
    css::file_map::reset_file_map();
}

#[test]
#[serial]
fn dynamic_class_uses_site_file_when_delivery_is_collapsed() {
    // Given
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::file_map::seed_file_numbers(&["a.tsx".into(), "child.tsx".into()]);
    css::file_map::set_canonical_map(std::collections::HashMap::from([(
        "child.tsx".into(),
        "a.tsx".into(),
    )]));
    let style = {
        let _scope = SiteScope::enter("child.tsx", "tone", &[]);
        ExtractDynamicStyle::new("color", 0, "tone", None).at(0)
    };
    // When
    let result = style.extract(Some("a.tsx"));
    // Then
    match result {
        super::style_property::StyleProperty::Variable {
            class_name,
            variable_name,
            ..
        } => {
            assert_eq!(class_name, "b-a");
            assert_eq!(variable_name, "---Sb-a");
        }
        super::style_property::StyleProperty::ClassName(_) => panic!("dynamic class lost variable"),
    }
    css::class_map::with_class_map(|map| {
        assert_eq!(map.len(), 1);
        assert_eq!(map["D9-1"].len(), 1);
    });
    css::file_map::reset_canonical_map();
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
}

#[test]
#[serial]
fn provenance_join_leaves_original_owner_available_without_affecting_content() {
    // Given
    let mut value = ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None));
    let before = match &value {
        ExtractStyleValue::Static(style) => style.content_name(),
        other => panic!("{other:?}"),
    };
    // When
    value.join_naming(Naming::Risky);
    value.set_style_order(0);
    // Then
    match value {
        ExtractStyleValue::Static(style) => {
            assert_eq!(style.counter_owner, CounterOwner::Inactive);
            assert_ne!(style.content_name(), before);
            assert_eq!(style.style_order, Some(0));
        }
        other => panic!("{other:?}"),
    }
}
