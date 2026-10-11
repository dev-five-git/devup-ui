use super::*;
use crate::extractor::extract_style_from_expression::{
    LiteralHandling, extract_style_from_expression,
};
use oxc_allocator::{Allocator, GetAllocator};
use oxc_ast::{ast::Statement, builder::AstBuilder};
use oxc_parser::Parser;
use oxc_span::SourceType;

fn parsed_props<'a>(ast: &AstBuilder<'a>, source: &'a str) -> Vec<ExtractStyleProp<'a>> {
    let mut parsed = Parser::new(ast.allocator(), source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
        panic!("rule expression required")
    };
    extract_style_from_expression(
        ast,
        None,
        &mut statement.expression,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
    )
    .styles
}

#[test]
#[serial]
fn overlap_when_parsed_responsive_or_keyed_rules_share_a_cascade_key_is_detected() {
    for source in [
        "({color:['red','blue']});",
        "({color:{a:'red',b:'blue'}[key]});",
        "({positioning:position});",
        "(flag?{positioning:position}:{color:'red'});",
    ] {
        let allocator = Allocator::default();
        let ast = AstBuilder::new(&allocator);
        let earlier = parsed_props(&ast, source);
        let matching = parsed_props(&ast, "({color:'black',top:0});");
        let unrelated = parsed_props(&ast, "({backgroundColor:'white'});");
        assert!(
            crate::composition::overlaps(&earlier, &matching),
            "{source}"
        );
        assert!(
            !crate::composition::overlaps(&earlier, &unrelated),
            "{source}"
        );
    }
}

#[test]
#[serial]
fn overlap_when_only_selector_breakpoint_or_order_differs_is_absent() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let earlier = parsed_props(&ast, "({styleOrder:2,color:'red'});");
    for source in [
        "({styleOrder:3,color:'blue'});",
        "({styleOrder:2,_hover:{color:'blue'}});",
        "({styleOrder:2,color:[null,'blue']});",
    ] {
        let later = parsed_props(&ast, source);
        assert!(!crate::composition::overlaps(&earlier, &later), "{source}");
    }
}

#[test]
#[serial]
fn conditional_when_dynamic_typography_has_no_key_preserves_the_unkeyed_choice() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut composed = crate::composition::Composition::default();
    composed.apply(&ast, parsed_props(&ast, "({color:'red'});"));
    composed.apply(
        &ast,
        parsed_props(&ast, "(flag?{typography:preset}:{color:'blue'});"),
    );
    assert!(composed.unconditional().is_none());
    let styles = composed.into_props();
    assert_eq!(styles.len(), 2);
    assert!(matches!(&styles[1], ExtractStyleProp::Conditional { .. }));
}

#[test]
#[serial]
fn conditional_when_enum_is_keyed_keeps_the_selected_later_winner() {
    let source = "import {Box} from '@devup-ui/react';const key='a';const a=<Box color={{a:'red',b:'blue'}[key]} />;";
    let actual = whole::evaluate(source, "a.props.className");
    assert_eq!(actual.element, "color-0-red--255-a");
}

#[test]
#[serial]
fn parsed_enum_when_nested_in_a_condition_competes_per_key() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut composed = crate::composition::Composition::default();
    composed.apply(&ast, parsed_props(&ast, "({top:'7px'});"));
    composed.apply(
        &ast,
        parsed_props(&ast, "(flag?{positioning:position}:{top:'9px'});"),
    );
    let mut props = composed.into_props();
    reset_class_map();
    css::debug::set_debug(true);
    let classes = crate::gen_class_name::gen_class_names(&ast, &mut props, None, None)
        .required("parsed finite enum must emit classes");
    css::debug::set_debug(false);
    let generated = crate::utils::expression_to_code(&classes);
    for (flag, position, expected) in [
        (true, "top", "top-0-0--255"),
        (true, "missing", "top-0-7px--255"),
        (false, "top", "top-0-9px--255"),
    ] {
        let actual = whole::evaluate_code(
            &format!("const flag={flag};const position='{position}';const a={generated};"),
            "a.trim()",
        );
        assert_eq!(actual.element, expected);
    }
}

#[test]
#[serial]
fn css_prop_when_runtime_styled_keys_overlap_is_rejected_but_distinct_keys_compile() {
    let prefix = format!(
        "{EMOTION}import {{styled}} from '@devup-ui/react'; const Card=styled.div({{styleOrder:2,_hover:{{color:['red','pink']}}}},p=>({{opacity:p.opacity}}));"
    );
    let source = format!(
        "{prefix}\nconst a=<Card as='section' css={{{{styleOrder:2,_hover:{{color:['blue','green']}}}}}}/>;"
    );
    let actual = compile_emotion(&source)
        .err()
        .required("overlap must be rejected");
    assert!(actual.starts_with("a.tsx:3:"), "{actual}");
    assert!(actual.contains("override"), "{actual}");
    for rules in [
        "{styleOrder:3,_hover:{color:'blue'}}",
        "{styleOrder:2,color:'blue'}",
        "{styleOrder:2,_hover:{backgroundColor:'blue'}}",
    ] {
        let source = format!("{prefix}const a=<Card as='section' css={{{rules}}}/>;");
        let actual = compile_emotion(&source);
        assert!(actual.is_ok(), "{rules}: {actual:?}");
    }
}
