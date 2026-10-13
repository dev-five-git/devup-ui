use rstest::rstest;

use super::super::plan::EscapeKind;
use super::{Allocator, Parser, SemanticBuilder, SourceType, names, selected, text};

#[test]
fn interface_computed_call_does_not_become_an_initialization_owner() {
    let source = "import {style} from '@vanilla-extract/css';interface Host {[runtime()]:string}const box=style({color:'violet'});";

    let plan = selected(source);

    assert_eq!(names(&plan.units), vec!["box"]);
    assert_eq!(plan.native_calls.len(), 1);
    assert_eq!(
        text(source, plan.native_calls[0].span),
        "style({color:'violet'})"
    );
    assert_eq!(
        plan.reads
            .iter()
            .map(|read| read.name.as_str())
            .collect::<Vec<_>>(),
        vec!["style"]
    );
}

#[test]
fn closure_keeps_native_dependency_without_activating_an_unused_object_callback() {
    let source = "import {style} from '@vanilla-extract/css';const browser=window.document;const box={className:style({color:'violet'}),unused:()=>browser};";

    let plan = selected(source);

    assert_eq!(names(&plan.units), vec!["box"]);
    assert_eq!(
        plan.reads
            .iter()
            .map(|read| read.name.as_str())
            .collect::<Vec<_>>(),
        vec!["style"]
    );
    assert_eq!(plan.escapes.len(), 0);
}

#[test]
fn native_helper_inside_erased_interface_has_no_consumable_runtime_owner() {
    let source = concat!(
        "import {style} from '@vanilla-extract/css';\ninterface Host {\n",
        "[(()=>{function make(){return style({color:'red'})}return make()})()]:string\n}\n",
        "const box=style({color:'violet'});"
    );

    let plan = selected(source);

    assert_eq!(names(&plan.units), vec!["box"]);
    assert_eq!(names(&plan.consumed), Vec::<&str>::new());
    assert_eq!(plan.native_calls.len(), 1);
    assert_eq!(
        text(source, plan.native_calls[0].span),
        "style({color:'violet'})"
    );
    assert_eq!(plan.escapes.len(), 0);
    let result = crate::ordinary_ve::execution::policy::check(
        crate::vanilla_extract::Stylesheet {
            filename: "/interface.ts",
            code: source,
            source,
            edits: &[],
        },
        &plan,
    );
    assert_eq!(result, Ok(()));
}

#[test]
fn exported_native_helper_has_a_runtime_escape_at_its_real_declaration() {
    let source = "import {style} from '@vanilla-extract/css';\nexport function make(){return style({color:'violet'})}";

    let plan = selected(source);

    assert_eq!(names(&plan.consumed), Vec::<&str>::new());
    assert_eq!(plan.escapes.len(), 2);
    assert_eq!(text(source, plan.escapes[0].span), "make");
    assert_eq!(plan.escapes[0].kind, EscapeKind::RuntimeHelper);
    let result = crate::ordinary_ve::execution::policy::check(
        crate::vanilla_extract::Stylesheet {
            filename: "/helper.ts",
            code: source,
            source,
            edits: &[],
        },
        &plan,
    );
    assert_eq!(text(source, plan.escapes[1].span), "style");
    assert_eq!(plan.escapes[1].kind, EscapeKind::NativeValue);
    assert_eq!(result, Err("/helper.ts:2:17: a native styling helper remains reachable at runtime. Fix: call the helper only from exact initializers and export the computed result".into()));
}

#[rstest]
#[case("ns.css({color:'violet'})")]
#[case("ns['css']({color:'violet'})")]
fn ordinary_namespace_member_is_not_reported_as_a_native_escape(#[case] read: &str) {
    let source = format!("import * as ns from '@devup-ui/react';{read};");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);

    let plan =
        super::super::select_for_package(&parsed.program, &built.semantic, "@devup-ui/react");

    assert_eq!(plan.escapes.len(), 0);
    assert_eq!(plan.checks.len(), 0);
    assert_eq!(plan.roots.len(), 0);
}
