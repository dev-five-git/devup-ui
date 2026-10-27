use super::*;
use rstest::rstest;

#[rstest]
#[case("rest")]
#[case("className")]
fn local_shape_when_callback_capture_is_unresolved_reports_binding_error(
    #[case] name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: these names collide with locals introduced by generated styled scopes.
    let allocator = Allocator::default();
    let source = format!(
        "import {{styled}} from '@devup-ui/react';\nfunction f() {{ const inner = {{color: () => {name}}}; return styled.div(inner); }}"
    );
    let mut parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let offset = u32::try_from(source.find("inner =").ok_or("missing binding")?)?;
    // When
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut parsed.program,
        "capture.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &FxHashSet::default(),
    );
    // Then: the rejected shape stays a binding read, never a relocated callback.
    assert_eq!(
        inlined.errors,
        vec![(offset, crate::utils::local_style_error("inner"))]
    );
    assert_eq!(
        crate::located_errors(
            "capture.tsx",
            &source,
            &[],
            inlined.errors,
            parsed.program.source_type
        ),
        format!(
            "capture.tsx:2:22: {}",
            crate::utils::local_style_error("inner")
        )
    );
    let generated = oxc_codegen::Codegen::new().build(&parsed.program).code;
    assert!(generated.contains("styled.div(inner)"), "{generated}");
    Ok(())
}

#[test]
fn local_shape_when_callback_capture_is_resolved_retains_lexical_read() {
    // Given
    let allocator = Allocator::default();
    let source = "import {styled} from '@devup-ui/react'; function f(p) { const rest = p.color; const inner = {color: () => rest}; return styled.div(inner); }";
    let mut parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    // When
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut parsed.program,
        "capture.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &FxHashSet::default(),
    );
    // Then
    assert_eq!(inlined.errors, vec![]);
    let generated = oxc_codegen::Codegen::new().build(&parsed.program).code;
    assert!(
        generated.contains("color: () => __devup_rest"),
        "{generated}"
    );
}

#[rstest]
#[case(true)]
#[case(false)]
fn renamed_shorthand_when_capture_alias_changes_preserves_public_key_and_value(
    #[case] rename: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: only rest is renamed; the unrelated shorthand is a control.
    let allocator = Allocator::default();
    let mut parsed = Parser::new(
        &allocator,
        "function f() { const rest = 'purple'; const unchanged = 'green'; return {rest, unchanged}; }",
        SourceType::tsx(),
    ).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let symbol = scoping
        .symbol_ids()
        .find(|symbol| scoping.symbol_name(*symbol) == "rest")
        .ok_or("missing captured binding")?;
    let aliases = if rename {
        FxHashMap::from_iter([(symbol, "__devup_rest".to_string())])
    } else {
        FxHashMap::default()
    };
    // When
    local_capture_reads::Rename {
        builder: &AstBuilder::new(&allocator),
        scoping: &scoping,
        aliases: &aliases,
    }
    .visit_program(&mut parsed.program);
    let generated = oxc_codegen::Codegen::new().build(&parsed.program).code;
    let script = format!("{generated}\nJSON.stringify(f());");
    let value = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .map_err(|error| error.to_string())?;
    let json = value
        .as_string()
        .ok_or("expected JSON result")?
        .to_std_string_escaped();
    // Then: a shorthand accidentally kept after renaming would change the key.
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json)?,
        serde_json::json!({"rest": "purple", "unchanged": "green"})
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn local_shape_when_emotion_styles_read_runtime_leaves_preserves_pixel_context() {
    // Given
    let source = "import styled from '@emotion/styled';function make(input){const shape={padding:2,color:input.color};return styled.div(shape);}const Choice=make({color:'red'});";
    let option = ExtractOption {
        single_css: true,
        import_aliases: std::collections::HashMap::from([(
            "@emotion/styled".into(),
            crate::ImportAlias::DefaultToNamed("styled".into()),
        )]),
        ..Default::default()
    };
    let output =
        crate::extract("local-shape.tsx", source, option).unwrap_or_else(|error| panic!("{error}"));
    // When
    let observed = crate::styled_source_tests::captured_render::execute(
        ("local-shape.tsx", None),
        &output,
        "const element=render({target:Choice,props:{}});JSON.stringify([values(element,'color'),values(element,'padding')]);",
    );
    // Then
    assert_eq!(observed, serde_json::json!([["red"], ["2px"]]));
}

#[rstest]
#[case("{}", "blue")]
#[case("{color:'green'}", "red")]
#[serial_test::serial]
fn imported_class_when_used_as_a_controller_selects_by_declarations_not_metadata(
    #[case] rules: &str,
    #[case] expected: &str,
) {
    // Given
    let module = format!("import {{css}} from '@devup-ui/react';export const mixin=css({rules});");
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./base").then(|| {
            crate::ModuleResolution::Resolved(crate::ResolvedModule {
                path: "/base.tsx".into(),
                code: module.clone(),
                source_type: None,
            })
        })
    };
    let source = "import {styled} from '@devup-ui/react';import {mixin} from './base';const Choice=styled.div(mixin?{color:'red'}:{color:'blue'});";
    // When
    let output = crate::extract_with_modules(
        "/entry.tsx",
        source,
        ExtractOption {
            single_css: true,
            ..Default::default()
        },
        false,
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then
    let colors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "color" => Some(style.value()),
            _ => None,
        })
        .collect();
    assert_eq!(colors, [expected]);
}
