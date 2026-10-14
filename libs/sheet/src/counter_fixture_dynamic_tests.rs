use super::counter_fixture_support::{address, fixture, produced, state};
use css::{
    class_map::{get_class_map, set_class_map},
    sparse_site::SourceFile,
};
use extractor::extract_style::{ExtractDynamicStyle, ProducerPolicy};
use std::collections::HashMap;

#[test]
#[serial_test::serial]
fn numeric_roles_allocate_only_classes_when_sites_are_constructed_authentically() {
    // Given: two syntax roles use one normalized offset in a registered source.
    let _state = state();
    let styles = fixture("a", || {
        [
            ExtractDynamicStyle::new("color", 0, "tone", None).at(2),
            ExtractDynamicStyle::new("color", 0, "tone !important", None).at_role(2, 3),
        ]
    });
    let before = HashMap::from([("empty".into(), HashMap::new())]);
    set_class_map(before);
    // When: cloned/deferred adapters request shared classes after TLS ends.
    let receipts = styles
        .clone()
        .map(|style| produced(style.counter_produce(None)));
    // Then: numeric D9/position/role names have no separate variable or reset slots.
    for (style, role) in styles.iter().zip([0, 3]) {
        assert_eq!(style.producer_policy(), ProducerPolicy::CounterOriginal(0));
        assert_eq!(
            style.site(),
            Some(&css::Site {
                file: SourceFile::D9(0),
                at: 2,
                role,
            })
        );
    }
    assert_eq!(receipts[0].variable, "---pSa-c");
    assert_eq!(receipts[1].variable, "---pSa-c-d");
    assert_eq!(
        receipts[0].site,
        Some(css::Site {
            file: SourceFile::D9(0),
            at: 2,
            role: 0
        })
    );
    assert_eq!(
        receipts[1].site,
        Some(css::Site {
            file: SourceFile::D9(0),
            at: 2,
            role: 3
        })
    );
    assert_eq!(receipts[0].variable_allocation, None);
    assert_eq!(receipts[1].variable_allocation, None);
    address(
        &receipts[0].class,
        ("", "color-0-var(---pSa-c)--255", 0, "pa"),
    );
    address(
        &receipts[1].class,
        ("", "color-0-var(---pSa-c-d) !important--255", 1, "pb"),
    );
    assert_eq!(
        get_class_map(),
        HashMap::from([
            ("empty".into(), HashMap::new()),
            (
                String::new(),
                HashMap::from([
                    ("color-0-var(---pSa-c)--255".into(), 0),
                    ("color-0-var(---pSa-c-d) !important--255".into(), 1),
                ])
            ),
        ])
    );
}

#[rstest::rstest]
#[case(false, "color-1---255")]
#[case(true, "color-1-!important--255")]
#[serial_test::serial]
fn no_site_requests_class_before_variable_when_both_use_shared_namespace(
    #[case] important: bool,
    #[case] class_key: &str,
) {
    // Given: a genuine no-site constructor and a pre-existing non-length seed slot.
    let _state = state();
    let style = fixture("a", || {
        ExtractDynamicStyle::new(
            "color",
            1,
            if important { "tone !important" } else { "tone" },
            None,
        )
    });
    set_class_map(HashMap::from([
        ("empty".into(), HashMap::new()),
        (String::new(), HashMap::from([("seed".into(), 9)])),
    ]));
    // When: the real adapter makes its class and variable requests.
    let receipt = produced(style.counter_produce(None));
    // Then: actual slots prove CLASS then VARIABLE, with zero reset reservations.
    assert_eq!(style.site(), None);
    assert_eq!(style.producer_policy(), ProducerPolicy::CounterOriginal(0));
    address(&receipt.class, ("", class_key, 1, "pb"));
    let variable = receipt
        .variable_allocation
        .as_ref()
        .unwrap_or_else(|| panic!("no-site variable receipt"));
    address(variable, ("", "color-1-", 2, "--pc"));
    assert_eq!(receipt.variable, "--pc");
    assert_eq!(receipt.identifier, "tone");
    assert_eq!(receipt.important, important);
    assert_eq!(
        get_class_map(),
        HashMap::from([
            ("empty".into(), HashMap::new()),
            (
                String::new(),
                HashMap::from([
                    ("seed".into(), 9),
                    (class_key.into(), 1),
                    ("color-1-".into(), 2),
                ])
            ),
        ])
    );
}

#[test]
#[serial_test::serial]
fn assignment_original_can_differ_when_site_is_attached_in_another_fixture() {
    // Given: parent construction and assignment site belong to different registered files.
    let _state = state();
    let parent = fixture("a", || ExtractDynamicStyle::new("color", 0, "tone", None));
    let style = fixture("b", || parent.at_role(2, 1));
    // When: a deferred producer requests a private class under unrelated delivery.
    let receipt = produced(style.counter_produce(Some("delivery")));
    // Then: both real registry members retain independent authority.
    assert_eq!(receipt.class.original, 0);
    assert_eq!(
        receipt.site,
        Some(css::Site {
            file: SourceFile::D9(1),
            at: 2,
            role: 1
        })
    );
    assert_eq!(receipt.variable, "---pSb-c-b");
    assert_eq!(receipt.variable_allocation, None);
    address(
        &receipt.class,
        ("D9-0", "color-0-var(---pSb-c-b)--255-a", 0, "pa-a"),
    );
    assert_eq!(
        css::file_map::get_original_ids(),
        std::collections::BTreeMap::from([("a".into(), 0), ("b".into(), 1),])
    );
    assert_eq!(
        get_class_map(),
        HashMap::from([(
            "D9-0".into(),
            HashMap::from([("color-0-var(---pSb-c-b)--255-a".into(), 0)])
        ),])
    );
}
