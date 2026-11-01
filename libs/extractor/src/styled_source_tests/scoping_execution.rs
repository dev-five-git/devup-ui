use super::execute_extracted;
use crate::{ExtractOption, ImportAlias, ResolvedModule};
use rstest::rstest;
use serial_test::serial;

const SOURCE: &str = "import {styled,css} from '@devup-ui/react';const trace=[];const callback=function(p,unused=(trace.push('default'),0)){trace.push(this===globalThis?'global':'receiver',arguments.length);return p.value;};const Base=styled.div({p:2});const Card=styled(Base)({p:callback});const observe=()=>trace;";

#[derive(Clone, Copy, Debug)]
enum Surface {
    Extract,
    WithoutMap,
    Modules(bool),
    ModulesPlain,
}

fn compile(source: &str, single_css: bool, surface: Surface) -> crate::ExtractOutput {
    let mut option = ExtractOption {
        single_css,
        import_aliases: std::collections::HashMap::from([
            ("@emotion/react".into(), ImportAlias::NamedToNamed),
            (
                "@emotion/styled".into(),
                ImportAlias::DefaultToNamed("styled".into()),
            ),
        ]),
        ..ExtractOption::default()
    };
    if matches!(surface, Surface::ModulesPlain) {
        option.import_aliases.clear();
    }
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./base").then(|| ResolvedModule {
            path: "/composition/base.ts".into(),
            code: String::new(),
        })
    };
    let result = match surface {
        Surface::Extract => crate::extract("/composition/entry.ts", source, option),
        Surface::WithoutMap => {
            crate::extract_without_source_map("/composition/entry.ts", source, option)
        }
        Surface::Modules(map) => {
            crate::extract_with_modules("/composition/entry.ts", source, option, map, &resolver)
        }
        Surface::ModulesPlain => {
            crate::extract_with_modules("/composition/entry.ts", source, option, false, &resolver)
        }
    };
    let output = result.unwrap_or_else(|error| panic!("{surface:?}: {error}"));
    assert_eq!(
        output.map.is_some(),
        matches!(surface, Surface::Extract | Surface::Modules(true))
    );
    output
}

#[rstest]
#[case::number("4", serde_json::json!(["16px"]))]
#[case::null("null", serde_json::json!(["8px"]))]
#[case::undefined("undefined", serde_json::json!(["8px"]))]
#[case::false_value("false", serde_json::json!(["8px"]))]
#[serial]
fn original_binding_survives_when_import_discovery_reads_recovered_callbacks(
    #[case] value: &str,
    #[case] padding: serde_json::Value,
    #[values(false, true)] single_css: bool,
    #[values(
        Surface::Extract,
        Surface::WithoutMap,
        Surface::Modules(false),
        Surface::Modules(true),
        Surface::ModulesPlain
    )]
    surface: Surface,
) {
    // Given: the exact confirmed source/config, with independent CSS-mode cases.
    let _state = crate::test_state::TestStateGuard::new();
    // When: extract and execute the complete renderer, observing the authored closure.
    let output = compile(SOURCE, single_css, surface);
    let bucket = (!single_css).then(|| css::file_map::canonical("/composition/entry.ts"));
    let actual = execute_extracted(
        ("/composition/entry.ts", bucket.as_deref()),
        &output,
        &format!(
            "const e=render({{target:Card,props:{{value:{value}}}}});JSON.stringify([observe(),values(e,'padding')]);"
        ),
    );
    // Then: source defaults/this/arguments and absent-value Keep remain intact.
    assert_eq!(
        actual,
        serde_json::json!([["default", "global", 1], padding])
    );
}

#[rstest]
#[case::whole("const callback=(p,unused=(trace.push('default'),0))=>({p:p.value});const Card=styled(Base)(callback);", serde_json::json!(["16px"]), serde_json::json!(["default"]))]
#[case::inline("const Card=styled(Base)({p:function(p,unused=(trace.push('default'),0)){trace.push(this===globalThis?'global':'receiver',arguments.length);return p.value;}});", serde_json::json!(["16px"]), serde_json::json!(["default", "global", 1]))]
#[case::unknown("function make(callback){return styled(Base)({ p: callback });}const Card=make(function(){trace.push('called');return 4;});", serde_json::json!(["function"]), serde_json::json!([]))]
#[serial]
fn callback_contract_survives_when_module_discovery_runs(
    #[case] rules: &str,
    #[case] padding: serde_json::Value,
    #[case] trace: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    // Given: related original-function and unproven-value controls.
    let _state = crate::test_state::TestStateGuard::new();
    let source = format!(
        "import {{styled,css}} from '@devup-ui/react';const trace=[];const Base=styled.div({{p:2}});{rules}const observe=()=>trace;"
    );
    // When: use the exact alias-enabled module/no-map surface.
    let output = compile(&source, single_css, Surface::Modules(false));
    let bucket = (!single_css).then(|| css::file_map::canonical("/composition/entry.ts"));
    let actual = execute_extracted(
        ("/composition/entry.ts", bucket.as_deref()),
        &output,
        "const e=render({target:Card,props:{ value: 4 }});JSON.stringify([observe(),values(e,'padding').map(value=>typeof value==='function'?'function':value)]);",
    );
    // Then: known original functions invoke once; unknown callables stay values.
    assert_eq!(actual, serde_json::json!([trace, padding]));
}

#[test]
fn semantic_identity_survives_when_import_discovery_follows_callback_recovery() {
    // Given: recovered callbacks and their original scoping, before rendering.
    let arena = oxc_allocator::Allocator::default();
    let mut program = crate::parsing::parse(&arena, "/composition/entry.ts", SOURCE)
        .unwrap_or_else(|error| panic!("scoping fixture: {error:?}"));
    let builder = oxc_ast::builder::AstBuilder::new(&arena);
    let option = ExtractOption::default();
    let mut inlined = crate::imported_constants::inline_constants(
        &builder,
        &mut program,
        "/composition/entry.ts",
        &option,
        None,
        crate::css_prop::CssProp::Off,
        &Default::default(),
    );
    let identities = |program: &oxc_ast::ast::Program<'_>| {
        program
            .body
            .iter()
            .filter_map(|statement| {
                let oxc_ast::ast::Statement::VariableDeclaration(declaration) = statement else {
                    return None;
                };
                Some(
                    declaration
                        .declarations
                        .iter()
                        .map(|declaration| {
                            let binding = declaration
                                .id
                                .get_binding_identifier()
                                .unwrap_or_else(|| panic!("fixture binding identifier"));
                            (binding.name.to_string(), binding.symbol_id.get())
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect::<Vec<_>>()
    };
    let before = identities(&program);
    let scoping = inlined
        .scoping
        .as_ref()
        .unwrap_or_else(|| panic!("retained scoping"))
        .clone();
    println!(
        "retained symbols={} bindings={before:?}",
        scoping.symbol_ids().count()
    );
    // When: even an empty resolver makes import discovery inspect this program.
    crate::imported_styled::read(
        &builder,
        &program,
        "/composition/entry.ts",
        &option,
        Some(&|_, _| None),
        crate::css_prop::CssProp::Off,
        &mut inlined.scoping,
    );
    let after = identities(&program);
    println!("post-import bindings={after:?}");
    // Then: the AST still belongs to the retained semantic phase.
    assert_eq!(after, before);
    assert!(std::rc::Rc::ptr_eq(
        &scoping,
        inlined
            .scoping
            .as_ref()
            .unwrap_or_else(|| panic!("post-discovery scoping"))
    ));
}
