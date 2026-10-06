use sheet::StyleSheet;
use std::cell::RefCell;

thread_local! {
    static ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(crate) fn check() -> Result<(), String> {
    ERROR.with_borrow(|error| match error {
        Some(error) => Err(error.clone()),
        None => Ok(()),
    })
}

pub(crate) fn record(result: &Result<(), String>) {
    ERROR.with_borrow_mut(|error| *error = result.as_ref().err().cloned());
}

pub(crate) fn clear() {
    ERROR.with_borrow_mut(|error| *error = None);
}

fn content_name(name: &str) -> bool {
    let Some(marker) = name.rfind(|character: char| character.is_ascii_uppercase()) else {
        return false;
    };
    let Some(domain) = marker
        .checked_sub(1)
        .and_then(|index| name.as_bytes().get(index))
    else {
        return false;
    };
    if !matches!(domain, b'O' | b'R' | b'K') {
        return false;
    }
    let payload = &name[marker + 1..];
    match name.as_bytes()[marker] {
        b'H' => {
            payload.len() == 16
                && payload
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        }
        b'L' => {
            payload.len() <= 16
                && payload.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'-')
                })
        }
        _ => false,
    }
}

pub(crate) fn validate(sheet: &StyleSheet) -> Result<(), String> {
    for property in sheet
        .properties
        .values()
        .flat_map(|orders| orders.values())
        .flat_map(|levels| levels.values())
        .flatten()
    {
        if property.owner_reset {
            crate::cache_source_names::validate(&property.property, &sheet.names)?;
        }
        if let Some(reference) = property.value.strip_prefix("var(") {
            crate::cache_source_names::validate(reference, &sheet.names)?;
        }
    }
    let atoms = sheet
        .properties
        .values()
        .flat_map(|orders| orders.values())
        .flat_map(|levels| levels.values())
        .flatten()
        .map(|property| property.class_name.as_str());
    let frames = sheet
        .keyframes
        .values()
        .flat_map(|frames| frames.keys().map(String::as_str));
    for name in atoms.chain(frames) {
        if content_name(name) && !sheet.names.contains_key(name) {
            return Err(format!(
                "cached generated name `{name}` has no exact descriptor claim; rebuild all caches with the current naming contract"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_content_grammar_does_not_confuse_legacy_counters_or_names() {
        for name in ["OLcolor-vred", "prefix-RHaaaaaaaaaaaaaaaa", "KL"] {
            assert!(content_name(name), "{name}");
        }
        for name in [
            "a-a",
            "Rcolor-vred",
            "Ksfrom",
            "a1-h-123",
            "OHshort",
            "OLlonger_than_sixteen_payload",
            "prefix-L",
        ] {
            assert!(!content_name(name), "{name}");
        }
    }
}
