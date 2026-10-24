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
#[case("css({ w: (() => 1) + 1 })", Plan::Run)]
#[case("css({ w: (() => 1).prototype })", Plan::Run)]
#[case("css({ w: { callback: () => 1 } })", Plan::Run)]
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

#[rstest]
#[case("true ? f : 1", Plan::Run)]
#[case("true ? 1 : f", Plan::Run)]
#[case("true && f", Plan::Run)]
#[case("f || 1", Plan::Run)]
#[case("null ?? f", Plan::Run)]
#[case("true ? { nested: [f] } : { nested: [1] }", Plan::Run)]
#[case("true ? [{ callback: f }] : [1]", Plan::Run)]
#[case("true && { nested: [f] }", Plan::Run)]
#[case("[{ callback: f }] || 1", Plan::Run)]
#[case("true ? f : f", Plan::Run)]
#[case("f && f", Plan::Run)]
#[case("true ? { nested: [f] } : { nested: [f] }", Plan::Run)]
#[case("true ? 1 : 2", Plan::Plain)]
#[case("true && 1", Plan::Plain)]
#[case("1 || 2", Plan::Plain)]
#[case("null ?? 2", Plan::Plain)]
#[case("true ? { nested: [1] } : { nested: [2] }", Plan::Plain)]
#[case("true ? [{ width: 1 }] : [2]", Plan::Plain)]
#[case("true && { nested: [1] }", Plan::Plain)]
#[case("[{ width: 1 }] || 2", Plan::Plain)]
fn joined_style_values_require_data_in_every_branch(
    #[case] expression: &str,
    #[case] expected: Plan,
) {
    // Given
    let code = format!(
        "import {{ css }} from '@devup-ui/react'; const f = () => 1; export const rule = css({{ w: {expression} }});"
    );
    // When
    let actual = plan(
        &code,
        "join.css.ts",
        &ExtractOption::default(),
        None,
        &|_| false,
    );
    // Then
    assert_eq!(actual, expected, "{expression}");
}

#[rstest]
#[case("export default class Card {}")]
#[case("export default class { static color = 'red'; }")]
fn default_exported_classes_require_evaluation(#[case] declaration: &str) {
    // Given
    let code = format!(
        "import {{ css }} from '@devup-ui/react'; export const rule = css({{ color: 'red' }}); {declaration}"
    );
    // When
    let actual = plan(
        &code,
        "class.css.js",
        &ExtractOption::default(),
        None,
        &|_| false,
    );
    // Then
    assert_eq!(actual, Plan::Run);
}

#[rstest]
#[case("boundary.css.ts", true)]
#[case("boundary.css.unsupported", false)]
fn import_proof_requires_a_supported_source_type(#[case] filename: &str, #[case] expected: bool) {
    // Given
    let code = "import { css } from '@devup-ui/react'; export const rule = css({ color: 'red' });";
    // When
    let actual = super::imports_plain(code, filename, &ExtractOption::default(), None);
    // Then
    assert_eq!(actual, expected);
}
