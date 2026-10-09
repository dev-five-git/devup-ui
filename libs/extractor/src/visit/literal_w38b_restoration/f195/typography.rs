use super::*;

mod w38i_d;

struct TypographyKeys(Vec<String>);

impl TypographyKeys {
    fn registered() -> Self {
        let previous = css::theme_tokens::get_typography_keys();
        css::theme_tokens::set_typography_keys(vec!["body-1".to_string(), "heading".to_string()]);
        Self(previous)
    }
}

impl Drop for TypographyKeys {
    fn drop(&mut self) {
        css::theme_tokens::set_typography_keys(std::mem::take(&mut self.0));
    }
}

#[rstest]
#[case("css({typography:state.preset})", "body-1", "typo-body-1")]
#[case("css({typography:state.preset})", "", "")]
#[case("css({typography:`body-${1}`})", "unused", "typo-body-1")]
#[case("css({typography:`body-${state.part}`})", "1", "typo-body-1")]
#[serial]
fn local_typography_when_present_or_absent_preserves_selected_class_and_reads(
    #[case] call: &str,
    #[case] preset: &str,
    #[case] expected: &str,
) {
    // Given: registered theme names and real local source, without a fabricated payload.
    let _keys = TypographyKeys::registered();
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';<ClassNames>{{({{css,cx}})=>{call}}}</ClassNames>;"
    );
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, &source);
    let span = value.span();
    let setup = format!(
        "const state={{get preset(){{trace.push('preset');return '{preset}';}},get part(){{trace.push('part');return '{preset}';}}}}"
    );
    // When: original local typography preparation and emission selects a class.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: static templates read nothing; dynamic values read only their source once.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    let reads = match call {
        "css({typography:state.preset})" => vec!["preset"],
        "css({typography:`body-${1}`})" => vec![],
        "css({typography:`body-${state.part}`})" => vec!["part"],
        _ => panic!("unlisted typography source"),
    };
    assert_eq!(
        evaluated(&value, &setup),
        serde_json::json!([expected, reads])
    );
}

#[rstest]
#[case("body-1", true)]
#[case("missing", false)]
#[serial]
fn local_typography_when_nested_and_responsive_keeps_registered_selection(
    #[case] preset: &str,
    #[case] present: bool,
) {
    // Given: registered names and a selector's level-one dynamic preset.
    let _keys = TypographyKeys::registered();
    let _debug = DebugMode::enabled();
    let source = "import {ClassNames} from '@emotion/react';<ClassNames>{({css,cx})=>css({_hover:{typography:[null,state.preset]}})}</ClassNames>;";
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, source);
    let span = value.span();
    // When: the real registry-backed member selection emits the selected preset atom.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: only the present level-one hover preset applies; missing keys are empty.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    let style = visitor.styles.iter().find(|style| {
        matches!(style, ExtractStyleValue::Static(style) if style.property() == "typography" && style.value() == "body-1" && style.level() == 1 && style.selector().is_some())
    }).unwrap_or_else(|| panic!("registered responsive hover preset"));
    let expected = if present {
        class_of(style)
    } else {
        String::new()
    };
    let setup = format!("const state={{get preset(){{trace.push('preset');return '{preset}';}}}}");
    assert_eq!(
        evaluated(&value, &setup),
        serde_json::json!([expected, ["preset"]])
    );
}

#[test]
#[serial]
fn local_typography_when_font_size_is_explicit_yields_only_on_the_matching_selector() {
    // Given: responsive typography and font-size share hover, but not focus.
    let _keys = TypographyKeys::registered();
    let _debug = DebugMode::enabled();
    let source = "import {ClassNames} from '@emotion/react';<ClassNames>{({css,cx})=>css({_hover:{typography:['body-1','body-1'],fontSize:[null,'19px']},_focus:{typography:'body-1'}})}</ClassNames>;";
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, source);
    // When: original local emission runs typography yielding before naming.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: hover skips font-size from level one while focus keeps its preset intact.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    let hover = css::style_selector::StyleSelector::from("hover");
    let focus = css::style_selector::StyleSelector::from("focus");
    assert!(visitor.styles.iter().any(|style| {
        matches!(style, ExtractStyleValue::Static(style) if style.property() == "typography" && style.value() == "body-1|font-size:1" && style.selector() == Some(&hover))
    }));
    assert!(visitor.styles.iter().any(|style| {
        matches!(style, ExtractStyleValue::Static(style) if style.property() == "typography" && style.value() == "body-1" && style.selector() == Some(&focus))
    }));
    assert!(visitor.styles.iter().any(|style| {
        matches!(style, ExtractStyleValue::Static(style) if style.property() == "font-size" && style.value() == "19px" && style.level() == 1 && style.selector() == Some(&hover))
    }));
    let actual = evaluated(&value, "");
    let applied: std::collections::BTreeSet<_> = actual[0]
        .as_str()
        .unwrap_or_else(|| panic!("class"))
        .split_whitespace()
        .collect();
    let expected: Vec<_> = visitor.styles.iter().map(class_of).collect();
    assert_eq!(applied, expected.iter().map(String::as_str).collect());
    assert_eq!(actual[1], serde_json::json!([]));
}
