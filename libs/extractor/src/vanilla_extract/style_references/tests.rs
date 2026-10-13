use super::StyleReferences;
use rstest::rstest;

#[rstest]
#[case("f0_0 a b:hover &", ".f0_0:hover &")]
#[case(".f0_0:hover &", ".f0_0:hover &")]
#[case("f0_0 a bf0_1 c", ".f0_0.f0_1")]
#[case("f0_0 a b, f0_1 c", ".f0_0, .f0_1")]
#[case("a b:hover &", "a b:hover &")]
#[case("span[data-label=\"f0_0 a b\"] &", "span[data-label=\"f0_0 a b\"] &")]
fn selector_identity_is_resolved_when_complete_registered_values_are_used(
    #[case] input: &str,
    #[case] expected: &str,
) {
    let mut references = StyleReferences::default();
    references.register("f0_0 a b".into(), "f0_0".into());
    references.register("f0_1 c".into(), "f0_1".into());
    assert_eq!(references.selector(input), expected);
}

#[test]
fn coincidental_atomic_literals_remain_external_when_no_registration_matches() {
    let mut references = StyleReferences::default();
    references.register("f0_0 a b".into(), "f0_0".into());
    assert_eq!(references.tokens("a"), None);
    assert_eq!(
        references.tokens("f0_0 a b external"),
        Some(vec![
            ("f0_0".into(), true),
            ("a".into(), true),
            ("b".into(), true),
            ("external".into(), false),
        ])
    );
}

#[test]
fn selector_rule_rewriting_preserves_declaration_values_and_element_selectors() {
    let mut references = StyleReferences::default();
    references.register("f0_0 a b".into(), "f0_0".into());
    let input = r#"{"content":"f0_0 a b","selectors":{"f0_0 a b:hover &":{"color":"red"},"a b &":{"color":"blue"}},"@media":{"print":{"selectors":{"f0_0 a b:focus &":{"opacity":1}}}}}"#;
    let expected = r#"{"content":"f0_0 a b","selectors":{".f0_0:hover &":{"color":"red"},"a b &":{"color":"blue"}},"@media":{"print":{"selectors":{".f0_0:focus &":{"opacity":1}}}}}"#;
    assert_eq!(
        super::super::selector_rules::rewrite(input, &references),
        expected
    );
}
