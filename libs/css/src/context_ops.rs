//! Context-aware operations for CSS class name generation.
//!
//! These functions accept an `ExtractionContext` parameter instead of using
//! global state, enabling stateless extraction for parallel processing.
//!
//! Each function has a corresponding global-state version in the parent module
//! for backward compatibility.

use crate::num_to_nm_base::num_to_nm_base;
use crate::optimize_value::optimize_value;
use bimap::BiHashMap;
use std::collections::HashMap;

/// Context state for CSS extraction (mirrors ExtractionContext from extractor crate)
///
/// This is a simplified view of the state needed by css crate functions.
/// The extractor crate's ExtractionContext wraps this.
#[derive(Debug, Default)]
pub struct CssContext {
    /// Maps filename → (style_key → class_number)
    pub class_map: HashMap<String, HashMap<String, usize>>,
    /// Bidirectional map: filename ↔ file_number
    pub file_map: BiHashMap<String, usize>,
    /// Optional prefix for all generated class names
    pub prefix: Option<String>,
    /// Debug mode flag
    pub debug: bool,
}

impl CssContext {
    /// Creates a new empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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

    /// Resets all state.
    pub fn reset(&mut self) {
        self.class_map.clear();
        self.file_map.clear();
    }
}

fn encode_selector(selector: &str) -> String {
    let mut result = String::with_capacity(selector.len() * 2);
    for c in selector.chars() {
        match c {
            '&' => result.push_str("_a_"),
            ':' => result.push_str("_c_"),
            '(' => result.push_str("_lp_"),
            ')' => result.push_str("_rp_"),
            '[' => result.push_str("_lb_"),
            ']' => result.push_str("_rb_"),
            '=' => result.push_str("_eq_"),
            '>' => result.push_str("_gt_"),
            '<' => result.push_str("_lt_"),
            '~' => result.push_str("_tl_"),
            '+' => result.push_str("_pl_"),
            ' ' => result.push_str("_s_"),
            '*' => result.push_str("_st_"),
            '.' => result.push_str("_d_"),
            '#' => result.push_str("_h_"),
            ',' => result.push_str("_cm_"),
            '"' => result.push_str("_dq_"),
            '\'' => result.push_str("_sq_"),
            '/' => result.push_str("_sl_"),
            '\\' => result.push_str("_bs_"),
            '%' => result.push_str("_pc_"),
            '^' => result.push_str("_cr_"),
            '$' => result.push_str("_dl_"),
            '|' => result.push_str("_pp_"),
            '@' => result.push_str("_at_"),
            '!' => result.push_str("_ex_"),
            '?' => result.push_str("_qm_"),
            ';' => result.push_str("_sc_"),
            '{' => result.push_str("_lc_"),
            '}' => result.push_str("_rc_"),
            '-' => result.push('-'),
            '_' => result.push('_'),
            _ if c.is_ascii_alphanumeric() => result.push(c),
            _ => {
                result.push_str("_u");
                result.push_str(&format!("{:04x}", c as u32));
                result.push('_');
            }
        }
    }
    result
}

