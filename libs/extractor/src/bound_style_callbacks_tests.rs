use crate::{ExtractOption, ExtractStyleValue, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

fn compile(source: &str) -> Result<crate::ExtractOutput, Box<dyn std::error::Error>> {
    extract("callback.tsx", source, ExtractOption::default())
}

#[rstest]
#[case("p => ({'@layer': {base: {color: p.color}}})", false)]
#[case("function(p) { return {'@layer': {base: {color: p.color}}}; }", false)]
#[case("p => ({'@layer': {base: {color: p.color}}})", true)]
#[case("function(p) { return {'@layer': {base: {color: p.color}}}; }", true)]
#[case("({color = 'red'}) => ({'@layer': {base: {color}}})", true)]
#[case("(p = {}) => ({'@layer': {base: {color: p.color}}})", false)]
#[serial]
fn compiles_bound_callback_like_inline(
    #[case] callback: &str,
    #[case] local: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let body = format!("const rules = {callback}; return styled.div(rules);");
    let body = if local {
        format!("export function make() {{ {body} }}")
    } else {
        format!("const rules = {callback}; export const C = styled.div(rules);")
    };
    let source = format!("import {{styled}} from '@devup-ui/react';\n{body}");
    // When
    let output = compile(&source)?;
    // Then
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Dynamic(style) if style.layer.as_deref() == Some("base"))),
        "{output:?}"
    );
    assert!(output.code.contains("\"--"), "{}", output.code);
    assert!(
        !output.code.contains("styled.div(rules)"),
        "{}",
        output.code
    );
    Ok(())
}

#[rstest]
#[case("p => ({'@layer': {inherit: {color: p.color}}})")]
#[case("function(p) { return {'@layer': {inherit: {color: p.color}}}; }")]
#[serial]
fn reports_original_invalid_layer_key(
    #[case] callback: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{styled}} from '@devup-ui/react';\nconst rules = {callback};\nexport const C = styled.div(rules);"
    );
    // When
    let error = compile(&source)
        .err()
        .ok_or("expected invalid layer error")?
        .to_string();
    // Then
    assert!(error.starts_with("callback.tsx:2:"), "{error}");
    assert!(error.contains("@layer inherit"), "{error}");
    Ok(())
}

#[rstest]
#[case("let rules = p => ({color:p.color}); export const C = styled.div(rules);")]
#[case(
    "const rules = p => ({color:p.color}); rules = p => ({color:'red'}); export const C = styled.div(rules);"
)]
#[case("const rules = p => ({color:p.color}); consume(rules); export const C = styled.div(rules);")]
#[case("export const C = styled.div(rules); const rules = p => ({color:p.color});")]
#[case("const rules = p => unknown(p); export const C = styled.div(rules);")]
#[case("const rules = async p => ({color:p.color}); export const C = styled.div(rules);")]
#[case("export const rules = p => ({color:p.color}); export const C = styled.div(rules);")]
#[case(
    "export function make() { const rules = p => { const x = p.color; return { color:x}; }; return styled.div(rules); }"
)]
#[case("const rules = p => ({ color:rest}); export const C = styled.div(rules);")]
#[case(
    "const rules = p => ({ color:missing}); export function make(missing) { return styled.div(rules); }"
)]
#[serial]
fn rejects_unsafe_bound_callbacks(#[case] body: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!("import {{styled}} from '@devup-ui/react';\n{body}");
    // When
    let error = compile(&source)
        .err()
        .ok_or("expected unsafe callback error")?
        .to_string();
    // Then
    assert!(error.starts_with("callback.tsx:2:"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn preserves_application_arguments_inside_callback() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = "import {styled} from '@devup-ui/react'; const rules = p => ({color: consume(p.color)}); export const C = styled.div(rules);";
    // When
    let output = compile(source)?;
    // Then
    assert!(output.code.contains("consume(p.color)"), "{}", output.code);
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
    );
    Ok(())
}

#[rstest]
#[case(
    "const rules = p => ({color:p.color}); export function make() { const rules = p => ({color:'blue'}); return styled.div(rules); }",
    "blue"
)]
#[serial]
fn resolves_lexical_constants(
    #[case] body: &str,
    #[case] expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!("import {{styled}} from '@devup-ui/react'; {body}");
    // When
    let output = compile(&source)?;
    // Then
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property == "color" && style.value == expected)), "{output:?}");
    Ok(())
}

#[rstest]
#[case("rest")]
#[case("style")]
#[case("__devupValue")]
#[case("color")]
#[serial]
fn preserves_capture_across_generated_and_user_shadowing(
    #[case] name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{styled}} from '@devup-ui/react'; export function outer({name}) {{ const rules = p => ({{color:{name}}}); return function inner({name}) {{ return styled.div(rules); }}; }}"
    );
    // When
    let output = compile(&source)?;
    // Then
    assert!(
        output
            .code
            .contains(&format!("function outer(__devup_{name}")),
        "{}",
        output.code
    );
    assert!(
        output.code.contains(&format!("function inner({name})")),
        "{}",
        output.code
    );
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Dynamic(style) if style.identifier().contains(&format!("__devup_{name}")))));
    Ok(())
}

#[test]
#[serial]
fn permits_deferred_outer_read_before_its_declaration() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = "import {styled} from '@devup-ui/react'; export function make() { const rules = p => ({ color:later}); const later = getColor(); return styled.div(rules); }";
    // When
    let output = compile(source)?;
    // Then
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Dynamic(style) if style.identifier().contains("__devup_later"))), "{output:?}");
    Ok(())
}

#[rstest]
#[case("export const C = styled.div` ${rules} `;")]
#[case("export const C = styled('div')(rules);")]
#[serial]
fn resolves_callback_at_styled_rule_sites(
    #[case] use_site: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{styled}} from '@devup-ui/react'; const rules = p => ({{color:p.color}}); {use_site}"
    );
    // When
    let output = compile(&source)?;
    // Then
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Dynamic(_))),
        "{output:?}"
    );
    Ok(())
}

#[test]
#[serial]
fn leaves_ordinary_function_props_and_application_data_unchanged()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = "import {styled, Box} from '@devup-ui/react'; const rules = p => ({color:p.color}); export const data = consume(rules); export const button = <Box onClick={rules} />; export const C = styled.div({color:p => consume({width:2}, p.color)});";
    // When
    let output = compile(source)?;
    // Then
    assert!(output.code.contains("consume(rules)"), "{}", output.code);
    assert!(
        output.code.contains(&format!("onClick={{{}}}", "rules")),
        "{}",
        output.code
    );
    assert!(
        output
            .code
            .contains(&format!("consume({{width:{}}},p.color)", 2)),
        "{}",
        output.code
    );
    Ok(())
}
