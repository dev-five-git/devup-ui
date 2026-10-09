use std::{borrow::Cow, cell::RefCell, collections::BTreeMap};

use css::{Site, sparse_site::SourceFile};

use crate::import_alias_visit::{Edit, source_offset as edited_offset};

/// Normalize received source before Devup makes any alias or evaluation edits.
pub(crate) fn normalize_source(source: &str) -> Cow<'_, str> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    if source.contains("\r\n") {
        Cow::Owned(source.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(source)
    }
}

struct SiteContext {
    file: SourceFile,
    source: String,
    edits: Vec<Vec<Edit>>,
    removed: Vec<usize>,
    folded_owners: BTreeMap<u32, u32>,
    assignments: BTreeMap<(usize, usize), String>,
    errors: Vec<(u32, String)>,
}

thread_local! {
    static SITES: RefCell<Option<SiteContext>> = const { RefCell::new(None) };
}

pub(crate) fn counter_owner() -> css::CounterOwner {
    SITES.with_borrow(|context| {
        context
            .as_ref()
            .map_or(css::CounterOwner::Inactive, |context| {
                css::CounterOwner::from_source(&context.file)
            })
    })
}

/// Own edit layers map positions back to received source; upstream maps never participate.
pub(crate) struct SiteScope(Option<SiteContext>);

impl SiteScope {
    pub(crate) fn enter(filename: &str, source: &str, edits: &[&[Edit]]) -> Self {
        Self::initialize(SourceFile::from_source(filename, source), source, edits)
    }

    #[cfg(test)]
    pub(crate) fn enter_numbered(original: u32, source: &str, edits: &[&[Edit]]) -> Self {
        Self::initialize(SourceFile::D9(original), source, edits)
    }

    fn initialize(file: SourceFile, source: &str, edits: &[&[Edit]]) -> Self {
        let mut removed: Vec<_> = source
            .bytes()
            .enumerate()
            .filter_map(|(at, byte)| (byte == b'\r').then_some(at))
            .collect();
        if source.starts_with('\u{feff}') {
            drop(removed.splice(0..0, 0..3));
        }
        let context = SiteContext {
            file,
            source: source.to_string(),
            edits: edits.iter().map(|edits| edits.to_vec()).collect(),
            removed,
            folded_owners: BTreeMap::new(),
            assignments: BTreeMap::new(),
            errors: Vec::new(),
        };
        Self(SITES.with(|sites| sites.borrow_mut().replace(context)))
    }
}

pub(crate) fn binding_name(start: u32) -> String {
    let mut name = format!(
        "__devupAssignment{}",
        crate::provenance::source_offset(start)
    );
    SITES.with_borrow(|context| {
        if let Some(context) = context {
            while context.source.contains(&name) {
                name.push('_');
            }
        }
    });
    name
}

impl Drop for SiteScope {
    fn drop(&mut self) {
        SITES.with(|sites| *sites.borrow_mut() = self.0.take());
    }
}

/// Keep naming ownership in raw AST coordinates, separate from value/error spans.
pub(crate) fn retain_folded_owner(selected: u32, owner: u32) {
    SITES.with_borrow_mut(|context| {
        if let Some(context) = context {
            let selected = crate::provenance::source_offset(selected);
            let owner = crate::provenance::source_offset(owner);
            let owner = context.folded_owners.get(&owner).copied().unwrap_or(owner);
            if selected != owner {
                context.folded_owners.insert(selected, owner);
            }
        }
    });
}

/// Check assignments exactly within this extraction, never between legitimate environments.
pub(crate) fn site_at(start: u32, role: usize, assignment: &str) -> Option<Site> {
    SITES.with(|sites| {
        sites.borrow_mut().as_mut().map(|context| {
            const { assert!(usize::BITS >= u32::BITS) };
            let raw_start = super::provenance::source_offset(start);
            let owner = context
                .folded_owners
                .get(&raw_start)
                .copied()
                .unwrap_or(raw_start);
            #[expect(
                clippy::expect_used,
                reason = "supported native and wasm32 targets hold every Oxc u32 offset"
            )]
            let start_at = usize::try_from(owner)
                .expect("Oxc u32 offset fits the target's usize");
            let at = context.edits.iter().fold(
                start_at,
                |at, edits| edited_offset(edits, at),
            );
            let at = at - context.removed.partition_point(|removed| *removed < at);
            match context.assignments.entry((at, role)) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(assignment.to_string());
                }
                std::collections::btree_map::Entry::Occupied(entry) => {
                    if entry.get() != assignment {
                        context.errors.push((start, crate::utils::build_time_error(
                            "dynamic style",
                            assignment,
                            &format!("its source role {role} at offset {at} also assigns `{}`; Devup must preserve distinct source roles", entry.get()),
                        )));
                    }
                }
            }
            Site { file: context.file.clone(), at, role }
        })
    })
}

pub(crate) fn site_errors() -> Vec<(u32, String)> {
    SITES.with(|sites| {
        sites
            .borrow_mut()
            .as_mut()
            .map_or_else(Vec::new, |context| std::mem::take(&mut context.errors))
    })
}

pub(crate) fn contains_site(span: oxc_span::Span, site: &Site) -> bool {
    SITES.with_borrow(|context| {
        context.as_ref().is_some_and(|context| {
            let received = |offset| {
                usize::try_from(crate::provenance::source_offset(offset))
                    .ok()
                    .map(|at| {
                        let at = context
                            .edits
                            .iter()
                            .fold(at, |at, edits| edited_offset(edits, at));
                        at - context.removed.partition_point(|removed| *removed < at)
                    })
            };
            received(span.start)
                .zip(received(span.end))
                .is_some_and(|(start, end)| (start..end).contains(&site.at))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
