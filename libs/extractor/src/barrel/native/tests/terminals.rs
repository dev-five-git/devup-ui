use rstest::rstest;

use super::{Shape, TestResult, facts, resolver, walker};

#[test]
fn star_names_terminate_when_a_cycle_also_exports_a_real_native_terminal() -> TestResult {
    // Given
    let modules = [
        (
            "./a",
            "/a.ts",
            "export * from './b';export {style} from '@vanilla-extract/css';",
        ),
        ("./b", "/b.ts", "export * from './a';export default 13;"),
    ];
    // When
    let result = facts("import * as native from './a';", &modules);
    // Then
    let shape = result.bindings.values().next().ok_or("native namespace")?;
    assert_eq!(shape.paths(), vec![vec!["style".to_string()]]);
    assert!(matches!(
        shape.member("style").as_deref(),
        Some(Shape::Api("style"))
    ));
    assert_eq!(shape.member("default").map(|shape| shape.paths()), None);
    assert_eq!(
        result.dependencies.into_iter().collect::<Vec<_>>(),
        ["/a.ts", "/b.ts"]
    );
    Ok(())
}

#[test]
fn recursive_namespace_keeps_an_exact_api_when_only_the_backedge_member_fails() -> TestResult {
    // Given
    let modules = [
        (
            "./a",
            "/a.ts",
            "export * as nested from './b';export {style} from '@vanilla-extract/css';",
        ),
        ("./b", "/b.ts", "export * as back from './a';"),
    ];
    // When
    let result = facts("import * as native from './a';", &modules);
    // Then
    let shape = result.bindings.values().next().ok_or("native namespace")?;
    let nested = shape.member("nested").ok_or("namespace export")?;
    let back = nested.member("back").ok_or("recursive member")?;
    assert!(matches!(back.as_ref(), Shape::Failed(cause)
        if cause == "recursive native namespace needs an exact terminal member"));
    assert!(matches!(
        shape.member("style").as_deref(),
        Some(Shape::Api("style"))
    ));
    assert_eq!(
        shape.paths(),
        vec![
            vec!["nested".to_string(), "back".to_string()],
            vec!["style".to_string()]
        ]
    );
    assert_eq!(
        result.dependencies.into_iter().collect::<Vec<_>>(),
        ["/a.ts", "/b.ts"]
    );
    Ok(())
}

#[rstest]
#[case("export {style as make} from './missing';")]
#[case("export * from './missing';")]
fn absent_candidate_does_not_become_native_when_an_ordinary_barrel_cannot_resolve_it(
    #[case] code: &str,
) {
    // Given
    let modules = [("./api", "/ordinary.ts", code)];
    let resolver = resolver(&modules);
    let mut walker = walker(&resolver);
    // When
    let terminal = walker.native_import("./api", "/entry.ts", Some("make"));
    // Then
    assert!(matches!(terminal, super::Terminal::Absent));
    assert_eq!(
        walker.dependencies.into_iter().collect::<Vec<_>>(),
        ["/ordinary.ts"]
    );
}

#[rstest]
#[case(Shape::Api("style"))]
#[case(Shape::Failed("unreadable export".into()))]
#[case(Shape::OriginalFailure("/api.ts:2:3: changed API. Fix: retain immutable binding".into()))]
fn terminal_shape_has_no_nested_member_when_its_path_is_already_a_leaf(#[case] shape: Shape) {
    // Given shape above
    // When
    let member = shape.member("style");
    // Then
    assert_eq!(member.map(|shape| shape.paths()), None);
    assert_eq!(shape.paths(), vec![Vec::<String>::new()]);
}

#[test]
fn changed_ordinary_alias_retains_its_origin_when_the_binding_is_not_a_native_api() {
    // Given
    let modules = [
        (
            "./api",
            "/ordinary-alias.ts",
            "import {plain} from './ordinary';const alias=plain;alias.extra=1;export {alias};",
        ),
        (
            "./ordinary",
            "/ordinary-value.ts",
            concat!("export const plain={", "value:13", "};"),
        ),
    ];
    let resolver = resolver(&modules);
    let mut walker = walker(&resolver);
    // When
    let terminal = walker.native_import("./api", "/entry.ts", Some("alias"));
    // Then
    assert!(
        matches!(terminal, super::Terminal::Binding { owner, name, api: None }
        if owner == "/ordinary-value.ts" && name == "plain")
    );
    assert_eq!(
        walker.dependencies.into_iter().collect::<Vec<_>>(),
        ["/ordinary-alias.ts", "/ordinary-value.ts"]
    );
}
