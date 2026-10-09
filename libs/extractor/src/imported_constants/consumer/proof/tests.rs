use oxc_ast::{
    AstKind,
    ast::{IdentifierReference, Statement},
};
use rstest::rstest;

use super::super::tests::parsed;
use super::StyleSymbols;

pub(super) fn input_of(source: &str, name: &str) -> bool {
    let mut result = None;
    parsed(source, |program, semantic| {
        let style = StyleSymbols::from_program(
            program,
            semantic.scoping(),
            &crate::ExtractOption::default(),
        );
        let identifier = semantic
            .nodes()
            .iter()
            .filter_map(|node| match node.kind() {
                AstKind::IdentifierReference(identifier) if identifier.name == name => {
                    Some(identifier)
                }
                _ => None,
            })
            .max_by_key(|identifier| identifier.span.start)
            .unwrap_or_else(|| panic!("missing actual reference {name}"));
        result = Some(super::input(identifier, semantic, &style));
    });
    result.unwrap_or_else(|| panic!("parser callback did not classify input"))
}

#[rstest]
#[case("value", true)]
#[case("palette.fg", true)]
#[case("palette[key]", true)]
#[case("value+fixed", true)]
#[case("`${value}px`", true)]
#[case("value?fixed:3", true)]
#[case("value||fixed", true)]
#[case("take(value)", true)]
#[case("Math.max(value,2)", true)]
#[case("3", false)]
#[case("{color:value}", false)]
#[case("Math.max(1,2)", false)]
#[case("window.name", false)]
#[case("palette[window.name]", false)]
#[case("take(value,window.name)", false)]
fn closed_expression_when_inputs_are_proven_distinguishes_consumer_observations(
    #[case] expression: &str,
    #[case] expected: bool,
) {
    // Given
    let source =
        format!("import {{take,palette,key,value}} from './data';const fixed=3;({expression});");
    parsed(&source, |program, semantic| {
        let style = StyleSymbols::from_program(
            program,
            semantic.scoping(),
            &crate::ExtractOption::default(),
        );
        let Statement::ExpressionStatement(statement) = program
            .body
            .last()
            .unwrap_or_else(|| panic!("missing expression"))
        else {
            panic!("not an expression statement")
        };
        let expression = crate::utils::unwrap_syntax_only(&statement.expression);
        let known =
            |identifier: &IdentifierReference<'_>| super::input(identifier, semantic, &style);
        // When
        let actual = super::closed(expression, &style, &known);
        // Then
        assert_eq!(actual, expected, "{source}");
    });
}

#[rstest]
#[case("palette.fg", true)]
#[case("palette[key]", true)]
#[case("palette[unknown]", false)]
#[case("`${value}px`", true)]
#[case("`${unknown}px`", false)]
#[case("take(value)", true)]
#[case("take(unknown)", false)]
fn specialized_proof_when_member_template_or_call_is_read_checks_all_inputs(
    #[case] expression: &str,
    #[case] expected: bool,
) {
    // Given
    let source = format!("import {{take,palette,key,value}} from './data';{expression};");
    parsed(&source, |program, semantic| {
        let style = StyleSymbols::from_program(
            program,
            semantic.scoping(),
            &crate::ExtractOption::default(),
        );
        let Statement::ExpressionStatement(statement) = program
            .body
            .last()
            .unwrap_or_else(|| panic!("missing expression"))
        else {
            panic!("not an expression statement")
        };
        let known =
            |identifier: &IdentifierReference<'_>| super::input(identifier, semantic, &style);
        // When
        let actual = match &statement.expression {
            oxc_ast::ast::Expression::StaticMemberExpression(member) => {
                super::member((&member.object, None), &style, &known)
            }
            oxc_ast::ast::Expression::ComputedMemberExpression(member) => {
                super::member((&member.object, Some(&member.expression)), &style, &known)
            }
            oxc_ast::ast::Expression::TemplateLiteral(template) => {
                super::template(template, &style, &known)
            }
            oxc_ast::ast::Expression::CallExpression(call) => super::call(call, &style, &known),
            _ => panic!("fixture is not a specialized read"),
        };
        // Then
        assert_eq!(actual, expected, "{source}");
    });
}

#[rstest]
#[case("()=>value", "function scope()", false)]
#[case("function(){return value}", "function scope()", false)]
#[case("<div/>", "function scope()", false)]
#[case("<></>", "function scope()", false)]
#[case("changing=1", "function scope()", false)]
#[case("changing++", "function scope()", false)]
#[case("await value", "async function scope()", false)]
#[case("yield value", "function* scope()", false)]
#[case("value", "function scope()", true)]
fn embedded_expression_when_runtime_only_invalidates_an_allowed_outer_call(
    #[case] argument: &str,
    #[case] wrapper: &str,
    #[case] expected: bool,
) {
    // Given
    let target = format!("take({argument},value)");
    let source =
        format!("import {{take,value}} from './data';let changing=0;{wrapper}{{{target};}}");
    parsed(&source, |program, semantic| {
        let style = StyleSymbols::from_program(
            program,
            semantic.scoping(),
            &crate::ExtractOption::default(),
        );
        let call = semantic
            .nodes()
            .iter()
            .find_map(|node| match node.kind() {
                AstKind::CallExpression(call) if call.span.source_text(&source) == target => {
                    Some(call)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing parsed outer call"));
        let known =
            |identifier: &IdentifierReference<'_>| super::input(identifier, semantic, &style);
        // When
        let actual = super::call(call, &style, &known);
        // Then
        assert_eq!(actual, expected, "{source}");
    });
}

#[rstest]
#[case("function view(value){return value;}", "value")]
#[case("import {css} from '@devup-ui/react';css;", "css")]
#[case("import {Box} from '@devup-ui/react';Box;", "Box")]
#[case("let value;value;", "value")]
#[case("class Value{}Value;", "Value")]
#[case("const value=value;value;", "value")]
#[case("const left=right,right=left;left;", "left")]
fn input_when_binding_is_local_style_uninitialized_or_cyclic_is_rejected(
    #[case] source: &str,
    #[case] name: &str,
) {
    // Given: each name refers to a real semantic reference in the fixture.
    // When
    let actual = input_of(source, name);
    // Then
    assert!(!actual, "{source}");
}

#[rstest]
#[case("function read(n){return n?read(n-1):'blue'}read;")]
#[case("const read=n=>n?read(n-1):'blue';read;")]
#[case("const read=function(n){return n?read(n-1):'blue'};read;")]
#[case("function read(n){return n?other(n-1):'blue'}function other(n){return read(n)}read;")]
fn input_when_recursion_is_callable_keeps_closed_classification(#[case] source: &str) {
    // Given
    // When
    let actual = input_of(source, "read");
    // Then
    assert!(actual, "{source}");
}
