use std::sync::Arc;

pub(crate) mod source_ids;

use crate::{
    atom_hoist, atom_name, class_num_for_key, content_hash::FingerprintBits,
    content_name::ContentName, debug::is_debug, encode_selector, num_to_nm_base::num_to_nm_base,
    with_prefix, write_u8,
};
use source_ids::original_id;

/// D9 original identity or normalized received source text, never a path fallback.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SourceFile {
    D9(u32),
    Unnumbered(Arc<str>),
}

impl SourceFile {
    /// Look up D9 by filename; otherwise retain received source without BOM or CR bytes.
    #[must_use]
    pub fn from_source(filename: &str, source: &str) -> Self {
        if let Some(number) = original_id(filename) {
            Self::D9(number)
        } else {
            let source = source.strip_prefix('\u{feff}').unwrap_or(source);
            Self::Unnumbered(Arc::from(source.replace('\r', "")))
        }
    }
}

/// Sparse source position plus a role index from the source's syntax order.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Site {
    pub file: SourceFile,
    pub at: usize,
    pub role: usize,
}

/// The numeric alphabet has no separator; class-name ad-blocker fixups do not apply.
fn number(number: usize) -> String {
    let mut name = num_to_nm_base(number);
    name.retain(|character| character != '-');
    name
}

impl Site {
    /// Grammar: `---<prefix>S(<base37>|U<L|H><payload>)-<base37>[-<role>]`.
    /// The source file part is at most 18 bytes; lossless wins ties with 80 bits.
    #[must_use]
    pub fn variable_name(&self, prefix: &str) -> String {
        self.variable_name_with_bits(prefix, FingerprintBits::PRODUCTION)
    }

    /// Width is explicit for the same production naming/collision path.
    #[must_use]
    pub fn variable_name_with_bits(&self, prefix: &str, bits: FingerprintBits) -> String {
        let mut name = format!("---{prefix}S");
        match &self.file {
            SourceFile::D9(file) => {
                const { assert!(usize::BITS >= u32::BITS) };
                #[expect(
                    clippy::expect_used,
                    reason = "supported native and wasm32 targets hold every D9 u32"
                )]
                let file = usize::try_from(*file).expect("D9 u32 fits the target's usize");
                name.push_str(&number(file));
            }
            SourceFile::Unnumbered(source) => {
                name.push_str(&ContentName::source(source).name_with_bits("", bits));
            }
        }
        name.push('-');
        name.push_str(&number(self.at));
        if self.role != 0 {
            name.push('-');
            name.push_str(&number(self.role));
        }
        name
    }
}

#[must_use]
pub fn sheet_to_variable_name(property: &str, level: u8, selector: Option<&str>) -> String {
    sheet_to_variable_name_at(property, level, selector, None)
}

/// Name the variable for a dynamic assignment's original source site.
///
/// Known sites use only original source identity, source offset and source role in every
/// naming mode. One assignment can feed several selectors or properties;
/// distinct assignments need distinct sites because custom properties inherit.
#[must_use]
pub fn sheet_to_variable_name_at(
    property: &str,
    level: u8,
    selector: Option<&str>,
    site: Option<Site>,
) -> String {
    let _admission = crate::admission::enter();
    if let Some(site) = site {
        return with_prefix(|prefix| site.variable_name(prefix));
    }
    if atom_hoist::is_atom_hoist() {
        return with_prefix(|prefix| {
            format!(
                "--{prefix}v1-{}-{level}-{}",
                atom_name::hex(property.trim()),
                atom_name::hex(selector.unwrap_or_default())
            )
        });
    }
    if is_debug() {
        let selector = selector.unwrap_or_default().trim();
        let encoded = if selector.is_empty() {
            String::new()
        } else {
            encode_selector(selector)
        };
        with_prefix(|prefix| {
            let mut result =
                String::with_capacity(2 + prefix.len() + property.len() + 4 + encoded.len());
            result.push_str("--");
            result.push_str(prefix);
            result.push_str(property);
            result.push('-');
            write_u8(&mut result, level);
            result.push('-');
            result.push_str(&encoded);
            result
        })
    } else {
        let trimmed_selector = selector.unwrap_or_default().trim();
        let base_name = class_num_for_key("", |key| {
            key.reserve(property.len() + 4 + trimmed_selector.len());
            key.push_str(property);
            key.push('-');
            write_u8(key, level);
            key.push('-');
            key.push_str(trimmed_selector);
        });
        with_prefix(|prefix| {
            let mut result = String::with_capacity(2 + prefix.len() + base_name.len());
            result.push_str("--");
            result.push_str(prefix);
            result.push_str(&base_name);
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_keeps_numbers_and_roles_injective_when_base37_crosses_a_digit() {
        // Given: both the first multi-digit number and the ad-blocker suffix.
        let sites = [(0, 0, 0), (27, 30, 0), (30, 27, 0), (27, 30, 1)];
        // When: sparse identities are encoded.
        let names: Vec<_> = sites
            .into_iter()
            .map(|(file, at, role)| {
                Site {
                    file: SourceFile::D9(file),
                    at,
                    role,
                }
                .variable_name("test-")
            })
            .collect();
        // Then: separators distinguish file, position and role without decimal fallback.
        assert_eq!(
            names,
            [
                "---test-Sa-a",
                "---test-Saa-ad",
                "---test-Sad-aa",
                "---test-Saa-ad-b"
            ]
        );
    }

    #[test]
    #[serial_test::serial]
    fn source_encoding_keeps_unnumbered_and_d9_names_disjoint() {
        // Given: source text resembling numeric names and containing namespace or escape characters.
        crate::file_map::set_original_ids(std::collections::BTreeMap::new());
        let sources = ["a", "U", "a-b", "a_db", "é"];
        // When: unnumbered source text is encoded losslessly.
        let names: Vec<_> = sources
            .into_iter()
            .map(|source| {
                Site {
                    file: SourceFile::from_source("/unnumbered.tsx", source),
                    at: 0,
                    role: 0,
                }
                .variable_name("")
            })
            .collect();
        // Then: neither a D9 identity nor another original source can alias them.
        assert_eq!(
            names,
            [
                "---SULa-a",
                "---SUL_x55_-a",
                "---SULa_db-a",
                "---SULa_udb-a",
                "---SUL_xe9_-a"
            ]
        );
    }

    #[test]
    #[serial_test::serial]
    fn unnumbered_identity_retains_source_when_bom_and_cr_are_received() {
        // Given: equivalent received sources and no D9 identities.
        crate::file_map::set_original_ids(std::collections::BTreeMap::new());
        let inputs = [
            ("/a.tsx", "a\nb"),
            ("/b.tsx", "\u{feff}a\r\nb"),
            ("/c.tsx", "a\rb"),
        ];
        // When: the boundary constructs each source identity.
        let files = inputs.map(|(filename, source)| SourceFile::from_source(filename, source));
        // Then: normalization retains the full source text, not the filename or a hash.
        assert_eq!(
            files,
            [
                SourceFile::Unnumbered(Arc::from("a\nb")),
                SourceFile::Unnumbered(Arc::from("a\nb")),
                SourceFile::Unnumbered(Arc::from("ab")),
            ]
        );
    }
}
