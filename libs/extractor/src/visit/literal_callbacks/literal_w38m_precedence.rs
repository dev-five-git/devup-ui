use super::compile;
use super::literal_w38m_support::{GETTERS, fixture, policy_error};
use serial_test::serial;

#[test]
#[serial]
fn comment_precedence_when_real_spread_text_is_ineligible_retains_later_source_locations() {
    // Given: the exact repeated-spread fixtures rejected even without order metadata.
    let spread = "...{color:'red',_active:`/*${trace.push('spread')}*/color:yellow`}";
    let root = format!(
        "_focus:`/*${{trace.push('before')}}*/color:green`,{spread},color:'purple',_disabled:`/*${{trace.push('between')}}*/color:gray`,{spread},color:'orange',_hover:`ORDERcolor:blue`,_focus:`/*${{trace.push('after')}}*/color:green`"
    );
    let source = fixture(
        &root.replace(
            "ORDER",
            "style-order:${p=>{trace.push('callback');return;}};",
        ),
        GETTERS,
    );
    let reference = fixture(&root.replace("ORDER", ""), GETTERS);
    // When: public extraction checks both source forms under the existing policy.
    for input in [&reference, &source] {
        let error = compile(input)
            .err()
            .unwrap_or_else(|| panic!("ineligible text compiled"));
        // Then: retained occurrences, not removed earlier duplicates, own the exact errors.
        let expected = vec![
            policy_error(input, "`/*${trace.push('between')}*/color:gray`", 0),
            policy_error(input, "`/*${trace.push('spread')}*/color:yellow`", 1),
            policy_error(input, "`/*${trace.push('after')}*/color:green`", 0),
        ];
        let actual = error
            .lines()
            .filter(|line| line.contains("`styled()`"))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}
