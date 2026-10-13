use super::*;
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn closed_block_when_followed_by_value_hole_keeps_hover_and_one_read() {
    // Given: raw Emotion text without metadata, followed by a runtime value.
    let source = format!(
        "{EMOTION}const read=()=>(trace.push('value'),'green');const a=<div css={{`&:hover{{color:red}}background:${{read()}};`}} />;"
    );
    // When: the real CSS-prop compiler and emitted element run.
    let compiled = compile_emotion(&source).required("raw CSS prop");
    let actual = whole::evaluate_code(&compiled.code, "a");
    // Then: the declaration remains scoped and the value lives on the element.
    assert_eq!(actual.trace, serde_json::json!(["value"]));
    assert!(compiled.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property()=="color" && style.value()=="red" && style.selector().is_some())));
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .required("element variables")
            .values()
            .any(|value| value == "green")
    );
}

#[test]
#[serial]
fn comment_when_css_is_unterminated_keeps_valid_js_empty() {
    // Given: valid JS containing an unclosed CSS comment, not an external class.
    let source = "import {css} from '@devup-ui/react';const a=css('/* unfinished:color:red');";
    // When: public extraction runs.
    let actual = output(source);
    // Then: no rule or class is invented.
    assert_eq!(actual.styles.len(), 0);
    assert_eq!(whole::evaluate_code(&actual.code, "a").element, "");
}

#[rstest]
#[case("co${key}:red", "key")]
#[case(concat!("${selector}", "{color:red}"), "selector")]
#[serial]
fn ordered_text_when_key_is_unknown_reports_authored_tag(#[case] body: &str, #[case] token: &str) {
    // Given: order forces object lowering, but the declaration/prelude is unknowable.
    let source = format!(
        "import {{css}} from '@devup-ui/react';\nconst a=({token})=>css`style-order:2;{body}`;"
    );
    let line = source.lines().nth(1).required("source line");
    let column = line.find("css`").required("authored tag") + 1;
    // When: public extraction rejects it.
    let actual = error(&source);
    // Then: public preflight retains the original tag and identifies the unreadable hole.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
    assert!(actual.contains(token), "{actual}");
}

#[rstest]
#[case(r"\style-order:2;color:red", ("color", "red", 2))]
#[case(r"\73 tyle-order:3;color:blue", ("color", "blue", 3))]
#[case(r"\z:keep;style-order:2;color:red", (r"\z", "keep", 2))]
#[serial]
fn escaped_key_when_reserved_canonicalizes_only_metadata(
    #[case] body: &str,
    #[case] expected: (&str, &str, u8),
) {
    let (property, value, order) = expected;
    // Given: raw tag escapes are CSS escapes, including a nonreserved control.
    let source = format!("import {{css}} from '@devup-ui/react';const a=css`{body}`;");
    // When: the public text route compiles.
    let actual = output(&source);
    // Then: metadata is stripped; other escaped spelling survives.
    assert!(actual.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property()==property && style.value()==value && style.style_order()==Some(order))));
    assert!(!actual.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if crate::style_order::reserved(style.property()))));
}

#[test]
#[serial]
fn quoted_text_when_finite_selects_canonical_orders() {
    // Given: a quoted prefix requires the mixed-text producer, not exact-hole cloning.
    let source = "import {css} from '@devup-ui/react';const a=(on)=>css`style-order:'1${on?2:3}';color:red`;";
    // When: both executions of the emitted function run.
    let actual = whole::evaluate(source, "[a(true),a(false)]");
    // Then: neither CSS spacing scale nor numeric coercion changes 12/13.
    assert!(
        actual.element[0]
            .as_str()
            .required("true class")
            .contains("--12-")
    );
    assert!(
        actual.element[1]
            .as_str()
            .required("false class")
            .contains("--13-")
    );
}

#[test]
#[serial]
fn quoted_text_when_unresolved_rejects_original_value() {
    // Given: unresolved text must not be coerced into a numeric order.
    let source = "import {css} from '@devup-ui/react';\nconst a=(read)=>css`style-order:'1${read()}';color:red`;";
    let column = source
        .lines()
        .nth(1)
        .required("line")
        .find("'1")
        .required("quoted value")
        + 1;
    // When: strict metadata parsing runs.
    let actual = error(source);
    // Then: the quoted authored value remains the diagnostic origin.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
    assert!(actual.contains("styleOrder"), "{actual}");
}
