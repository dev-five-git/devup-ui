use super::*;

const HOVER: &str = "color-0-blue-_a__c_hover-255";
const RULES: &str = "const rules = { selectors: { '&:hover': { color: 'blue' } } };";
const MODULES: &[(&str, &str)] = &[(
    "/src/rules.ts",
    "export const rules = { selectors: { '&:hover': { color: 'blue' } } };",
)];

#[rstest]
#[case::record((RULES, "css`${rules}`", &[HOVER][..]))]
#[case::direct(("", "css`${{ selectors: { '&:hover': { color: 'blue' } } }}`", &[HOVER][..]))]
#[case::alias(("const rules = { selectors: { '&:hover': { color: 'blue' } } }; const alias = rules;", "css`${alias}`", &[HOVER][..]))]
#[case::imported(("import { rules as alias } from './rules';", "css`${alias}`", &[HOVER][..]))]
#[case::namespace_local_record(("import * as Devup from '@devup-ui/react'; const local = { selectors: { '&:hover': { color: 'teal' } } };", "Devup.css`${local}`", &["color-0-teal-_a__c_hover-255"][..]))]
#[case::namespace_imported_record(("import * as Devup from '@devup-ui/react'; import { rules as alias } from './rules';", "Devup.css`${alias}`", &[HOVER][..]))]
#[case::normal_call((RULES, "css(rules)", &[HOVER][..]))]
#[case::compiled_class(("const base = css({ color: 'red' });", "css`${base};`", &["color-0-red--255"][..]))]
#[case::mixed_scalar(("const rules = { selectors: { '&:hover': { color: 'blue' } } }; const tone = 'red';", "css`${rules}; color: ${tone}; margin: 0;`", &[HOVER, "color-0-red--255", "margin-0-0--255"][..]))]
#[case::follow_up(("const rules = { selectors: { '&:hover': { color: 'blue' } } }; const tag = css`${rules}; color: red;`;", "css(tag, { color: 'green', p: 1 })", &[HOVER, "color-0-green--255", "padding-0-4px--255"][..]))]
#[case::scalar_statement(("const text = 'color: red;';", "css`${text} margin: 0;`", &["color-0-red--255", "margin-0-0--255"][..]))]
#[case::object_after_text(("const rules = { color: 'blue', selectors: { '&:hover': { color: 'blue' } } };", "css`color: red; ${rules};`", &[HOVER, "color-0-blue--255"][..]))]
#[case::text_after_object(("const rules = { color: 'blue', selectors: { '&:hover': { color: 'blue' } } };", "css`${rules}; color: red;`", &[HOVER, "color-0-red--255"][..]))]
#[case::tag_last(("const rules = { color: 'blue', selectors: { '&:hover': { color: 'blue' } } }; const tag = css`color: red; ${rules};`;", "css({ color: 'green', p: 1 }, tag)", &[HOVER, "color-0-blue--255", "padding-0-4px--255"][..]))]
#[serial]
fn card_selects_linked_atoms_when_tag_statements_are_composed(
    #[case] sample: (&str, &str, &[&str]),
    #[values("/src/item3.ts", "/src/item3.css.ts")] filename: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    css::debug::set_debug(true);
    let (binding, expression, expected) = sample;
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\n{binding}\nexport const card = {expression};"
    );
    // When
    let output = extract_with_modules(
        filename,
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
        false,
        &memory_resolver(MODULES),
    )
    .unwrap_or_else(|error| panic!("{source}\n{error}"));
    // Then: card is a string whose selected tokens link to CSS-generator atoms.
    let aliases_base =
        expression == "css`${base};`" && output.code.contains("export const card = base;");
    let selected = if aliases_base {
        output.code.replace("const base =", "export const base =")
    } else {
        output.code.clone()
    };
    let export = if aliases_base { "base" } else { "card" };
    assert_composed_export_atoms(&selected, export, expected);
    let declaration = format!("export const {export} = \"");
    let classes = selected
        .split_once(&declaration)
        .unwrap()
        .1
        .split_once('"')
        .unwrap()
        .0;
    for class in classes.split_whitespace() {
        assert!(output.styles.iter().any(|style| {
            matches!(style.extract((!single_css).then_some(filename)), Some(StyleProperty::ClassName(name)) if name == class)
        }), "selected class {class} has no emitted style: {output:?}");
    }
    if binding.contains("from './rules';") {
        assert!(
            output
                .dependencies
                .iter()
                .any(|path| path == "/src/rules.ts")
        );
    }
}

