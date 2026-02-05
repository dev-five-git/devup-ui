//! Extraction context for stateless CSS extraction.
//!
//! This module provides `ExtractionContext` which holds all mutable state
//! needed during CSS extraction. By passing context explicitly rather than
//! using global state, we enable parallel file processing for Turbopack
//! multi-core builds.
//!
//! # Usage
//!
//! ```rust,ignore
//! use extractor::{ExtractionContext, extract_with_context, ExtractOption};
//!
//! // Create a shared context for the build
//! let mut ctx = ExtractionContext::new();
//!
//! // Extract styles from multiple files (can be parallelized)
//! let result1 = extract_with_context("file1.tsx", code1, options, &mut ctx)?;
//! let result2 = extract_with_context("file2.tsx", code2, options, &mut ctx)?;
//! ```

use bimap::BiHashMap;
use std::collections::HashMap;

/// Extraction context holding all mutable state for CSS extraction.
///
/// This struct replaces the global `GLOBAL_CLASS_MAP`, `GLOBAL_FILE_MAP`,
/// and `GLOBAL_PREFIX` statics, enabling stateless extraction that can
/// be parallelized across multiple files.
///
/// # Thread Safety
///
/// The context itself is NOT thread-safe. For parallel extraction:
/// - Use separate contexts per thread, then merge
/// - Or wrap in `Arc<Mutex<ExtractionContext>>` if shared access needed
///
/// The WASM boundary maintains a single global context for backward
/// compatibility with the existing JS API.
#[derive(Debug, Default)]
pub struct ExtractionContext {
    /// Maps filename → (style_key → class_number)
    ///
    /// Used for generating unique, deterministic class names.
    /// The outer key is the filename (empty string for global styles).
    /// The inner map tracks which style combinations have been seen
    /// and assigns sequential numbers for minimal class names.
    pub(crate) class_map: HashMap<String, HashMap<String, usize>>,

    /// Bidirectional map: filename ↔ file_number
    ///
    /// Used for per-file class name prefixes to avoid collisions
    /// between files while keeping class names short.
    pub(crate) file_map: BiHashMap<String, usize>,

    /// Optional prefix for all generated class names.
    ///
    /// When set, all class names will be prefixed with this string.
    /// Example: prefix "app-" → class names become "app-a", "app-b", etc.
    pub(crate) prefix: Option<String>,

    /// Debug mode flag.
    ///
    /// When true, generates human-readable class names for debugging.
    /// Example: "background-0-red-hover-255" instead of "a"
    pub(crate) debug: bool,
}

impl ExtractionContext {
    /// Creates a new empty extraction context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a context with a specific prefix.
    #[must_use]
    pub fn with_prefix(prefix: Option<String>) -> Self {
        Self {
            prefix,
            ..Default::default()
        }
    }

    /// Sets the class name prefix.
    pub fn set_prefix(&mut self, prefix: Option<String>) {
        self.prefix = prefix;
    }

    /// Gets the current prefix.
    #[must_use]
    pub fn get_prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    /// Sets debug mode.
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    /// Gets debug mode.
    #[must_use]
    pub fn is_debug(&self) -> bool {
        self.debug
    }

    /// Resets the class map (for testing).
    pub fn reset_class_map(&mut self) {
        self.class_map.clear();
    }

    /// Resets the file map (for testing).
    pub fn reset_file_map(&mut self) {
        self.file_map.clear();
    }

    /// Resets all state (for testing).
    pub fn reset(&mut self) {
        self.class_map.clear();
        self.file_map.clear();
    }

    /// Gets the file number for a filename, creating a new mapping if needed.
    pub fn get_file_num(&mut self, filename: &str) -> usize {
        let len = self.file_map.len();
        if !self.file_map.contains_left(filename) {
            self.file_map.insert(filename.to_string(), len);
        }
        *self.file_map.get_by_left(filename).unwrap()
    }

    /// Gets the filename for a file number.
    #[must_use]
    pub fn get_filename(&self, file_num: usize) -> Option<&str> {
        self.file_map.get_by_right(&file_num).map(String::as_str)
    }

    /// Sets the class map from external data (for WASM interop).
    pub fn set_class_map(&mut self, map: HashMap<String, HashMap<String, usize>>) {
        self.class_map = map;
    }

    /// Gets a clone of the class map (for WASM interop).
    #[must_use]
    pub fn get_class_map(&self) -> HashMap<String, HashMap<String, usize>> {
        self.class_map.clone()
    }