/// Generate a class name for a style sheet entry (context-aware version).
///
/// This is the context-aware equivalent of `sheet_to_classname`.
pub fn sheet_to_classname_with_context(
    ctx: &mut CssContext,
    property: &str,
    level: u8,
    value: Option<&str>,
    selector: Option<&str>,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> String {
    // Copy prefix to avoid borrow issues
    let prefix = ctx.prefix.clone().unwrap_or_default();
    let debug = ctx.debug;

    // base style
    let filename = if style_order == Some(0) {
        None
    } else {
        filename
    };
    if debug {
        let selector = selector.unwrap_or_default().trim();
        let file_suffix = filename
            .map(|v| format!("-{}", ctx.get_file_num(v)))
            .unwrap_or_default();
        format!(
            "{}{}-{}-{}-{}-{}{}",
            prefix,
            property.trim(),
            level,
            optimize_value(value.unwrap_or_default()),
            if selector.is_empty() {
                String::new()
            } else {
                encode_selector(selector)
            },
            style_order.unwrap_or(255),
            file_suffix,
        )
    } else {
        let file_num_suffix = filename
            .map(|v| format!("-{}", ctx.get_file_num(v)))
            .unwrap_or_default();
        let key = format!(
            "{}-{}-{}-{}-{}{}",
            property.trim(),
            level,
            optimize_value(value.unwrap_or_default()),
            selector.unwrap_or_default().trim(),
            style_order.unwrap_or(255),
            file_num_suffix,
        );
        let filename_str = filename.map(|v| v.to_string()).unwrap_or_default();
        let file_num = if !filename_str.is_empty() {
            Some(ctx.get_file_num(&filename_str))
        } else {
            None
        };
        let file_entry = ctx.class_map.entry(filename_str).or_default();
        let class_num = if let Some(&num) = file_entry.get(&key) {
            num_to_nm_base(num)
        } else {
            let len = file_entry.len();
            file_entry.insert(key, len);
            num_to_nm_base(len)
        };
        if let Some(fnum) = file_num {
            format!("{}{}-{}", prefix, num_to_nm_base(fnum), class_num)
        } else {
            format!("{}{}", prefix, class_num)
        }
    }
}

/// Generate a CSS variable name (context-aware version).
///
/// This is the context-aware equivalent of `sheet_to_variable_name`.
pub fn sheet_to_variable_name_with_context(
    ctx: &mut CssContext,
    property: &str,
    level: u8,
    selector: Option<&str>,
) -> String {
    // Copy prefix to avoid borrow issues
    let prefix = ctx.prefix.clone().unwrap_or_default();
    let debug = ctx.debug;

    if debug {
        let selector = selector.unwrap_or_default().trim();
        format!(
            "--{}{}-{}-{}",
            prefix,
            property,
            level,
            if selector.is_empty() {
                String::new()
            } else {
                encode_selector(selector)
            }
        )
    } else {
        let key = format!(
            "{}-{}-{}",
            property,
            level,
            selector.unwrap_or_default().trim()
        );
        let file_entry = ctx.class_map.entry(String::new()).or_default();
        if let Some(&num) = file_entry.get(&key) {
            format!("--{}{}", prefix, num_to_nm_base(num))
        } else {
            let len = file_entry.len();
            file_entry.insert(key, len);
            format!("--{}{}", prefix, num_to_nm_base(len))
        }
    }
}

/// Generate a keyframes animation name (context-aware version).
///
/// This is the context-aware equivalent of `keyframes_to_keyframes_name`.
pub fn keyframes_to_keyframes_name_with_context(
    ctx: &mut CssContext,
    keyframes: &str,
    filename: Option<&str>,
) -> String {
    // Copy prefix to avoid borrow issues
    let prefix = ctx.prefix.clone().unwrap_or_default();
    let debug = ctx.debug;

    if debug {
        format!("{}k-{keyframes}", prefix)
    } else {
        let key = format!("k-{keyframes}");
        let filename_str = filename.map(|v| v.to_string()).unwrap_or_default();
        let file_num = if !filename_str.is_empty() {
            Some(ctx.get_file_num(&filename_str))
        } else {
            None
        };
        let file_entry = ctx.class_map.entry(filename_str).or_default();
        let class_num = if let Some(&num) = file_entry.get(&key) {
            num_to_nm_base(num).to_string()
        } else {
            let len = file_entry.len();
            file_entry.insert(key, len);
            num_to_nm_base(len).to_string()
        };
        if let Some(fnum) = file_num {
            format!("{}{}-{}", prefix, num_to_nm_base(fnum), class_num)
        } else {
            format!("{}{}", prefix, class_num)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sheet_to_classname_basic() {
        let mut ctx = CssContext::new();
        ctx.debug = false;

        assert_eq!(
            sheet_to_classname_with_context(
                &mut ctx,
                "background",
                0,
                Some("red"),
                None,
                None,
                None
            ),
            "a"
        );
        assert_eq!(
            sheet_to_classname_with_context(&mut ctx, "color", 0, Some("blue"), None, None, None),
            "b"
        );
        // Same style returns same class
        assert_eq!(
            sheet_to_classname_with_context(
                &mut ctx,
                "background",
                0,
                Some("red"),
                None,
                None,
                None
            ),
            "a"
        );
    }

    #[test]
    fn test_sheet_to_classname_debug() {
        let mut ctx = CssContext::new();
        ctx.debug = true;

        assert_eq!(
            sheet_to_classname_with_context(
                &mut ctx,
                "background",
                0,
                Some("red"),
                None,
                None,
                None
            ),
            "background-0-red--255"
        );
    }

    #[test]
    fn test_sheet_to_classname_with_prefix() {
        let mut ctx = CssContext::new();
        ctx.debug = false;
        ctx.prefix = Some("app-".to_string());

        assert_eq!(
            sheet_to_classname_with_context(
                &mut ctx,
                "background",
                0,
                Some("red"),
                None,
                None,
                None
            ),
            "app-a"
        );
    }

    #[test]
    fn test_sheet_to_classname_with_filename() {
        let mut ctx = CssContext::new();
        ctx.debug = false;

        let class1 = sheet_to_classname_with_context(
            &mut ctx,
            "background",
            0,
            Some("red"),
            None,
            None,
            Some("file1.tsx"),
        );
        let class2 = sheet_to_classname_with_context(
            &mut ctx,
            "background",
            0,
            Some("red"),
            None,
            None,
            Some("file2.tsx"),
        );
        // Different files should have different prefixes
        assert_ne!(class1, class2);
    }

    #[test]
    fn test_sheet_to_variable_name_basic() {
        let mut ctx = CssContext::new();
        ctx.debug = false;

        assert_eq!(
            sheet_to_variable_name_with_context(&mut ctx, "background", 0, None),
            "--a"
        );
        assert_eq!(
            sheet_to_variable_name_with_context(&mut ctx, "color", 0, None),
            "--b"
        );
    }

    #[test]
    fn test_sheet_to_variable_name_debug() {
        let mut ctx = CssContext::new();
        ctx.debug = true;

        assert_eq!(
            sheet_to_variable_name_with_context(&mut ctx, "background", 0, None),
            "--background-0-"
        );
    }

    #[test]
    fn test_keyframes_basic() {
        let mut ctx = CssContext::new();
        ctx.debug = false;

        assert_eq!(
            keyframes_to_keyframes_name_with_context(&mut ctx, "spin", None),
            "a"
        );
        assert_eq!(
            keyframes_to_keyframes_name_with_context(&mut ctx, "spin", None),
            "a"
        );
        assert_eq!(
            keyframes_to_keyframes_name_with_context(&mut ctx, "fade", None),
            "b"
        );
    }

    #[test]
    fn test_keyframes_debug() {
        let mut ctx = CssContext::new();
        ctx.debug = true;

        assert_eq!(
            keyframes_to_keyframes_name_with_context(&mut ctx, "spin", None),
            "k-spin"
        );
    }

    #[test]
    fn test_keyframes_with_filename() {
        let mut ctx = CssContext::new();
        ctx.debug = false;

        let name1 = keyframes_to_keyframes_name_with_context(&mut ctx, "spin", Some("file1.tsx"));
        let name2 = keyframes_to_keyframes_name_with_context(&mut ctx, "spin", Some("file2.tsx"));
        // Different files should have different prefixes
        assert_ne!(name1, name2);
    }
}
