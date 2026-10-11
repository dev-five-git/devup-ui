use super::initializers;

#[test]
fn initializer_ranges_include_local_constants_but_ignore_exported_functions() {
    // Given
    let source = "const local = 1; export function helper() {} export const token = 'red';";
    // When
    let ranges = initializers(source);
    // Then
    let values = ranges
        .iter()
        .map(|(name, start, end)| (name.as_str(), &source[*start..*end]))
        .collect::<Vec<_>>();
    assert_eq!(values, [("local", "1"), ("token", "'red'")]);
}
