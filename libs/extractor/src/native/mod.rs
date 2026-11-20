//! React Native output, as style objects instead of CSS.
//!
//! The styles of Devup UI elements compile to style objects hoisted into one
//! `StyleSheet.create` per file, passed to the React Native element through
//! `style`, with no CSS and no class names.
//!
//! What the build can decide compiles statically: literal values, values the
//! element reads at runtime as they are, conditions between them, the
//! platform blocks `_ios`, `_android` and `_web`, and theme tokens when the
//! theme has one value for them. A value that differs by theme compiles to
//! one style object a theme, and the element picks one by the index the
//! runtime of `@devup-ui/react/native` gives. Responsive arrays and
//! `_pressed` need a runtime that this version does not include, and a prop
//! React Native has no style for is a build error naming the prop.

mod element;
mod errors;
mod imports;
mod props;
mod sheet;
mod style;
mod theme;
mod values;

#[cfg(test)]
mod tests;

use std::error::Error;
use std::sync::Arc;

use oxc_allocator::Allocator;
use oxc_semantic::SemanticBuilder;
use rustc_hash::FxHashSet;
use strum_macros::{Display, EnumString};

use crate::ExtractOutput;

pub use theme::{NativeTheme, TokenLevels, TypographyProperties};

/// What extraction compiles for
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ExtractTarget {
    /// Class names and CSS
    #[default]
    Web,
    /// React Native style objects
    Native(NativeTarget),
}

/// The React Native build a file is compiled for
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTarget {
    /// The platform Metro builds the bundle for
    pub platform: NativePlatform,
    /// Whether the React Native runtime may be imported for what needs it;
    /// with `false`, such styles are build errors
    pub runtime: bool,
    /// The theme tokens resolve in
    pub theme: Arc<NativeTheme>,
}

/// The platform Metro builds a bundle for
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum NativePlatform {
    Ios,
    Android,
    Web,
}

/// The package React Native components are imported from
pub(crate) const REACT_NATIVE: &str = "react-native";

/// The module of the React Native runtime: the theme store and the hook and
/// component that pick a style object by theme
pub(crate) const RUNTIME: &str = "@devup-ui/react/native";

/// Compile `code` for React Native, or tell every place it cannot be, each
/// led by `filename:line:column`
pub(crate) fn extract(
    filename: &str,
    code: &str,
    package: &str,
    target: NativeTarget,
    source_map: bool,
) -> Result<ExtractOutput, Box<dyn Error>> {
    if !code.contains(package) {
        return Ok(unchanged(code));
    }
    let source_type = crate::supported_source_type(filename)?;
    let allocator = Allocator::default();
    let program = crate::parsing::checked(&allocator, code, source_type)
        .map_err(|errors| crate::located_errors(filename, code, &[], errors))?;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let compiled = element::compile(&program, code, package, &target, &scoping);
    if !compiled.errors.is_empty() {
        return Err(crate::located_errors(filename, code, &[], compiled.errors).into());
    }
    if compiled.replacements.is_empty() {
        return Ok(unchanged(code));
    }
    let (rewritten, edits) = apply(code, compiled.replacements);
    let mut output = crate::emotion_namespace::Rewritten {
        filename,
        code: &rewritten,
        source: code,
        edits: &[&edits],
    }
    .output();
    output.map = output.map.filter(|_| source_map);
    Ok(output)
}

/// `code` as it is, with nothing to compile
fn unchanged(code: &str) -> ExtractOutput {
    ExtractOutput {
        styles: FxHashSet::default(),
        code: code.to_string(),
        map: None,
        css_file: None,
        dependencies: Vec::new(),
        component_markers: crate::ComponentMarkers::default(),
    }
}

/// A replacement of `code[start..end]` by a text
pub(super) type Replacement = (usize, usize, String);

/// `code` with `replacements` made, and the edits mapping it back
fn apply(
    code: &str,
    mut replacements: Vec<Replacement>,
) -> (String, Vec<crate::import_alias_visit::Edit>) {
    replacements.sort_by_key(|(start, end, _)| (*start, *end));
    let mut rewritten = String::with_capacity(code.len());
    let mut edits = Vec::with_capacity(replacements.len());
    let mut at = 0;
    for (start, end, text) in replacements {
        rewritten.push_str(&code[at..start]);
        rewritten.push_str(&text);
        edits.push((start, end, text.len()));
        at = end;
    }
    rewritten.push_str(&code[at..]);
    (rewritten, edits)
}
