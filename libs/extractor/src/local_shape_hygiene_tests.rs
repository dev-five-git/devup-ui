use super::*;
use oxc_allocator::CloneIn;
use oxc_ast::ast::{Expression, JSXAttributeValue, Statement};
use oxc_ast_visit::{VisitMut, walk_mut};

#[rstest]
#[case("const rest = p.color;", "{color: () => rest}", "")]
#[case("const style = p.color;", "{color: function() { return style; }}", "")]
#[case("const className = p.color;", "{color: () => className}", "")]
#[case(
    "const color = p.color;",
    "{color: () => color}",
    "{ const color = 'orange'; return styled.div(inner); }"
)]
#[case("const rest = p.color;", "{_hover: {color: () => rest}}", "")]
#[case(
    "let rest = 'orange';",
    "{color: () => rest}",
    "rest = p.color; return styled.div(inner);"
)]
#[serial]
fn prepared_function_leaves_when_cloned_keep_lexical_purple(
    #[case] binding: &str,
    #[case] shape: &str,
    #[case] tail: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let tail = if tail.is_empty() {
        "return styled.div(inner);"
    } else {
        tail
    };
    let code = format!(
        "import {{styled}} from '@devup-ui/react'; function f(p) {{ {binding} const inner = {shape}; {tail} }}"
    );
    // When
    let output = extract("local-hygiene.tsx", &code, emotion_option())?;
    // Then
    assert_eq!(
        rendered_variables(&output.code)?,
        serde_json::json!(["purple"])
    );
    Ok(())
}

/// Evaluate generated styled callbacks, substituting their rendered element's
/// style object for JSX so Boa observes the actual variable reads.
fn rendered_variables(code: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    struct RenderStyle<'a>(&'a oxc_allocator::Allocator);
    impl<'a> VisitMut<'a> for RenderStyle<'a> {
        fn visit_expression(&mut self, expression: &mut Expression<'a>) {
            if let Expression::JSXElement(element) = expression {
                let value = element
                    .opening_element
                    .attributes
                    .iter()
                    .find_map(|attribute| {
                        let attribute = attribute.as_attribute()?;
                        attribute
                            .name
                            .as_identifier()
                            .filter(|name| name.name == "style")?;
                        match attribute.value.as_ref()? {
                            JSXAttributeValue::ExpressionContainer(container) => {
                                container.expression.as_expression()
                            }
                            JSXAttributeValue::StringLiteral(_)
                            | JSXAttributeValue::Element(_)
                            | JSXAttributeValue::Fragment(_) => None,
                        }
                    });
                assert!(value.is_some(), "expected generated style object");
                if let Some(value) = value {
                    *expression = value.clone_in(self.0);
                }
                return;
            }
            walk_mut::walk_expression(self, expression);
        }
    }
    let allocator = oxc_allocator::Allocator::default();
    let mut parsed = oxc_parser::Parser::new(&allocator, code, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    RenderStyle(&allocator).visit_program(&mut parsed.program);
    parsed
        .program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let compiled = oxc_codegen::Codegen::new().build(&parsed.program).code;
    let script = format!(
        "const __devupForwardRef = fn => props => fn(props, undefined); {compiled}\nJSON.stringify(Object.values(f({{color:'purple'}})({{}})));"
    );
    let json = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .map_err(|error| format!("generated styled callback failed: {error}\n{script}"))?
        .as_string()
        .ok_or("expected JSON")?
        .to_std_string_escaped();
    Ok(serde_json::from_str(&json)?)
}

#[rstest]
#[case("<div />")]
#[case("<div style />")]
#[case("<div style='red' />")]
#[case("<div style={/* missing expression */} />")]
#[case("<div {...props} />")]
fn rendered_oracle_when_generated_style_is_missing_rejects(#[case] jsx: &str) {
    // Given
    let code = format!("function f() {{ return () => {jsx}; }}");
    // When
    let result = std::panic::catch_unwind(|| rendered_variables(&code));
    // Then
    assert!(result.is_err(), "missing style must fail before evaluation");
}

#[rstest]
#[case("rest")]
#[case("style")]
#[case("className")]
#[case("DevupAs")]
#[serial]
fn captured_roots_when_generated_locals_shadow_source_keep_purple(
    #[case] name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react'; function f(p) {{ const {name} = {{color:p.color}}; const C = styled.div({name}); return C; }}"
    );
    // When
    let output = extract("local-hygiene.tsx", &code, emotion_option())?;
    // Then
    assert_eq!(
        rendered_variables(&output.code)?,
        serde_json::json!(["purple"])
    );
    Ok(())
}

#[rstest]
#[case(
    "const __devup_rest = {color:'orange'}; const __devup_rest1 = {color:'blue'};",
    "rest"
)]
#[case("const __devup_style = {color:'orange'};", "style")]
#[case("function deeper(__devup_rest) { return __devup_rest; }", "rest")]
#[case("function deeper() { return __devup_rest; }", "rest")]
#[serial]
fn captured_alias_when_existing_or_unresolved_names_collide_is_fresh(
    #[case] declarations: &str,
    #[case] name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react'; function f(p) {{ {declarations} const {name} = {{color:p.color}}; const C = styled.div({name}); return C; }}"
    );
    // When
    let output = extract("local-hygiene.tsx", &code, emotion_option())?;
    // Then
    assert_eq!(
        rendered_variables(&output.code)?,
        serde_json::json!(["purple"])
    );
    assert!(
        !output
            .code
            .contains(&format!("const __devup_{name} = {{ color: p.color }}")),
        "{}",
        output.code
    );
    Ok(())
}

#[rstest]
#[case("const inner = [, {color:p.color}];", "inner[`1`][`color`]")]
#[case("const inner = [...[, {color:p.color}]];", "inner[`1`][`color`]")]
#[serial]
fn captured_sparse_array_when_expanded_retains_original_index(
    #[case] declaration: &str,
    #[case] read: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{css}} from '@emotion/react'; function f(p) {{ {declaration} return <div css={{inner}} />; }}"
    );
    // When
    let output = extract("local-hygiene.tsx", &code, emotion_option())?;
    // Then
    assert!(output.code.contains(read), "{}", output.code);
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
            .count(),
        1
    );
    Ok(())
}

#[rstest]
#[case("() => tone")]
#[case("function() { return tone; }")]
#[serial]
fn callback_when_later_binding_is_deferred_compiles(
    #[case] callback: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react'; function f(p) {{ const inner = {{color:{callback}}}; const tone = p.color; const C = styled.div(inner); return C; }}"
    );
    // When
    let output = extract("local-hygiene.tsx", &code, emotion_option())?;
    // Then
    assert_eq!(
        rendered_variables(&output.code)?,
        serde_json::json!(["purple"])
    );
    Ok(())
}

#[rstest]
#[case("(() => tone)()")]
#[case("(function() { return tone; })()")]
#[case("(() => { const later = () => tone; return tone; })()")]
#[serial]
fn callback_when_invoked_before_binding_initializes_is_tdz(
    #[case] read: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{styled}} from '@devup-ui/react';\nfunction f(p) {{ const inner = {{color:{read}}}; const tone = p.color; const C = styled.div(inner); return C; }}"
    );
    // When
    let error = extract("local-hygiene.tsx", &code, emotion_option())
        .err()
        .ok_or("early IIFE must fail")?
        .to_string();
    // Then
    assert!(error.starts_with("local-hygiene.tsx:2:"), "{error}");
    assert!(error.contains("inner"), "{error}");
    Ok(())
}
