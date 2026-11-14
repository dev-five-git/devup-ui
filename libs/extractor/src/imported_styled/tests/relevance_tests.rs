use super::*;
use crate::has_devup_ui_with;
use rstest::rstest;

fn resolve(specifier: &str, _: &str) -> Option<ResolvedModule> {
    let code = match specifier {
        "./base" => base_module(),
        "./forward" => "export { Dyn } from './base';".to_string(),
        "./types" => "export type { Dyn } from './base';".to_string(),
        "./object" => "export const Dyn = { withComponent: (tag) => tag };".to_string(),
        "./unreadable" => {
            "import styled from '@emotion/styled'; export const Dyn = styled(unknown)`color:red;`;"
                .to_string()
        }
        _ => return None,
    };
    Some(ResolvedModule {
        path: format!("/src/{}.tsx", specifier.trim_start_matches("./")),
        code,
    })
}

#[rstest]
#[case(
    "named",
    "import { Dyn } from './base'; export const A = Dyn.withComponent('a');"
)]
#[case(
    "reexport",
    "import { Dyn } from './forward'; export const A = Dyn.withComponent('a');"
)]
#[case(
    "namespace",
    "import * as UI from './base'; export const A = UI.Dyn.withComponent('a');"
)]
#[case(
    "trigger",
    "import { styled } from '@devup-ui/react'; import { Dyn } from './base'; export const A = Dyn.withComponent('a');"
)]
#[serial]
fn imported_retargets_are_relevant_without_package_imports(#[case] name: &str, #[case] code: &str) {
    reset();
    let option = option();
    assert!(has_devup_ui_with(
        "/src/app.tsx",
        code,
        &option.package,
        &option.import_aliases,
        Some(&resolve),
    ));
    let output = extract_with_modules("/src/app.tsx", code, option, false, &resolve).unwrap();
    assert_ne!(output.css_file, None);
    assert_debug_snapshot!(name, format!("{}\n{:?}", output.code, output.dependencies));
}

#[rstest]
#[case(
    "import { Dyn } from './base'; export function retarget(Dyn) { return Dyn.withComponent('a'); }"
)]
#[case(
    "import * as UI from './base'; export function retarget(UI) { return UI.Dyn.withComponent('a'); }"
)]
#[case("import type { Dyn } from './base'; export const A = Dyn.withComponent('a');")]
#[case("import { type Dyn } from './base'; export const A = Dyn.withComponent('a');")]
#[case("import { Dyn } from './object'; export const A = Dyn.withComponent('a');")]
#[case("import { Dyn } from './missing'; export const A = Dyn.withComponent('a');")]
#[case("import { Dyn } from './unreadable'; export const A = Dyn.withComponent('a');")]
#[case("import { Missing } from './base'; export const A = Missing.withComponent('a');")]
#[case("import { Dyn } from './types'; export const A = Dyn.withComponent('a');")]
#[case(
    "import './base'; const Dyn = { withComponent: tag => tag }; export const A = Dyn.withComponent('a');"
)]
#[case("import * as UI from './base'; export const A = UI[key].withComponent('a');")]
#[case("import * as UI from './base'; export const A = UI.more.Dyn.withComponent('a');")]
#[case("import './base'; export const A = unknown.withComponent('a');")]
#[case("import './base'; export const A = factory().withComponent('a');")]
#[case("import { Dyn } from './base'; export const A = Dyn;")]
#[case("const A = 'withComponent';")]
#[case("import { Dyn } from './base'; // withComponent\nexport const A = Dyn;")]
#[case("import { Dyn } from './base'; export const A = ; // withComponent")]
#[serial]
fn unrelated_retargets_remain_unchanged(#[case] code: &str) {
    reset();
    let option = option();
    assert!(!has_devup_ui_with(
        "/src/app.tsx",
        code,
        &option.package,
        &option.import_aliases,
        Some(&resolve),
    ));
    let output = extract_with_modules("/src/app.tsx", code, option, false, &resolve).unwrap();
    assert_eq!(output.code, code);
    assert_eq!(output.dependencies, Vec::<String>::new());
    assert_eq!(output.css_file, None);
}

#[test]
#[serial]
fn retargets_without_a_resolver_remain_unchanged() {
    let code = "import { Dyn } from './base'; export const A = Dyn.withComponent('a');";
    let option = option();
    assert!(!has_devup_ui_with(
        "/src/app.tsx",
        code,
        &option.package,
        &option.import_aliases,
        None,
    ));
    assert_eq!(
        crate::extract_without_source_map("/src/app.tsx", code, option)
            .unwrap()
            .code,
        code
    );
}

