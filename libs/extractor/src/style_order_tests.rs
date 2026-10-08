use std::error::Error;

use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serial_test::serial;

use crate::style_order::static_order;
use crate::utils::ParsedStyleOrder;
use crate::{ExtractOption, extract};

const FIXTURES: &str = include_str!("../../../test-fixtures/style-order.json");
const IMPORTS: &str = "import { Box, css, styled, keyframes } from '@devup-ui/react';\nimport { jsx as render } from 'react/jsx-runtime';\n";

fn expected_location(source: &str, expression: &str) -> Result<String, Box<dyn Error>> {
    let leaf = expression
        .rsplit_once(" : ")
        .or_else(|| expression.rsplit_once(" && "))
        .map_or(expression, |(_, leaf)| leaf);
    let root = source
        .rfind(expression)
        .ok_or("missing fixture expression")?;
    let relative = expression.rfind(leaf).ok_or("missing fixture branch")?;
    let before = source
        .get(..root + relative)
        .ok_or("invalid fixture offset")?;
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .ok_or("missing fixture line")?
        .chars()
        .count()
        + 1;
    Ok(format!("order.tsx:{line}:{column}:"))
}

fn check_case(source: &str, expression: &str, valid: bool) -> Result<(), Box<dyn Error>> {
    reset_class_map();
    reset_file_map();
    let output = extract("order.tsx", source, ExtractOption::default());
    if valid {
        assert!(output.is_ok(), "{expression}: {output:?}");
    } else {
        let error = output
            .err()
            .ok_or_else(|| format!("invalid styleOrder unexpectedly compiled: {source}"))?
            .to_string();
        let expected = expected_location(source, expression)?;
        assert!(
            error.lines().any(|line| line.starts_with(&expected)),
            "{expected}\n{error}"
        );
        assert!(error.contains("`styleOrder()` cannot use"), "{error}");
        assert!(error.contains("an integer from 1 to 254"), "{error}");
        assert!(error.contains("use a numeric literal"), "{error}");
    }
    Ok(())
}

#[test]
#[serial]
fn consumed_orders_follow_shared_lint_fixtures() -> Result<(), Box<dyn Error>> {
    let fixtures: serde_json::Value = serde_json::from_str(FIXTURES)?;
    for fixture in fixtures
        .as_array()
        .ok_or("styleOrder fixtures must be an array")?
    {
        let expression = fixture["expression"].as_str().ok_or("missing expression")?;
        let valid = fixture["valid"].as_bool().ok_or("missing validity")?;
        for body in [
            format!("const view = <Box bg='red' styleOrder={{{expression}}} />;"),
            format!("const view = render(Box, {{ bg: 'red', styleOrder: {expression} }});"),
            format!("const rule = css({{ color: 'red', styleOrder: {expression} }});"),
            format!("const Card = styled('div', {{ color: 'red', styleOrder: {expression} }});"),
            format!(
                "const fade = keyframes({{ from: {{ opacity: 0, styleOrder: {expression} }} }});"
            ),
        ] {
            let source = format!("{IMPORTS}{body}");
            check_case(&source, expression, valid)?;
        }
        if expression.starts_with('\'') {
            let source = format!("{IMPORTS}const view = <Box bg='red' styleOrder={expression} />;");
            check_case(&source, expression, valid)?;
        }
    }
    Ok(())
}

#[test]
fn scalar_orders_distinguish_number_values_and_cooked_strings() -> Result<(), Box<dyn Error>> {
    let allocator = Allocator::default();
    for (source, expected) in [
        ("1", Some(1)),
        ("254", Some(254)),
        ("'1'", Some(1)),
        ("'254'", Some(254)),
        ("100.0", Some(100)),
        ("1e2", Some(100)),
        ("+100", Some(100)),
        ("-(-1)", Some(1)),
        ("+\"1\"", Some(1)),
        ("+\"01\"", Some(1)),
        ("-\"-1\"", Some(1)),
        ("+`01`", Some(1)),
        ("+`\\u0031`", Some(1)),
        ("+(\"1\" as const)", Some(1)),
        ("+\"1.5\"", None),
        ("+\"255\"", None),
        ("+\"nope\"", None),
        ("!1", None),
        ("~1", None),
        ("typeof 1", None),
        ("'100.0'", None),
        ("'1e2'", None),
        ("'01'", None),
        ("`\\u0031`", Some(1)),
        ("`\\u0031\\u0030`", Some(10)),
        ("`\\u0030\\u0031`", None),
        ("`\\u0020\\u0031`", None),
        ("0", None),
        ("255", None),
        ("1.5", None),
        ("-1", None),
        ("1e999", None),
        ("`1${runtime}`", None),
        ("+`1${runtime}`", None),
        ("-(+`1${runtime}`)", None),
        ("+runtime", None),
    ] {
        let expression = Parser::new(&allocator, source, SourceType::ts())
            .parse_expression()
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(static_order(&expression), expected, "{source}");
    }
    Ok(())
}

