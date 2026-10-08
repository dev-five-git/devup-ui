use super::*;
use rstest::rstest;

#[test]
fn undefined_when_reference_has_no_semantic_identity_is_opaque()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: raw parser AST cannot distinguish an unbound name from a shadowed one.
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let expression = oxc_parser::Parser::new(&allocator, "undefined", oxc_span::SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    let values = crate::style_values::StyleValues::default();
    let imports = rustc_hash::FxHashMap::default();
    let inline_css = rustc_hash::FxHashMap::default();
    // When
    let result = normalize(
        &ast,
        &expression,
        &StyledBindings {
            imports: &imports,
            values: &values,
            inline_css: &inline_css,
        },
    );
    // Then
    assert!(matches!(result, Err(RuleError::Opaque)));
    assert_eq!(crate::utils::readable_code(&expression), "undefined");
    Ok(())
}

#[test]
fn undefined_when_semantics_proves_unbound_normalizes_to_empty_rules()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    // When
    let normalized = normalize_source(&ast, "undefined;")?;
    // Then
    assert!(matches!(normalized, Expression::NullLiteral(_)));
    assert_eq!(crate::utils::readable_code(&normalized), "null");
    Ok(())
}

fn normalize_source<'a>(
    ast: &AstBuilder<'a>,
    source: &'a str,
) -> Result<Expression<'a>, Box<dyn std::error::Error>> {
    let parsed =
        oxc_parser::Parser::new(ast.allocator(), source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let statement = parsed
        .program
        .body
        .first()
        .and_then(|statement| match statement {
            oxc_ast::ast::Statement::ExpressionStatement(statement) => Some(statement),
            _ => None,
        })
        .ok_or("expected expression statement")?;
    let values = crate::style_values::StyleValues::new(scoping);
    let imports = rustc_hash::FxHashMap::default();
    let inline_css = rustc_hash::FxHashMap::default();
    normalize(
        ast,
        &statement.expression,
        &StyledBindings {
            imports: &imports,
            values: &values,
            inline_css: &inline_css,
        },
    )
    .map_err(|_| "readable choices did not normalize".into())
}

fn assert_props_lowering_contract(expression: &Expression<'_>) {
    assert!(
        matches!(
            expression,
            Expression::ObjectExpression(_)
                | Expression::NullLiteral(_)
                | Expression::ConditionalExpression(_)
        ),
        "props lowering received unnormalized rules: {expression:?}"
    );
    if let Expression::ConditionalExpression(branch) = expression {
        assert!(crate::utils::is_pure(&branch.test));
        assert_props_lowering_contract(&branch.consequent);
        assert_props_lowering_contract(&branch.alternate);
    }
}

#[rstest]
#[case("unknownRules")]
#[case("p.on ? {color: 'red'} : unknownRules")]
#[case("check(p) ? {color: 'red'} : null")]
fn props_lowering_oracle_when_rules_are_unnormalized_rejects(
    #[case] source: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let source = format!("({source});");
    let parsed = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let statement = parsed
        .program
        .body
        .first()
        .ok_or("expected expression statement")?;
    assert!(matches!(
        statement,
        oxc_ast::ast::Statement::ExpressionStatement(_)
    ));
    if let oxc_ast::ast::Statement::ExpressionStatement(statement) = statement {
        let expression = crate::utils::unwrap_syntax_only(&statement.expression);
        // When
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_props_lowering_contract(expression);
        }));
        // Then
        assert!(
            result.is_err(),
            "unsupported leaves and impure tests must fail"
        );
    }
    Ok(())
}

#[rstest]
#[case("")]
#[case("const rules = {};")]
fn normalization_oracle_when_expression_statement_is_missing_rejects(#[case] source: &str) {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    // When
    let result = normalize_source(&ast, source);
    // Then
    assert!(result.is_err(), "missing expression statement must fail");
}

#[rstest]
#[case("p.on && { color: 'red' }")]
#[case("p.on ? { color: 'red' } : undefined")]
#[case("p.on ? false : true")]
#[case("(p.on && { color: 'red' }) ?? { color: 'blue' }")]
#[case("0 && unknownRules()")]
#[case("'' || { color: 'blue' }")]
fn normalized_rules_when_sent_to_props_lowering_have_only_pure_branches_and_object_or_null_leaves(
    #[case] source: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = format!("({source});");
    // When
    let normalized = normalize_source(&ast, &source)?;
    // Then
    assert_props_lowering_contract(&normalized);
    Ok(())
}

#[rstest]
#[case("0 && unknownRules()", "green")]
#[case("1 && { color: 'red' }", "red")]
#[case("'' || { color: 'blue' }", "blue")]
#[case("'yes' && { color: 'red' }", "red")]
#[case("0 ?? unknownRules()", "green")]
#[case("'' ?? unknownRules()", "green")]
#[case("({ color: 'red' }) && { color: 'blue' }", "blue")]
fn normalized_literal_when_selected_produces_expected_rule_value(
    #[case] source: &str,
    #[case] expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: parser input is not pre-folded by the public extraction pipeline.
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = format!("({source});");
    // When
    let normalized = normalize_source(&ast, &source)?;
    let code = crate::utils::expression_to_code(&normalized);
    let code = crate::css_utils::rm_last_semi_colon(&code);
    let code = format!("({code})?.color ?? 'green'");
    let mut context = boa_engine::Context::default();
    let result = context
        .eval(boa_engine::Source::from_bytes(&code))
        .map_err(|error| format!("generated expression failed to evaluate: {error}"))?
        .to_string(&mut context)
        .map_err(|error| format!("evaluated value failed string conversion: {error}"))?
        .to_std_string_escaped();
    // Then
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn numeric_nan_when_short_circuited_does_not_read_unknown_rules()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: NaN is a numeric AST value even though source spells it as a name.
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let expression = Expression::new_logical_expression(
        SPAN,
        Expression::new_numeric_literal(
            SPAN,
            f64::NAN,
            None,
            oxc_syntax::number::NumberBase::Decimal,
            &ast,
        ),
        LogicalOperator::And,
        Expression::new_identifier(SPAN, "unknown", &ast),
        &ast,
    );
    // When
    let values = crate::style_values::StyleValues::default();
    let imports = rustc_hash::FxHashMap::default();
    let inline_css = rustc_hash::FxHashMap::default();
    let normalized = normalize(
        &ast,
        &expression,
        &StyledBindings {
            imports: &imports,
            values: &values,
            inline_css: &inline_css,
        },
    )
    .map_err(|_| "falsy NaN did not normalize")?;
    // Then
    assert!(matches!(normalized, Expression::NullLiteral(_)));
    Ok(())
}
