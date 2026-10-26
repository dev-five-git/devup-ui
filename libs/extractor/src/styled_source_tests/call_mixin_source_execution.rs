use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::alias(
    "const original=css({color:'red'});const mixin=original;",
    "p=>p.on?mixin:null",
    "red"
)]
#[case::empty_alias(
    "const original=css({});const mixin=original;",
    "p=>mixin||{color:'red'}",
    "red"
)]
#[case::inline("", "p=>p.on?css({color:'red'}):null", "red")]
#[case::inline_empty("", "p=>css({})||{color:'red'}", "red")]
#[case::written_empty("", "p=>({})||{color:'red'}", "blue")]
#[case::empty_and("const mixin=css({});", "p=>mixin&&opaque()", "blue")]
#[case::nonempty_or("const mixin=css({color:'red'});", "p=>mixin||opaque()", "red")]
#[serial]
fn source_identity_selects_rules_when_known_mixins_are_named_aliased_or_inline(
    #[case] setup: &str,
    #[case] callback: &str,
    #[case] expected: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: known CSS class truthiness differs from a written empty object's truthiness.
    let source = format!(
        "{setup}const Base=styled.div({{color:'blue'}});const Card=styled(Base)({callback});"
    );
    // When: the full generated renderer chooses the callback's whole rules.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const element=render({target:Card,props:{ on: true }});JSON.stringify(values(element,'color'));",
        single_css,
    );
    // Then: only the selected declarations survive D1; unreachable opaque calls do not execute.
    assert_eq!(observed, serde_json::json!([expected]));
}

#[rstest]
#[serial]
fn class_values_remain_strings_when_callback_rule_objects_read_a_known_mixin(
    #[values(false, true)] single_css: bool,
) {
    // Given: one class is read as data, an ordinary declaration and a callback property value.
    let source = "const mixin=css({color:'red'});const alias=mixin;const data={text:alias};const Card=styled.div({content:alias},p=>p.on?{backgroundColor:alias,borderColor:`a${alias}b`}:{});";
    // When: actual generated rules and application data read the same authored class.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{ on: true }});JSON.stringify([typeof data.text,data.text===mixin,values(element,'content')[0]===mixin,values(element,'background-color')[0]===mixin,values(element,'border-color')[0]==='a'+mixin+'b']);",
        single_css,
    );
    // Then: preserving rule identity cannot turn class-string values into rule objects.
    assert_eq!(
        observed,
        serde_json::json!(["string", true, true, true, true])
    );
}

#[rstest]
#[serial]
fn semantic_shadowing_keeps_property_values_when_a_parameter_shares_the_mixin_name(
    #[values(false, true)] single_css: bool,
) {
    // Given: a local parameter shadows a known CSS binding only inside the factory.
    let source = "const mixin=css({color:'red'});function make(mixin){return styled.div(p=>p.on?{color:mixin}:{});}const Card=make('orange');";
    // When: the generated callback reads the parameter rather than the outer CSS class.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{ on: true }});JSON.stringify(values(element,'color'));",
        single_css,
    );
    // Then: binding identity, not a matching name, determines source recovery.
    assert_eq!(observed, serde_json::json!(["orange"]));
}
