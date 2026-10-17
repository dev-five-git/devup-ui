use super::*;
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("[[1,2,3],[4,5,6]][1]", true)]
#[case("([[1,2],[3,4]] as const)[(1)]", true)]
#[case("[false,null,'red',[]][2]", true)]
#[case("[[1,2],,[3,4]][0]", false)]
#[case("[[1,,2],[3,4]][1]", false)]
#[case("[[1,2]][1]", false)]
#[case("[][0]", false)]
#[case("[[1,2]][0.5]", false)]
#[case("[[1,2]][-1]", false)]
#[case("[[1,2]][1e999]", false)]
#[case("[[1,2]][4294967296]", false)]
#[case("[[1,2]]['0']", false)]
#[case("[[1,2]][`0${read()}`]", false)]
#[case("[[1,2]][{toString(){return '0'}}]", false)]
#[case("[[1,2]][index]", false)]
#[case("[[1,2]]?.[0]", false)]
#[case("[[1,2],...items][0]", false)]
#[case("[[1,2],[...items]][0]", false)]
#[case("[[1,2],{get value(){return 3}}][0]", false)]
#[case("({get 0(){return [1,2]}})[0]", false)]
#[case("[[1,2],read()][0]", false)]
#[case("[[1,2],value][0]", false)]
#[case("[[1,2],state.value][0]", false)]
#[case("[[1,2],`${value}`][0]", false)]
#[case("[[1,2],-1][0]", false)]
#[case("[[1,2],1n][0]", false)]
#[case("[[1,2],1n/0n][0]", false)]
#[case("([[1,2]] satisfies number[][])[0]", false)]
fn literal_boundary_when_own_position_and_entire_table_are_proven(
    #[case] source: &str,
    #[case] eligible: bool,
) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    let expression = match &statement.expression {
        Expression::ChainExpression(chain) => match &chain.expression {
            oxc_ast::ast::ChainElement::ComputedMemberExpression(member) => member,
            _ => panic!("computed chain required"),
        },
        Expression::ComputedMemberExpression(member) => member,
        _ => panic!("computed member required"),
    };
    assert_eq!(literal_own_selection(expression), eligible, "{source}");
}

#[test]
fn literal_boundary_when_handled_preserves_ast_and_allocates_no_choice() {
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, "[[1,2,3],[4,5,6]][1]", SourceType::tsx()).parse();
    let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
        panic!("expression required")
    };
    let original = crate::utils::readable_code(&statement.expression);
    let Expression::ComputedMemberExpression(member) = &mut statement.expression else {
        panic!("computed member required")
    };
    let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    let mut captured = Vec::new();
    assert!(visitor.capture_selection(member, &mut captured));
    assert_eq!(captured.len(), 0);
    assert_eq!(crate::utils::readable_code(&statement.expression), original);
    assert_eq!(visitor.names.fresh("__devupChoice"), "__devupChoice0");
}

#[test]
fn literal_boundary_when_table_is_deep_uses_an_iterative_proof() {
    let allocator = Allocator::default();
    let builder = oxc_ast::builder::AstBuilder::new(&allocator);
    let mut table = Expression::new_null_literal(SPAN, &builder);
    for _ in 0..4096 {
        table = Expression::new_array_expression(
            SPAN,
            oxc_allocator::Vec::from_array_in([table.into()], &builder),
            &builder,
        );
    }
    let key = Expression::new_numeric_literal(
        SPAN,
        0.0,
        None,
        oxc_syntax::number::NumberBase::Decimal,
        &builder,
    );
    let member = oxc_ast::ast::ComputedMemberExpression::new(SPAN, table, key, false, &builder);
    assert!(literal_own_selection(&member));
}
