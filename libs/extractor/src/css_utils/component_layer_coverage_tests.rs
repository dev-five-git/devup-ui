use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("[ , '@layer base, theme;', enabled && '@layer {color:red;}' ]", 2)]
#[case("enabled && '@layer base, theme;'", 1)]
#[case("enabled ? '@layer base, theme;' : '@layer {color:red;}'", 2)]
#[case("[null, false, runtime, '@layer base {color:red;}']", 0)]
fn layer_policy_when_nested_in_rule_choices_checks_every_readable_branch(
    #[case] source: &str,
    #[case] count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    // When
    let errors = super::expression_layer_errors(&expression, "css");
    // Then
    assert_eq!(errors.len(), count);
    for (offset, error) in errors {
        assert!(
            source
                .get(usize::try_from(offset)?..)
                .is_some_and(|tail| tail.starts_with('\''))
        );
        assert!(error.contains("globalCss"), "{error}");
    }
    Ok(())
}

#[test]
fn layer_escape_when_hex_ends_in_crlf_consumes_both_whitespace_characters() {
    // Given
    let source = "\\62\r\nase.reset";
    // When
    let name = crate::css_utils::parse_layer_name(source);
    // Then
    assert_eq!(name.as_deref(), Some("base.reset"));
}

#[rstest]
#[case("p => ({'@layer': {inherit: {color: p.color}}})")]
#[case("p => { return {'@layer': {inherit: {color: p.color}}}; }")]
#[case("function(p) { return {'@layer': {inherit: {color: p.color}}}; }")]
#[case("p => [{'@layer': {inherit: {color: p.color}}}]")]
#[case("p => p.active ? {'@layer': {inherit: {color: p.color}}} : {}")]
#[case("p => ({'@layer': {inherit: {color: p.color}}}) || {}")]
#[case("p => ({} || {'@layer': {inherit: {color: p.color}}})")]
#[case("p => ({'@layer': {inherit: {color: p.color}}}) ?? {}")]
#[case("p => ({} ?? {'@layer': {inherit: {color: p.color}}})")]
#[serial_test::serial]
fn returned_layers_when_invalid_report_original_key_coordinates(
    #[case] callback: &str,
    #[values(false, true)] local: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = if local {
        format!(
            "import {{ styled }} from '@devup-ui/react';\nexport function make() {{\n return styled.div({callback});\n}}"
        )
    } else {
        format!(
            "import {{ styled }} from '@devup-ui/react';\nexport const result = styled.div({callback});"
        )
    };
    let offset = source.find("inherit").ok_or("missing fixture key")?;
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .ok_or("missing fixture line")?
        .chars()
        .count()
        + 1;
    // When
    let error = crate::extract(
        "returned-layer.tsx",
        &source,
        crate::ExtractOption::default(),
    )
    .err()
    .ok_or("expected invalid layer error")?
    .to_string();
    // Then
    assert!(
        error.lines().any(|error| error
            .starts_with(&format!("returned-layer.tsx:{line}:{column}:"))
            && error.contains("@layer inherit")),
        "{error}"
    );
    assert!(
        error.contains("@layer inherit") && error.contains("name the layer"),
        "{error}"
    );
    Ok(())
}

#[rstest]
#[case("getWidth({'@layer': {inherit: 1}, padding: 3})")]
#[case("new Width({'@layer': {inherit: 1}, padding: 3})")]
#[serial_test::serial]
fn value_calls_when_arguments_look_like_layers_preserve_application_data(
    #[case] value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{ styled }} from '@devup-ui/react';\nexport const result = styled.div({{ width: {value} }});"
    );
    // When
    let output = crate::extract("data-layer.tsx", &source, crate::ExtractOption::default())?;
    // Then
    let compact: String = output
        .code
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    assert!(
        compact.contains("inherit:1") && compact.contains("padding:3"),
        "{}",
        output.code
    );
    assert!(!compact.contains("padding:\"3px\""), "{}", output.code);
    assert!(output.styles.iter().any(|style| matches!(style, crate::ExtractStyleValue::Dynamic(style) if style.property() == "width")));
    Ok(())
}
