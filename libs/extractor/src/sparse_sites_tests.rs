use super::*;
use crate::compiler_policy::tests::{ObserveScope, associated, failed, input, offset, reset};
use crate::extract_style::compiler_receipts::{PRODUCTIONS, ProducedData};
use serial_test::serial;
type Early = Box<dyn FnMut()>;
thread_local! {
    pub(crate) static EARLY: std::cell::RefCell<Option<Early>> = const { std::cell::RefCell::new(None) };
}
pub(crate) fn observe_variable() {
    EARLY.with_borrow_mut(|observer| {
        if let Some(observer) = observer {
            observer();
        }
    });
}
pub(crate) struct EarlyScope;
impl Drop for EarlyScope {
    fn drop(&mut self) {
        EARLY.set(None);
    }
}

#[test]
#[serial]
fn early_stylex_variable_observation_reserves_nothing_then_binds_real_order() {
    reset();
    let seen = std::rc::Rc::new(std::cell::Cell::new(false));
    let observed = seen.clone();
    EARLY.set(Some(Box::new(move || {
        if !observed.replace(true) {
            assert_eq!(PRODUCTIONS.get(), 0);
            assert_eq!(
                css::class_map::get_class_map(),
                std::collections::HashMap::new()
            );
        }
    })));
    let _scope = EarlyScope;
    let (_, _, receipts, graph) = associated(input(
        "A.tsx",
        "import * as stylex from '@stylexjs/stylex'; const styles=stylex.create({base:(height)=>({height})}); const a=stylex.props(styles.base(value));",
    ));
    assert!(seen.get());
    assert_eq!(PRODUCTIONS.get(), 1);
    assert_eq!(graph.variables[0].receipts, [receipts[0].id]);
    assert!(
        matches!(&receipts[0].produced, ProducedData::Dynamic(value) if value.variable_allocation.is_none())
    );
}
#[test]
#[serial]
fn terminal_roles_keep_bom_crlf_alias_edit_coordinates() {
    reset();
    let source = "\u{feff}import {css} from '@emotion/react';\r\nconst pick=()=> 'red';\r\nconst s=css({color:pick()});";
    let option = crate::ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".into(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..Default::default()
    };
    let normalized = crate::provenance::normalize_source(source);
    let (transformed, _) = crate::import_alias_visit::transform_import_aliases_with_edits(
        &normalized,
        "aliases.tsx",
        &option.package,
        &option.import_aliases,
    );
    let at = offset(&transformed, "pick()");
    let _observer = ObserveScope::enter(move |retry| {
        assert!(!retry);
        crate::provenance::site_at(at, 0, "first");
        crate::provenance::site_at(at, 0, "second");
    });
    let mut source = input("aliases.tsx", source);
    source.option = option;
    source.source_map = true;
    let error = failed(source);
    assert!(error.to_string().contains("aliases.tsx:3:20"), "{error}");
}

#[test]
fn folded_owner_normalizes_once_when_nested_folds_have_marked_spans() {
    // Given: raw AST offsets include a prelude, BOM and CRLF.
    let edits = [(0, 0, 10)];
    let _scope = SiteScope::enter("/source.tsx", "\u{feff}a\r\nbcd", &[&edits]);
    retain_folded_owner(17, 0x10 | (1 << 31));
    retain_folded_owner(0x12 | (1 << 31), 17);
    // When: the final branch supplies two source-ordered roles.
    let first = site_at(18, 0, "left").unwrap_or_else(|| panic!("active scope"));
    let second = site_at(18, 1, "right").unwrap_or_else(|| panic!("active scope"));
    // Then: both use the outer owner, normalized once, with distinct roles.
    assert_eq!((first.at, first.role), (2, 0));
    assert_eq!((second.at, second.role), (2, 1));
    assert_ne!(first.variable_name(""), second.variable_name(""));
    assert_eq!(site_errors(), vec![]);
}

