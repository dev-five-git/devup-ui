use super::*;
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn tagged_css_prop_when_order_getter_false_selects_once() {
    // Given: tagged ordered CSS entering Emotion's element route.
    let source = format!(
        "{EMOTION}import {{css}} from '@emotion/react';const state={{get active(){{trace.push('order');return false}}}};const a=<div css={{css`color:red;style-order:${{state.active?2:3}}`}} />;"
    );
    // When: public compilation and emitted element execution run.
    let compiled = compile_emotion(&source).required("tagged CSS prop");
    let actual = whole::evaluate_code(&compiled.code, "a.props.className");
    // Then: order is read once and selected red has order 3.
    assert_eq!(actual.trace, serde_json::json!(["order"]));
    assert!(actual.element.as_str().required("class").contains("--3-"));
}

#[test]
#[serial]
fn raw_css_prop_when_key_unknown_rejects_at_preflight() {
    // Given: raw text has an unknowable declaration name.
    let source = format!("{EMOTION}const a=(key)=><div css={{`co${{key}}:red`}} />;");
    let column = source
        .lines()
        .nth(1)
        .required("line")
        .rfind("key")
        .required("hole")
        + 1;
    // When: raw-template placement preflight rejects before lowering.
    let actual = compile_emotion(&source).required_err("unknown CSS key");
    // Then: original source coordinate survives; this is not a later-guard hit claim.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
}

#[test]
#[serial]
fn local_css_when_raw_runtime_value_has_no_element_rejects() {
    // Given: ClassNames local css returns a class, not a variable host.
    let source = "import {ClassNames} from '@emotion/react';const read=()=>(trace.push('local'),'red');const a=<ClassNames>{({css})=><div className={css(`color:${read()}`)} />}</ClassNames>;";
    // When: compilation inspects the raw TemplateLiteral call argument.
    let actual = compile_emotion(source).required_err("runtime class value");
    // Then: arbitrary values do not become standalone runtime styling.
    assert!(actual.contains("build time"), "{actual}");
}

#[test]
#[serial]
fn local_css_when_raw_call_controller_is_unresolved_preserves_rejection() {
    // Given: public preflight rejects this raw call before the local capture seam.
    let source = "import {ClassNames} from '@emotion/react';const read=()=>(trace.push('local'),false);const a=<ClassNames>{({css})=><div className={css(`color:${read()?'red':'blue'}`)} />}</ClassNames>;";
    // When: public extraction runs without executing rejected code.
    let actual = compile_emotion(source).required_err("unresolved raw call controller");
    // Then: the source expression stays a located build-time error, not runtime styling.
    assert!(actual.starts_with("a.tsx:1:132:"), "{actual}");
}

#[rstest]
#[case("{color:true?'red':'blue'}", "red")]
#[case(
    "{color:true?'red':'blue',backgroundColor:false?'black':'white'}",
    "white"
)]
#[serial]
fn standalone_when_constant_alternatives_normalize_preserves_values(
    #[case] rules: &str,
    #[case] value: &str,
) {
    // Given: constants may normalize; the output shape is intentionally not prescribed.
    let source = format!(
        "import {{css}} from '@devup-ui/react';const a=css({rules});const b=css(a,{{color:'green'}});"
    );
    // When: both producer and later consumer execute.
    let actual = whole::evaluate(&source, "[a,b]");
    // Then: producer values survive, and later green wins over red.
    assert!(
        actual.element[0]
            .as_str()
            .required("producer")
            .contains(value)
    );
    let class = actual.element[1].as_str().required("consumer");
    assert!(class.contains("green") && !class.contains("red"), "{class}");
}

#[rstest]
#[case("{color:true?'red':'blue'}", "red")]
#[case(
    "{color:true?'red':'blue',backgroundColor:false?'black':'white'}",
    "white"
)]
#[serial]
fn local_css_when_constant_alternatives_normalize_preserves_values(
    #[case] rules: &str,
    #[case] value: &str,
) {
    // Given: real local calls whose normalization may erase a conditional shape.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';const a=<ClassNames>{{({{css}})=><div className={{css({rules})}} />}}</ClassNames>;"
    );
    // When: the compiler's actual local child executes.
    let compiled = compile_emotion(&source).required("local constant alternatives");
    let actual = whole::evaluate_code(&compiled.code, "a.props.className");
    // Then: declarations survive without claiming a particular AST restoration arm.
    assert!(
        actual
            .element
            .as_str()
            .required("local class")
            .contains(value)
    );
}

#[test]
#[serial]
fn local_css_when_finite_lookup_is_captured_preserves_present_and_missing_keys() {
    // Given: finite local selection reads distinct keys at each child invocation.
    let source = "import {ClassNames} from '@emotion/react';const render=(key)=>{const read=()=>(trace.push(key),key);return <ClassNames>{({css})=><div className={css({color:{red:'red',blue:'blue'}[read()]})} />}</ClassNames>};";
    // When: real emitted local lookup handles both known entries and the missing key.
    let compiled = compile_emotion(source).required("local finite lookup");
    let actual = whole::evaluate_code(
        &compiled.code,
        "[render('red').props.className,render('blue').props.className,render('missing').props.className]",
    );
    // Then: each read is once-only and missing input keeps the empty fallback.
    assert_eq!(actual.trace, serde_json::json!(["red", "blue", "missing"]));
    assert!(actual.element[0].as_str().required("red").contains("red"));
    assert!(actual.element[1].as_str().required("blue").contains("blue"));
    assert_eq!(actual.element[2], "");
}

#[test]
#[serial]
fn tagged_fallback_when_unreadable_mixin_locates_original_tag() {
    // Given: composition cannot read the root call and a declaration is runtime-only.
    let source = "import {css} from '@devup-ui/react';\nconst a=(value)=>css`color:${value};${makeRules()}`;";
    let column = source
        .lines()
        .nth(1)
        .required("line")
        .find("css`")
        .required("tag")
        + 1;
    // When: the fallback rejects without executing the unknown factory.
    let actual = error(source);
    // Then: tag origin remains authored.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
}

#[test]
#[serial]
fn finite_lookup_when_captured_preserves_keys_and_empty_fallback() {
    // Given: map selection may emit a logical lookup wrapped by capture.
    let source = "import {css} from '@devup-ui/react';const a=(key)=>css({color:{red:'red',blue:'blue'}[(trace.push(key),key)]});";
    // When: present and missing keys execute through emitted code.
    let actual = whole::evaluate(source, "[a('red'),a('blue'),a('missing')]");
    // Then: keys are read once and missing input supplies no class.
    assert_eq!(actual.trace, serde_json::json!(["red", "blue", "missing"]));
    assert!(actual.element[0].as_str().required("red").contains("red"));
    assert!(actual.element[1].as_str().required("blue").contains("blue"));
    assert_eq!(actual.element[2], "");
}
