use super::{unwrap_syntax_only, unwrap_syntax_only_mut};
use oxc_allocator::{Allocator, FromIn};
use oxc_ast::ast::{Expression, ObjectPropertyKind, Program, Statement, Str, StringLiteral};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType, Span};
use rstest::rstest;

fn parsed<'a>(allocator: &'a Allocator, source: &'a str) -> Program<'a> {
    let parsed = Parser::new(allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .with_check_syntax_error(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    drop(built);
    parsed.program
}

fn span(source: &str, fragment: &str) -> Result<Span, Box<dyn std::error::Error>> {
    let start = source.find(fragment).ok_or("missing span fragment")?;
    let end = start.checked_add(fragment.len()).ok_or("span overflow")?;
    Ok(Span::new(u32::try_from(start)?, u32::try_from(end)?))
}

fn color_slot<'a, 'b>(
    expression: &'b mut Expression<'a>,
) -> Result<&'b mut StringLiteral<'a>, Box<dyn std::error::Error>> {
    let Expression::ObjectExpression(object) = expression else {
        return Err("expected object operand".into());
    };
    let Some(ObjectPropertyKind::ObjectProperty(property)) = object.properties.first_mut() else {
        return Err("expected color property".into());
    };
    assert_eq!(property.key.static_name().as_deref(), Some("color"));
    let Expression::StringLiteral(literal) = &mut property.value else {
        return Err("expected color literal".into());
    };
    Ok(literal)
}

#[rstest]
#[case("const result=(<string>('blue' as string));", "'blue'", "literal")]
#[case("const result=('blue' as string);", "'blue'", "literal")]
#[case("const result=(1+2);", "1+2", "binary")]
fn operand_is_preserved_when_immutable_wrappers_are_removed(
    #[case] source: &str,
    #[case] fragment: &str,
    #[case] expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = Allocator::default();
    let program = parsed(&allocator, source);
    let Some(Statement::VariableDeclaration(declaration)) = program.body.first() else {
        return Err("expected result declaration".into());
    };
    let initializer = declaration
        .declarations
        .first()
        .and_then(|value| value.init.as_ref())
        .ok_or("missing initializer")?;
    // When
    let operand = unwrap_syntax_only(initializer);
    // Then
    let kind = match operand {
        Expression::StringLiteral(literal) => {
            assert_eq!(literal.value.as_str(), "blue");
            "literal"
        }
        Expression::BinaryExpression(_) => "binary",
        _ => return Err("unexpected operand kind".into()),
    };
    assert_eq!(kind, expected);
    assert_eq!(operand.span(), span(source, fragment)?);
    Ok(())
}

#[rstest]
#[case(
    concat!("const rules=(<", "{color", ":string}>", "{color:", "'red'});"),
    concat!("<", "{color", ":string}>", "{color:", "'red'}")
)]
#[case(
    "const rules=({color:'red'} as {color:string});",
    "{color:'red'} as {color:string}"
)]
fn original_object_changes_when_mutable_wrapper_operand_is_written(
    #[case] source: &str,
    #[case] wrapper: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = Allocator::default();
    let mut program = parsed(&allocator, source);
    let Some(Statement::VariableDeclaration(declaration)) = program.body.first_mut() else {
        return Err("expected rules declaration".into());
    };
    let initializer = declaration
        .declarations
        .first_mut()
        .and_then(|value| value.init.as_mut())
        .ok_or("missing initializer")?;
    // When
    color_slot(unwrap_syntax_only_mut(initializer))?.value = Str::from_in("blue", &allocator);
    // Then: inspect the original tree without either unwrapping helper.
    assert_eq!(initializer.span(), span(source, &format!("({wrapper})"))?);
    let Expression::ParenthesizedExpression(parenthesized) = initializer else {
        return Err("outer parentheses lost".into());
    };
    assert_eq!(parenthesized.expression.span(), span(source, wrapper)?);
    let object = match &mut parenthesized.expression {
        Expression::TSTypeAssertion(assertion) => &mut assertion.expression,
        Expression::TSAsExpression(assertion) => &mut assertion.expression,
        _ => return Err("outer assertion lost".into()),
    };
    assert_eq!(object.span(), span(source, "{color:'red'}")?);
    let literal = color_slot(object)?;
    assert_eq!(literal.value.as_str(), "blue");
    assert_eq!(literal.span, span(source, "'red'")?);
    Ok(())
}
