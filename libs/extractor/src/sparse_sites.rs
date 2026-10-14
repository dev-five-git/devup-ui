use std::{borrow::Cow, cell::RefCell, collections::BTreeMap};

use css::{Site, sparse_site::SourceFile};

use crate::extract_style::ProducerPolicy;
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
    policy: ProducerPolicy,
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

pub(crate) fn producer_policy() -> ProducerPolicy {
    SITES.with_borrow(|context| {
        context
            .as_ref()
            .map_or(ProducerPolicy::Current, |context| context.policy)
    })
}
pub(crate) fn source() -> Option<String> {
    SITES.with_borrow(|sites| sites.as_ref().map(|site| site.source.clone()))
}

/// Own edit layers map positions back to received source; upstream maps never participate.
pub(crate) struct SiteScope(Option<SiteContext>);

impl SiteScope {
    pub(crate) fn enter(filename: &str, source: &str, edits: &[&[Edit]]) -> Self {
        let file = SourceFile::from_source(filename, source);
        Self::initialize((file, ProducerPolicy::Current), source, edits)
    }

    #[cfg(test)]
    pub(crate) fn enter_numbered(original: u32, source: &str, edits: &[&[Edit]]) -> Self {
        Self::initialize(
            (SourceFile::D9(original), ProducerPolicy::Current),
            source,
            edits,
        )
    }

    pub(crate) fn enter_counter_numbered(original: u32, source: &str, edits: &[&[Edit]]) -> Self {
        Self::initialize(
            (
                SourceFile::D9(original),
                ProducerPolicy::CounterOriginal(original),
            ),
            source,
            edits,
        )
    }

    pub(crate) fn suspend_counter() -> CounterSuspension {
        CounterSuspension(match producer_policy() {
            ProducerPolicy::CounterOriginal(_) => SITES.replace(None),
            ProducerPolicy::Current => None,
        })
    }

    pub(crate) fn enter_counter_generated(original: u32, source: &str) -> Self {
        Self::initialize(
            (
                SourceFile::Unnumbered("".into()),
                ProducerPolicy::CounterOriginal(original),
            ),
            source,
            &[],
        )
    }

    fn initialize(
        (file, policy): (SourceFile, ProducerPolicy),
        source: &str,
        edits: &[&[Edit]],
    ) -> Self {
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
            policy,
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

pub(crate) struct CounterSuspension(Option<SiteContext>);
impl Drop for CounterSuspension {
    fn drop(&mut self) {
        if let Some(parent) = self.0.take() {
            SITES.set(Some(parent));
        }
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
#[path = "sparse_sites_tests.rs"]
pub(crate) mod tests;
