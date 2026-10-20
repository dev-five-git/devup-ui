//! Host-independent nonnegative integer conversion.

/// Converts a signed input without silently wrapping negative values.
///
/// # Errors
/// Returns the standard conversion error for a negative input.
pub fn nonnegative(value: i32) -> Result<u32, std::num::TryFromIntError> {
    u32::try_from(value)
}
