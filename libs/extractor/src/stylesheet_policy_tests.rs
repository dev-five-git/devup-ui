use rstest::rstest;

use super::{Plan, plan};
use crate::ExtractOption;

#[path = "stylesheet_policy_import_tests.rs"]
mod imports;

const HEAD: &str = "import { css, globalCss, keyframes, styled } from '@devup-ui/react';\n";

fn plan_of(code: &str) -> Plan {
    plan(code, "a.css.ts", &ExtractOption::default(), None, &|_| {
        false
    })
}

fn plan_with_head(code: &str) -> Plan {
    plan_of(&format!("{HEAD}{code}"))
}

#[rstest]
#[case(
    "import type { DevupProps } from '@devup-ui/react';\nimport { css } from '@devup-ui/react';\nexport const a = css({ color: 'red' });"
)]
#[case(
    "import { type DevupProps, css } from '@devup-ui/react';\nexport const a = css({ color: 'red' });"
)]
#[case("import { css } from '@devup-ui/react/compat';\nexport const a = css({ color: 'red' });")]
#[case(
    "'use client';\nimport { css } from '@devup-ui/react';\nexport const a = css({ color: 'red' });"
)]
#[case("")]
#[case("import * as ui from '@devup-ui/react';\nexport const a = ui.css({ color: 'red' });")]
#[case(
    "import * as ui from '@devup-ui/react';\nexport const Card = ui.styled.div({ color: 'red' });"
)]
#[case(
    "import { css } from '@devup-ui/react';\ndeclare const injected: number;\ntype A = 1;\nexport type { A };\nexport { type A as B };\nexport default interface Props {}\n;\nexport const a = css({ color: 'red' });"
)]
fn a_stylesheet_of_devup_styles_alone_is_extracted_as_written(#[case] code: &str) {
    assert_eq!(plan_of(code), Plan::Plain, "{code}");
}

#[rstest]
#[case("export const a = css({ color: 'red', p: 4, _hover: { color: ['blue', null, 'green'] } });")]
#[case("const base = { color: 'red', p: 4 };\nexport const a = css({ ...base, m: 1 });")]
#[case(
    "const sizes = [1, 2];\nconst base = { w: sizes[1] * 2 };\nexport const a = css({ ...base, ...{ h: Math.max(1, 2), top: -Math.PI }, bg: `${base.w}px` });"
)]
#[case(
    "const size = 4;\nconst other = size > 2 ? 'a' : 'b';\nexport const a = css({ w: size, c: other, m: undefined, [`x${size}`]: 1 });"
)]
#[case(
    "export const a = css({ color: 'red' });\nexport const b = css(a, { m: -1 });\nexport default b;"
)]
#[case(
    "export const a = css({ color: 'red' });\nexport { a as renamed };\nexport type Props = { a: string };\nexport interface Other {}"
)]
#[case(
    "globalCss({ body: { m: 0 } });\nexport const k = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } });"
)]
#[case(
    "export const Card = styled('div', { bg: 'white' });\nexport const Link = styled.a.attrs({ href: '#' })({ color: 'red' });\nexport const Big = styled(Card)({ p: 8 });"
)]
#[case("const rules = { color: 'red' };\nexport const a = css`color: ${rules.color};`;")]
#[case(
    "type Tokens = { a: string };\nconst tokens: Tokens = { a: 'red' } as const;\nexport const a = css({ color: tokens.a as string, w: Math.round(2.5)! });"
)]
fn constants_references_and_spreads_the_visitor_reads_stay_plain(#[case] code: &str) {
    assert_eq!(plan_with_head(code), Plan::Plain, "{code}");
}