    /// Sets the file map from external data (for WASM interop).
    pub fn set_file_map(&mut self, map: BiHashMap<String, usize>) {
        self.file_map = map;
    }

    /// Gets a clone of the file map (for WASM interop).
    #[must_use]
    pub fn get_file_map(&self) -> BiHashMap<String, usize> {
        self.file_map.clone()
    }

    /// Gets or creates a class number for a style key in a file.
    ///
    /// This is the core method for generating deterministic class names.
    /// Given a style key (property-value-selector combination), it returns
    /// a unique number that can be converted to a short class name.
    pub fn get_or_create_class_num(&mut self, filename: &str, style_key: &str) -> usize {
        let file_map = self.class_map.entry(filename.to_string()).or_default();
        if let Some(&num) = file_map.get(style_key) {
            num
        } else {
            let num = file_map.len();
            file_map.insert(style_key.to_string(), num);
            num
        }
    }

    /// Gets an existing class number without creating a new one.
    #[must_use]
    pub fn get_class_num(&self, filename: &str, style_key: &str) -> Option<usize> {
        self.class_map
            .get(filename)
            .and_then(|m| m.get(style_key).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context() {
        let ctx = ExtractionContext::new();
        assert!(ctx.class_map.is_empty());
        assert!(ctx.file_map.is_empty());
        assert!(ctx.prefix.is_none());
        assert!(!ctx.debug);
    }

    #[test]
    fn test_with_prefix() {
        let ctx = ExtractionContext::with_prefix(Some("app-".to_string()));
        assert_eq!(ctx.get_prefix(), Some("app-"));
    }

    #[test]
    fn test_set_prefix() {
        let mut ctx = ExtractionContext::new();
        ctx.set_prefix(Some("test-".to_string()));
        assert_eq!(ctx.get_prefix(), Some("test-"));
        ctx.set_prefix(None);
        assert_eq!(ctx.get_prefix(), None);
    }

    #[test]
    fn test_debug_mode() {
        let mut ctx = ExtractionContext::new();
        assert!(!ctx.is_debug());
        ctx.set_debug(true);
        assert!(ctx.is_debug());
    }

    #[test]
    fn test_file_num_mapping() {
        let mut ctx = ExtractionContext::new();

        // First file gets 0
        assert_eq!(ctx.get_file_num("file1.tsx"), 0);
        // Same file returns same number
        assert_eq!(ctx.get_file_num("file1.tsx"), 0);
        // Second file gets 1
        assert_eq!(ctx.get_file_num("file2.tsx"), 1);
        // Lookup by number
        assert_eq!(ctx.get_filename(0), Some("file1.tsx"));
        assert_eq!(ctx.get_filename(1), Some("file2.tsx"));
        assert_eq!(ctx.get_filename(99), None);
    }

    #[test]
    fn test_class_num_generation() {
        let mut ctx = ExtractionContext::new();

        // First style in file gets 0
        assert_eq!(ctx.get_or_create_class_num("", "bg-red"), 0);
        // Same style returns same number
        assert_eq!(ctx.get_or_create_class_num("", "bg-red"), 0);
        // Different style gets next number
        assert_eq!(ctx.get_or_create_class_num("", "color-blue"), 1);
        // Different file has its own numbering
        assert_eq!(ctx.get_or_create_class_num("file.tsx", "bg-red"), 0);

        // Lookup without creation
        assert_eq!(ctx.get_class_num("", "bg-red"), Some(0));
        assert_eq!(ctx.get_class_num("", "unknown"), None);
    }

    #[test]
    fn test_reset() {
        let mut ctx = ExtractionContext::new();
        ctx.get_file_num("file.tsx");
        ctx.get_or_create_class_num("", "style");

        ctx.reset();

        assert!(ctx.class_map.is_empty());
        assert!(ctx.file_map.is_empty());
    }

    #[test]
    fn test_set_and_get_maps() {
        let mut ctx = ExtractionContext::new();

        // Set class map
        let mut class_map = HashMap::new();
        let mut inner = HashMap::new();
        inner.insert("key".to_string(), 42);
        class_map.insert("file".to_string(), inner);
        ctx.set_class_map(class_map.clone());
        assert_eq!(ctx.get_class_map(), class_map);

        // Set file map
        let mut file_map = BiHashMap::new();
        file_map.insert("test.tsx".to_string(), 5);
        ctx.set_file_map(file_map.clone());
        assert_eq!(ctx.get_file_map(), file_map);
    }
}
