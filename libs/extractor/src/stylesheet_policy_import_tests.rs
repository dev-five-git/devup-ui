use super::{Plan, plan};
use crate::{ExtractOption, ResolvedModule};

#[test]
fn an_import_is_data_when_the_build_knows_its_value_whole() {
    let resolver = |specifier: &str, _: &str| {
        Some(ResolvedModule {
            path: format!("/src/{}.ts", specifier.trim_start_matches("./")),
            code: match specifier {
                "./mutated" => "export const mutated = { a: 1 }; mutated.a = 2;",
                "./computed" => "export const computed = Date.now();",
                _ => "export const RED = 'red';\nexport const LIST = [1, 2];\nexport function twice(n) { return n * 2; }",
            }
            .to_string(),
        })
    };
    let option = ExtractOption::default();
    let plan_of = |code: &str, is_static: &dyn Fn(&str) -> bool| {
        plan(
            &format!("import {{ css }} from '@devup-ui/react';\n{code}"),
            "/src/a.css.ts",
            &option,
            Some(&resolver),
            is_static,
        )
    };
    let no = |_: &str| false;
    for code in [
        "import { RED } from './tokens';\nexport const a = css({ color: RED });",
        "import { RED, LIST } from './tokens';\nexport const a = css({ color: RED, w: LIST[1] });",
        "import { RED as COLOR } from './tokens';\nexport const a = css({ color: COLOR });",
    ] {
        assert_eq!(plan_of(code, &no), Plan::Plain, "{code}");
    }
    for code in [
        "import { mutated } from './mutated';\nexport const a = css({ ...mutated });",
        "import { computed } from './computed';\nexport const a = css({ w: computed });",
        "import { twice } from './tokens';\nexport const a = css({ w: twice(2) });",
        "import { missing } from './tokens';\nexport const a = css({ w: missing });",
        "import * as tokens from './tokens';\nexport const a = css({ w: tokens.RED });",
    ] {
        assert_eq!(plan_of(code, &no), Plan::Run, "{code}");
    }
    let classes = |name: &str| name == "card";
    assert_eq!(
        plan_of(
            "import { card } from './tokens';\nexport const a = css(card, { p: 1 });",
            &classes
        ),
        Plan::Plain
    );
    assert_eq!(
        plan_of(
            "import { card, other } from './tokens';\nexport const a = css(card, other);",
            &classes
        ),
        Plan::Run
    );
}

#[rstest::rstest]
#[case("export const RED = 'red'; throw new Error('helper boom');")]
#[case("export const RED = 'red'; try { Date.now(); } catch {}")]
#[case("export const RED = 'red'; unknownRuntime();")]
fn known_exports_and_static_callbacks_cannot_prove_module_effects(#[case] helper: &str) {
    // Given
    let helper = helper.to_string();
    let resolver = move |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/src/helper.ts".to_string(),
            code: helper.clone(),
        })
    };
    let source = "import { css } from '@devup-ui/react'; import { RED } from './helper'; export const card = css({ color: RED });";
    // When
    let actual = plan(
        source,
        "/src/card.css.ts",
        &ExtractOption::default(),
        Some(&resolver),
        &|_| true,
    );
    // Then
    assert_eq!(actual, Plan::Run);
}

#[test]
fn a_transitive_effect_keeps_literal_imports_on_the_evaluated_path() {
    // Given
    let resolver = |specifier: &str, _: &str| {
        Some(match specifier {
            "./helper" => ResolvedModule {
                path: "/src/helper.ts".to_string(),
                code: "import { color } from './effect'; export const RED = color;".to_string(),
            },
            _ => ResolvedModule {
                path: "/src/effect.ts".to_string(),
                code: "export const color = 'red'; throw new Error('nested boom');".to_string(),
            },
        })
    };
    let source = "import { css } from '@devup-ui/react'; import { RED } from './helper'; export const card = css({ color: RED });";
    // When
    let actual = plan(
        source,
        "/src/card.css.ts",
        &ExtractOption::default(),
        Some(&resolver),
        &|_| false,
    );
    // Then
    assert_eq!(actual, Plan::Run);
}