#[rstest]
#[case("export const a = css({ color: 'red' });\nthrow new Error('fixture boom');")]
#[case("throw new Error('fixture boom');\nexport const red = css({ color: 'red' });")]
#[case("try { css({ color: 'red' }); } catch {}")]
#[case("const t = Date.now();\nexport const a = css({ w: t });")]
#[case("export const a = css({ w: Math.random() });")]
#[case("export const a = css({ w: Math.sin(1) });")]
#[case("export const a = css({ w: process.env.WIDTH });")]
#[case("export const a = css({ w: typeof window });")]
#[case("export const a = css({ w: window.innerWidth });")]
#[case("export const a = css({ w: isDark ? 1 : 2 });")]
#[case("export const a = css({ w: new Date().getFullYear() });")]
#[case("export const a = css({ get w() { return 1; } });")]
#[case("export const a = css({ set w(value) {} });")]
#[case("export const a = css({ w() { return 1; } });")]
#[case("export const a = css({ w: () => 1 });")]
#[case("export const a = css({ w: function () { return 1; } });")]
#[case("export const a = css({ [window.key]: 1 });")]
#[case("export const a = css({ __proto__: { w: 1 } });")]
#[case("const o = { get a() { return 1; } };\nexport const a = css({ ...o });")]
#[case("const o = { a: 1 };\no.a = 2;\nexport const a = css({ ...o });")]
#[case("const o = { a: 1 };\nObject.assign(o, { a: 2 });\nexport const a = css({ ...o });")]
#[case("let width = 1;\nexport const a = css({ w: width });")]
#[case("var width = 1;\nexport const a = css({ w: width });")]
#[case("const items = [];\nitems.push(1);\nexport const a = css({ w: items[0] });")]
#[case("export const a = css({ w: 2 ** 3 });")]
#[case("export const a = css({ w: delete globalThis.x });")]
#[case("export const a = css({ w: 'w' in {} });")]
#[case("export const a = css({ w: x?.y });")]
#[case("export const a = css({ w: String(1) });")]
#[case("export const a = css({ w: Math.max });")]
#[case("export const a = css({ w: later });\nconst later = 1;")]
#[case("export const a = css({ w: a });")]
#[case("const a = css({ w: a2 }), a2 = 1;")]
#[case("export const [a, b] = [1, 2];")]
#[case("export const { a } = { a: 1 };")]
#[case("export const a;")]
#[case("export function f() {}\nexport const a = css({ w: 1 });")]
#[case("function f() {}\nexport const a = css({ w: 1 });")]
#[case("export class A {}")]
#[case("export default function () {}")]
#[case("enum Size { Small = 1 }\nexport const a = css({ w: Size.Small });")]
#[case("export const a = <div />;")]
#[case("export * from './other';")]
#[case("export { other } from './other';")]
#[case("export const a = other`color: red;`;")]
#[case("css({ color: 'red' }).length;")]
#[case("1;")]
#[case("{ const x = 1; }")]
#[case("if (true) { css({ color: 'red' }); }")]
#[case("for (const x of []) css({ w: x });")]
#[case("export const a = css({ w: await Promise.resolve(1) });")]
#[case("export const a = css({ w: 1 };")]
#[case("export const a = css({ w: 1 });\nexport const a = css({ w: 2 });")]
#[case("export const a = css({ w: this.x });")]
#[case("const unused = null.color; export const a = css({ color: 'red' });")]
#[case("const unused = [...null]; export const a = css({ color: 'red' });")]
#[case("const unused = ({}).missing.color; export const a = css({ color: 'red' });")]
#[case(
    "const unused = { valueOf: null, toString: null } + 1; export const a = css({ color: 'red' });"
)]
#[case("const a = 1;\nconst a = 2;\nexport const b = css({ w: a });")]
fn any_other_effect_makes_it_a_stylesheet_to_run(#[case] code: &str) {
    assert_eq!(plan_with_head(code), Plan::Run, "{code}");
}

