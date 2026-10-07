use super::*;

#[rstest::rstest]
#[case("css({styleOrder:flag?left():right(),color:'red'})")]
#[case("jsx(Box,{styleOrder:flag?left():right(),color:'red'})")]
#[serial]
fn unresolved_order_arms_when_tied_report_first_authored_leaf(#[case] usage: &str) {
    // Given: unresolved leaves in the real CSS and factory entrypoints.
    let source = format!(
        "{BOX}{JSX_RUNTIME}import {{css}} from '@devup-ui/react';\nconst render=(flag,left,right)=>{usage};"
    );
    let line = source.lines().last().required("metadata line");
    let metadata = line.find("styleOrder:").required("metadata occurrence");
    let column = metadata + line[metadata..].find("left()").required("left leaf") + 1;
    // When: extraction rejects both leaves without computable bindings.
    let actual = error(&source);
    // Then: the first diagnostic is authored, never a synthetic capture.
    assert!(
        actual.starts_with(&format!("a.tsx:4:{column}:")),
        "{actual}"
    );
    assert!(!actual.contains("__devupBranch"), "{actual}");
    assert!(!actual.contains("a.tsx:1:1:"), "{actual}");
}

#[test]
#[serial]
fn global_order_when_controller_is_pure_computes_only_controller_leaves() {
    // Given: genuine exact controllers with conditional and absence structure.
    for (value, expected) in [
        ("true?2:3", Some(2)),
        ("false?2:3", Some(3)),
        ("true&&2", Some(2)),
        ("false&&2", Some(0)),
    ] {
        let value = value.replace("true", "guard()").replace("false", "stop()");
        let source = format!(
            "import {{globalCss}} from '@devup-ui/react';const guard=()=>true;const stop=()=>false;globalCss({{body:{{styleOrder:{value},color:'red'}}}});"
        );
        // When: the public evaluator retries the static global metadata.
        let actual = orders(&source);
        // Then: the controller chooses a static layer or absence.
        assert_eq!(actual, vec![("color".to_string(), 0, expected)], "{source}");
    }
}

#[rstest::rstest]
#[case("tag`style-order:${flag&&2};color:red`;", false)]
#[case(
    "tag`&:hover{style-order:/*before*/${flag&&2}/*after*/;color:red}`;",
    false
)]
#[case(r"tag`style-order:${flag&&2};content:\unicode`;", true)]
fn template_locator_when_parsed_preserves_raw_text_and_hole_spans(
    #[case] source: &str,
    #[case] invalid_escape: bool,
) {
    // Given: real tagged templates, including a valid tag with cooked None.
    use oxc_span::GetSpan;
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let oxc_ast::ast::Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("fixture must be an expression statement");
    };
    let oxc_ast::ast::Expression::TaggedTemplateExpression(tagged) = &statement.expression else {
        panic!("fixture must be a tagged template");
    };
    assert_eq!(
        tagged
            .quasi
            .quasis
            .last()
            .required("last quasi")
            .value
            .cooked
            .is_none(),
        invalid_escape
    );
    let ast = oxc_ast::builder::AstBuilder::new(&allocator);
    // When: the infallible raw constructor and metadata locator consume the template.
    let text = crate::css_utils::literal::CssText::from_template(&ast, &tagged.quasi, Some(source));
    let holes = crate::imported_constants::order_metadata::holes(&ast, &tagged.quasi);
    // Then: only the authored metadata hole is located, with its original span.
    assert_eq!(holes, vec![tagged.quasi.expressions[0].span()]);
    assert_eq!(text.holes[0].1.span(), holes[0]);
    assert_eq!(text.offset(text.holes[0].0.start), holes[0].start);
    assert!(text.text.contains("__DEVUP_HOLE_0__"));
    assert_eq!(text.text.contains(r"\unicode"), invalid_escape);
}

#[test]
#[serial]
fn global_order_when_pure_controller_has_invalid_alternate_still_rejects_it() {
    // Given: a pure true controller cannot excuse an invalid alternate.
    let source = "import {globalCss} from '@devup-ui/react';\nconst guard=()=>true;globalCss({body:{styleOrder:guard()?2:'01',color:'red'}});";
    let column = source
        .lines()
        .nth(1)
        .required("metadata line")
        .find("'01'")
        .required("invalid arm")
        + 1;
    // When: metadata validation precedes any controller selection.
    let actual = error(source);
    // Then: the invalid authored arm remains a located error.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
}

#[test]
fn template_constructor_when_wrapped_preserves_outer_fallback_origin() {
    // Given: an authored TypeScript wrapper around real template text.
    use oxc_span::GetSpan;
    let source = "(`style-order:${flag&&2};color:red` as string);";
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let oxc_ast::ast::Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("fixture must be an expression statement");
    };
    let ast = oxc_ast::builder::AstBuilder::new(&allocator);
    // When: generic kind dispatch uses the shared typed template constructor.
    let text =
        crate::css_utils::literal::CssText::with_source(&ast, &statement.expression, Some(source))
            .required("template kind must be supported");
    // Then: source offsets, hole spans and the outer fallback origin survive.
    assert_eq!(
        usize::try_from(text.offset(text.text.find("color").required("color token")))
            .required("offset fits"),
        source.find("color").required("authored color")
    );
    assert_eq!(
        text.offset(text.text.len()),
        statement.expression.span().start
    );
    assert_eq!(
        usize::try_from(text.holes[0].1.span().start).required("hole offset fits"),
        source.find("flag&&2").required("authored hole")
    );
}
