use crate::visit::DevupVisitor;
use oxc_ast::ast::Statement;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("style-order:255;style-order:${p=>{return;}};", 1)]
#[case("style-order:${p=>255};style-order:${p=>{return;}};", 1)]
#[case("style-order:${p=>p.stop?2:255};style-order:${p=>{return;}};", 1)]
#[case("style-order:${false?255:2};style-order:${p=>{return;}};", 1)]
#[case("style-order:${false&&255};style-order:${p=>{return;}};", 1)]
#[case("style-order:${p=>{return;}};style-order:255;", 1)]
#[case("style-order:${p=>{return;}};&:hover{style-order:255;color:blue}", 1)]
#[case("style-order:${p=>{return;return 255;}};", 2)]
#[serial]
fn invalid_candidates_when_absence_normalizes_metadata_remain_definitive(
    #[case] body: &str,
    #[case] diagnostic_count: usize,
) {
    // Given: actual template suppliers contain invalid removed, later or inner metadata.
    let source = format!("`{body}color:red`;");
    let allocator = oxc_allocator::Allocator::default();
    let mut parsed =
        oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
        panic!("literal fixture")
    };
    let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    let value = crate::utils::unwrap_syntax_only_mut(&mut statement.expression);
    assert!(crate::css_utils::literal_tree::lower(
        &visitor.ast,
        value,
        crate::css_utils::literal_tree::Scope {
            source: Some(&source),
            global: false,
        }
    ));
    let offset = u32::try_from(
        source
            .find("255")
            .unwrap_or_else(|| panic!("invalid token")),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let mut captures = Vec::new();
    let mut renders = Vec::new();
    // When: the actual consumer normalizes and its caller validates retained metadata.
    assert!(visitor.capture_literal_styled(value, &mut captures, &mut renders));
    visitor.check_style_orders(value, false);
    // Then: unreachable/unselected 255 never becomes omission or evaluation retry.
    assert_eq!(
        visitor.error_disposition,
        crate::ErrorDisposition::Definitive
    );
    assert_eq!(
        visitor.errors,
        vec![(offset, crate::style_order::invalid_order("255")); diagnostic_count]
    );
}
