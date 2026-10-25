use super::{fixtures, modules};
use rstest::rstest;
use serial_test::serial;

fn compile_css(
    files: Vec<(String, String)>,
    single_css: bool,
) -> Result<crate::ExtractOutput, Box<dyn std::error::Error>> {
    let (name, source) = files
        .last()
        .unwrap_or_else(|| panic!("entry source"))
        .clone();
    let resolver = move |specifier: &str, _: &str| {
        let path = format!("/dynamic-css/{}", specifier.trim_start_matches("./"));
        files
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(name, source)| {
                crate::ModuleResolution::Resolved(crate::ResolvedModule {
                    path: name.clone(),
                    code: source.clone(),
                    source_type: None,
                })
            })
    };
    crate::extract_with_modules(
        &name,
        &source,
        crate::ExtractOption {
            single_css,
            import_aliases: std::collections::HashMap::from([(
                "@emotion/react".into(),
                crate::ImportAlias::NamedToNamed,
            )]),
            ..crate::ExtractOption::default()
        },
        false,
        &resolver,
    )
}

#[rstest]
#[serial]
fn imported_stages_preserve_css_overlap_refusal_when_only_the_origin_owns_the_key(
    #[values(false, true)] single_css: bool,
) {
    // Given: the same overlapping CSS prop on local and imported captured retargets.
    let _state = crate::test_state::TestStateGuard::new();
    let (mut files, _) = fixtures::dynamic_css();
    files[1].1 = files[1]
        .1
        .replace("backgroundColor:'blue'", "fontSize:'99px'");
    let local = files[1]
        .1
        .replace("import {styled} from '@devup-ui/react';", "")
        .replace("import {Dyn} from './base.js';", "");
    let local = vec![(files[1].0.clone(), format!("{}{local}", files[0].1))];
    // When: both complete source graphs reach the unchanged CSS overlap guard.
    let Err(local) = compile_css(local, single_css) else {
        panic!("local overlap is refused");
    };
    let Err(imported) = compile_css(files, single_css) else {
        panic!("inherited overlap is refused");
    };
    let local = local.to_string();
    let imported = imported.to_string();
    // Then: structural origin coverage retains the same located refusal, not an empty summary.
    assert_eq!(
        imported
            .splitn(4, ':')
            .nth(3)
            .unwrap_or_else(|| panic!("located imported error")),
        local
            .splitn(4, ':')
            .nth(3)
            .unwrap_or_else(|| panic!("located local error"))
    );
}

#[rstest]
#[case("shadow", "foreign")]
#[case("missing", "TypeError")]
#[case("dynamic", "TypeError")]
#[case("type", "ReferenceError")]
#[serial]
fn imported_stages_preserve_lookup_limits_when_the_receiver_is_not_a_value_component(
    #[case] kind: &str,
    #[case] expected: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: a real dynamic export with a shadowed, missing, dynamic or type-only receiver.
    let _state = crate::test_state::TestStateGuard::new();
    let source = match kind {
        "shadow" => {
            "import * as UI from './base.js';const call=UI=>UI.Dyn.withComponent('a');export const App=()=>call({Dyn:{withComponent:()=>({target:'foreign',props:{}})}});"
        }
        "missing" => {
            "import * as UI from './base.js';export const App=()=>{try{return UI.Missing.withComponent('a');}catch(error){return {target:error.name,props:{}};}};"
        }
        "dynamic" => {
            "import * as UI from './base.js';const call=key=>UI[key].withComponent('a');export const App=()=>{try{return call('Dyn');}catch(error){return {target:error.name,props:{}};}};"
        }
        "type" => {
            "import type * as UI from './base.js';export const App=()=>{try{return UI.Dyn.withComponent('a');}catch(error){return {target:error.name,props:{}};}};"
        }
        _ => panic!("known control"),
    };
    let files = [
        (
            "/limits/base.js",
            "import {styled} from '@devup-ui/react';export const Dyn=styled.button({color:p=>p.tone});",
        ),
        ("/limits/entry.ts", source),
    ];
    let outputs = modules::compile(&files, single_css);
    // When: the full emitted module graph executes the original foreign or unsupported access.
    let actual = modules::execute(
        &outputs,
        "import {App} from '/limits/entry.ts';globalThis.observed=render(App()).target;",
    );
    // Then: no semantic namespace definition is fabricated for that access.
    assert_eq!(actual, expected);
}

#[rstest]
#[serial]
fn imported_stages_remain_retained_when_a_css_prop_is_added_to_a_dynamic_handle(
    #[values(false, true)] single_css: bool,
) {
    // Given: a retarget-only dynamic handle and an already-admitted disjoint CSS prop.
    let _state = crate::test_state::TestStateGuard::new();
    let (files, observer) = fixtures::dynamic_css();
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    // When: CSS lowering sees the handle, then the complete renderer executes.
    let actual = modules::execute(&outputs, &observer);
    // Then: dynamic retention never claims Inline or complete fixed-slot eligibility.
    assert_eq!(actual["rawIdentity"], true);
    assert_eq!(actual["rawSlots"], 0);
    assert_eq!(actual["target"], "a");
    assert_eq!(actual["props"]["ref"], 7);
    assert_eq!(actual["props"]["children"], "child");
}
