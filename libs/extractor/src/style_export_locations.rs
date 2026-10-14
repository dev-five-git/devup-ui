use std::collections::BTreeSet;

use boa_engine::{JsObject, JsValue};
use css::style_origin::RealLocation;

use crate::vanilla_extract::CollectedStyles;

pub(crate) fn references(value: &JsValue) -> BTreeSet<String> {
    let mut references = BTreeSet::new();
    let mut pending = vec![value.clone()];
    let mut seen = Vec::new();
    while let Some(value) = pending.pop() {
        if let Some(text) = value.as_string() {
            references.extend(
                text.to_std_string_escaped()
                    .split_whitespace()
                    .map(str::to_string),
            );
        } else if let Some(object) = value.as_object() {
            if seen
                .iter()
                .any(|visited| JsObject::equals(visited, &object))
            {
                continue;
            }
            seen.push(object.clone());
            let raw = object.borrow();
            pending.extend(
                raw.properties()
                    .index_properties()
                    .filter_map(|(_, descriptor)| descriptor.value().cloned()),
            );
            for key in raw.shape().keys() {
                if let Some(descriptor) = raw.properties().get(&key)
                    && let Some(value) = descriptor.value()
                {
                    pending.push(value.clone());
                }
            }
        }
    }
    references
}

pub(crate) fn associate(collected: &mut CollectedStyles, exports: &[(String, String)], file: &str) {
    for (binding, name) in exports {
        let mut pending = vec![name.clone()];
        let mut seen = BTreeSet::new();
        while let Some(name) = pending.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            if let Some(entry) = collected
                .styles
                .get_mut(&name)
                .or_else(|| collected.keyframes.get_mut(&name))
            {
                pending.extend(entry.bases.iter().cloned());
                let location = RealLocation::ModuleExport {
                    file: file.into(),
                    binding: Some(binding.clone()),
                };
                if entry
                    .location
                    .as_ref()
                    .is_none_or(|current| location < *current)
                {
                    entry.location = Some(location);
                }
            }
        }
    }
    for entry in collected
        .styles
        .values_mut()
        .chain(collected.keyframes.values_mut())
    {
        if entry.origin.is_none() && entry.location.is_none() {
            entry.location = Some(RealLocation::ModuleExport {
                file: file.into(),
                binding: None,
            });
        }
    }
}
