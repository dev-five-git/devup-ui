use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::{Inlined, inline_constants};
use crate::ExtractOption;
use crate::css_prop::CssProp;

fn inline(code: &str) -> (String, Inlined) {
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, code, SourceType::tsx())
        .parse()
        .program;
    let result = inline_constants(
        &AstBuilder::new(&allocator),
        &mut program,
        "tdz.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
    );
    (Codegen::new().build(&program).code, result)
}

#[test]
fn immediate_style_reads_when_lexical_bindings_are_uninitialized() {
    for body in [
        "const view = <Box color={color} />; const color = 'red';",
        "const view = <Box color={color} />; let color = 'red';",
        "{ const view = <Box color={color} />; const color = 'red'; }",
        "function render() { const view = <Box color={color} />; const color = 'red'; return view; }",
        "const render = () => { const view = <Box color={color} />; let color = 'red'; return view; };",
        "const view = <Box color={color} />; class color {}",
        "const color = <Box color={color} />;",
        "const color = Math.max(color, 1); const view = <Box zIndex={color} />;",
        "const color = later; const view = <Box color={color} />; const later = 'red';",
        "const view = (() => <Box color={color} />)(); const color = 'red';",
        "const view = (function () { return <Box color={color} />; })(); const color = 'red';",
        "function render() { const color = later; const later = color; return <Box color={color} />; }",
    ] {
        let source = format!("import {{ Box }} from '@devup-ui/react';\n{body}");
        let (_, result) = inline(&source);
        assert_eq!(result.errors.len(), 1, "{source}");
        let (at, message) = &result.errors[0];
        let name = if body.contains("later") {
            "later"
        } else {
            "color"
        };
        assert_eq!(
            &source[usize::try_from(*at).unwrap_or_else(|error| panic!("try_from at: {error}"))..]
                [..name.len()],
            name
        );
        assert!(message.contains("ReferenceError"), "{message}");
        let located = crate::located_errors("tdz.tsx", &source, &[], result.errors);
        assert!(located.starts_with("tdz.tsx:2:"), "{located}");
    }
}

#[test]
fn deferred_style_reads_when_outer_bindings_initialize_before_invocation() {
    for body in [
        "function Render() { return <Box color={color} />; } const color = 'red';",
        "const Render = () => <Box color={color} />; const color = 'red';",
        "register(() => <Box color={color} />); const color = 'red';",
        "function outer() { const Render = () => <Box color={color} />; const color = 'red'; return Render; }",
        "{ const Render = () => <Box color={color} />; const color = 'red'; }",
    ] {
        let (code, result) = inline(&format!("import {{ Box }} from '@devup-ui/react';\n{body}"));
        assert_eq!(result.errors, vec![], "{body}");
        assert!(code.contains("color={\"red\"}"), "{code}");
    }
}

#[test]
fn var_reads_when_initializers_have_not_run_stay_dynamic() {
    let source = "import { Box } from '@devup-ui/react'; const view = <Box color={color} />; var color = 'red';";
    let (code, result) = inline(source);
    assert_eq!(result.errors, vec![]);
    assert!(code.contains("color={color}"), "{code}");
}

#[test]
#[serial_test::serial]
fn var_reads_when_rendered_before_assignment_use_css_variables() {
    let output = super::exact_tests::extracted("import { Box } from '@devup-ui/react'; const view = <Box color={color} />; var color = 'red';", "").unwrap_or_else(|error| panic!("var_reads_when_rendered_before_assignment_use_css_variables: {error}"));
    assert!(output.code.contains("--"), "{}", output.code);
}

#[test]
fn style_reads_when_shadowed_bindings_have_distinct_initialization() {
    let source = "import { Box } from '@devup-ui/react'; const color = 'red'; function outer() { const view = <Box color={color} />; const color = 'green'; return view; }";
    let (_, result) = inline(source);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(
        &source[usize::try_from(result.errors[0].0)
            .unwrap_or_else(|error| panic!("try_from error offset: {error}"))..][..5],
        "color"
    );
}

#[test]
fn nonstyle_reads_when_preinitialization_is_not_a_style_dependency_are_untouched() {
    let (code, result) = inline(
        "import { Box } from '@devup-ui/react'; observe(color); const color = 'red'; const view = <Box color={color} />;",
    );
    assert_eq!(result.errors, vec![]);
    assert!(code.contains("observe(color)"), "{code}");
    assert!(code.contains("color={\"red\"}"), "{code}");
}

#[test]
#[serial_test::serial]
fn class_self_reads_when_static_field_initialization_has_started() {
    for source in [
        "import { Box } from '@devup-ui/react'; export class Palette { static tone = 'red'; static view = <Box color={Palette.tone} />; }",
        "import { Box } from '@devup-ui/react'; class Palette { static tone = 'red'; static { const view = <Box color={Palette.tone} />; } }",
    ] {
        let output = super::exact_tests::extracted(source, "").unwrap_or_else(|error| {
            panic!("class_self_reads_when_static_field_initialization_has_started: {error}")
        });
        assert!(output.code.contains("Palette.tone"), "{}", output.code);
        assert!(output.code.contains("className="), "{}", output.code);
    }
}

#[test]
fn class_self_reads_when_method_execution_is_deferred() {
    let source = "import { Box } from '@devup-ui/react'; class Palette { view() { return <Box color={Palette.tone} />; } static tone = 'red'; }";
    let (_, result) = inline(source);
    assert_eq!(result.errors, vec![]);
}

#[test]
fn class_self_reads_when_class_binding_is_still_uninitialized() {
    for body in [
        "const view = <Box color={Palette.tone} />; class Palette { static tone = 'red'; }",
        "class Palette extends (<Box color={Palette.tone} />) { static tone = 'red'; }",
        "class Palette { static tone = 'red'; static [<Box color={Palette.tone} />] = 1; }",
    ] {
        let source = format!("import {{ Box }} from '@devup-ui/react';\n{body}");
        let (_, result) = inline(&source);
        assert_eq!(result.errors.len(), 1, "{source}");
        let (at, message) = &result.errors[0];
        assert_eq!(
            &source[usize::try_from(*at)
                .unwrap_or_else(|error| panic!("try_from class read offset: {error}"))..][..7],
            "Palette"
        );
        assert!(message.contains("ReferenceError"), "{message}");
        let located = crate::located_errors("tdz.tsx", &source, &[], result.errors);
        assert!(located.starts_with("tdz.tsx:2:"), "{located}");
    }
}

#[test]
fn lexical_reads_when_static_initialization_precedes_their_declaration() {
    for body in [
        "class Palette { static view = <Box color={tone} />; } const tone = 'red';",
        "class Palette { static { const view = <Box color={tone} />; const tone = 'red'; } }",
    ] {
        let source = format!("import {{ Box }} from '@devup-ui/react'; {body}");
        let (_, result) = inline(&source);
        assert_eq!(result.errors.len(), 1, "{source}");
        assert!(result.errors[0].1.contains("ReferenceError"));
    }
}
