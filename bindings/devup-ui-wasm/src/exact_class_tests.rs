use super::*;
use exact_test_support::{authority, exact, fixture, mutate_authority};
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn exact_success_keeps_authoritative_mutations_and_sequential_sheet_entries() {
    // Given
    fixture();
    // When
    let result = exact(|| {
        mutate_authority();
        with_style_sheet(|sheet| assert!(sheet.css.contains_key("tentative.tsx")));
        with_style_sheet_mut(|sheet| sheet.add_import("tentative.tsx", "url('committed.css')"));
        css::class_map::reset_class_map();
        set_class_map(HashMap::from([(
            "committed".into(),
            HashMap::from([("kept".into(), 88)]),
        )]));
        Ok(())
    });
    // Then
    assert_eq!(result, Ok(()));
    assert_eq!(
        css::class_map::get_class_map(),
        HashMap::from([("committed".into(), HashMap::from([("kept".into(), 88)]))])
    );
    assert!(with_style_sheet(
        |sheet| sheet.imports["tentative.tsx"].contains("url('committed.css')")
    ));
    assert!(css::file_map::original_id("tentative.tsx").is_some());
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn abort_restores_full_class_map_after_owner_reset_or_replacement(#[case] replace: bool) {
    // Given
    fixture();
    let before = authority();
    // When
    let result: Result<(), String> = exact(|| {
        if replace {
            set_class_map(HashMap::from([("replacement".into(), HashMap::new())]));
        } else {
            css::class_map::reset_class_map();
        }
        let inner = css::class_map::Attempt::begin();
        let _name = css::keyframes_to_keyframes_name("new after replacement", None);
        inner.commit();
        Err("abort".into())
    });
    // Then
    assert_eq!(result, Err("abort".into()));
    assert_eq!(authority(), before);
    reset_build_state_internal();
}

#[test]
#[serial]
fn abort_reverses_both_no_site_requests_after_inner_commit() {
    use css::allocation_input::{LegacyDeclaration, LegacyInput, LegacyVariable, capture_context};
    // Given
    fixture();
    let before = authority();
    // When
    let result: Result<(), String> = exact(|| {
        let inner = css::class_map::Attempt::begin();
        let class = LegacyInput::Declaration(LegacyDeclaration {
            property: "width".into(),
            level: 0,
            value: None,
            selector: None,
            order: None,
        });
        let class_context = capture_context(&class, None, css::CounterOwner::Inactive);
        let class = css::counter_names::allocate_name(&class, &class_context);
        let variable = LegacyInput::Variable(LegacyVariable {
            property: "width".into(),
            level: 0,
            selector: Some(format!(".{}", class.name)),
        });
        let context = capture_context(&variable, None, css::CounterOwner::Inactive);
        let _variable = css::counter_names::allocate_name(&variable, &context);
        assert_eq!(css::class_map::get_class_map()[""].len(), 2);
        inner.commit();
        Err("output error".into())
    });
    // Then
    assert_eq!(result, Err("output error".into()));
    assert_eq!(authority(), before);
    reset_build_state_internal();
}
