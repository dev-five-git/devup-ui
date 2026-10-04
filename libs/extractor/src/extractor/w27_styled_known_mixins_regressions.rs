use super::super::w27_props_rule_choices_tests::support::{active, declarations};
use crate::{ExtractOption, ExtractStyleValue};

fn compile(template: &str) -> crate::ExtractOutput {
    let source = format!(
        "import {{ styled, css }} from '@devup-ui/react'; const mixin = css({{ color: 'red', padding: '4px' }}); export const Choice = styled.div`{template}`;"
    );
    crate::extract(
        "context-mixin.tsx",
        &source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

#[test]
#[serial_test::serial]
fn known_mixin_inherits_context_when_surrounded_by_layer_selector_and_media() {
    // Given: the same atom competes inside each enclosing context.
    for (open, close, layer, selector) in [
        ("@layer base {", "}", Some("base"), None),
        ("&:hover {", "}", None, Some(":hover")),
        (
            "@media print { &:hover { @layer base {",
            "} } }",
            Some("base"),
            Some("print"),
        ),
    ] {
        let output = compile(&format!("{open} ${{mixin}}; color: blue; {close}"));
        // When: the component's class expression runs.
        let selected = active(&output, "{}");
        // Then: red is replaced only in the matching context; padding inherits it.
        assert_eq!(
            declarations(&selected),
            [
                ("color".into(), "blue".into()),
                ("padding".into(), "4px".into())
            ]
        );
        for value in selected {
            let ExtractStyleValue::Static(style) = value else {
                panic!("static atom required")
            };
            assert_eq!(style.layer(), layer);
            match selector {
                Some(selector) => assert!(
                    style
                        .selector()
                        .is_some_and(|value| value.to_string().contains(selector))
                ),
                None => assert_eq!(style.selector(), None),
            }
        }
    }
}

#[test]
#[serial_test::serial]
fn conditional_known_mixin_composes_per_property_when_later_color_is_always_present() {
    // Given: a known callback mixin followed by an unconditional competing color.
    for template in [
        "${p => p.on && mixin}; color: blue;",
        "@layer base { &:hover { ${p => p.on && mixin}; color: blue; } }",
    ] {
        let output = compile(template);
        // When / Then: blue always wins, and only the enabled branch supplies padding.
        for on in [true, false] {
            let mut expected = vec![("color".into(), "blue".into())];
            if on {
                expected.push(("padding".into(), "4px".into()));
            }
            assert_eq!(
                declarations(&active(&output, &format!("{{ on: {on} }}"))),
                expected
            );
        }
    }
}

#[test]
#[serial_test::serial]
fn context_sequence_keeps_prior_atom_when_later_conditional_mixin_is_absent() {
    let output = compile(
        "@layer base { &:hover { color: blue; ${p => p.on ? mixin : null}; } } color: green;",
    );
    for on in [true, false] {
        let selected = active(&output, &format!("{{ on: {on} }}"));
        let mut expected = [
            ("color".into(), if on { "red" } else { "blue" }.into()),
            ("color".into(), "green".into()),
        ]
        .into_iter()
        .chain(on.then(|| ("padding".into(), "4px".into())))
        .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(declarations(&selected), expected);
    }
}

#[test]
#[serial_test::serial]
fn unknown_nested_mixin_fails_located_instead_of_applying_global_classes() {
    // Given: opaque classes cannot be recontextualized.
    for mixin in ["runtime", "p => p.on && runtime"] {
        let source = format!(
            "import {{ styled }} from '@devup-ui/react'; import {{ runtime }} from './runtime'; export const Choice = styled.div`@layer base {{ &:hover {{ ${{{mixin}}}; color: blue; }} }}`;"
        );
        // When: extraction must refuse the unknown nested statement.
        let error = crate::extract("unknown-nested.tsx", &source, ExtractOption::default())
            .err()
            .unwrap_or_else(|| panic!("unknown nested mixin compiled"));
        // Then: the error identifies the source interpolation.
        assert!(error.to_string().contains("unknown-nested.tsx:1:"));
        assert!(error.to_string().contains("cannot use"));
        assert!(error.to_string().contains("runtime"));
    }
}

#[test]
#[serial_test::serial]
fn contextual_callback_preserves_dynamic_lengths_and_theme_bindings_when_enabled() {
    // Given: a written callback object inside selector and layer contexts.
    let output = compile(
        "@layer base { &:hover { ${p => p.on && { padding: p.padding, color: p.theme.colors.primary }}; } }",
    );
    // When: only the selected property's CSS-variable expression is evaluated.
    let selected = active(&output, "{ on: true, padding: 8 }");
    let padding = selected
        .iter()
        .find_map(|style| match style {
            ExtractStyleValue::Dynamic(style) if style.property() == "padding" => Some(style),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing dynamic padding"));
    // Then: numeric lengths stay pixels, and theme paths remain static variables.
    assert_eq!(
        super::super::w27_props_rule_choices_tests::support::evaluate(
            padding.identifier(),
            "{ padding: 8 }"
        ),
        "8px"
    );
    assert_eq!(padding.layer(), Some("base"));
    assert!(
        padding
            .selector()
            .is_some_and(|selector| selector.to_string().contains(":hover"))
    );
    assert!(selected.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.value() == "var(--colors-primary)")));
    assert_eq!(active(&output, "{ on: false }"), vec![]);
}

#[test]
#[serial_test::serial]
fn context_lexer_ignores_data_braces_when_known_mixin_follows_comments_and_quoted_values() {
    let output =
        compile("@layer base { &:hover { content: '}'; /* } */ ${mixin}; color: blue; } }");
    let selected = active(&output, "{}");
    assert_eq!(
        declarations(&selected)
            .iter()
            .filter(|(property, _)| property == "color")
            .cloned()
            .collect::<Vec<_>>(),
        [("color".into(), "blue".into())]
    );
    assert!(selected.iter().all(|style| matches!(style, ExtractStyleValue::Static(style) if style.layer() == Some("base") && style.selector().is_some_and(|selector| selector.to_string().contains(":hover")))));
}

#[test]
#[serial_test::serial]
fn contextual_composition_preserves_distinct_keys_when_mixin_has_inner_selectors_breakpoints_and_layers()
 {
    let source = "import { styled, css } from '@devup-ui/react'; const mixin = css({ color: ['red', 'orange'], '&:focus': { color: 'yellow' }, '@layer': { inner: { padding: '4px' } } }); export const Choice = styled.div`@layer outer { @media print { &:hover { ${mixin}; color: blue; &:focus { color: purple; } } } }`;";
    let output = crate::extract(
        "nested-key-mixin.tsx",
        source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let selected = active(&output, "{}");
    let mut keys: Vec<_> = selected
        .iter()
        .map(|value| {
            let ExtractStyleValue::Static(style) = value else {
                panic!("static atom required")
            };
            (
                style.property().to_string(),
                style.value().to_string(),
                style.level(),
                style.layer().map(str::to_string),
                style.selector().map(ToString::to_string),
            )
        })
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            (
                "color".into(),
                "blue".into(),
                0,
                Some("outer".into()),
                Some("@media print &:hover".into())
            ),
            (
                "color".into(),
                "orange".into(),
                1,
                Some("outer".into()),
                Some("@media print &:hover".into())
            ),
            (
                "color".into(),
                "purple".into(),
                0,
                Some("outer".into()),
                Some("@media print &:hover:focus".into())
            ),
            (
                "padding".into(),
                "4px".into(),
                0,
                Some("outer.inner".into()),
                Some("@media print &:hover".into())
            ),
        ]
    );
}

#[test]
#[serial_test::serial]
fn unknown_written_rule_branches_fail_when_top_level_callback_cannot_be_read_as_styles() {
    for mixin in [
        "p => p.on ? { color: 'blue' } : p.rules",
        "p => p.on && { ...p.rules }",
        "p => check(p) ? { color: 'red' } : { color: 'blue' }",
    ] {
        let source = format!(
            "import {{ styled }} from '@devup-ui/react'; export const Choice = styled.div`${{{mixin}}}; color: green;`;"
        );
        let error = crate::extract("unknown-rule-mixin.tsx", &source, ExtractOption::default())
            .err()
            .unwrap_or_else(|| panic!("unknown rule shape compiled"));
        assert!(error.to_string().contains("unknown-rule-mixin.tsx:1:"));
        assert!(error.to_string().contains("cannot use"));
    }
}
