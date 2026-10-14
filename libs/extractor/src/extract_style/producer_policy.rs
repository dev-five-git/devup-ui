/// Identity policy captured when a producer constructs an extractor record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProducerPolicy {
    /// Preserve the production identity and naming contract.
    Current,
    /// Retain an original numeric owner for the dormant counter path.
    CounterOriginal(u32),
}

impl ProducerPolicy {
    pub(super) const fn declaration_owner(original: u32, order: Option<u8>) -> Option<u32> {
        match order {
            Some(0) => None,
            Some(_) | None => Some(original),
        }
    }
}
