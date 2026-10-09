use std::{
    collections::BTreeMap,
    sync::{LazyLock, RwLock},
};

pub type Declarations = Vec<(u8, String, String)>;
static PRESETS: LazyLock<RwLock<BTreeMap<String, Declarations>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));

pub fn set(presets: BTreeMap<String, Declarations>) {
    let _admission = crate::admission::enter();
    crate::admission::assert_administration_allowed("content_typography::set");
    let _root = crate::root_held::RootHeld::enter("content_typography");
    *PRESETS
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = presets;
}

/// The effective declarations of one conditional typography atom.
pub fn declarations(value: &str, level: u8) -> Declarations {
    let _admission = crate::admission::enter();
    let (preset, yielded) = value.split_once('|').unwrap_or((value, ""));
    let yielded: Vec<_> = yielded
        .split(',')
        .filter_map(|item| {
            let (property, level) = item.split_once(':')?;
            Some((property, level.parse::<u8>().ok()?))
        })
        .collect();
    let root = crate::root_held::RootHeld::enter("content_typography");
    let presets = PRESETS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut result: Declarations = Vec::new();
    for (from, property, value) in presets.get(preset).into_iter().flatten() {
        let target = level.max(*from);
        match result
            .iter_mut()
            .find(|(level, name, _)| *level == target && name == property)
        {
            Some((_, _, previous)) => previous.clone_from(value),
            None => result.push((target, property.clone(), value.clone())),
        }
    }
    drop(presets);
    drop(root);
    result.retain(|(level, property, _)| {
        !yielded
            .iter()
            .any(|(name, from)| *name == property.as_str() && level >= from)
    });
    result
}

pub fn identity(value: &str, level: u8) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let declarations = declarations(value, level);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &u64::try_from(declarations.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for (level, property, value) in declarations {
        bytes.push(level);
        for field in [&property, &value] {
            bytes.extend_from_slice(&u64::try_from(field.len()).unwrap_or(u64::MAX).to_be_bytes());
            bytes.extend_from_slice(field.as_bytes());
        }
    }
    let mut identity = String::from("t");
    for byte in bytes {
        identity.push(char::from(HEX[usize::from(byte >> 4)]));
        identity.push(char::from(HEX[usize::from(byte & 15)]));
    }
    identity
}
