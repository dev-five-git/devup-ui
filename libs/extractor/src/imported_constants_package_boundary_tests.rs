use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::{ModuleScope, Modules};
use crate::ExtractOption;

#[rstest]
#[case("@devup-ui/react-values", false)]
#[case("@devup-ui/react", true)]
#[case("@devup-ui/react/styles", true)]
#[case("@devup-ui/react/compat", true)]
fn package_boundary_css_styles_only_reads_api_exports(
    #[case] source: &str,
    #[case] expected: bool,
) {
    // Given
    let allocator = Allocator::default();
    let code = format!("import {{css}} from '{source}';css({{color:'blue'}});");
    let program = Parser::new(&allocator, &code, SourceType::tsx())
        .parse()
        .program;
    let mut scope = ModuleScope::new("/helper.ts", &program, None);
    let Statement::ImportDeclaration(import) = &program.body[0] else {
        panic!("import fixture");
    };
    scope.import(import);
    let Statement::ExpressionStatement(statement) = &program.body[1] else {
        panic!("call fixture");
    };
    let Expression::CallExpression(call) = &statement.expression else {
        panic!("call fixture");
    };
    let option = ExtractOption::default();
    let mut modules = Modules {
        resolver: None,
        option: &option,
        exports: Default::default(),
        loading: vec![],
    };
    // When
    let styles = scope.css_styles(&mut modules, call);
    // Then
    assert_eq!(styles.is_some(), expected);
    assert_eq!(scope.is_style_api(&modules, &call.callee), expected);
    assert_eq!(scope.is_style_import(&option, "css"), expected);
}
