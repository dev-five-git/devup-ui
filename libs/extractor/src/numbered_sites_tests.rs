use css::{CounterOwner, file_map::*, sparse_site::SourceFile};
use serial_test::serial;

use crate::{
    extract_style::extract_static_style::ExtractStaticStyle,
    sparse_sites::{
        SiteScope, binding_name, contains_site, counter_owner, normalize_source,
        retain_folded_owner, site_at, site_errors,
    },
};

#[test]
#[serial]
fn original_ids_remain_independent_when_seeded_and_unknown_files_share_delivery() {
    // Given: path-sorted originals collapse to fewer delivery numbers.
    reset_file_map();
    reset_canonical_map();
    set_canonical_map(std::collections::HashMap::from([
        ("child.tsx".into(), "a.tsx".into()),
        ("unknown.tsx".into(), "a.tsx".into()),
    ]));
    seed_file_numbers(&["z.tsx".into(), "child.tsx".into(), "a.tsx".into()]);
    // When: registration happens before explicitly numbered scope entry.
    let child = get_or_insert_original_id("child.tsx").unwrap_or_else(|error| panic!("{error}"));
    let unknown =
        get_or_insert_original_id("unknown.tsx").unwrap_or_else(|error| panic!("{error}"));
    let name = {
        let _scope = SiteScope::enter_numbered(unknown, "tone", &[]);
        site_at(0, 0, "tone")
            .unwrap_or_else(|| panic!("active numbered scope"))
            .variable_name("prefix-")
    };
    // Then: originals retain 0/1/2, append at 3, and never borrow delivery 0.
    assert_eq!((child, unknown), (1, 3));
    assert_eq!(get_or_insert_original_id("unknown.tsx"), Ok(3));
    assert_eq!(original_id("z.tsx"), Some(2));
    assert_eq!(get_file_num_by_filename(&canonical("child.tsx")), 0);
    assert_eq!(get_file_num_by_filename("z.tsx"), 1);
    assert_eq!(get_file_num_by_filename(&canonical("unknown.tsx")), 0);
    assert_eq!(get_file_map().len(), 2);
    assert_eq!(name, "---prefix-Sd-a");
    reset_canonical_map();
    reset_file_map();
}

#[test]
#[serial]
fn identical_source_text_never_aliases_when_unknown_filenames_are_registered() {
    // Given: two unseeded files with identical text and syntax roles.
    reset_file_map();
    let source = "tone";
    // When: each exact filename is registered before its numbered scope.
    let sites = ["first.tsx", "second.tsx"].map(|filename| {
        let id = get_or_insert_original_id(filename).unwrap_or_else(|error| panic!("{error}"));
        let _scope = SiteScope::enter_numbered(id, source, &[]);
        site_at(0, 0, source).unwrap_or_else(|| panic!("active numbered scope"))
    });
    // Then: distinct D9 IDs, not a source-text fallback, own the variable names.
    assert_eq!(sites[0].file, SourceFile::D9(0));
    assert_eq!(sites[1].file, SourceFile::D9(1));
    assert_eq!(sites[0].variable_name(""), "---Sa-a");
    assert_eq!(sites[1].variable_name(""), "---Sb-a");
    assert_ne!(sites[0], sites[1]);
    assert_eq!(get_file_map().len(), 0);
    reset_file_map();
}

#[test]
#[serial]
fn no_style_original_consumes_an_id_when_registered_before_scope_entry() {
    // Given: an unknown source has no styling at all.
    reset_file_map();
    let plain = get_or_insert_original_id("plain.ts").unwrap_or_else(|error| panic!("{error}"));
    {
        let _scope = SiteScope::enter_numbered(plain, "export const answer = 42", &[]);
        assert_eq!(counter_owner(), CounterOwner::D9(0));
    }
    // When: a later source with a style registers and enters.
    let styled = get_or_insert_original_id("styled.tsx").unwrap_or_else(|error| panic!("{error}"));
    let site = {
        let _scope = SiteScope::enter_numbered(styled, "tone", &[]);
        site_at(0, 0, "tone").unwrap_or_else(|| panic!("active numbered scope"))
    };
    // Then: no-style registration is retained independently of delivery allocation.
    assert_eq!(get_or_insert_original_id("plain.ts"), Ok(0));
    assert_eq!(styled, 1);
    assert_eq!(site.variable_name(""), "---Sb-a");
    assert_eq!(get_file_map().len(), 0);
    reset_file_map();
}

