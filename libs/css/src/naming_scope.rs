use crate::{content_hash::FingerprintBits, content_name::ContentName};

/// A content atom is global only when #751 already delivers its rules in one shared sheet.
#[must_use]
pub fn local_file(filename: Option<&str>, order: u8) -> Option<&str> {
    filename.filter(|file| order != 0 && !crate::atom_hoist::is_hoisted_bucket(file))
}

#[must_use]
pub fn fallback(filename: Option<&str>, order: u8) -> Option<ContentName> {
    local_file(filename, order)
        .filter(|file| crate::file_map::original_id(file).is_none())
        .map(|file| ContentName::scope(&crate::naming_root::key(&crate::file_map::canonical(file))))
}

/// Content names and exact claims use this same scope prefix and fingerprint width.
#[must_use]
pub fn name(content: &ContentName, source: (Option<&str>, u8), bits: FingerprintBits) -> String {
    let prefix = crate::get_prefix().unwrap_or_default();
    let scope = match local_file(source.0, source.1) {
        None => String::new(),
        Some(file) => {
            (match crate::file_map::original_id(file) {
                Some(id) => {
                    crate::num_to_nm_base::num_to_nm_base(usize::try_from(id).unwrap_or_default())
                }
                None => fallback(Some(file), source.1)
                    .map_or_else(String::new, |scope| scope.name_with_bits("", bits)),
            }) + "-"
        }
    };
    content.name_with_bits(&format!("{prefix}{scope}"), bits)
}
