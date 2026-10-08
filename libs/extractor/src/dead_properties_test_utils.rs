pub(super) fn position(source: &str, needle: &str) -> usize {
    match source.find(needle) {
        Some(position) => position,
        None => panic!("fixture has no {needle:?}"),
    }
}

pub(super) fn failure(result: Result<crate::ExtractOutput, Box<dyn std::error::Error>>) -> String {
    match result {
        Err(error) => error.to_string(),
        Ok(output) => panic!("expected declaration error, got {output:?}"),
    }
}
