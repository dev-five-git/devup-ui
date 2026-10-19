use crate::{ExtractOption, extract, extract_without_source_map};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("#__PURE__", "bootstrap();")]
#[case("@__NO_SIDE_EFFECTS__", "function bootstrap() {}")]
#[serial]
fn annotated_header_keeps_its_jsx_instruction_and_node_semantics(
    #[case] annotation: &str,
    #[case] statement: &str,
    #[values(true, false)] source_map: bool,
) {
    let source = format!(
        "/* {annotation}\n * @jsx h\n */\n{statement}\nimport {{css}} from '@devup-ui/react';\nexport const cls=css({{width:'401px'}});\nexport const anchor=1;"
    );
    reset_class_map();
    reset_file_map();

    let output = if source_map {
        extract("annotation.tsx", &source, ExtractOption::default())
    } else {
        extract_without_source_map("annotation.tsx", &source, ExtractOption::default())
    }
    .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(output.code.matches("@jsx h").count(), 1, "{}", output.code);
    let first_import = output
        .code
        .find("import ")
        .unwrap_or_else(|| panic!("missing stylesheet import"));
    assert!(
        output.code[..first_import].contains("@jsx h"),
        "{}",
        output.code
    );
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let annotation_survives = parsed.program.body.iter().any(|statement| match statement {
        Statement::ExpressionStatement(statement) => {
            matches!(&statement.expression, Expression::CallExpression(call) if call.pure)
        }
        Statement::FunctionDeclaration(function) => function.pure,
        _ => false,
    });
    assert!(annotation_survives, "{}", output.code);
    if source_map {
        crate::pragma_path_tests::assert_anchor_map(&source, &output);
    } else {
        assert_eq!(output.map, None);
    }
}
