//! Source-safe names for generated instrumentation helpers.

pub(crate) fn fresh_name(base: &str, source: &str) -> String {
    let mut candidate = base.to_string();
    while source.contains(&candidate) {
        candidate.push('_');
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::fresh_name;
    use rstest::rstest;

    #[rstest]
    #[case("__devup_read_site_123__")]
    #[case("__devup_operation_site_123__")]
    fn chooses_unused_suffix_when_source_contains_collisions(#[case] base: &str) {
        // Given: valid JavaScript with the base and two occupied suffixes.
        let source = format!("const {base} = 1, {base}_ = 2, {base}__ = 3;");
        // When
        let chosen = fresh_name(base, &source);
        // Then
        assert_eq!(chosen, format!("{base}___"));
        assert!(!source.contains(&chosen));
    }

    #[test]
    fn preserves_base_when_source_has_no_collision() {
        // Given
        let source = "const value = 1;";
        // When
        let chosen = fresh_name("__helper__", source);
        // Then
        assert_eq!(chosen, "__helper__");
    }
}