#[rstest]
#[case("")]
#[case("import { styled } from '@devup-ui/react';")]
#[serial]
fn literal_namespace_retargets_match_dot_with_and_without_trigger(#[case] trigger: &str) {
    let dot = format!(
        "{trigger}import * as UI from './base'; export const A = (UI.Dyn).withComponent('a');"
    );
    reset();
    let expected = extract_with_modules("/src/app.tsx", &dot, option(), false, &resolve).unwrap();
    let computed = dot.replace("UI.Dyn", "(UI['Dyn'] as typeof UI.Dyn)!");
    let option = option();
    assert!(has_devup_ui_with(
        "/src/app.tsx",
        &dot,
        &option.package,
        &option.import_aliases,
        Some(&resolve),
    ));
    assert!(has_devup_ui_with(
        "/src/app.tsx",
        &computed,
        &option.package,
        &option.import_aliases,
        Some(&resolve),
    ));
    reset();
    let actual = extract_with_modules("/src/app.tsx", &computed, option, false, &resolve).unwrap();
    // Staged rendering keeps the authored receiver. Normalize only that one
    // expression, not imports, forwarding, target selection or render stages.
    let receiver = "(UI[\"Dyn\"] as typeof UI.Dyn)!";
    assert_eq!(actual.code.matches(receiver).count(), 1);
    assert_eq!(expected.code.matches("UI.Dyn").count(), 1);
    assert_eq!(actual.code.replace(receiver, "UI.Dyn"), expected.code);
    assert!(!actual.code.contains(".withComponent("));
    assert!(actual.code.contains("import * as UI from \"./base\";"));
    assert_eq!(actual.styles, expected.styles);
    assert_eq!(actual.dependencies, expected.dependencies);
    assert_eq!(actual.dependencies, ["/src/base.tsx"]);
    assert!(actual.css_file.is_some());
}

#[rstest]
#[case("import { Dyn } from './base';", "Dyn", "Dyn")]
#[case("import * as UI from './base';", "UI", "UI.Dyn")]
#[case("import * as UI from './base';", "UI", "UI['Dyn']")]
#[serial]
fn admitted_retargets_preserve_shadowed_occurrences(
    #[case] import: &str,
    #[case] shadow: &str,
    #[case] expression: &str,
) {
    let code = format!(
        "{import} export function untouched({shadow}) {{ return {expression}.withComponent('section'); }} export const A = {expression}.withComponent('a');"
    );
    let option = option();
    assert!(has_devup_ui_with(
        "/src/app.tsx",
        &code,
        &option.package,
        &option.import_aliases,
        Some(&resolve),
    ));
    reset();
    let output = extract_with_modules("/src/app.tsx", &code, option, false, &resolve).unwrap();
    assert_eq!(output.code.matches(".withComponent(").count(), 1);
    let receiver = expression.replace('\'', "\"");
    assert!(output.code.contains(&format!(
        "export function untouched({shadow}) {{\n\treturn {receiver}.withComponent(\"section\");\n}}"
    )));
    assert!(output.code.contains(&import.replace('\'', "\"")));
    // The two receivers remain, plus any named import and parameter bindings.
    // The admitted call must not copy the origin's dynamic callback.
    assert_eq!(
        output.code.matches(&receiver).count(),
        2 + import.matches(expression).count() + shadow.matches(expression).count()
    );
    assert!(!output.code.contains("20px"));
    assert!(output.css_file.is_some());
    assert_eq!(output.dependencies, ["/src/base.tsx"]);
}

#[test]
#[serial]
fn unsupported_retarget_source_types_remain_irrelevant() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "import { Dyn } from './base'; export const A = Dyn.withComponent('a');";

    let reading = crate::imported_styled::relevance::read(
        &allocator,
        "/src/app.unknown",
        source,
        &option(),
        Some(&resolve),
    );

    assert_eq!(reading.components.len(), 0);
    assert_eq!(reading.dependencies.len(), 0);
}

#[test]
#[serial]
fn imported_retarget_with_variables_merges_them_into_the_composed_style() {
    // Given a component extending an imported one with a CSS variable of its own
    reset();
    let code = "import styled from '@emotion/styled'; import { Pick } from './base'; export const B = styled(Pick)`width: ${(p) => p.w};`;";
    // When it is compiled
    let output = extract_with_modules("/src/app.tsx", code, option(), false, &resolve).unwrap();
    // Then its stage spreads the variables a composing component gives before its own
    assert_debug_snapshot!(
        "own_variables",
        format!("{}\n{:?}", output.code, output.dependencies)
    );
}
