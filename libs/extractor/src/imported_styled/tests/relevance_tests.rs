use super::*;
use crate::has_devup_ui_with;
use rstest::rstest;

fn resolve(specifier: &str, _: &str) -> Option<ResolvedModule> {
    let code = match specifier {
        "./base" => base_module(),
        "./forward" => "export { Dyn } from './base';".to_string(),
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
