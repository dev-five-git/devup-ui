//! Production names that never depend on which build, environment or
//! processing order reached a style first.
//!
//! Short counter names are only handed out inside a namespace one original
//! file owns, and only to styles whose existence does not depend on how an
//! import resolves ([`Naming::Own`]). Everything else is named by what it
//! declares, so equal styles share one name wherever they are met.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::sync::{LazyLock, Mutex};

use crate::CounterOwner;

/// Whether a style may take a slot of its file's counter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Naming {
    /// Value and existence derived solely from this file's source and theme.
    #[default]
    Own,
    /// Chosen or valued by something that may differ between environments.
    Risky,
}

impl Naming {
    #[must_use]
    pub const fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Own, Self::Own) => Self::Own,
            _ => Self::Risky,
        }
    }
}

pub use crate::sparse_site::Site;

/// Shared eligibility for class allocation and content/scope registry claims.
#[must_use]
pub fn owned_private_counter(
    owner: CounterOwner,
    delivery: (Option<&str>, u8),
    naming: Naming,
) -> Option<u32> {
    let _admission = crate::admission::enter();
    let (filename, order) = delivery;
    if naming != Naming::Own || order == 0 {
        return None;
    }
    let file = filename.filter(|file| !crate::atom_hoist::is_hoisted_bucket(file))?;
    match owner {
        CounterOwner::D9(id) => Some(id),
        CounterOwner::Unnumbered => None,
        CounterOwner::Inactive => private_counter(Some(file), naming, order),
    }
}

static COLLAPSED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// Remember which buckets hold more than one original file.
pub fn set_collapsed_buckets(canonical: &HashMap<String, String>) {
    let _admission = crate::admission::enter();
    crate::admission::assert_administration_allowed("set_collapsed_buckets");
    let _root = crate::root_held::RootHeld::enter("collapsed");
    *COLLAPSED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = canonical
        .iter()
        .filter(|(file, bucket)| file != bucket)
        .map(|(_, bucket)| bucket.clone())
        .collect();
}

/// Whether styles named in `filename`'s namespace are met from several files.
pub fn is_shared_namespace(filename: Option<&str>) -> bool {
    let _admission = crate::admission::enter();
    filename.is_none_or(|bucket| {
        let _root = crate::root_held::RootHeld::enter("collapsed");
        COLLAPSED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(bucket)
    })
}

/// Lowercase letters and digits stay; every other character becomes an
/// `_`-led code, so the encoding is lossless and holds no `-`.
pub fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        let code = match c {
            'a'..='z' | '0'..='9' => {
                out.push(c);
                continue;
            }
            '-' => 'd',
            '_' => 'u',
            '.' => 'p',
            '%' => 'c',
            '#' => 'h',
            '(' => 'o',
            ')' => 'e',
            ',' => 'm',
            ' ' => 's',
            '/' => 'l',
            ':' => 'n',
            '&' => 'a',
            '"' => 'q',
            _ => {
                let _ = write!(out, "_x{:x}_", u32::from(c));
                continue;
            }
        };
        out.push('_');
        out.push(code);
    }
}

/// The name of a class by what it declares.
pub fn content_class(
    property: &str,
    level: u8,
    value: Option<&str>,
    selector: &str,
    style_order: u8,
) -> String {
    let selector = (!selector.is_empty())
        .then(|| crate::style_selector::StyleSelector::Selector(selector.to_string()));
    crate::content_name::AtomContent {
        property,
        level,
        value,
        selector: selector.as_ref(),
        order: style_order,
        naming: Naming::Risky,
        layer: None,
        dynamic: false,
    }
    .content()
    .name("")
}

/// Only a source numbered before extraction may consume its private counter.
#[must_use]
pub fn private_counter(filename: Option<&str>, naming: Naming, order: u8) -> Option<u32> {
    let _admission = crate::admission::enter();
    if naming != Naming::Own || order == 0 || is_shared_namespace(filename) {
        return None;
    }
    filename
        .filter(|file| !crate::atom_hoist::is_hoisted_bucket(file))
        .and_then(crate::file_map::original_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn join_is_risky_when_either_side_is() {
        assert_eq!(Naming::Own.join(Naming::Own), Naming::Own);
        assert_eq!(Naming::Own.join(Naming::Risky), Naming::Risky);
        assert_eq!(Naming::Risky.join(Naming::Own), Naming::Risky);
        assert_eq!(Naming::Risky.join(Naming::Risky), Naming::Risky);
    }

    #[test]
    fn escape_is_lossless_and_free_of_separators() {
        let mut out = String::new();
        escape_into(&mut out, "a-B_1%é");
        assert_eq!(out, "a_d_x42__u1_c_xe9_");
    }

    #[test]
    fn content_class_tells_none_from_empty_value_and_keeps_defaults_out() {
        assert_eq!(content_class("color", 0, None, "", 255), "RLcolor");
        assert_eq!(content_class("color", 0, Some(""), "", 255), "RLcolor-v");
        let named = content_class("color", 1, Some("red"), "&:hover", 0);
        assert_eq!(named.len(), 18);
        assert!(named.starts_with("RH"));
    }

    #[test]
    #[serial]
    fn shared_namespace_is_the_base_sheet_or_a_collapsed_bucket() {
        set_collapsed_buckets(&HashMap::from([("/b.tsx".into(), "/a.tsx".into())]));

        assert!(is_shared_namespace(None));
        assert!(is_shared_namespace(Some("/a.tsx")));
        assert!(!is_shared_namespace(Some("/b.tsx")));
        assert!(!is_shared_namespace(Some("/c.tsx")));
        set_collapsed_buckets(&HashMap::new());
    }
}