#[test]
fn folded_owners_restore_when_nested_source_scope_ends() {
    // Given: an outer extraction has retained a selected branch's owner.
    let _outer = SiteScope::enter("/source.tsx", "abcdefghijkl", &[]);
    retain_folded_owner(10, 2);
    // When: a nested extraction records a different owner and ends.
    {
        let _inner = SiteScope::enter("/source.tsx", "abcdefghijkl", &[]);
        retain_folded_owner(10, 5);
        assert_eq!(site_at(10, 0, "inner").map(|site| site.at), Some(5));
    }
    // Then: the original extraction's metadata is restored, not leaked.
    assert_eq!(site_at(10, 0, "outer").map(|site| site.at), Some(2));
    assert_eq!(site_errors(), vec![]);
}

#[test]
fn folded_alias_error_keeps_branch_location_when_assignments_conflict() {
    // Given: two selected spans retain one authored owner and role.
    let _scope = SiteScope::enter("/source.tsx", "abcdefghijkl", &[]);
    retain_folded_owner(8, 2);
    retain_folded_owner(10, 2);
    let _ = site_at(8, 0, "first");
    // When: the other branch claims a distinct assignment in this extraction.
    let _ = site_at(10, 0, "second");
    // Then: the unchanged alias guard reports the real offending branch offset.
    let errors = site_errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].0, 10);
    assert!(errors[0].1.contains("`first`"));
    assert!(errors[0].1.contains("`second`"));
}

#[test]
fn positions_normalize_after_own_edits_when_cr_and_bom_precede_the_site() {
    // Given: a received BOM/CRLF source and an inserted Devup prelude.
    let source = "\u{feff}a\r\nb";
    let edits = [(0, 0, 10)];
    let _scope = SiteScope::enter("/source.tsx", source, &[&edits]);
    // When: an assignment points past the inserted prelude to `b`.
    let site = site_at(16, 0, "b").unwrap_or_else(|| panic!("scope provides a site"));
    // Then: only original-source LF bytes count, not the prelude, BOM or CR.
    assert_eq!(site.at, 2);
}

#[test]
fn exact_check_reports_collision_when_distinct_assignments_claim_one_role() {
    // Given: one span intentionally loses its role discriminator.
    let _scope = SiteScope::enter("/source.tsx", "value", &[]);
    let _ = site_at(0, 0, "first");
    // When: a different assignment claims that same source role.
    let _ = site_at(0, 0, "second");
    // Then: the collision is an error identifying both assignments, not silent reuse.
    let errors = site_errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].0, 0);
    assert!(errors[0].1.contains("`first`"));
    assert!(errors[0].1.contains("`second`"));
}

#[test]
fn exact_check_allows_reuse_when_assignment_and_role_agree() {
    // Given: the same assignment consumed under multiple selectors.
    let _scope = SiteScope::enter("/source.tsx", "value", &[]);
    let first = site_at(0, 0, "value");
    // When: that role is reused and a distinct source role assigns another value.
    let again = site_at(0, 0, "value");
    let other = site_at(0, 1, "other");
    // Then: reuse is legal, distinct roles stay disjoint, and no collision is reported.
    assert_eq!(first, again);
    assert_ne!(first, other);
    assert_eq!(site_errors(), vec![]);
}

#[test]
fn sites_are_absent_when_scope_has_ended() {
    // Given: no active source scope.
    // When: a legacy caller asks for a site or pending errors.
    retain_folded_owner(10, 2);
    let site = site_at(0, 0, "value");
    // Then: no arrival-based identity is invented.
    assert_eq!(site, None);
    assert_eq!(site_errors(), vec![]);
}

#[test]
fn source_roles_keep_their_names_when_extraction_order_reverses() {
    // Given: role indices written in source order, regardless of which becomes dynamic.
    let names = |reverse| {
        let _scope = SiteScope::enter("/source.tsx", "value", &[]);
        let roles = if reverse { [2, 1] } else { [1, 2] };
        roles
            .into_iter()
            .map(|role| {
                let site = site_at(0, role, if role == 1 { "left" } else { "right" })
                    .unwrap_or_else(|| panic!("scope provides a site"));
                (role, site.variable_name(""))
            })
            .collect::<BTreeMap<_, _>>()
    };
    // When: the two roles are emitted in opposite orders.
    let forward = names(false);
    let reverse = names(true);
    // Then: the source role, not an arrival sub-counter, owns each name.
    assert_eq!(forward, reverse);
    assert_ne!(forward[&1], forward[&2]);
}
