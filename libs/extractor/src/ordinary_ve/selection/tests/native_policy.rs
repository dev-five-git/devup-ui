use super::super::plan::{EscapeKind, ImportName, NativeBinding};
use super::super::{api_policy, apis::Apis};
use super::{Allocator, Parser, Selection, SemanticBuilder, SourceType, Span, selected, text};
use crate::{
    ResolvedModule,
    barrel::native::{Facts, Shape},
};
use oxc_ast::ast::{Expression, Statement};
use oxc_span::GetSpan;
use rstest::rstest;
use std::collections::BTreeSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FAILED: &str = "import '@vanilla-extract/css';export {style as make} from './missing';";
const ORIGINAL: &str =
    "import {style} from '@vanilla-extract/css';\nconst make=style;\nmake.extra=1;\nexport {make};";
const CHANGED: &str = "native API alias may be changed outside its exact initialization slice";

#[rstest]
#[case::failed_static(FAILED, "ns.make", false)]
#[case::failed_computed(FAILED, "ns['make']", false)]
#[case::original_static(ORIGINAL, "ns.make", true)]
#[case::original_computed(ORIGINAL, "ns['make']", true)]
fn policy_routes_member_failure_when_a_real_namespace_contains_an_invalid_export(
    #[case] bad: &str,
    #[case] read: &str,
    #[case] original: bool,
) -> TestResult {
    // Given
    let source = format!("import * as ns from './bad';\n{read}({{}});");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let bad = bad.to_string();
    let resolve = move |specifier: &str, importer: &str| {
        (specifier == "./bad" && importer == "/entry.ts").then(|| ResolvedModule {
            path: "/bad.ts".to_string(),
            code: bad.clone(),
        })
    };
    let facts = Facts::resolve(
        &parsed.program,
        ("/entry.ts", "@vanilla-extract/css"),
        Some(&resolve),
    );
    let apis = Apis::with_facts(&parsed.program, &built.semantic, facts);
    let scoping = built.semantic.scoping();
    let symbol = scoping
        .get_root_binding("ns".into())
        .ok_or("parsed namespace")?;
    let import = &apis.imports[&symbol];
    assert_eq!(apis.imports.len(), 1);
    assert_eq!(import.source, "./bad");
    assert_eq!(import.binding.name, "ns");
    assert_eq!(import.binding.span, Span::new(12, 14));
    assert_eq!(text(&source, import.binding.span), "ns");
    assert_eq!(import.specifier, Span::new(7, 14));
    assert_eq!(text(&source, import.specifier), "* as ns");
    assert_eq!(import.native, Some(NativeBinding::Namespace));
    assert!(!import.erased);
    assert!(matches!(import.imported, ImportName::Namespace));
    let Statement::ExpressionStatement(statement) = &parsed.program.body[1] else {
        panic!("authored call")
    };
    let Expression::CallExpression(call) = &statement.expression else {
        panic!("authored call")
    };
    let object = match &call.callee {
        Expression::StaticMemberExpression(member) => &member.object,
        Expression::ComputedMemberExpression(member) => &member.object,
        _ => panic!("authored member"),
    };
    let namespace = apis.shape(object).ok_or("source-derived namespace")?;
    assert!(matches!(namespace.as_ref(), Shape::Namespace(_)));
    assert_eq!(namespace.paths(), vec![vec!["make".to_string()]]);
    let member = namespace.member("make").ok_or("source-derived member")?;
    assert!(matches!(
        (member.as_ref(), original),
        (Shape::Failed(_), false) | (Shape::OriginalFailure(_), true)
    ));
    assert_eq!(apis.member(object, "make"), None);
    let span = call.callee.span();
    let expected_span = Span::new(29, if read == "ns.make" { 36 } else { 39 });
    assert_eq!(span, expected_span);
    assert_eq!(text(&source, span), read);
    assert_eq!(
        crate::locate("/entry.ts", &source, usize::try_from(span.start)?),
        "/entry.ts:2:1"
    );
    assert_eq!(apis.dependencies, BTreeSet::from(["/bad.ts".to_string()]));
    // When
    let checked = api_policy::errors(&parsed.program, &apis);
    // Then
    if original {
        assert_eq!(checked.errors.len(), 0);
        let failure = checked.failure.ok_or("original failure")?;
        assert!(failure.starts_with("/bad.ts:3:1:"), "{failure}");
        assert!(failure.contains(CHANGED), "{failure}");
        assert!(failure.contains("Fix:"), "{failure}");
        assert!(!failure.contains("/entry.ts"), "{failure}");
    } else {
        assert_eq!(checked.failure, None);
        assert_eq!(
            checked.errors,
            [(
                expected_span,
                "native re-export `./missing` cannot be read".to_string()
            )]
        );
    }
    Ok(())
}

fn assert_escape(
    plan: &Selection,
    source: &str,
    expected: (&str, Span, NativeBinding),
) -> TestResult {
    let (package, span, native) = expected;
    let local = match native {
        NativeBinding::Named { .. } => "style",
        NativeBinding::Namespace => "ns",
    };
    assert_eq!(plan.roots.len(), 0);
    assert_eq!(plan.native_calls.len(), 0);
    assert_eq!(plan.checks.len(), 0);
    assert_eq!(plan.dependencies.len(), 0);
    assert_eq!(plan.imports.len(), 1);
    let import = &plan.imports[0];
    assert_eq!(import.source, package);
    assert_eq!(import.binding.name, local);
    assert_eq!(import.native, Some(native));
    assert!(!import.erased);
    assert!(!import.preserved);
    match (&import.imported, native) {
        (ImportName::Named(name), NativeBinding::Named { api }) => assert_eq!(name, api),
        (ImportName::Namespace, NativeBinding::Namespace) => {}
        (ImportName::Default, _)
        | (ImportName::Named(_), NativeBinding::Namespace)
        | (ImportName::Namespace, NativeBinding::Named { .. }) => panic!("authored import kind"),
    }
    assert_eq!(
        plan.escapes
            .iter()
            .map(|escape| (escape.kind, escape.span, escape.symbol))
            .collect::<Vec<_>>(),
        [(EscapeKind::NativeValue, span, import.binding.symbol)]
    );
    assert_eq!(text(source, span), local);
    assert_eq!(
        crate::locate("/entry.ts", source, usize::try_from(span.start)?),
        "/entry.ts:2:1"
    );
    Ok(())
}

#[rstest]
#[case::named("import {style} from '@vanilla-extract/css';\nstyle.extra();", Span::new(44, 49), NativeBinding::Named { api: "style" })]
#[case::closed(
    "import * as ns from '@vanilla-extract/css';\nns['missing']();",
    Span::new(44, 46),
    NativeBinding::Namespace
)]
fn native_value_escapes_when_the_member_is_not_a_native_api(
    #[case] source: &str,
    #[case] span: Span,
    #[case] native: NativeBinding,
) -> TestResult {
    // Given: the authored named leaf or closed package namespace.
    // When
    let plan = selected(source);
    // Then
    assert_escape(&plan, source, ("@vanilla-extract/css", span, native))
}

#[test]
fn native_value_escapes_when_the_partial_namespace_itself_is_called() -> TestResult {
    // Given
    let source = "import * as ns from '@devup-ui/react';\nns();";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    // When
    let plan =
        super::super::select_for_package(&parsed.program, &built.semantic, "@devup-ui/react");
    // Then
    assert_escape(
        &plan,
        source,
        (
            "@devup-ui/react",
            Span::new(39, 41),
            NativeBinding::Namespace,
        ),
    )
}
