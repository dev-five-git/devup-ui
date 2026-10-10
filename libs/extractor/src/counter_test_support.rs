//! Constructor fixtures only; ordinary extraction always selects Current policy.

use crate::provenance::{SiteScope, normalize_source};

/// Exact original filename and source as received before normalization.
#[derive(Clone, Copy, Debug)]
pub struct FixtureRequest<'a> {
    /// Raw filename, without canonicalization or case folding.
    pub filename: &'a str,
    /// Received source, before alias or evaluation edits.
    pub source: &'a str,
}

/// The existing original-registration conversion error.
pub type FixtureError = std::num::TryFromIntError;

/// Construct owned IR under a registered original, restoring prior TLS on every exit.
///
/// This is not a Counter JavaScript compiler entry. Calling ordinary extraction
/// inside `build` still constructs Current IR and temporarily replaces this scope.
/// A closure returning `Result` retains its result as the successful outer value.
///
/// # Errors
/// Returns the registration error when the original registry length exceeds `u32::MAX`.
pub fn with_original<T>(
    request: FixtureRequest<'_>,
    build: impl FnOnce() -> T,
) -> Result<T, FixtureError> {
    let original = css::file_map::get_or_insert_original_id(request.filename)?;
    let normalized = normalize_source(request.source);
    let _scope = SiteScope::enter_counter_numbered(original, &normalized, &[]);
    Ok(build())
}
