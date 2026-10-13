use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, global_selector, reset, run, static_value};
use super::mixed_review::emitted_predicate;

#[rstest]
#[case("[]", "[]", "const [] = [];")]
#[case("{}", "{}", "const {} = {};")]
#[serial]
fn empty_native_destructuring_retains_its_pattern_kind(
    #[case] pattern: &str,
    #[case] value: &str,
    #[case] expected: &str,
) -> TestResult {
    reset();
    let code = format!(
        "import {{style}} from '@vanilla-extract/css';const {pattern}=(()=>{{style({{padding:7}});return {value}}})();throw new Error('host only');"
    );
    let output = run("/empty.ts", &code, &[])?;
    assert_eq!(static_value(&output, "padding"), "7px");
    assert!(output.code.contains(expected), "{}", output.code);
    assert!(!output.code.contains("@vanilla-extract/css"));
    Ok(())
}

#[test]
#[serial]
fn native_statement_without_bindings_becomes_void_after_emitting_global_css() -> TestResult {
    reset();
    let code = "import {globalStyle} from '@vanilla-extract/css';{globalStyle('body',{ margin:9 });}throw new Error('host only');";
    let output = run("/statement.ts", code, &[])?;
    assert_eq!(static_value(&output, "margin"), "9px");
    assert_eq!(global_selector(&output, "margin"), "body");
    assert!(output.code.contains("void 0;"), "{}", output.code);
    assert!(!output.code.contains("globalStyle"));
    Ok(())
}

#[test]
#[serial]
fn default_native_graph_runs_reconstruction_before_returning_the_value() -> TestResult {
    reset();
    let code = "import {style} from '@vanilla-extract/css';export default (()=>{style({padding:11});return Object.assign(Object.create(null),{answer:42})})();if(false){throw new Error('host only')}";
    let output = run("/default.ts", code, &[])?;
    assert_eq!(static_value(&output, "padding"), "11px");
    assert!(
        output.code.contains("export default (() => {"),
        "{}",
        output.code
    );
    let executable = output.code.replace("export default", "const result =");
    assert!(
        emitted_predicate(
            &executable,
            "Object.getPrototypeOf(result)===null && result.answer===42"
        ),
        "{}",
        output.code
    );
    Ok(())
}

#[rstest]
#[case("const a=style,b=a,keep=window.name;", "const keep = window.name;")]
#[case("const keep=window.name,a=style,b=a;", "const keep = window.name;")]
#[case(
    "const keep=window.name,a=style,b=a,last=document.title;",
    "last = document.title"
)]
#[serial]
fn consecutive_private_api_declarations_are_removed_without_removing_host_values(
    #[case] declarations: &str,
    #[case] retained: &str,
) -> TestResult {
    reset();
    let code = format!(
        "import {{style}} from '@vanilla-extract/css';{declarations}export const box=b({{padding:5}});"
    );
    let output = run("/declarations.ts", &code, &[])?;
    assert_eq!(static_value(&output, "padding"), "5px");
    assert!(output.code.contains(retained), "{}", output.code);
    assert!(!output.code.contains("a ="), "{}", output.code);
    assert!(!output.code.contains("b ="), "{}", output.code);
    Ok(())
}

#[rstest]
#[case(
    "import label,{style} from './api';const box=style({ padding:1 });console.log(label);",
    "import label from './api';"
)]
#[case(
    "import make,* as ns from './api';const box=make({ padding:1 });console.log(ns.label);",
    "import * as ns from './api';"
)]
fn native_import_removal_keeps_default_and_namespace_specifiers(
    #[case] code: &str,
    #[case] expected: &str,
) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let default_is_host = code.contains("import label");
    let resolver = move |_: &str, _: &str| {
        Some(crate::ResolvedModule {
            path: "/api.ts".into(),
            code: if default_is_host {
                "export {style} from '@vanilla-extract/css';export default 'host';"
            } else {
                "export {style as default} from '@vanilla-extract/css';export const label='host';"
            }
            .into(),
        })
    };
    let selection = super::super::selection::select_resolved(
        (&parsed.program, &built.semantic),
        ("/import.ts", "@vanilla-extract/css"),
        Some(&resolver),
    );
    let replacements = super::super::rewrite::imports(&parsed.program, code, &selection);
    assert_eq!(replacements.len(), 1);
    assert_eq!(replacements[0].text, expected);
    assert_eq!(
        Some(replacements[0].span.source_text(code).to_owned()),
        code.split_once(';').map(|(import, _)| format!("{import};"))
    );
}
