use super::*;
use rstest::rstest;

#[rstest]
#[case(
    "import {css} from '@devup-ui/react';function make(n){return n+'px';}export const a=css({w:make(2)});",
    "\"2px\""
)]
#[case(
    "import {css} from '@devup-ui/react';const n=2;export const a=css({w:(n+3)+'px'});",
    "\"5px\""
)]
#[case(
    "import {css} from '@devup-ui/react';const theme={color:'red'};export const a=css({color:theme.color});",
    "\"red\""
)]
fn original_build_time_evaluation_when_the_dependency_closure_is_exact_replaces_values(
    #[case] source: &str,
    #[case] expected: &str,
) {
    let actual = evaluate(
        source,
        "a.tsx",
        &crate::ExtractOption::default(),
        None,
        &crate::imported_constants::Unknown::default(),
    )
    .unwrap_or_else(|| panic!("{source}"));
    assert!(actual.0.contains(expected), "{}", actual.0);
    assert_ne!(actual.1.len(), 0);
}

#[test]
fn exact_math_when_resolved_numeric_operands_are_used_preserves_the_result() {
    let result = exact_math::evaluate(
        "imul",
        Some(&[
            exact_math::Operand::Number(2.0),
            exact_math::Operand::Number(3.0),
        ]),
    );
    assert_eq!(result, Some(6.0));
}
