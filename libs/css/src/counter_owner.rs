/// Original counter identity, independent of canonical delivery and diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CounterOwner {
    /// No extraction context; the low-level filename contract applies.
    #[default]
    Inactive,
    /// An unnumbered original must not borrow a canonical root's counter.
    Unnumbered,
    /// The original source's predeclared D9 number.
    D9(u32),
}

impl CounterOwner {
    #[must_use]
    pub const fn from_source(source: &crate::sparse_site::SourceFile) -> Self {
        match source {
            crate::sparse_site::SourceFile::D9(id) => Self::D9(*id),
            crate::sparse_site::SourceFile::Unnumbered(_) => Self::Unnumbered,
        }
    }
}
