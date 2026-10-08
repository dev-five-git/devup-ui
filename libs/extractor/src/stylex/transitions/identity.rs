pub(crate) enum IdentityDomain {
    Position,
    View,
    Keyframes,
    Variable,
}

/// Lossless hex components separated outside the hex alphabet: no finite hash,
/// arrival counter, file-number map or collision registry is involved.
pub(crate) fn name(filename: &str, domain: IdentityDomain, content: &str) -> String {
    let prefix = css::get_prefix().unwrap_or_default();
    let mut name = match domain {
        IdentityDomain::Position => format!("--{prefix}sxp-"),
        IdentityDomain::View => format!("{prefix}sxv-"),
        IdentityDomain::Keyframes => format!("{prefix}sxk-"),
        IdentityDomain::Variable => format!("--{prefix}sxvar-"),
    };
    for component in [filename, content] {
        for byte in component.bytes() {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            name.push(char::from(HEX[usize::from(byte >> 4)]));
            name.push(char::from(HEX[usize::from(byte & 15)]));
        }
        name.push('-');
    }
    name
}
