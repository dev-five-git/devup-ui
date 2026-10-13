use super::*;
use rstest::rstest;

/// Puts the process-wide class prefix back, also when an assertion fails.
struct Prefix;

impl Drop for Prefix {
    fn drop(&mut self) {
        css::set_prefix(None);
    }
}

#[rstest]
#[case::reserved_spelling("style-orde", 16)]
#[serial]
fn w38r_u6_styled_tag_when_keyframes_name_spells_the_order_key_applies_the_order(
    #[case] prefix: &str,
    #[case] earlier_classes: usize,
) {
    // Given: a configured prefix makes the 17th generated name of the file `style-order`.
    let props = [
        "color",
        "width",
        "height",
        "margin",
        "padding",
        "top",
        "left",
        "right",
        "bottom",
        "gap",
        "opacity",
        "order",
        "flex",
        "zIndex",
        "fontSize",
        "lineHeight",
    ];
    assert_eq!(props.len(), earlier_classes);
    let attributes = props
        .iter()
        .enumerate()
        .fold(String::new(), |mut text, (index, name)| {
            text.push_str(&[" ", name, "=\"", &(index + 1).to_string(), "\""].concat());
            text
        });
    let source = format!(
        "import {{Box,keyframes,styled}} from '@devup-ui/react';const __devupForwardRef=r=>r;const a=<Box{attributes}/>;const kf=keyframes({{from:{{opacity:0}},to:{{opacity:1}}}});const Card=styled.div`${{kf}}:2;background:blue;`;const c=Card({{}},null);"
    );
    let _restore = Prefix;
    css::set_prefix(Some(prefix.to_string()));
    reset_class_map();
    reset_file_map();
    // When: the name replaces the interpolation after the tag was prepared.
    let actual = extract(
        "a.tsx",
        &source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )
    .required("prefixed styled template compiles");
    // Then: the substituted key is read as the order key, not as a property.
    assert!(
        actual.code.contains("const kf = \"style-order\";"),
        "{}",
        actual.code
    );
    let backgrounds: Vec<_> = actual
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "background" => Some((
                style.value().to_string(),
                style.level(),
                style.style_order(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(backgrounds, vec![("blue".to_string(), 0, Some(2))]);
}

#[rstest]
#[case::literal_true("true", "red")]
#[case::literal_false("false", "blue")]
#[serial]
fn w38r_u6_css_when_condition_is_a_boolean_literal_selects_that_side(
    #[case] condition: &str,
    #[case] color: &str,
) {
    // Given: the condition of a composed choice is a literal, not a binding.
    let source = format!(
        "import {{css}} from '@devup-ui/react';const a=css({condition}?{{color:'red'}}:{{color:'blue'}});"
    );
    // When: the finite results of the generated choice are read.
    let actual = output(&source);
    let evaluated = whole::evaluate_code(&actual.code, "a");
    // Then: the literal chooses the side and both sides stay in the inventory.
    assert_eq!(evaluated.element, format!("color-0-{color}--255-a"));
    assert_eq!(evaluated.trace, serde_json::json!([]));
    inventory(
        &actual,
        &[("color", "red", 0, None), ("color", "blue", 0, None)],
    );
}

#[rstest]
#[case::runtime_class("return p.cls;", "(p) => { return p.cls; }")]
#[case::runtime_fallback("return p.cls||'x';", "(p) => { return p.cls || \"x\"; }")]
#[serial]
fn w38r_u6_styled_callback_when_block_returns_a_runtime_class_reports_original_call(
    #[case] statement: &str,
    #[case] readable: &str,
) {
    // Given: a block callback whose class cannot be read again after its value is captured.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';\nconst C=styled.div(p=>{{{statement}}});"
    );
    // When: preparation reads the rewritten return value a second time.
    let actual = error(&source);
    // Then: the original call is reported and nothing generated leaks into the message.
    assert_eq!(
        actual,
        format!(
            "a.tsx:2:9: Cannot compose `{readable}` at build time: each style must be a rule object, a class, or a condition choosing between them"
        )
    );
}
