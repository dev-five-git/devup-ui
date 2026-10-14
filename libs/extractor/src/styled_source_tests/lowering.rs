use super::*;
use oxc_semantic::SemanticBuilder;

#[rstest]
#[case("ts")]
#[case("mts")]
#[case("cts")]
#[case("js")]
#[case("mjs")]
#[case("cjs")]
#[serial]
fn output_parses_when_original_suffix_excludes_jsx(
    #[case] extension: &str,
    #[values(true, false)] single_css: bool,
    #[values(
        "styled('div', { color: 'red' })",
        "styled('div')({ color: 'red' })",
        "styled.div({ color: 'red' })",
        "styled.div`color: red;`"
    )]
    factory: &str,
) {
    // Given: a valid styled definition in its original non-JSX dialect.
    let filename = format!("test.{extension}");
    let source = format!("{IMPORT}export const Card = {factory};");
    // When: the real extractor compiles it.
    let output = compile(&filename, &source, single_css);
    // Then: the complete emitted module parses without JSX permission or ignored errors.
    let arena = Allocator::default();
    parse(&arena, &filename, &output);
}

#[rstest]
#[case("styled('Widget', {})", Target::Tag("Widget".into()))]
#[case("styled('motion.div', {})", Target::Tag("motion.div".into()))]
#[case("styled('my-card', {})", Target::Tag("my-card".into()))]
#[case("styled(Component, {})", Target::Component("Component".into()))]
#[case("styled(motion.div, {})", Target::Member("motion.div".into()))]
#[case("styled(Motion.Div.Inner, {})", Target::Member("Motion.Div.Inner".into()))]
#[case("styled(Box, {})", Target::Tag("div".into()))]
#[case("styled(forwardRef((props, ref) => createElement('div', { ...props, ref })))({})", Target::Component("DevupStyled".into()))]
#[serial]
fn render_target_is_preserved_when_styled_emits_a_call(
    #[case] factory: &str,
    #[case] expected: Target,
    #[values(true, false)] single_css: bool,
) {
    // Given: tags and component values with intentionally different identities.
    let source = format!(
        "{IMPORT}import {{ forwardRef, createElement }} from 'react';\nconst Card = {factory};"
    );
    // When: the real extractor compiles even an empty-style factory.
    let output = compile("test.ts", &source, single_css);
    // Then: the render call receives the literal or component AST, not a guessed string.
    let arena = Allocator::default();
    let program = parse(&arena, "test.ts", &output);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1, "{output}");
    assert_eq!(calls[0].target, expected);
    assert!(jsx_imports(&program).contains(&calls[0].helper), "{output}");
}

#[rstest]
#[case(".attrs({ role: 'button' })")]
#[case(".attrs(props => ({ role: props.role || 'button' }))")]
#[serial]
fn passthrough_props_remain_when_attrs_wrap_the_renderer(
    #[case] attrs: &str,
    #[values(true, false)] single_css: bool,
) {
    // Given: attrs wrap a renderer with caller as/ref/children.
    let source = format!(
        "{IMPORT}const Card = styled('article'){attrs}`color: red;`;\nconst element = Card({{ as: 'section', ref: callerRef, children: 'child' }});"
    );
    // When: extraction compiles the attrs-wrapped renderer.
    let output = compile("test.ts", &source, single_css);
    // Then: the actual default target and rest spread remain, with forwardedAs supplying output as.
    let arena = Allocator::default();
    let program = parse(&arena, "test.ts", &output);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1, "{output}");
    assert_eq!(calls[0].target, Target::Tag("article".into()));
    assert_eq!(
        calls[0]
            .props
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["...", "className", "style", "as"]
    );
    assert_eq!(
        calls[0].props[0].1,
        "(({ \"theme\": __devupOmit0, ...__devupDom }) => __devupDom)(rest)"
    );
    assert_eq!(calls[0].props[3].1, "forwardedAs");
    assert!(output.contains("as: \"section\""));
    assert!(output.contains("ref: callerRef"));
    assert!(output.contains("children: \"child\""));
    assert!(output.contains("role:"));
}

#[rstest]
#[serial]
fn helper_resolves_to_import_when_names_collide(#[values(true, false)] single_css: bool) {
    // Given: a binding, nested binding, free reference and shadowed existing jsx alias.
    let source = format!(
        "{IMPORT}import {{ jsx as existing }} from 'react/jsx-runtime';\nconst __devupJsx0 = 0;\nconst free = __devupJsx2;\nconst A = styled('div', {{}});\nfunction make(existing, __devupJsx1) {{ return styled('span', {{}}); }}\nconst element = existing('aside', {{ children: 'original' }});"
    );
    // When: the extractor compiles two renderers in different scopes.
    let output = compile("test.ts", &source, single_css);
    // Then: both calls resolve to the same imported jsx symbol, never a shadowing binding.
    let arena = Allocator::default();
    let program = parse(&arena, "test.ts", &output);
    let semantic = SemanticBuilder::new().build(&program).semantic;
    let calls = renders(&program);
    assert_eq!(calls.len(), 2, "{output}");
    assert_eq!(calls[0].helper, calls[1].helper);
    let imports = jsx_imports(&program);
    assert!(imports.contains(&"existing".into()));
    assert!(imports.contains(&calls[0].helper), "{output}");
    assert_eq!(
        imports
            .iter()
            .filter(|name| name.as_str() != "existing")
            .count(),
        1
    );
    let imported_symbol = program.body.iter().find_map(|statement| match statement {
        Statement::ImportDeclaration(import) if import.source.value == "react/jsx-runtime" => {
            import
                .specifiers
                .iter()
                .flatten()
                .find_map(|specifier| match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(named)
                        if named.local.name == calls[0].helper.as_str() =>
                    {
                        named.local.symbol_id.get()
                    }
                    _ => None,
                })
        }
        _ => None,
    });
    assert!(imported_symbol.is_some(), "{output}");
    for call in &calls {
        assert!(
            !["__devupJsx0", "__devupJsx1", "__devupJsx2", "existing"]
                .contains(&call.helper.as_str())
        );
        let reference = match call.reference {
            Some(reference) => reference,
            None => panic!("missing helper reference"),
        };
        assert_eq!(
            semantic.scoping().get_reference(reference).symbol_id(),
            imported_symbol
        );
    }
}
