use css::{
    Naming,
    content_name::AtomContent,
    sparse_site::{Site, SourceFile},
};
use sheet::{StyleSheet, StyleSheetProperty, cache_snapshot::ClassMap};

fn namespace_id(namespace: &str) -> Option<u32> {
    namespace.strip_prefix("D9-")?.parse().ok()
}

fn counter_label(number: usize) -> String {
    let encoded = Site {
        file: SourceFile::D9(0),
        at: number,
        role: 0,
    }
    .variable_name("");
    let label = encoded.trim_start_matches("---Sa-");
    match label.strip_suffix("ad") {
        Some(head) => format!("{head}a-d"),
        None => label.to_string(),
    }
}

fn variable_owner(value: &str, prefix: &str, owner: u32) -> bool {
    let variable = value.strip_prefix("var(").and_then(|value| {
        value
            .strip_suffix(')')
            .or_else(|| value.strip_suffix(") !important"))
    });
    let Ok(owner) = usize::try_from(owner) else {
        return false;
    };
    let mut number = counter_label(owner);
    number.retain(|character| character != '-');
    variable.is_some_and(|variable| variable.starts_with(&format!("---{prefix}S{number}-")))
}

fn declaration_matches(
    property: &StyleSheetProperty,
    position: (u8, u8),
    slot: (&str, u32),
) -> bool {
    let (key, owner) = slot;
    let prefix = css::get_prefix().unwrap_or_default();
    [false, true].into_iter().any(|dynamic| {
        let content = AtomContent {
            property: &property.property,
            value: Some(&property.value),
            naming: Naming::Own,
            level: position.0,
            order: position.1,
            selector: property.selector.as_ref(),
            layer: property.layer.as_deref(),
            dynamic,
        }
        .content();
        css::atom_name::hex(&content.lossless) == key
            && (!dynamic || variable_owner(&property.value, &prefix, owner))
    })
}

pub(super) fn validate(sheet: &StyleSheet, classes: &ClassMap) -> bool {
    if classes.keys().any(|namespace| {
        namespace_id(namespace)
            .is_some_and(|id| !sheet.source_ids.values().any(|source| *source == id))
            || (namespace.starts_with("D9-") && namespace_id(namespace).is_none())
    }) {
        return false;
    }
    let prefix = css::get_prefix().unwrap_or_default();
    sheet.properties.values().all(|orders| {
        orders.iter().all(|(order, levels)| {
            levels.iter().all(|(level, properties)| {
                properties.iter().all(|property| {
                    if property.owner_reset || sheet.names.contains_key(&property.class_name) {
                        return true;
                    }
                    let Some((owner, owner_label)) = sheet
                        .source_ids
                        .values()
                        .filter_map(|owner| {
                            usize::try_from(*owner)
                                .ok()
                                .map(|number| (*owner, counter_label(number)))
                        })
                        .filter(|(_, label)| {
                            property
                                .class_name
                                .starts_with(&format!("{prefix}{label}-"))
                        })
                        .max_by_key(|(_, label)| label.len())
                    else {
                        return true;
                    };
                    let Some(slots) = classes.get(&format!("D9-{owner}")) else {
                        return false;
                    };
                    slots.iter().any(|(key, slot)| {
                        property.class_name
                            == format!("{prefix}{owner_label}-{}", counter_label(*slot))
                            && if property.typography {
                                crate::cache_special_proof::counter_typography(
                                    property,
                                    (levels, *order),
                                    key,
                                )
                                .is_some()
                            } else {
                                let owns_reset = levels.values().flatten().any(|other| {
                                    other.owner_reset && other.class_name == property.class_name
                                });
                                (!owns_reset || key.starts_with("642d"))
                                    && declaration_matches(property, (*level, *order), (key, owner))
                            }
                    })
                })
            })
        })
    })
}
