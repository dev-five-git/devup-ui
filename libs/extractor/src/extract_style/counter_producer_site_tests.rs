use super::counter_producer_test_support::{cleanup, produced, reset, scope};
use super::{CounterProducerError, ExtractDynamicStyle};
use css::{
    Site,
    allocation_input::{LegacyInput, NameMode},
    class_map::get_class_map,
    sparse_site::SourceFile,
};
use rstest::rstest;
use serial_test::serial;
use std::{collections::BTreeSet, sync::Arc};

#[rstest]
#[case(NameMode::Counter)]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
#[serial]
fn numeric_sites_refine_class_when_files_roles_and_scopes_differ(#[case] mode: NameMode) {
    // Given: two originals and multiple syntax roles with identical controllers/property.
    reset(mode);
    let first = {
        let _scope = scope("a");
        ExtractDynamicStyle::new("color", 0, "tone !important", None).at(0)
    };
    let role = {
        let _scope = scope("a");
        ExtractDynamicStyle::new("color", 0, "tone !important", None).at_role(0, 1)
    };
    let second = {
        let _scope = scope("b");
        ExtractDynamicStyle::new("color", 0, "tone !important", None).at(0)
    };
    let before = get_class_map();
    // When: deferred records produce shared classes under a later unrelated scope.
    let _other = scope("later");
    let receipts = [first, role, second].map(|mut style| {
        style.style_order = Some(0);
        produced(style.counter_produce(Some("delivery")))
    });
    // Then: sites reserve no variable slot and each actual var consumer differs.
    assert_eq!(
        receipts
            .iter()
            .map(|r| r.variable.as_str())
            .collect::<Vec<_>>(),
        vec!["---pSa-a", "---pSa-a-b", "---pSb-a"]
    );
    assert_eq!(
        receipts
            .iter()
            .map(|r| r.class.allocation.name.clone())
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    for receipt in receipts {
        assert_eq!(receipt.variable_allocation, None);
        assert_eq!(receipt.identifier, "tone");
        assert!(receipt.important);
        let LegacyInput::Declaration(input) = receipt.class.input else {
            panic!("class")
        };
        assert_eq!(
            input.value,
            Some(format!("var({}) !important", receipt.variable))
        );
    }
    match mode {
        NameMode::Counter => assert_eq!(
            get_class_map().get("").map(std::collections::HashMap::len),
            Some(3)
        ),
        NameMode::Debug | NameMode::AtomHoist => assert_eq!(get_class_map(), before),
    }
    cleanup();
}

#[test]
#[serial]
fn folded_owner_keeps_normalized_numeric_site_when_alias_edits_are_reversed() {
    // Given: numbered raw source contains BOM/CRLF and an inserted alias prelude.
    reset(NameMode::Counter);
    let original =
        css::file_map::get_or_insert_original_id("a").unwrap_or_else(|error| panic!("{error}"));
    let edits = [(0, 0, 10)];
    let style = {
        let _scope = crate::sparse_sites::SiteScope::enter_counter_numbered(
            original,
            "\u{feff}a\r\nbcd",
            &[&edits],
        );
        crate::sparse_sites::retain_folded_owner(17, 16);
        ExtractDynamicStyle::new("color", 0, "tone", None).at_role(17, 2)
    };
    // When: a deferred record produces its assignment variable outside the scope.
    let receipt = produced(style.counter_produce(None));
    // Then: raw owner offset 6 becomes normalized offset 2, retaining syntax role 2.
    assert_eq!(
        receipt.site,
        Some(Site {
            file: SourceFile::D9(0),
            at: 2,
            role: 2
        })
    );
    assert_eq!(receipt.variable, "---pSa-c-c");
    cleanup();
}

#[rstest]
#[case(NameMode::Counter)]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
#[serial]
fn unnumbered_site_rejects_before_render_when_parent_is_counter(#[case] mode: NameMode) {
    // Given: a valid retained parent but a manually unnumbered assignment site.
    reset(mode);
    let _scope = scope("a");
    let mut style = ExtractDynamicStyle::new("color", 0, "tone", None);
    style.site = Some(Site {
        file: SourceFile::Unnumbered(Arc::from("source")),
        at: 0,
        role: 0,
    });
    style.style_order = Some(0);
    let before = get_class_map();
    // When: the dormant boundary attempts production, including no filename.
    let error = style.counter_produce(None).err();
    // Then: no hash fallback or counter request is possible.
    assert_eq!(error, Some(CounterProducerError::UnnumberedSite));
    assert_eq!(get_class_map(), before);
    cleanup();
}

#[test]
#[serial]
fn assignment_original_is_independent_when_site_is_attached_under_other_scope() {
    // Given: parent original a and captured assignment original b.
    reset(NameMode::Counter);
    let mut style = {
        let _scope = scope("a");
        ExtractDynamicStyle::new("color", 0, "tone", None)
    };
    {
        let _scope = scope("b");
        style = style.at(1);
    }
    // When: deferred production uses private class ownership.
    let receipt = produced(style.counter_produce(Some("delivery")));
    // Then: the parent and numeric assignment each retain their own authority.
    assert_eq!(receipt.class.original, 0);
    assert_eq!(receipt.class.allocation.name, "pa-a");
    assert_eq!(receipt.variable, "---pSb-b");
    cleanup();
}

#[rstest]
#[case(NameMode::Counter, false)]
#[case(NameMode::Counter, true)]
#[case(NameMode::Debug, false)]
#[case(NameMode::Debug, true)]
#[case(NameMode::AtomHoist, false)]
#[case(NameMode::AtomHoist, true)]
#[serial]
fn no_site_class_uses_legacy_value_when_mode_and_importance_choose_branch(
    #[case] mode: NameMode,
    #[case] important: bool,
) {
    // Given: a no-site dynamic with an explicit construction original.
    reset(mode);
    let _scope = scope("a");
    let style = ExtractDynamicStyle::new(
        "color",
        1,
        if important { "tone !important" } else { "tone" },
        None,
    );
    // When: the class and separate variable envelope are produced.
    let receipt = produced(style.counter_produce(None));
    // Then: only atom mode uses the full variable consumer in the class key.
    let LegacyInput::Declaration(input) = receipt.class.input else {
        panic!("class")
    };
    let expected = match mode {
        NameMode::Counter | NameMode::Debug => important.then(|| "!important".into()),
        NameMode::AtomHoist => Some(format!(
            "var({}){}",
            receipt.variable,
            if important { " !important" } else { "" }
        )),
    };
    assert_eq!(input.value, expected);
    let variable = receipt
        .variable_allocation
        .unwrap_or_else(|| panic!("variable"));
    assert_eq!(variable.original, 0);
    assert_eq!(variable.context.file, None);
    assert_eq!(variable.allocation.name, receipt.variable);
    cleanup();
}

#[test]
#[serial]
fn atom_precedes_debug_when_delivery_is_hoisted() {
    // Given: both modes enabled, a hoisted declaration bucket and explicit prefix.
    reset(NameMode::AtomHoist);
    css::debug::set_debug(true);
    css::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["root".into()])));
    let _scope = scope("a");
    let style = ExtractDynamicStyle::new("color", 0, "tone", Some("hover".into()));
    // When: no-site variable and class render from one captured context.
    let receipt = produced(style.counter_produce(Some("root")));
    // Then: a1 can be hoisted, while v1 stays global and no counters are reserved.
    assert_eq!(receipt.class.context.config.mode, NameMode::AtomHoist);
    assert!(receipt.class.allocation.name.starts_with("pa1-h-"));
    assert!(receipt.variable.starts_with("--pv1-"));
    assert_eq!(get_class_map().len(), 0);
    cleanup();
}

#[test]
#[serial]
fn no_site_dynamic_retains_original_when_two_empty_site_records_are_deferred() {
    // Given: neither record can recover its parent authority from an assignment site.
    reset(NameMode::Counter);
    let first = {
        let _scope = scope("a");
        ExtractDynamicStyle::new("color", 0, "tone", None)
    };
    let second = {
        let _scope = scope("b");
        ExtractDynamicStyle::new("color", 0, "tone", None)
    };
    // When: both deferred records produce private classes outside their original scopes.
    let receipts = [first, second].map(|style| produced(style.counter_produce(Some("delivery"))));
    // Then: class streams retain distinct parents while the no-site variable is shared.
    assert_eq!(receipts[0].class.original, 0);
    assert_eq!(receipts[1].class.original, 1);
    assert_eq!(receipts[0].class.allocation.name, "pa-a");
    assert_eq!(receipts[1].class.allocation.name, "pb-a");
    assert_eq!(receipts[0].variable, "--pa");
    assert_eq!(receipts[1].variable, "--pa");
    assert_eq!(
        receipts[1]
            .variable_allocation
            .as_ref()
            .map(|receipt| receipt.original),
        Some(1)
    );
    cleanup();
}
