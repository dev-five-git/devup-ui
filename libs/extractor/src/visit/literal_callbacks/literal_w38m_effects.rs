use super::compile;
use super::literal_w38m_support::{GETTERS, fixture, policy_error};
use serial_test::serial;

#[test]
#[serial]
fn comment_effects_when_real_spread_has_ineligible_text_retain_located_policy_rejection() {
    // Given: the exact no-order and ordered fixtures rejected by the ca8 receipts.
    let root = "_focus:`/*${trace.push('before')}*/color:green`,...{color:'red',_active:`/*${trace.push('spread')}*/color:yellow`},_disabled:`/*${trace.push('after')}*/color:gray`,_hover:`ORDERcolor:blue`";
    let source = fixture(
        &root.replace(
            "ORDER",
            "style-order:${p=>{trace.push('callback');return;}};",
        ),
        GETTERS,
    );
    let reference = fixture(&root.replace("ORDER", ""), GETTERS);
    // When: public extraction checks each actual source form.
    for input in [&reference, &source] {
        let error = compile(input)
            .err()
            .unwrap_or_else(|| panic!("ineligible text compiled"));
        // Then: the complete policy-error inventory retains all original source coordinates.
        let expected = vec![
            policy_error(input, "`/*${trace.push('before')}*/color:green`", 0),
            policy_error(input, "`/*${trace.push('spread')}*/color:yellow`", 0),
            policy_error(input, "`/*${trace.push('after')}*/color:gray`", 0),
        ];
        let actual = error
            .lines()
            .filter(|line| line.contains("`styled()`"))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}
