use crate::{with_style_sheet, with_style_sheet_mut};
use sheet::{
    cache_snapshot::{ClassMap, FileMap},
    live_checkpoint::LiveCheckpoint,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) enum Companion<T> {
    #[default]
    Unseen,
    Invalid,
    Supplied(T),
}
impl<T> Companion<T> {
    pub(super) const fn supplied(&self) -> Result<Option<&T>, sheet::snapshot6::EvidenceError> {
        match self {
            Self::Unseen => Ok(None),
            Self::Invalid => Err(sheet::snapshot6::EvidenceError::Companion),
            Self::Supplied(value) => Ok(Some(value)),
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

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Maps {
    pub(super) classes: ClassMap,
    pub(super) files: FileMap,
    originals: BTreeMap<String, u32>,
    plan: Option<BTreeSet<String>>,
}
impl Maps {
    pub(super) fn capture() -> Self {
        Self {
            classes: css::class_map::with_class_map(|map| {
                map.iter()
                    .map(|(file, slots)| {
                        (
                            file.clone(),
                            slots
                                .iter()
                                .map(|(key, slot)| (key.clone(), *slot))
                                .collect(),
                        )
                    })
                    .collect()
            }),
            files: css::file_map::get_file_map().into_iter().collect(),
            originals: css::file_map::get_original_ids(),
            plan: css::atom_hoist::atom_plan(),
        }
    }
    fn restore(self) {
        css::class_map::set_class_map(
            self.classes
                .into_iter()
                .map(|(file, slots)| (file, slots.into_iter().collect()))
                .collect(),
        );
        css::file_map::set_file_map(self.files.into_iter().collect());
        css::file_map::set_original_ids(self.originals);
        css::atom_hoist::restore_atom_plan(self.plan);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Session {
    pub(super) classes: ClassMap,
    pub(super) files: FileMap,
    before_maps: Maps,
    before_sheet: LiveCheckpoint,
    pub(super) seeds: Vec<Vec<String>>,
}
impl Session {
    pub(super) fn installed(before_maps: Maps, before_sheet: LiveCheckpoint) -> Self {
        let installed = Maps::capture();
        Self {
            classes: installed.classes,
            files: installed.files,
            before_maps,
            before_sheet,
            seeds: Vec::new(),
        }
    }
    fn retire(self) {
        self.before_maps.restore();
        with_style_sheet_mut(|sheet| self.before_sheet.restore(sheet));
        for batch in self.seeds {
            css::file_map::seed_file_numbers(&batch);
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Restore {
    pub(super) classes: Companion<ClassMap>,
    pub(super) files: Companion<FileMap>,
    pub(super) session: Option<Session>,
}
impl Restore {
    pub(super) fn retire(&mut self) {
        if let Some(session) = self.session.take() {
            session.retire();
        }
    }
    pub(super) fn before() -> (Maps, LiveCheckpoint) {
        (Maps::capture(), with_style_sheet(LiveCheckpoint::capture))
    }
}
