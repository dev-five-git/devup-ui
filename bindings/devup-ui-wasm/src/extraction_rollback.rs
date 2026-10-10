use crate::{with_style_sheet, with_style_sheet_mut};
pub(crate) use sheet::live_checkpoint::LiveCheckpoint as SheetData;

/// Outside CSS exact ownership and inside admission, so CSS abort runs first.
pub(crate) struct ExtractionRollback(Option<SheetData>);

impl ExtractionRollback {
    pub(crate) fn capture() -> Self {
        Self(Some(with_style_sheet(SheetData::capture)))
    }

    pub(crate) fn commit(mut self) {
        self.0 = None;
    }
}

impl Drop for ExtractionRollback {
    fn drop(&mut self) {
        if let Some(before) = self.0.take() {
            with_style_sheet_mut(|sheet| before.restore(sheet));
        }
    }
}