#[rstest]
#[serial]
fn opaque_class_survives_when_an_object_statement_is_composed(
    #[values(false, true)] single_css: bool,
) {
    // Given: runtime class input must stay opaque, unlike the known style record.
    let _state = crate::test_state::TestStateGuard::new();
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\n{RULES}\nfunction render(externalClass) {{ const card = css`${{externalClass}}; ${{rules}};`; return {{ props: {{ className: card }} }}; }}"
    );
    // When: execute the actual generated merge through the existing observer.
    let output = extract(
        "/src/opaque.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap();
    let observed = render_observation(
        &output,
        "(() => { const result = render('external'); const card = result.props.className; if (typeof card !== 'string' || card.split(/\\s+/).filter(x => x === 'external').length !== 1 || card.includes('[object Object]')) throw Error('invalid card'); return result; })()",
        (!single_css).then_some("/src/opaque.ts"),
    );
    // Then: blue belongs to a selected class, not merely an unused sheet atom.
    assert_eq!(observed, r#"[[],["blue"]]"#);
    assert_eq!(output.styles.len(), 1);
    let ExtractStyleValue::Static(style) = output.styles.iter().next().unwrap() else {
        panic!("expected a static hover declaration: {output:?}");
    };
    assert_eq!(style.selector(), Some(&"&:hover".into()));
    assert_eq!(style.level(), 0);
    assert_eq!(style.style_order(), None);
}

#[rstest]
#[case::changed("rules.selectors['&:hover'].color = 'green';")]
#[case::changed_alias("const alias = rules; alias.selectors['&:hover'].color = 'green';")]
#[case::escaped("register(rules);")]
#[serial]
fn changed_record_is_rejected_when_used_as_a_tag_statement(
    #[case] change: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: only the ordinary route promises static mutation/escape refusal.
    let _state = crate::test_state::TestStateGuard::new();
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\n{RULES}\n{change}\nexport const card = css`${{rules}}`;"
    );
    // When
    let error = extract(
        "/src/changed.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_err()
    .to_string();
    // Then: point at the authored statement read, never synthetic/inlined text.
    assert!(error.starts_with("/src/changed.ts:4:27: "), "{error}");
    assert!(
        error.contains("at build time") && error.contains("rules"),
        "{error}"
    );
}

#[rstest]
#[case::unresolved_leaf(("css`${{ selectors: { '&:hover': { color: globalThis.tone } } }}`", "globalThis.tone"))]
#[case::unresolved_record(("css`${rules}`", "rules"))]
#[case::value_slot(("css`color: ${rules};`", "rules"))]
#[case::property_slot(("css`${rules}: red;`", "rules"))]
#[serial]
fn object_leaves_and_nonstatement_slots_retain_authored_errors(
    #[case] sample: (&str, &str),
    #[values(false, true)] single_css: bool,
) {
    // Given: unresolved records stay distinct from legitimate opaque classes.
    let _state = crate::test_state::TestStateGuard::new();
    let (expression, leaf) = sample;
    let binding = if expression == "css`${rules}`" {
        "const rules = { selectors: { '&:hover': { color: globalThis.tone } } };"
    } else {
        RULES
    };
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\n{binding}\nexport const card = {expression};"
    );
    let line = source.lines().nth(2).unwrap();
    let column = if expression == "css`color: ${rules};`" {
        21
    } else {
        line.find(leaf).unwrap() + 1
    };
    // When
    let error = extract_with_modules(
        "/src/refused.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
        false,
        &memory_resolver(&[]),
    )
    .unwrap_err()
    .to_string();
    // Then: no object-to-rules expansion in CSS value/property positions.
    assert!(
        error.starts_with(&format!("/src/refused.ts:3:{column}: ")),
        "{error}"
    );
    assert!(error.contains("at build time"), "{error}");
}
