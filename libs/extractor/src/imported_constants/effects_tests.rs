use super::*;
use crate::imported_constants::exact_tests::{extracted, static_values};
use rstest::rstest;

#[rstest]
#[case(ChangeSite::Here(5), ChangeSite::Here(3), true)]
#[case(ChangeSite::Here(3), ChangeSite::Here(5), false)]
#[case(ChangeSite::Here(3), ChangeSite::Here(3), false)]
#[case(ChangeSite::In("b:1:1".into()), ChangeSite::In("a:1:1".into()), true)]
#[case(ChangeSite::In("a:1:1".into()), ChangeSite::In("b:1:1".into()), false)]
#[case(ChangeSite::In("a:1:1".into()), ChangeSite::In("a:1:1".into()), false)]
#[case(ChangeSite::In("a:1:1".into()), ChangeSite::Here(99), true)]
#[case(ChangeSite::Here(99), ChangeSite::In("a:1:1".into()), false)]
fn earliest_cause_when_sites_compete_preserves_the_location_policy(
    #[case] previous: ChangeSite,
    #[case] next: ChangeSite,
    #[case] replaces: bool,
) {
    // Given
    let previous = Rc::new(Change {
        name: "made".into(),
        site: previous,
        handed: true,
    });
    let next = Rc::new(Change {
        name: "made".into(),
        site: next,
        handed: false,
    });
    let mut changes = FxHashMap::from_iter([("made".into(), Some(previous.clone()))]);
    // When
    let updated = ModuleScope::keep_first(&mut changes, "made", next.clone());
    // Then
    assert_eq!(updated, replaces);
    assert!(Rc::ptr_eq(
        changes["made"]
            .as_ref()
            .unwrap_or_else(|| panic!("retained cause")),
        if replaces { &next } else { &previous }
    ));
}

#[rstest]
#[case("const made=Object.freeze({ p:1 });const alias=made;watch(alias);css({ p:made.p });")]
#[case("const made={ p:1 };const alias=made;Object.freeze(alias);watch(alias);css({ p:made.p });")]
#[case(
    "const made={ p:1 };const alias=made;const other=alias;Object.freeze(other);watch(made);css({ p:made.p });"
)]
#[case(
    "const scalar=()=>1;const made=Object.freeze({ p:1,child:{ p:2 },extra:scalar() });watch(made);css({ p:made.p });"
)]
#[case("const made=Object.freeze({ p:1 });const box={ made };watch(box);css({ p:made.p });")]
#[case("let other;watch(other);const made=Object.freeze({ p:1 });watch(made);css({ p:made.p });")]
#[serial_test::serial]
fn alias_escape_when_the_identity_is_frozen_beforehand_preserves_exactness(#[case] body: &str) {
    // Given
    let source = format!("import {{css}} from '@devup-ui/react';{body}");
    // When
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), vec!["4px".to_string()]);
}

#[test]
#[serial_test::serial]
fn imported_alias_when_the_dependency_escapes_reports_the_imported_origin() {
    // Given
    let source = "export const made={ p:1 };const alias=made;\nwatch(alias);";
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
    let mut scope = ModuleScope::new("/src/values.ts", &parsed.program, Some(source));
    let option = crate::ExtractOption::default();
    let mut modules = Modules {
        resolver: None,
        option: &option,
        exports: FxHashMap::default(),
        loading: Vec::new(),
    };
    // When
    let change = scope
        .change(&mut modules, "made")
        .unwrap_or_else(|| panic!("propagated imported cause"));
    // Then
    assert_eq!(change.name, "made");
    assert_eq!(change.site, ChangeSite::In("/src/values.ts:2:7".into()));
}

#[test]
#[serial_test::serial]
fn dependency_cause_when_aliases_share_a_hazard_uses_the_binding_name_tie_break() {
    // Given
    let source = "import { css } from '@devup-ui/react';const z={ p:1 };const a={ p:2 };const box={ z,a };watch(box);const f=()=>z.p+a.p;css({ p:f() });";
    // When
    let error = extracted(source, "")
        .err()
        .unwrap_or_else(|| panic!("escaped aliases must fail"));
    // Then
    assert!(error.contains("`a`"), "{error}");
}

#[test]
#[serial_test::serial]
fn frozen_namespace_when_nested_exports_escape_is_not_a_local_container_proof() {
    // Given
    let source = "import {css} from '@devup-ui/react';import * as values from './values';Object.freeze(values);watch(values);css({p:values.made.p});";
    // When
    let result = extracted(source, "export const made={ p: 1 };");
    // Then
    assert!(result.is_err(), "{result:?}");
}
