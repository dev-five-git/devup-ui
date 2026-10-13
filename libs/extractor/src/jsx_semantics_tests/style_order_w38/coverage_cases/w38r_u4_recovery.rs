use super::*;
use rstest::rstest;

#[rstest]
#[case::yes(true, "font-size-0-12px--255-a color-0-red--255-a typo-body")]
#[case::no(false, "font-size-0-12px--255-a color-0-blue--255-a typo-body")]
#[serial]
fn w38r_u4_normalised_when_typography_is_supplied_keeps_runtime_class_beside_declared_font(
    #[case] active: bool,
    #[case] classes: &str,
) {
    // Given: supplied typography is a normalized root, unlike external cx classes.
    let source = "import {ClassNames} from '@emotion/react';const render=(active,external)=><ClassNames>{({css,cx})=>css({fontSize:'12px',color:active?'red':'blue',typography:`${(trace.push('typo'),external)}`})}</ClassNames>;";
    // When: real local rule extraction visits skip declarations and supplied roots.
    let actual = compile_emotion(source).required("supplied typography compiles");
    let evaluated = whole::evaluate_code(&actual.code, &format!("render({active},'body')"));
    // Then: authored declaration and condition survive; supplied value runs once.
    assert_eq!(evaluated.element, serde_json::json!(classes));
    assert_eq!(evaluated.trace, serde_json::json!(["typo"]));
    inventory(
        &actual,
        &[
            ("font-size", "12px", 0, None),
            ("color", "red", 0, None),
            ("color", "blue", 0, None),
        ],
    );
}

#[rstest]
#[case::dynamic_and_unplaced("runtimeColor", "getKey()")]
#[serial]
fn w38r_u4_legacy_tag_when_segmentation_fails_reports_runtime_value_and_unplaced_key(
    #[case] value: &str,
    #[case] key: &str,
) {
    // Given: key interpolation defeats compose_css_template but a value hole remains.
    let prefix = format!("const a=css`color:${{{value}}};co${{");
    let source = format!("import {{css}} from '@devup-ui/react';\n{prefix}{key}}}lor:red;`;");
    // When: the public visitor falls through to legacy css_to_style_template.
    let actual = error(&source);
    // Then: both distinct failures retain their original source locations.
    assert_eq!(
        actual,
        format!(
            "a.tsx:2:9: `css()` cannot use `{value}` at build time: its values must be literals, theme tokens or constants, or be computed from them\na.tsx:2:{}: Cannot place `{key}` at build time: an interpolation in a selector or a property name must be a literal or a constant",
            prefix.len() + 1
        )
    );
}