#[rstest]
#[case("import { style } from '@devup-ui/react';\nexport const a = style({ color: 'red' });")]
#[case(
    "import { css, style } from '@devup-ui/react';\nexport const a = css({ color: 'red' });\nexport const b = style({ color: 'blue' });"
)]
#[case(
    "import { css } from '@devup-ui/react';\nimport { Box } from '@devup-ui/react';\nexport const a = css({ color: 'red' });"
)]
#[case("import css from '@devup-ui/react';\nexport const a = css({ color: 'red' });")]
#[case("import { css } from '@devup-ui/react-other';\nexport const a = css({ color: 'red' });")]
#[case("import { css } from 'other';\nexport const a = css({ color: 'red' });")]
#[case(
    "import { css } from '@devup-ui/react';\nimport './side-effect';\nexport const a = css({ color: 'red' });"
)]
#[case(
    "import { css } from '@devup-ui/react';\nimport {} from './side-effect';\nexport const a = css({ color: 'red' });"
)]
#[case(
    "import { css } from '@devup-ui/react';\nimport { tokens } from './tokens';\nexport const a = css({ color: tokens.red });"
)]
#[case(
    "import { css } from '@devup-ui/react';\nimport Math from './math';\nexport const a = css({ w: Math.max(1, 2) });"
)]
#[case("import '@devup-ui/react';\nconst a = 1;")]
fn imports_that_run_or_bind_what_is_not_data_make_it_a_stylesheet_to_run(#[case] code: &str) {
    assert_eq!(plan_of(code), Plan::Run, "{code}");
}

#[rstest]
#[case(
    "function f(css) { return css({ color: 'red' }); }\nexport const a = css({ color: 'blue' });"
)]
#[case("export const f = (css) => css({ color: 'red' });")]
#[case("{ const css = (rules) => rules; css({ color: 'red' }); }")]
#[case("const rules = css;\nexport const a = rules({ color: 'red' });")]
#[case("export const a = ((css) => css({ color: 'red' }))((rules) => rules);")]
#[case("const Math = { max: 5 };\nexport const a = css({ w: Math.max(1, 2) });")]
#[case("const css2 = { w: 1 };\nexport const a = css2({ color: 'red' });")]
#[case("const styled2 = 1;\nexport const a = styled2.div({ color: 'red' });")]
#[case(
    "import * as ui from '@devup-ui/react';\nexport const a = ((ui) => ui.css({ color: 'red' }))({ css: 1 });"
)]
#[case("import * as ui from '@devup-ui/react';\nexport const a = ui.missing({ color: 'red' });")]
#[case("import * as ui from '@devup-ui/react';\nexport const a = ui.css;")]
fn a_local_named_like_a_style_api_or_math_is_not_one(#[case] code: &str) {
    assert_eq!(plan_with_head(code), Plan::Run, "{code}");
}

#[test]
fn a_style_api_the_package_does_not_give_is_not_compiled() {
    assert_eq!(
        plan_of(
            "import { css, Box } from '@devup-ui/react';\nexport const a = css({ color: 'red' });"
        ),
        Plan::Run
    );
    assert_eq!(
        plan_of(
            "import { css as rules } from '@devup-ui/react';\nexport const a = rules({ color: 'red' });"
        ),
        Plan::Plain
    );
    assert_eq!(
        plan_of(
            "import { css } from '@devup-ui/react';\nconst rules = { w: css };\nexport const a = css(rules);"
        ),
        Plan::Run
    );
    assert_eq!(
        plan_of(
            "import { css } from '@devup-ui/react';\nexport const a = css.global({ color: 'red' });"
        ),
        Plan::Run
    );
}

#[test]
fn a_file_that_is_not_javascript_is_a_stylesheet_to_run() {
    assert_eq!(
        plan(
            "export const a = 1;",
            "a.css",
            &ExtractOption::default(),
            None,
            &|_| false
        ),
        Plan::Run
    );
}

#[test]
fn the_fixtures_that_pin_the_fallback_keep_their_plans() {
    assert_eq!(
        plan_of(
            "import type { DevupProps } from '@devup-ui/react';\nimport { css } from '@devup-ui/react';\nexport const a = css({ color: 'red' });"
        ),
        Plan::Plain
    );
    assert_eq!(
        plan_of(
            "import { css } from '@devup-ui/react';\nthrow new Error('fixture boom');\nexport const red = css({ color: 'red' });"
        ),
        Plan::Run
    );
}

#[rstest]
#[case("null")]
#[case("true")]
#[case("Math.PI")]
fn template_proof_checks_later_interpolations_when_early_value_is_not_exact(#[case] first: &str) {
    let code = format!(
        "const unused = `${{{first}}}${{null.color}}`; export const a = css({{ color: 'red' }});"
    );
    assert_eq!(plan_with_head(&code), Plan::Run);
}
