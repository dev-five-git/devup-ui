use super::*;
use serial_test::serial;

#[test]
#[serial]
fn metadata_audit_when_comments_split_numeric_tokens_rejects_joined_number() {
    for value in ["2/**/5", "+/**/2", "2./**/0", "2e/**/0"] {
        let source = format!(
            "import {{css}} from '@devup-ui/react'; const a=css`style-order:{value};color:red`; "
        );
        let message = compile(&source)
            .required_err("split numeric tokens must not become a new metadata Number");
        assert!(message.contains("styleOrder"), "{message}");
    }
}

#[test]
#[serial]
fn metadata_audit_when_comment_trivia_surrounds_one_numeric_token_accepts_it() {
    let result = output(
        "import {css} from '@devup-ui/react'; const a=css`style-order:/**/+2.0/**/;color:red`; ",
    );
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "color" && style.style_order() == Some(2))));
}

#[test]
#[serial]
fn metadata_audit_when_class_text_contains_frame_metadata_reports_located_no_effect() {
    let source = "import {css} from '@devup-ui/react'; const a=css`@keyframes spin{from{style-order:2;opacity:0}}`;";
    let message =
        compile(source).required_err("frame metadata must not disappear in a class text scope");
    let column = source
        .find("style-order")
        .required("fixture contains a directive")
        + 1;
    assert!(
        message.starts_with(&format!("a.tsx:1:{column}:")),
        "{message}"
    );
    assert!(
        message.contains("has no effect") && message.contains("styleOrder"),
        "{message}"
    );
}

#[test]
#[serial]
fn metadata_audit_when_class_text_contains_font_metadata_reports_located_no_effect() {
    let source = "import {css} from '@devup-ui/react'; const a=css`@font-face{style-order:2;font-family:test}`;";
    let message = compile(source)
        .required_err("descriptor metadata must not disappear in a class text scope");
    let column = source
        .find("style-order")
        .required("fixture contains a directive")
        + 1;
    assert!(
        message.starts_with(&format!("a.tsx:1:{column}:")),
        "{message}"
    );
    assert!(
        message.contains("has no effect") && message.contains("styleOrder"),
        "{message}"
    );
}

#[test]
#[serial]
fn metadata_audit_when_frame_selector_record_key_names_reserved_word_preserves_rule() {
    let result = output(
        "import {keyframes} from '@devup-ui/react'; const a=keyframes({from:{selectors:{styleOrder:{color:'red'}}}});",
    );
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Keyframes(style) if style.keyframes.values().flatten().any(|style| style.property() == "color" && style.value() == "red"))));
}

#[test]
#[serial]
fn metadata_audit_when_frame_record_value_has_actual_metadata_still_rejects() {
    let source = "import {keyframes} from '@devup-ui/react'; const a=keyframes({from:{selectors:{styleOrder:{styleOrder:2,color:'red'}}}});";
    let message = compile(source)
        .required_err("record values are declaration scopes with forbidden metadata");
    assert!(message.contains("has no effect"), "{message}");
}
