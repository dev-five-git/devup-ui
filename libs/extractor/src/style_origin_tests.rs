use super::style_origin::{OriginScope, at};
use css::style_origin::Origin;
use oxc_allocator::Allocator;
use oxc_ast_visit::VisitMut;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[test]
fn own_edit_ranges_keep_the_actual_replaced_expression_not_generated_text() {
    // Given
    let source = "const x = css(helper());";
    let computed = "const x = css({color:'red'});";
    let edits = [(14, 22, 13)];
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, computed, SourceType::tsx())
        .parse()
        .program;
    let _origins = OriginScope::enter(("real.tsx", source), &[&edits], &program);
    // When
    let origin = at(21)
        .0
        .unwrap_or_else(|| panic!("missing literal witness"));
    // Then
    assert_eq!(origin.file, "real.tsx");
    assert_eq!((origin.line, origin.column), (1, 15));
    assert_eq!(origin.expression, "helper()");
}

#[test]
fn risky_marks_and_inlined_children_keep_the_original_read_site() {
    // Given
    let source = concat!(
        "const token = 'red';\nconst x = css({",
        "color:token",
        "});"
    );
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, source, SourceType::tsx())
        .parse()
        .program;
    let _origins = OriginScope::enter(("source.tsx", source), &[], &program);
    let start = u32::try_from(source.rfind("token").unwrap_or_default()).unwrap_or_default();
    super::provenance::MarkRanges(&[(usize::try_from(start).unwrap_or_default(), source.len())])
        .visit_program(&mut program);
    // When
    let origin = at(start | (1 << 31))
        .0
        .unwrap_or_else(|| panic!("missing original witness"));
    // Then
    assert_eq!(
        (
            origin.file.as_str(),
            origin.line,
            origin.column,
            origin.expression.as_str()
        ),
        ("source.tsx", 2, 22, "token")
    );
}

#[test]
fn diagnostic_witnesses_never_change_style_semantic_equality() {
    // Given
    let mut first = super::extract_style::extract_static_style::ExtractStaticStyle::new(
        "color", "red", 0, None,
    );
    let mut second = first.clone();
    first.origin = Origin(
        Some(Box::new(css::style_origin::StyleOrigin {
            file: "a.tsx".into(),
            line: 3,
            column: 7,
            expression: "'red'".into(),
        })),
        None,
    );
    second.origin = Origin(
        Some(Box::new(css::style_origin::StyleOrigin {
            file: "b.tsx".into(),
            line: 9,
            column: 2,
            expression: "token".into(),
        })),
        None,
    );
    // When
    let distinct = std::collections::HashSet::from([first.clone(), second.clone()]);
    // Then
    assert_eq!(first, second);
    assert_eq!(first.cmp(&second), std::cmp::Ordering::Equal);
    assert_eq!(distinct.len(), 1);
}

#[test]
fn generated_style_markers_admit_only_the_recorded_factory_witness() {
    // Given
    let origin = css::style_origin::StyleOrigin {
        file: "actual.css.ts".into(),
        line: 8,
        column: 4,
        expression: "style(makeRule())".into(),
    };
    let generated = super::evaluation_origin::mark_location(
        "css({color:'red'})".into(),
        Some(&css::style_origin::RealLocation::Exact(origin.clone())),
    );
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, &generated, SourceType::tsx())
        .parse()
        .program;
    // When
    let _origins = OriginScope::generated(&mut program);
    let start = u32::try_from(generated.find("'red'").unwrap_or_default()).unwrap_or_default();
    // Then
    assert_eq!(at(start).0.as_deref(), Some(&origin));
    assert!(
        !oxc_codegen::Codegen::new()
            .build(&program)
            .code
            .contains(super::style_origin::MARKER)
    );
}

#[test]
fn synthetic_zero_spans_do_not_steal_a_real_directive_at_source_zero() {
    // Given
    let source = concat!("'use client'; const x=css({", "color:token", "});");
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, source, SourceType::tsx())
        .parse()
        .program;
    let _origins = OriginScope::enter(("actual.tsx", source), &[], &program);
    let start = u32::try_from(source.find("token").unwrap_or_default()).unwrap_or_default();
    let _parent = super::style_origin::CurrentOrigin::enter(oxc_span::Span::new(start, start + 5));
    // When
    let _synthetic = super::style_origin::CurrentOrigin::enter(oxc_span::SPAN);
    let style = super::extract_style::extract_static_style::ExtractStaticStyle::new(
        "color", "red", 0, None,
    );
    // Then
    assert_eq!(
        style.origin.0.map(|origin| origin.expression),
        Some("token".into())
    );
}