#[test]
fn conditional_orders_keep_the_implicit_false_branch_absent() -> Result<(), Box<dyn Error>> {
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, "on && 2", SourceType::ts())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    let order = crate::style_order::expression_order(&expression, &allocator);
    let ParsedStyleOrder::Conditional {
        consequent,
        alternate,
        ..
    } = order
    else {
        panic!("logical styleOrder must retain its condition");
    };
    assert_eq!((consequent, alternate), (Some(2), None));
    Ok(())
}

#[test]
#[serial]
fn order_validation_keeps_pass_through_data_and_runtime_authority_unchanged() {
    let source = format!("{IMPORTS}
const data = {{ styleOrder: '100px' }};
const native = <div styleOrder='100px' />;
const view = <Box data-values={{data}} props={{data}} styleVars={{{{ '--order': '100px' }}}} bg='red' />;
const styledData = styled.div.attrs({{ styleOrder: '100px' }})({{ color: 'red' }});
export const Runtime = (order) => <Box styleOrder={{order}} bg='red' />;
export const Shadowed = (undefined) => <Box styleOrder={{undefined}} bg='red' />;");
    reset_class_map();
    reset_file_map();
    assert!(extract("order.tsx", &source, ExtractOption::default()).is_ok());
}

#[test]
#[serial]
fn invalid_order_reports_the_written_branch_and_value_location() -> Result<(), Box<dyn Error>> {
    let expression = "on ? 1 : '100px'";
    let source = format!("{IMPORTS}const view = <Box styleOrder={{{expression}}} bg='red' />;");
    reset_class_map();
    reset_file_map();
    let error = extract("order.tsx", &source, ExtractOption::default())
        .err()
        .ok_or("invalid branch unexpectedly compiled")?
        .to_string();
    let expected = expected_location(&source, expression)?;
    assert!(error.contains("`'100px'`"), "{error}");
    assert!(
        error.lines().any(|line| line.starts_with(&expected)),
        "{error}"
    );
    Ok(())
}

#[test]
fn style_order_validation_checks_only_consumed_roots() -> Result<(), Box<dyn Error>> {
    let allocator = Allocator::default();
    let bindings = crate::style_values::StyleValues::default();
    for (source, count) in [
        ("({styleOrder: 1n})", 1),
        ("({styleOrder: /x/})", 1),
        ("({styleOrder: []})", 1),
        ("({styleOrder: {}})", 1),
        ("({styleOrder: class {}})", 1),
        ("({styleOrder: () => 1})", 1),
        ("({styleOrder: function() {}})", 1),
        ("({styleOrder: NaN})", 1),
        ("({styleOrder: Infinity})", 1),
        ("({styleOrder: void effect()})", 0),
        ("({styleOrder: !runtime})", 0),
        ("({styleOrder: ~runtime})", 0),
        ("({styleOrder: typeof runtime})", 0),
        ("({styleOrder: +`1${runtime}`})", 0),
        ("({styleOrder: +\"nope\"})", 1),
        ("({styleOrder: !1})", 1),
        ("({styleOrder: ~1})", 1),
        ("({styleOrder: typeof 1})", 1),
        ("({styleOrder: !!1})", 1),
        ("({styleOrder: delete 1})", 1),
        ("({styleOrder: `1${runtime}`})", 0),
        ("({...{styleOrder: 0}})", 1),
        ("[{styleOrder: 0}, , ...runtime]", 1),
        ("on ? {styleOrder: 0} : {styleOrder: 255}", 2),
        ("on && {styleOrder: 0}", 1),
        ("runtime", 0),
        ("({color: {styleOrder: 0}})", 0),
    ] {
        let value = Parser::new(&allocator, source, SourceType::tsx())
            .parse_expression()
            .map_err(|error| format!("{error:?}"))?;
        let mut errors = Vec::new();
        crate::style_order_validation::validate_rules(&value, &bindings, &mut errors);
        assert_eq!(errors.len(), count, "{source}: {errors:?}");
    }
    Ok(())
}

#[test]
#[serial]
fn style_order_jsx_shapes_report_written_values() -> Result<(), Box<dyn Error>> {
    for (attribute, value) in [
        ("styleOrder", "styleOrder"),
        ("styleOrder=<i />", "<i />"),
        ("styleOrder=<></>", "<></>"),
        ("styleOrder={<i />}", "<i />"),
        ("styleOrder={<></>}", "<></>"),
        ("styleOrder={on ? '01' : 2}", "'01'"),
        ("styleOrder={!1}", "!1"),
        ("styleOrder={~1}", "~1"),
        ("styleOrder={typeof 1}", "typeof 1"),
        ("styleOrder={on ? 2 : '\\u0030\\u0031'}", "'\\u0030\\u0031'"),
    ] {
        let source = format!("{IMPORTS}const view = <Box {attribute} bg='red' />;");
        reset_class_map();
        reset_file_map();
        let error = extract("order.tsx", &source, ExtractOption::default())
            .err()
            .ok_or("invalid JSX order compiled")?
            .to_string();
        assert!(error.contains(&format!("`{value}`")), "{error}");
        let expected = expected_location(&source, value)?;
        assert!(
            error.lines().any(|line| line.starts_with(&expected)),
            "{error}"
        );
    }
    Ok(())
}
