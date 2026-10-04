use rstest::rstest;

use super::{Plan, plan};
use crate::ExtractOption;

#[rstest]
#[case("css(...[{ color: 'red' }])", Plan::Plain)]
#[case("css(...'ab')", Plan::Plain)]
#[case("css(...null)", Plan::Run)]
#[case("css(...{ color: 'red' })", Plan::Run)]
#[case("css({ color: [...'ab'][0] })", Plan::Plain)]
#[case("css({ color: [...['red']][0] })", Plan::Plain)]
#[case("css({ color: [, 'red'][0] })", Plan::Plain)]
#[case("css({ color: ({ ...['red'] })[0] })", Plan::Plain)]
#[case("css({ color: ({ ...'red' })[0] })", Plan::Plain)]
#[case("css({ color: ({ ...'😀' })[0] })", Plan::Plain)]
#[case("css({ ...null, ...true, ...1, color: 'red' })", Plan::Plain)]
#[case("css({ ...(true ? {} : []) })", Plan::Run)]
#[case("css({ color: {}.constructor })", Plan::Run)]
#[case("css({ color: {}.__proto__ })", Plan::Run)]
#[case("css({ color: {}.toString })", Plan::Run)]
#[case("css({ color: {}.valueOf })", Plan::Run)]
#[case("css({ color: {}.missing })", Plan::Plain)]
#[case("css({ w: [1].length, h: 'ab'.length })", Plan::Plain)]
#[case("css({ color: 'ab'[0] })", Plan::Plain)]
#[case("css({ color: 'ab'[9] })", Plan::Plain)]
#[case("css({ color: '😀'[0] })", Plan::Plain)]
#[case("css({ color: `${{}}` })", Plan::Run)]
#[case("css({ color: true && 'red' })", Plan::Plain)]
#[case("css({ color: 'red' || 'red' })", Plan::Plain)]
#[case("css({ color: (1, 'red') })", Plan::Plain)]
#[case("css({ color: Math.max(...[1, 2]) })", Plan::Run)]
fn dispatch_runs_only_when_plain_data_cannot_prove_nonthrowing_evaluation(
    #[case] expression: &str,
    #[case] expected: Plan,
) {
    // Given
    let code =
        format!("import {{ css }} from '@devup-ui/react'; export const rule = {expression};");
    // When
    let actual = plan(
        &code,
        "boundary.css.ts",
        &ExtractOption::default(),
        None,
        &|_| false,
    );
    // Then
    assert_eq!(actual, expected, "{expression}");
}
