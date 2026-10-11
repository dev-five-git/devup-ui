use rstest::rstest;

use super::super::{api_policy, apis::Apis, plan::EscapeKind};
use super::{Allocator, Parser, SemanticBuilder, SourceType, names, selected, text};
use crate::vanilla_extract::Stylesheet;

#[test]
fn declared_alias_does_not_become_a_runtime_native_binding() {
    let source = "import {style} from '@vanilla-extract/css';declare const erased:typeof style;const box=style({ padding:1 });";
    let plan = selected(source);
    assert_eq!(names(&plan.units), vec!["box"]);
    assert_eq!(plan.native_calls.len(), 1);
    assert_eq!(plan.native_calls[0].api, "style");
    assert_eq!(plan.escapes.len(), 0);
    assert_eq!(plan.provenance_errors, vec![]);
}

#[test]
fn array_alias_does_not_invent_a_named_api_from_a_namespace() {
    let source =
        "import * as ns from '@vanilla-extract/css';\nconst [make]=ns;make({ padding:1 });";
    let plan = selected(source);
    assert_eq!(plan.native_calls.len(), 0);
    assert_eq!(plan.roots.len(), 0);
    assert_eq!(plan.escapes.len(), 1);
    assert_eq!(plan.escapes[0].kind, EscapeKind::NativeValue);
    assert_eq!(text(source, plan.escapes[0].span), "ns");
    let result = crate::ordinary_ve::execution::policy::check(
        Stylesheet {
            filename: "/array.ts",
            code: source,
            source,
            edits: &[],
        },
        &plan,
    );
    assert_eq!(result, Err("/array.ts:2:14: native API binding `ns` may be changed outside its exact initialization slice. Fix: use an unambiguous immutable native API binding and keep its exact reads in initializers".into()));
}

#[rstest]
#[case("ns[key]", true)]
#[case("ns.missing", false)]
#[case("ns['missing']", false)]
fn handed_off_namespace_members_keep_dynamic_mutation_and_missing_member_policies_distinct(
    #[case] read: &str,
    #[case] dynamic: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = format!("import * as ns from '@vanilla-extract/css';\nconsume({read});");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let apis = Apis::for_package(&parsed.program, &built.semantic, "@vanilla-extract/css");
    let checked = api_policy::errors(&parsed.program, &apis);
    assert_eq!(checked.failure, None);
    if dynamic {
        assert_eq!(checked.errors.len(), 1);
        assert_eq!(
            crate::locate(
                "/policy.ts",
                &source,
                usize::try_from(checked.errors[0].0.start)?
            ),
            "/policy.ts:2:9"
        );
        assert_eq!(
            checked.errors[0].1,
            "native API binding `ns` may be changed outside its exact initialization slice"
        );
    } else {
        assert_eq!(checked.errors, vec![]);
    }
    let plan = super::super::select(&parsed.program, &built.semantic);
    let result = crate::ordinary_ve::execution::policy::check(
        Stylesheet {
            filename: "/policy.ts",
            code: &source,
            source: &source,
            edits: &[],
        },
        &plan,
    );
    let expected = if dynamic {
        "/policy.ts:2:9: native API binding `ns` may be changed outside its exact initialization slice. Fix: use an unambiguous immutable native API binding and keep its exact reads in initializers"
    } else {
        "/policy.ts:2:9: a native styling API escapes its exact initialization slice. Fix: call the API in an exact initializer instead of exporting or handing off the API"
    };
    assert_eq!(result, Err(expected.into()));
    Ok(())
}

#[rstest]
#[case("import type {style} from '@vanilla-extract/css';")]
#[case("import {type style} from '@vanilla-extract/css';")]
#[case("import type * as style from '@vanilla-extract/css';")]
fn type_only_native_import_with_a_same_named_runtime_binding_reports_only_its_value_read(
    #[case] import: &str,
) {
    let source = format!(
        "{import}\ntype Native=typeof style;\nconst style=()=>0;\nconst box=style({{padding:1}});"
    );
    let plan = selected(&source);
    assert_eq!(plan.provenance_errors.len(), 1);
    assert_eq!(text(&source, plan.provenance_errors[0].0), "style");
    assert_eq!(
        plan.provenance_errors[0].1,
        "native API input comes from a type-only import"
    );
    let result = crate::ordinary_ve::execution::policy::check(
        Stylesheet {
            filename: "/types.ts",
            code: &source,
            source: &source,
            edits: &[],
        },
        &plan,
    );
    assert_eq!(result, Err("/types.ts:4:11: native API input comes from a type-only import. Fix: use an unambiguous immutable native API binding and keep its exact reads in initializers".into()));
}
