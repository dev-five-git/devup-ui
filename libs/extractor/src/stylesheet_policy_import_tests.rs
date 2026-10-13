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

#[rstest::rstest]
#[case::diamond(false, Plan::Plain)]
#[case::cycle(true, Plan::Run)]
fn import_graph_requires_a_completed_proof_before_reusing_a_module(
    #[case] cycle: bool,
    #[case] expected: Plan,
) {
    // Given
    let resolver = move |specifier: &str, importer: &str| {
        let (path, code) = match (specifier, importer) {
            ("./left", "/src/card.css.ts" | "/src/shared.ts") => (
                "/src/left.ts",
                "import { RED } from './shared'; export const LEFT = RED;",
            ),
            ("./right", "/src/card.css.ts") => (
                "/src/right.ts",
                "import { RED } from './shared'; export const RIGHT = RED;",
            ),
            ("./shared", "/src/left.ts" | "/src/right.ts") => (
                "/src/shared.ts",
                if cycle {
                    "import './left'; export const RED = 'red';"
                } else {
                    "export const RED = 'red';"
                },
            ),
            _ => return None,
        };
        Some(ResolvedModule {
            path: path.to_string(),
            code: code.to_string(),
        })
    };
    let source = "import { css } from '@devup-ui/react'; import { LEFT } from './left'; import { RIGHT } from './right'; export const card = css({ color: LEFT, backgroundColor: RIGHT });";
    // When
    let actual = plan(
        source,
        "/src/card.css.ts",
        &ExtractOption::default(),
        Some(&resolver),
        &|_| false,
    );
    // Then
    assert_eq!(actual, expected);
}

#[rstest::rstest]
#[case::unchanged("", Plan::Plain)]
#[case::changed("tokens.color = 'blue';", Plan::Run)]
fn imported_data_requires_evaluation_when_the_root_changes_it(
    #[case] mutation: &str,
    #[case] expected: Plan,
) {
    // Given
    let resolver = |specifier: &str, importer: &str| {
        assert_eq!((specifier, importer), ("./tokens", "/src/card.css.ts"));
        Some(ResolvedModule {
            path: "/src/tokens.ts".to_string(),
            code: "export const tokens = { color: 'red' };".to_string(),
        })
    };
    let source = format!(
        "import {{ css }} from '@devup-ui/react'; import {{ tokens }} from './tokens'; {mutation} export const card = css(tokens);"
    );
    // When
    let actual = plan(
        &source,
        "/src/card.css.ts",
        &ExtractOption::default(),
        Some(&resolver),
        &|_| false,
    );
    // Then
    assert_eq!(actual, expected);
}