#[test]
#[serial]
fn numbered_positions_and_roles_normalize_once_when_folded_after_multiple_edits() {
    // Given: replacement then prelude insertion follow received BOM/CRLF bytes.
    reset_file_map();
    let id = get_or_insert_original_id("normalized.tsx").unwrap_or_else(|error| panic!("{error}"));
    let source = "\u{feff}a\r\nbcd";
    let replacement = [(4, 5, 3)];
    let prelude = [(0, 0, 10)];
    let _scope = SiteScope::enter_numbered(id, source, &[&prelude, &replacement]);
    retain_folded_owner(19, 0x12 | (1 << 31));
    retain_folded_owner(0x14 | (1 << 31), 19);
    // When: source roles are visited in reverse order, including a repeat assignment.
    let right = site_at(20, 1, "right").unwrap_or_else(|| panic!("active scope"));
    let left = site_at(20, 0, "left").unwrap_or_else(|| panic!("active scope"));
    let reused = site_at(20, 0, "left");
    // Then: edit reversal precedes BOM/CR removal and folded ownership is applied once.
    assert_eq!(normalize_source(source), "a\nbcd");
    assert_eq!((left.at, left.role, right.at, right.role), (2, 0, 2, 1));
    assert_eq!(left.variable_name(""), "---Sa-c");
    assert_eq!(right.variable_name(""), "---Sa-c-b");
    assert_eq!(reused, Some(left.clone()));
    assert!(contains_site(oxc_span::Span::new(18, 19), &left));
    assert_eq!(site_errors(), vec![]);
    reset_file_map();
}

#[test]
#[serial]
fn numbered_conflict_reports_branch_offset_when_folded_assignments_share_a_role() {
    // Given: two selected branches retain the same authored owner.
    reset_file_map();
    let id = get_or_insert_original_id("conflict.tsx").unwrap_or_else(|error| panic!("{error}"));
    let _scope = SiteScope::enter_numbered(id, "abcdefghijkl", &[]);
    retain_folded_owner(8, 2);
    retain_folded_owner(10, 2);
    let _ = site_at(8, 0, "first");
    // When: another assignment claims that owner's role in this extraction.
    let _ = site_at(10, 0, "second");
    // Then: the branch's real offset and both conflicting assignments are reported.
    let errors = site_errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].0, 10);
    assert!(errors[0].1.contains("`first`"));
    assert!(errors[0].1.contains("`second`"));
    assert_eq!(site_errors(), vec![]);
    reset_file_map();
}

#[test]
#[serial]
fn numbered_nested_scopes_restore_context_when_static_clones_are_deferred() {
    // Given: the outer scope owns folded positions, assignments and a pending error.
    reset_file_map();
    let outer_id = get_or_insert_original_id("outer.tsx").unwrap_or_else(|error| panic!("{error}"));
    let inner_id = get_or_insert_original_id("inner.tsx").unwrap_or_else(|error| panic!("{error}"));
    let outer = SiteScope::enter_numbered(outer_id, "__devupAssignment0", &[]);
    retain_folded_owner(10, 2);
    let _ = site_at(10, 0, "outer");
    let _ = site_at(10, 0, "conflict");
    let first = ExtractStaticStyle::new("color", "red", 0, None);
    // When: an inner scope constructs a deferred clone, then ends.
    let deferred = {
        let _inner = SiteScope::enter_numbered(inner_id, "inner", &[]);
        assert_eq!(counter_owner(), CounterOwner::D9(1));
        assert_eq!(binding_name(0), "__devupAssignment0");
        assert_eq!(site_at(10, 0, "inner").map(|site| site.at), Some(10));
        assert_eq!(site_errors(), vec![]);
        let style = ExtractStaticStyle::new_basic("color", "red", 0, None);
        let cloned = style.clone();
        assert_eq!(style.counter_owner, CounterOwner::D9(1));
        cloned
    };
    let restored = ExtractStaticStyle::new("color", "red", 0, None);
    // Then: outer metadata returns while the clone retains its construction owner.
    assert_eq!(counter_owner(), CounterOwner::D9(0));
    assert_eq!(binding_name(0), "__devupAssignment0_");
    assert_eq!(site_at(10, 0, "outer").map(|site| site.at), Some(2));
    assert_eq!(site_errors().len(), 1);
    assert_eq!(first.counter_owner, CounterOwner::D9(0));
    assert_eq!(restored.counter_owner, CounterOwner::D9(0));
    drop(outer);
    assert_eq!(deferred.counter_owner, CounterOwner::D9(1));
    assert_eq!(counter_owner(), CounterOwner::Inactive);
    assert_eq!(site_at(0, 0, "after"), None);
    reset_file_map();
}
