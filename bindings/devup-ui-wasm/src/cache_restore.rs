use std::{cell::RefCell, collections::BTreeMap};

use sheet::{
    StyleSheet,
    cache_snapshot::{CacheRestore, ClassMap, FileMap},
};

use crate::{administration::with_administration, cache_names, with_style_sheet_mut};

#[path = "cache_allocator_proof.rs"]
mod allocator;

struct Authority {
    classes: ClassMap,
    files: FileMap,
    before: StyleSheet,
    before_classes: ClassMap,
    before_files: FileMap,
    before_sources: BTreeMap<String, u32>,
    before_plan: Option<std::collections::BTreeSet<String>>,
    fresh_seeds: Vec<Vec<String>>,
}

#[derive(Default)]
enum Companion<T> {
    #[default]
    Unseen,
    Invalid,
    Supplied(T),
}

impl<T: PartialEq> Companion<T> {
    fn matches(&self, expected: &T) -> bool {
        match self {
            Self::Unseen => true,
            Self::Invalid => false,
            Self::Supplied(value) => value == expected,
        }
    }
}

impl<T> From<Option<T>> for Companion<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Supplied(value),
            None => Self::Invalid,
        }
    }
}

#[derive(Default)]
struct Restore {
    classes: Companion<ClassMap>,
    files: Companion<FileMap>,
    authority: Option<Authority>,
}

thread_local! {
    static RESTORE: RefCell<Restore> = RefCell::new(Restore::default());
}

pub(crate) fn absent() -> StyleSheet {
    let mut sheet = StyleSheet::default();
    sheet.cache_restore = CacheRestore::Rejected;
    sheet
}

fn set_classes(classes: ClassMap) {
    css::class_map::set_class_map(
        classes
            .into_iter()
            .map(|(file, classes)| (file, classes.into_iter().collect()))
            .collect(),
    );
}

fn set_files(files: FileMap) {
    css::file_map::set_file_map(files.into_iter().collect());
}

fn rollback(state: &mut Restore) {
    if let Some(mut authority) = state.authority.take() {
        with_style_sheet_mut(|current| {
            authority.before.theme = std::mem::take(&mut current.theme);
            *current = authority.before;
        });
        set_classes(authority.before_classes);
        set_files(authority.before_files);
        css::file_map::set_original_ids(authority.before_sources);
        css::atom_hoist::restore_atom_plan(authority.before_plan);
        for files in authority.fresh_seeds {
            css::file_map::seed_file_numbers(&files);
        }
    }
}

pub(crate) fn clear() {
    with_administration("cache_restore::clear", || {
        RESTORE.with_borrow_mut(|state| *state = Restore::default());
    });
}

pub(crate) fn seeded(files: &[String]) {
    with_administration("cache_restore::seeded", || {
        RESTORE.with_borrow_mut(|state| {
            if let Some(authority) = &mut state.authority {
                authority.fresh_seeds.push(files.to_vec());
            }
        });
    });
}

pub(crate) fn classes(incoming: Option<ClassMap>) {
    with_administration("cache_restore::classes", || {
        let incoming = Companion::from(incoming);
        RESTORE.with_borrow_mut(|state| {
            if state
                .authority
                .as_ref()
                .is_some_and(|authority| incoming.matches(&authority.classes))
            {
                state.classes = Companion::Unseen;
                return;
            }
            if state
                .authority
                .as_ref()
                .is_some_and(|authority| !incoming.matches(&authority.classes))
            {
                rollback(state);
            }
            state.classes = incoming;
        });
    });
}

pub(crate) fn files(incoming: Option<FileMap>) {
    with_administration("cache_restore::files", || {
        let incoming = Companion::from(incoming);
        RESTORE.with_borrow_mut(|state| {
            if state
                .authority
                .as_ref()
                .is_some_and(|authority| incoming.matches(&authority.files))
            {
                state.files = Companion::Unseen;
                return;
            }
            if state
                .authority
                .as_ref()
                .is_some_and(|authority| !incoming.matches(&authority.files))
            {
                rollback(state);
            }
            state.files = incoming;
        });
    });
}

pub(crate) fn import(mut incoming: StyleSheet) -> Result<(), String> {
    with_administration("cache_restore::import", || {
        let result =
            RESTORE.with_borrow_mut(|state| match std::mem::take(&mut incoming.cache_restore) {
                CacheRestore::Rejected => {
                    rollback(state);
                    Ok(())
                }
                CacheRestore::Manual => {
                    if cache_names::validate(&incoming).is_err() {
                        return Ok(());
                    }
                    with_style_sheet_mut(|current| {
                        let claims =
                            sheet::name_registry::preflight(&current.names, incoming.names.clone())
                                .map_err(|error| error.to_string())?;
                        incoming.names = current.names.clone();
                        incoming.names.extend(claims);
                        incoming.theme = std::mem::take(&mut current.theme);
                        css::atom_hoist::restore_atom_plan(incoming.atom_plan.clone());
                        *current = incoming;
                        Ok(())
                    })
                }
                CacheRestore::Serialized { classes, files } => {
                    if !allocator::validate(&incoming, &classes)
                        || cache_names::validate(&incoming).is_err()
                        || !state.classes.matches(&classes)
                        || !state.files.matches(&files)
                    {
                        rollback(state);
                        return Ok(());
                    }
                    let claims = crate::with_style_sheet(|current| {
                        sheet::name_registry::preflight(&current.names, incoming.names.clone())
                            .map_err(|error| error.to_string())
                    })?;
                    rollback(state);
                    with_style_sheet_mut(|current| {
                        incoming.names = current.names.clone();
                        incoming.names.extend(claims);
                        let before_classes = css::class_map::with_class_map(|map| {
                            map.iter()
                                .map(|(file, classes)| {
                                    (
                                        file.clone(),
                                        classes
                                            .iter()
                                            .map(|(key, id)| (key.clone(), *id))
                                            .collect(),
                                    )
                                })
                                .collect()
                        });
                        let before_files = css::file_map::with_file_map(|map| {
                            map.iter().map(|(file, id)| (file.clone(), *id)).collect()
                        });
                        let before_sources = css::file_map::get_original_ids();
                        let before_plan = css::atom_hoist::atom_plan();
                        set_classes(classes.clone());
                        set_files(files.clone());
                        css::file_map::set_original_ids(incoming.source_ids.clone());
                        css::atom_hoist::restore_atom_plan(incoming.atom_plan.clone());
                        incoming.theme = std::mem::take(&mut current.theme);
                        let before = std::mem::replace(current, incoming);
                        state.authority = Some(Authority {
                            classes,
                            files,
                            before,
                            before_classes,
                            before_files,
                            before_sources,
                            before_plan,
                            fresh_seeds: Vec::new(),
                        });
                        state.classes = Companion::Unseen;
                        state.files = Companion::Unseen;
                        Ok(())
                    })
                }
            });
        cache_names::record(&result);
        result
    })
}
