//! Dormant Rust-only raw6 protocol; public JavaScript still dispatches cache4.
use crate::{
    cache6_session::{Companion, Restore, Session},
    with_style_sheet, with_style_sheet_mut,
};
use sheet::{
    cache_snapshot::{ClassMap, FileMap},
    snapshot6::{self, EvidenceError},
};
use std::cell::RefCell;

thread_local! {
    pub(super) static RESTORE: RefCell<Restore> = RefCell::new(Restore::default());
}

fn admitted<T>(build: impl FnOnce() -> Result<T, EvidenceError>) -> Result<T, EvidenceError> {
    css::admission::with_admission(|| {
        css::admission::check_administration_allowed().map_err(EvidenceError::ActiveExact)?;
        build()
    })
}

const fn cold(result: Result<(), EvidenceError>) -> Result<(), EvidenceError> {
    match result {
        Err(error @ EvidenceError::ActiveExact(_)) => Err(error),
        Ok(())
        | Err(
            EvidenceError::Schema
            | EvidenceError::Map
            | EvidenceError::State
            | EvidenceError::Configuration
            | EvidenceError::Cleanup
            | EvidenceError::Companion
            | EvidenceError::Kernel(_),
        ) => Ok(()),
    }
}

/// Adopt only strict admitted raw6; malformed candidates remain silent cold.
/// # Errors
/// Returns typed active-exact rejection before any session or live mutation.
pub fn import(bytes: &[u8]) -> Result<(), EvidenceError> {
    admitted(|| {
        if let Err(error) = with_style_sheet(snapshot6::check_install_target) {
            return cold(Err(error));
        }
        RESTORE.with_borrow_mut(|state| {
            let candidate = snapshot6::parse(bytes)
                .and_then(snapshot6::validate_snapshot)
                .and_then(|candidate| {
                    candidate
                        .compare_companions(state.classes.supplied()?, state.files.supplied()?)?;
                    Ok(candidate)
                });
            match candidate {
                Err(error) => {
                    state.retire();
                    cold(Err(error))
                }
                Ok(candidate) => {
                    state.retire();
                    let (maps, sheet) = Restore::before();
                    match with_style_sheet_mut(|target| candidate.install_into(target)) {
                        Ok(()) => {
                            state.session = Some(Session::installed(maps, sheet));
                            state.classes = Companion::Unseen;
                            state.files = Companion::Unseen;
                            Ok(())
                        }
                        Err(error) => cold(Err(error)),
                    }
                }
            }
        })
    })
}

/// Corroborate the original snapshot's class map, not later live additions.
/// # Errors
/// Returns typed active-exact rejection before companion or session mutation.
pub fn classes(incoming: Option<ClassMap>) -> Result<(), EvidenceError> {
    admitted(|| {
        if let Err(error) = with_style_sheet(snapshot6::check_install_target) {
            return cold(Err(error));
        }
        RESTORE.with_borrow_mut(|state| {
            let incoming = Companion::from(incoming);
            match &state.session {
                Some(session) if incoming.matches(&session.classes) => {
                    state.classes = Companion::Unseen;
                }
                Some(_) => {
                    state.retire();
                    state.classes = incoming;
                }
                None => state.classes = incoming,
            }
            Ok(())
        })
    })
}

/// Corroborate the original snapshot's file map, not later live additions.
/// # Errors
/// Returns typed active-exact rejection before companion or session mutation.
pub fn files(incoming: Option<FileMap>) -> Result<(), EvidenceError> {
    admitted(|| {
        if let Err(error) = with_style_sheet(snapshot6::check_install_target) {
            return cold(Err(error));
        }
        RESTORE.with_borrow_mut(|state| {
            let incoming = Companion::from(incoming);
            match &state.session {
                Some(session) if incoming.matches(&session.files) => {
                    state.files = Companion::Unseen;
                }
                Some(_) => {
                    state.retire();
                    state.files = incoming;
                }
                None => state.files = incoming,
            }
            Ok(())
        })
    })
}

/// Record one seed call without flattening batches or selecting configuration.
/// # Errors
/// Returns typed active-exact rejection before journal mutation.
pub fn seeded(files: &[String]) -> Result<(), EvidenceError> {
    admitted(|| {
        record_seed(files);
        Ok(())
    })
}

/// Forget the protocol at an existing build reset, without replaying old data.
/// # Errors
/// Returns typed active-exact rejection before session mutation.
pub fn clear() -> Result<(), EvidenceError> {
    admitted(|| {
        forget();
        Ok(())
    })
}

pub(crate) fn record_seed(files: &[String]) {
    RESTORE.with_borrow_mut(|state| {
        if let Some(session) = &mut state.session {
            session.seeds.push(files.to_vec());
        }
    });
}

pub(crate) fn forget() {
    RESTORE.with_borrow_mut(|state| *state = Restore::default());
}
