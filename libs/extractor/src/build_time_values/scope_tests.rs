use super::has_build_time_values;
use crate::ExtractOption;

#[test]
fn erased_bindings_are_runtime_only_inputs() {
    let option = ExtractOption::default();
    for body in [
        "declare const a: number, b: number; export const x = css({ w: a + b });",
        "declare function getHover(): object; export const x = <Box _hover={getHover()} />;",
        "export declare function getHover(): object; export const x = <Box _hover={getHover()} />;",
    ] {
        // Given erased declarations referenced by a style expression.
        let code = format!("import {{ css, Box }} from '@devup-ui/react'; {body}");
        // When selecting exact computations, then none may execute those inputs.
        assert!(
            !has_build_time_values("a.tsx", &code, &option, None),
            "{body}"
        );
    }
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
