//! What a vanilla-extract stylesheet exports, read the way the bundler
//! extracts it: its theme objects, theme classes and class names become the
//! literals its own CSS uses.

use std::cell::RefCell;

use crate::{ExtractOption, ExtractOutput, ModuleResolver, utils::is_vanilla_extract_file};

thread_local! {
    /// Stylesheets being extracted for an importer, so one importing itself
    /// through other modules reads as unknown instead of recursing
    static COMPILING: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Marks `path` as being compiled until dropped
struct Compiling;

impl Compiling {
    fn enter(path: &str) -> Option<Self> {
        COMPILING.with_borrow_mut(|stack| {
            (!stack.iter().any(|entry| entry == path)).then(|| {
                stack.push(path.to_string());
                Self
            })
        })
    }
}

impl Drop for Compiling {
    fn drop(&mut self) {
        COMPILING.with_borrow_mut(Vec::pop);
    }
}

/// The stylesheet at `path` extracted, when it is one and its extraction
/// succeeds: its code exports what the stylesheet's own CSS uses
pub(super) fn compiled(
    path: &str,
    code: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
) -> Option<ExtractOutput> {
    if !is_vanilla_extract_file(path) {
        return None;
    }
    let _compiling = Compiling::enter(path)?;
    crate::extract_with_source_map(path, code, option.clone(), false, resolver).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stylesheet_being_compiled_reads_as_unknown_until_it_ends() {
        // Given
        let outer = Compiling::enter("/src/a.css.ts");
        // When
        let inner = compiled(
            "/src/a.css.ts",
            "export const a = 1;",
            &ExtractOption::default(),
            None,
        );
        // Then
        assert!(outer.is_some());
        assert!(inner.is_none());
        drop(outer);
        assert!(Compiling::enter("/src/a.css.ts").is_some());
    }
}
