use crate::utils::compile_regex;
use regex_lite::Regex;
use std::{borrow::Cow, sync::LazyLock};

static TOKEN: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"\$\w[\w.-]*"));

/// The declaration value after theme references are expanded for emission.
pub fn emitted(value: &str) -> Cow<'_, str> {
    if !value.contains('$') {
        return Cow::Borrowed(value);
    }
    match TOKEN.replace_all(value, |caps: &regex_lite::Captures| {
        format!("var(--{})", caps[0][1..].replace('.', "-"))
    }) {
        Cow::Owned(value) => Cow::Owned(value),
        Cow::Borrowed(_) => Cow::Borrowed(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effective_values_expand_only_authored_theme_tokens() {
        assert_eq!(emitted("1px solid $line.100"), "1px solid var(--line-100)");
        assert_eq!(emitted("$text"), "var(--text)");
        assert_eq!(emitted("var(--text)"), "var(--text)");
        assert_eq!(emitted("$"), "$");
    }
}
