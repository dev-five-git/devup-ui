use std::collections::HashMap;

use crate::sheet_checkpoint::SheetCheckpoint;
use bimap::BiHashMap;

/// Owned parsed transport, not certification of the exports' semantic completeness.
pub(super) struct HostCheckpoint {
    sheet: SheetCheckpoint,
    classes: HashMap<String, HashMap<String, usize>>,
    files: BiHashMap<String, usize>,
    canonical: HashMap<String, String>,
    reservation: css::slot_reservation::Snapshot,
}

impl HostCheckpoint {
    pub(super) fn parse(parts: [&str; 5]) -> Result<Self, serde_json::Error> {
        let [sheet, classes, files, canonical, reservation] = parts;
        Ok(Self {
            sheet: serde_json::from_str(sheet)?,
            classes: serde_json::from_str(classes)?,
            files: serde_json::from_str(files)?,
            canonical: serde_json::from_str(canonical)?,
            reservation: serde_json::from_str(reservation)?,
        })
    }

    pub(super) fn restore(&self) {
        self.sheet.restore();
        css::class_map::set_class_map(self.classes.clone());
        css::file_map::set_file_map(self.files.clone());
        crate::import_canonical_map_internal(self.canonical.clone());
        crate::reservation::import_reservation_state_internal(self.reservation);
    }
}
