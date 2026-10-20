use super::{compile, execute, styles};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    "css:outerCss",
    "theme:innerTheme",
    "outerCss({color:innerTheme.brand})"
)]
#[case(
    "theme:outerTheme",
    "css:innerCss",
    "innerCss({color:outerTheme.brand})"
)]
#[case(
    "css:outerCss",
    "theme:innerTheme",
    "outerCss`color:${innerTheme.brand};`"
)]
#[case(
    "theme:outerTheme",
    "css:innerCss",
    "innerCss`color:${outerTheme.brand};`"
)]
#[serial]
fn class_names_nested_binding_crosses_callback_when_api_and_theme_have_different_owners(
    #[case] outer: &str,
    #[case] inner: &str,
    #[case] call: &str,
) {
    // Given: genuine APIs and themes belong to different active callback bindings.
    let source = format!(
        "import {{ClassNames as Names}} from '@emotion/react';const App=()=> <Names>{{({{{outer}}})=><Names>{{({{{inner}}})=>({{styled:!!({call})}})}}</Names>}}</Names>;"
    );
    // When: compile and execute the emitted render function.
    let output = compile(&source);
    let value = execute(&output);
    // Then: both wrappers/APIs disappear and the crossing theme becomes a CSS variable.
    assert_eq!(styles(&output), [("color", "var(--brand)")]);
    assert_eq!(value, r#"{"styled":true}"#);
    assert!(!output.code.contains("Names"), "{}", output.code);
}

#[test]
#[serial]
fn class_names_nested_binding_keeps_ordinary_shadows_when_both_themes_cross() {
    // Given: both callbacks contribute APIs/themes alongside independent ordinary bindings.
    let source = "import {ClassNames} from '@emotion/react';const App=()=> <ClassNames>{({css:outerCss,theme:outerTheme})=><ClassNames>{({css:innerCss,cx:innerCx,theme:innerTheme})=>({title:((innerTheme)=>innerTheme.brand)({brand:'kept'}),local:((outerCss,innerCx)=>[outerCss({padding:[1,2]}),innerCx({local:2})])(v=>v,v=>v),styled:!!innerCx({picked:true},outerCss({color:innerTheme.inner}),innerCss({backgroundColor:outerTheme.outer}))})}</ClassNames>}</ClassNames>;";
    // When: compile and execute the combined crossing/shadow case.
    let output = compile(source);
    let value = execute(&output);
    // Then: genuine crossed styles compile while ordinary values retain their exact behavior.
    assert_eq!(
        styles(&output),
        [
            ("background-color", "var(--outer)"),
            ("color", "var(--inner)")
        ]
    );
    assert_eq!(
        value,
        r#"{"title":"kept","local":[{"padding":[1,2]},{"local":2}],"styled":true}"#
    );
}

#[test]
#[serial]
fn class_names_nested_binding_prepares_all_themes_when_one_call_reads_multiple_callbacks() {
    // Given: one real API reads two active themes through a nested ordinary function.
    let source = "import {ClassNames} from '@emotion/react';const App=()=> <ClassNames>{({css,theme:outer})=><ClassNames>{({theme:inner})=>({styled:!!(()=>css({color:inner.brand,backgroundColor:outer.brand}))()})}</ClassNames>}</ClassNames>;";
    // When: compile and execute the captured real API call.
    let output = compile(source);
    let value = execute(&output);
    // Then: neither active theme remains an unsupported read or runtime dependency.
    assert_eq!(
        styles(&output),
        [
            ("background-color", "var(--brand)"),
            ("color", "var(--brand)")
        ]
    );
    assert_eq!(value, r#"{"styled":true}"#);
}

#[test]
#[serial]
fn class_names_nested_binding_keeps_component_shadow_when_preparing_genuine_outer_calls() {
    // Given: a genuine outer API is captured inside an ordinary same-spelling component.
    let source = "import {ClassNames as Names} from '@emotion/react';const App=()=> <Names>{({css:outerCss})=>((Names)=><Names marker={7}>{({css,theme})=>({local:css({padding:[1,2]}),theme:theme.brand,styled:outerCss({color:'blue'})})}</Names>)(Local)}</Names>;";
    // When: prepare and compile the real outer wrapper.
    let output = compile(source);
    // Then: only the actual API emits styles; ordinary component callbacks stay intact.
    assert_eq!(styles(&output), [("color", "blue")]);
    assert!(
        output.code.contains("<Names marker={7}>"),
        "{}",
        output.code
    );
    assert!(output.code.contains("padding: [1, 2]"), "{}", output.code);
    assert!(output.code.contains("theme.brand"), "{}", output.code);
}
