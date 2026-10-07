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
    if let Err(incoming) = result {
        ERROR.with_borrow_mut(|error| {
            if error.is_none() {
                *error = Some(incoming.clone());
            }
        });
    }
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
    crate::cache_atom_proof::validate(sheet)?;
    for (file, orders) in &sheet.properties {
        for property in orders.values().flat_map(|levels| levels.values()).flatten() {
            if property.hoisted
                && !sheet
                    .atom_plan
                    .as_ref()
                    .is_some_and(|plan| plan.contains(file))
            {
                return Err(format!(
                    "cached hoisted atom `{}` has no frozen bucket proof",
                    property.class_name
                ));
            }
        }
    }
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
    let atoms = sheet.properties.iter().flat_map(|(file, orders)| {
        orders.iter().flat_map(move |(order, levels)| {
            levels.values().flatten().map(move |property| {
                (
                    property.class_name.as_str(),
                    !file.is_empty() && *order != 0 && !property.hoisted,
                )
            })
        })
    });
    let frames = sheet
        .keyframes
        .values()
        .flat_map(|frames| frames.keys().map(|name| (name.as_str(), false)));
    let prefix = css::get_prefix().unwrap_or_default();
    for (name, per_file) in atoms.chain(frames) {
        let generated = name.strip_prefix(&prefix).is_some_and(content_name);
        if per_file
            && generated
            && name
                .rfind(|character: char| character.is_ascii_uppercase())
                .and_then(|index| index.checked_sub(1))
                == Some(prefix.len())
            && name.starts_with(&prefix)
        {
            return Err(format!(
                "cached per-file generated name `{name}` has no sheet namespace; rebuild all caches with the current naming contract"
            ));
        }
        if generated && !sheet.names.contains_key(name) {
            return Err(format!(
                "cached generated name `{name}` has no exact descriptor claim; rebuild all caches with the current naming contract"
            ));
        }
        if generated
            && let Some(domain) = name
                .rfind(|character: char| character.is_ascii_uppercase())
                .and_then(|index| index.checked_sub(1))
            && let Some(scope) = name[..domain].strip_suffix('-')
            && scope.len() >= prefix.len()
            && let Some(marker) = scope.rfind('F')
            && marker >= prefix.len()
            && matches!(scope.as_bytes().get(marker + 1), Some(b'L' | b'H'))
        {
            let claim = sheet.names.get(scope).ok_or_else(|| format!(
                "cached generated scope `{scope}` has no exact sheet-key claim; rebuild all caches with the current naming contract"
            ))?;
            let content = css::content_name::ContentName::scope(&claim.content);
            if claim.descriptor != content.descriptor || content.name(&scope[..marker]) != scope {
                return Err(format!(
                    "cached generated scope `{scope}` has a corrupt exact sheet-key claim; rebuild all caches"
                ));
            }
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
            "OXred",
        ] {
            assert!(!content_name(name), "{name}");
        }
    }
}
