use super::has_build_time_values;
use crate::ExtractOption;

#[rstest::rstest]
#[case("await 'red'", false)]
#[case("'red'.toUpperCase()", true)]
fn build_time_candidate_when_evaluation_can_suspend_is_rejected(
    #[case] value: &str,
    #[case] expected: bool,
) {
    let source = format!("import {{css}} from '@devup-ui/react'; css({{color: {value}}});");

    let actual = has_build_time_values("await.tsx", &source, &ExtractOption::default(), None);

    assert_eq!(actual, expected);
}

#[test]
fn a_local_named_like_a_style_api_computes_nothing() {
    let option = ExtractOption::default();
    let prelude = "import { css, Box } from '@devup-ui/react';
import * as ui from '@devup-ui/react';
const double = (n) => n * 2;
";
    for code in [
        "export const a = css({ w: double(2) });",
        "export const a = ui.css({ w: double(2) });",
        "export const a = <Box {...double(2)} />;",
        "export const a = <ui.Box {...double(2)} />;",
    ] {
        assert!(
            has_build_time_values("a.tsx", &format!("{prelude}{code}"), &option, None),
            "{code}"
        );
    }
    for code in [
        "export function f(css) { return css({ w: double(2) }); }",
        "export function f(ui) { return ui.css({ w: double(2) }); }",
        "export function f(Box) { return <Box {...double(2)} />; }",
        "export function f(ui) { return <ui.Box {...double(2)} />; }",
    ] {
        assert!(
            !has_build_time_values("a.tsx", &format!("{prelude}{code}"), &option, None),
            "{code}"
        );
    }
}
