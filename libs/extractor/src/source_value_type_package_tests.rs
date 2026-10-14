use std::{cell::RefCell, rc::Rc};

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::super::{Scope, ValueType, read};
use crate::ResolvedModule;

#[rstest]
#[case("@devup-ui/react", "@devup-ui/react", ValueType::Unproven)]
#[case("@devup-ui/react", "@devup-ui/react/tokens", ValueType::Unproven)]
#[case("@custom/ui", "@custom/ui", ValueType::Unproven)]
#[case("@custom/ui", "@custom/ui/tokens", ValueType::Unproven)]
#[case("@devup-ui/react", "@devup-ui/react-values", ValueType::Number)]
#[case("@devup-ui/react", "@devup-ui/react-values/tokens", ValueType::Number)]
#[case("@custom/ui", "@custom/ui-values", ValueType::Number)]
#[case("@custom/ui", "@devup-ui/react", ValueType::Number)]
fn imported_facts_and_resolver_calls_when_package_boundary_is_checked(
    #[case] package: &str,
    #[case] specifier: &str,
    #[case] expected: ValueType,
) {
    // Given: a resolvable numeric export, even for the configured styling package.
    let source = format!("import {{size}} from '{specifier}'; useValue(size)");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&calls);
    let resolver = move |specifier: &str, importer: &str| {
        recorded
            .borrow_mut()
            .push((specifier.to_string(), importer.to_string()));
        Some(ResolvedModule {
            path: "/resolved.ts".to_string(),
            code: "export const size:number = runtime();".to_string(),
        })
    };

    // When: the real source graph captures facts using the configured package.
    let (_scope, dependencies) =
        Scope::enter(&parsed.program, "/entry.tsx", Some(&resolver), package);
    let actual = read(&parsed.program);

    // Then: skipped imports stay unproven without resolver calls or dependencies.
    assert_eq!(actual, vec![expected]);
    let (expected_calls, expected_dependencies) = match expected {
        ValueType::Number => (
            vec![(specifier.to_string(), "/entry.tsx".to_string())],
            vec!["/resolved.ts".to_string()],
        ),
        ValueType::Unproven => (vec![], vec![]),
        ValueType::String | ValueType::NonNumericString => {
            panic!("fixture expects only numeric or skipped imports")
        }
    };
    assert_eq!(*calls.borrow(), expected_calls);
    assert_eq!(
        dependencies.into_iter().collect::<Vec<_>>(),
        expected_dependencies
    );
}
