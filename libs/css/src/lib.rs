pub mod class_map;
mod constant;
pub mod debug;
pub mod file_map;
pub mod is_special_property;
mod num_to_nm_base;
pub mod optimize_multi_css_value;
pub mod optimize_value;
pub mod rm_css_comment;
mod selector_separator;
pub mod style_selector;
pub mod utils;

use std::collections::BTreeMap;

use crate::constant::{
    COLOR_HASH, F_SPACE_RE, GLOBAL_ENUM_STYLE_PROPERTY, GLOBAL_STYLE_PROPERTY, ZERO_RE,
};
use crate::debug::is_debug;
use crate::file_map::get_file_num_by_filename;
use crate::num_to_nm_base::num_to_nm_base;
use crate::optimize_value::optimize_value;
use crate::style_selector::StyleSelector;
use crate::utils::to_kebab_case;

#[cfg(target_arch = "wasm32")]
mod prefix_state {
    use std::cell::RefCell;
    thread_local! {
        static GLOBAL_PREFIX: RefCell<Option<String>> = const { RefCell::new(None) };
    }
    pub fn set_prefix(prefix: Option<String>) {
        GLOBAL_PREFIX.with(|p| *p.borrow_mut() = prefix);
    }
    pub fn get_prefix() -> Option<String> {
        GLOBAL_PREFIX.with(|p| p.borrow().clone())
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod prefix_state {
    use std::sync::LazyLock;
    use std::sync::Mutex;
    static GLOBAL_PREFIX: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));
    pub fn set_prefix(prefix: Option<String>) {
        *GLOBAL_PREFIX.lock().unwrap() = prefix;
    }
    pub fn get_prefix() -> Option<String> {
        GLOBAL_PREFIX.lock().unwrap().clone()
    }
}

pub use prefix_state::{get_prefix, set_prefix};

pub fn merge_selector(class_name: &str, selector: Option<&StyleSelector>) -> String {
    if let Some(selector) = selector {
        match selector {
            StyleSelector::Selector(value) => {
                let mut dot_class = String::with_capacity(1 + class_name.len());
                dot_class.push('.');
                dot_class.push_str(class_name);
                value.replace("&", &dot_class)
            }
            StyleSelector::At { selector: s, .. } => {
                if let Some(s) = s {
                    let mut dot_class = String::with_capacity(1 + class_name.len());
                    dot_class.push('.');
                    dot_class.push_str(class_name);
                    s.replace("&", &dot_class)
                } else {
                    let mut result = String::with_capacity(1 + class_name.len());
                    result.push('.');
                    result.push_str(class_name);
                    result
                }
            }
            StyleSelector::Global(v, _) => v.to_string(),
        }
    } else {
        let mut result = String::with_capacity(1 + class_name.len());
        result.push('.');
        result.push_str(class_name);
        result
    }
}

pub fn disassemble_property(property: &str) -> Vec<String> {
    GLOBAL_STYLE_PROPERTY
        .get(property)
        .map(|v| match v.len() {
            1 => vec![v[0].to_string()],
            _ => v.iter().map(|v| v.to_string()).collect(),
        })
        .unwrap_or_else(|| {
            vec![if (property.starts_with("Webkit")
                && property.len() > 6
                && property.chars().nth(6).unwrap().is_uppercase())
                || (property.starts_with("Moz")
                    && property.len() > 3
                    && property.chars().nth(3).unwrap().is_uppercase())
                || (property.starts_with("ms")
                    && property.len() > 2
                    && property.chars().nth(2).unwrap().is_uppercase())
            {
                format!("-{}", to_kebab_case(property))
            } else {
                to_kebab_case(property)
            }]
        })
}

pub fn add_selector_params(selector: StyleSelector, params: &str) -> StyleSelector {
    match selector {
        StyleSelector::Selector(value) => StyleSelector::Selector(format!("{}({})", value, params)),
        StyleSelector::Global(value, file) => {
            StyleSelector::Global(format!("{}({})", value, params), file)
        }
        StyleSelector::At {
            kind,
            query,
            selector,
        } => StyleSelector::At {
            kind,
            query: query.to_string(),
            selector: selector.map(|s| format!("{}({})", s, params)),
        },
    }
}

pub fn get_enum_property_value(property: &str, value: &str) -> Option<Vec<(String, String)>> {
    if let Some(map) = GLOBAL_ENUM_STYLE_PROPERTY.get(property) {
        if let Some(map) = map.get(value) {
            Some(
                map.entries()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            )
        } else {
            Some(vec![])
        }
    } else {
        None
    }
}

pub fn get_enum_property_map(property: &str) -> Option<BTreeMap<&str, BTreeMap<&str, &str>>> {
    if let Some(map) = GLOBAL_ENUM_STYLE_PROPERTY.get(property) {
        let mut ret = BTreeMap::new();
        for (k, v) in map.entries() {
            let mut tmp = BTreeMap::new();
            v.entries().for_each(|(k, v)| {
                tmp.insert(*k, *v);
            });
            ret.insert(*k, tmp);
        }
        Some(ret)
    } else {
        None
    }
}

pub fn keyframes_to_keyframes_name(keyframes: &str, filename: Option<&str>) -> String {
    let prefix = get_prefix().unwrap_or_default();
    if is_debug() {
        let mut result = String::with_capacity(prefix.len() + 2 + keyframes.len());
        result.push_str(&prefix);
        result.push_str("k-");
        result.push_str(keyframes);
        result
    } else {
        let mut key = String::with_capacity(2 + keyframes.len());
        key.push_str("k-");
        key.push_str(keyframes);
        let filename_key = filename.unwrap_or_default();
        let class_num = class_map::with_class_map_mut(|map| {
            let file_entry = map.entry(filename_key.to_string()).or_default();
            if let Some(&num) = file_entry.get(&key) {
                num_to_nm_base(num)
            } else {
                let len = file_entry.len();
                file_entry.insert(key, len);
                num_to_nm_base(len)
            }
        });
        if let Some(fname) = filename {
            let file_num = num_to_nm_base(get_file_num_by_filename(fname));
            let mut result =
                String::with_capacity(prefix.len() + file_num.len() + 1 + class_num.len());
            result.push_str(&prefix);
            result.push_str(&file_num);
            result.push('-');
            result.push_str(&class_num);
            result
        } else {
            let mut result = String::with_capacity(prefix.len() + class_num.len());
            result.push_str(&prefix);
            result.push_str(&class_num);
            result
        }
    }
}

/// ASCII lookup table for selector encoding. `None` means pass through (alphanumeric, `-`, `_`)
/// or fall through to the Unicode escape path.
const SELECTOR_ENCODE: [Option<&str>; 128] = {
    let mut table: [Option<&str>; 128] = [None; 128];
    table[b'&' as usize] = Some("_a_");
    table[b':' as usize] = Some("_c_");
    table[b'(' as usize] = Some("_lp_");
    table[b')' as usize] = Some("_rp_");
    table[b'[' as usize] = Some("_lb_");
    table[b']' as usize] = Some("_rb_");
    table[b'=' as usize] = Some("_eq_");
    table[b'>' as usize] = Some("_gt_");
    table[b'<' as usize] = Some("_lt_");
    table[b'~' as usize] = Some("_tl_");
    table[b'+' as usize] = Some("_pl_");
    table[b' ' as usize] = Some("_s_");
    table[b'*' as usize] = Some("_st_");
    table[b'.' as usize] = Some("_d_");
    table[b'#' as usize] = Some("_h_");
    table[b',' as usize] = Some("_cm_");
    table[b'"' as usize] = Some("_dq_");
    table[b'\'' as usize] = Some("_sq_");
    table[b'/' as usize] = Some("_sl_");
    table[b'\\' as usize] = Some("_bs_");
    table[b'%' as usize] = Some("_pc_");
    table[b'^' as usize] = Some("_cr_");
    table[b'$' as usize] = Some("_dl_");
    table[b'|' as usize] = Some("_pp_");
    table[b'@' as usize] = Some("_at_");
    table[b'!' as usize] = Some("_ex_");
    table[b'?' as usize] = Some("_qm_");
    table[b';' as usize] = Some("_sc_");
    table[b'{' as usize] = Some("_lc_");
    table[b'}' as usize] = Some("_rc_");
    table
};

fn encode_selector(selector: &str) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(selector.len() * 3);
    for c in selector.chars() {
        if c.is_ascii() {
            let byte = c as u8;
            if let Some(encoded) = SELECTOR_ENCODE[byte as usize] {
                result.push_str(encoded);
            } else if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                result.push(c);
            } else {
                // ASCII but not in table and not alphanumeric/-/_
                let _ = write!(result, "_u{:04x}_", c as u32);
            }
        } else {
            let _ = write!(result, "_u{:04x}_", c as u32);
        }
    }
    result
}

pub fn sheet_to_classname(
    property: &str,
    level: u8,
    value: Option<&str>,
    selector: Option<&str>,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> String {
    let prefix = get_prefix().unwrap_or_default();
    // base style
    let filename = if style_order == Some(0) {
        None
    } else {
        filename
    };
    let optimized = optimize_value(value.unwrap_or_default());
    if is_debug() {
        let selector = selector.unwrap_or_default().trim();
        let encoded = if selector.is_empty() {
            String::new()
        } else {
            encode_selector(selector)
        };
        let file_suffix = filename.map(get_file_num_by_filename);
        let order = style_order.unwrap_or(255);
        let prop = property.trim();
        // Estimate capacity: prefix + prop + separators + level(1-3) + optimized + encoded + order(1-3) + file
        let mut result =
            String::with_capacity(prefix.len() + prop.len() + optimized.len() + encoded.len() + 16);
        result.push_str(&prefix);
        result.push_str(prop);
        result.push('-');
        write_u8(&mut result, level);
        result.push('-');
        result.push_str(&optimized);
        result.push('-');
        result.push_str(&encoded);
        result.push('-');
        write_u8(&mut result, order);
        if let Some(fnum) = file_suffix {
            result.push('-');
            result.push_str(&num_to_nm_base(fnum));
        }
        result
    } else {
        let trimmed_selector = selector.unwrap_or_default().trim();
        let order = style_order.unwrap_or(255);
        let file_num = filename.map(get_file_num_by_filename);
        let trimmed_prop = property.trim();

        // Build key with pre-allocated capacity
        let mut key = String::with_capacity(
            trimmed_prop.len() + optimized.len() + trimmed_selector.len() + 16,
        );
        key.push_str(trimmed_prop);
        key.push('-');
        write_u8(&mut key, level);
        key.push('-');
        key.push_str(&optimized);
        key.push('-');
        key.push_str(trimmed_selector);
        key.push('-');
        write_u8(&mut key, order);
        if let Some(fnum) = file_num {
            key.push('-');
            key.push_str(&num_to_nm_base(fnum));
        }

        let filename_key = filename.unwrap_or_default();
        let clas_num = class_map::with_class_map_mut(|map| {
            let file_entry = map.entry(filename_key.to_string()).or_default();
            if let Some(&num) = file_entry.get(&key) {
                num_to_nm_base(num)
            } else {
                let len = file_entry.len();
                file_entry.insert(key, len);
                num_to_nm_base(len)
            }
        });
        if filename.is_some() {
            let mut result = String::with_capacity(prefix.len() + 8 + clas_num.len());
            result.push_str(&prefix);
            result.push_str(&num_to_nm_base(file_num.unwrap()));
            result.push('-');
            result.push_str(&clas_num);
            result
        } else {
            let mut result = String::with_capacity(prefix.len() + clas_num.len());
            result.push_str(&prefix);
            result.push_str(&clas_num);
            result
        }
    }
}

/// Write a u8 value to a string without allocating via format!
#[inline]
fn write_u8(s: &mut String, v: u8) {
    if v >= 100 {
        s.push((b'0' + v / 100) as char);
        s.push((b'0' + (v / 10) % 10) as char);
        s.push((b'0' + v % 10) as char);
    } else if v >= 10 {
        s.push((b'0' + v / 10) as char);
        s.push((b'0' + v % 10) as char);
    } else {
        s.push((b'0' + v) as char);
    }
}

pub fn sheet_to_variable_name(property: &str, level: u8, selector: Option<&str>) -> String {
    let prefix = get_prefix().unwrap_or_default();
    if is_debug() {
        let selector = selector.unwrap_or_default().trim();
        let encoded = if selector.is_empty() {
            String::new()
        } else {
            encode_selector(selector)
        };
        let mut result =
            String::with_capacity(2 + prefix.len() + property.len() + 4 + encoded.len());
        result.push_str("--");
        result.push_str(&prefix);
        result.push_str(property);
        result.push('-');
        write_u8(&mut result, level);
        result.push('-');
        result.push_str(&encoded);
        result
    } else {
        let trimmed_selector = selector.unwrap_or_default().trim();
        let mut key = String::with_capacity(property.len() + 4 + trimmed_selector.len());
        key.push_str(property);
        key.push('-');
        write_u8(&mut key, level);
        key.push('-');
        key.push_str(trimmed_selector);
        class_map::with_class_map_mut(|map| {
            let file_entry = map.entry(String::new()).or_default();
            let base_name = if let Some(&num) = file_entry.get(&key) {
                num_to_nm_base(num)
            } else {
                let len = file_entry.len();
                file_entry.insert(key, len);
                num_to_nm_base(len)
            };
            let mut result = String::with_capacity(2 + prefix.len() + base_name.len());
            result.push_str("--");
            result.push_str(&prefix);
            result.push_str(&base_name);
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        class_map::{get_class_map, reset_class_map, set_class_map},
        debug::set_debug,
        style_selector::AtRuleKind,
    };

    use super::*;
    use rstest::rstest;
    use serial_test::serial;

    #[rstest]
    #[case("hover", "hover")]
    #[case("&:hover", "_a__c_hover")]
    #[case("&::before", "_a__c__c_before")]
    #[case("nth(1)", "nth_lp_1_rp_")]
    #[case("nth-child(2)", "nth-child_lp_2_rp_")]
    #[case("&:nth-child(2n+1)", "_a__c_nth-child_lp_2n_pl_1_rp_")]
    #[case("[data-theme=dark]", "_lb_data-theme_eq_dark_rb_")]
    #[case("& > div", "_a__s__gt__s_div")]
    #[case("& + span", "_a__s__pl__s_span")]
    #[case("& ~ p", "_a__s__tl__s_p")]
    #[case(".class-name", "_d_class-name")]
    #[case("#id-name", "_h_id-name")]
    #[case("&:hover:focus", "_a__c_hover_c_focus")]
    #[case("&::placeholder", "_a__c__c_placeholder")]
    #[case(
        ":root[data-theme=\"dark\"]",
        "_c_root_lb_data-theme_eq__dq_dark_dq__rb_"
    )]
    #[case("&:not(.active)", "_a__c_not_lp__d_active_rp_")]
    #[case("simple", "simple")]
    #[case("with-dash", "with-dash")]
    #[case("with_underscore", "with_underscore")]
    #[case("CamelCase123", "CamelCase123")]
    // Additional cases for full coverage of special characters
    #[case("<", "_lt_")]
    #[case("*", "_st_")]
    #[case(",", "_cm_")]
    #[case("'", "_sq_")]
    #[case("/", "_sl_")]
    #[case("\\", "_bs_")]
    #[case("%", "_pc_")]
    #[case("^", "_cr_")]
    #[case("$", "_dl_")]
    #[case("|", "_pp_")]
    #[case("@", "_at_")]
    #[case("!", "_ex_")]
    #[case("?", "_qm_")]
    #[case(";", "_sc_")]
    #[case("{", "_lc_")]
    #[case("}", "_rc_")]
    // ASCII not in lookup table and not alphanumeric/-/_  (line 212 branch)
    #[case("`", "_u0060_")]
    #[case("\t", "_u0009_")]
    #[case("\x01", "_u0001_")]
    // Unicode character (non-ASCII)
    #[case("한글", "_ud55c__uae00_")]
    #[case("emoji😀", "emoji_u1f600_")]
    fn test_encode_selector(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(encode_selector(input), expected);
    }

    #[test]
    #[serial]
    fn test_sheet_to_variable_name() {
        reset_class_map();
        set_debug(false);
        assert_eq!(sheet_to_variable_name("background", 0, None), "--a");
        assert_eq!(
            sheet_to_variable_name("background", 0, Some("hover")),
            "--b"
        );
        assert_eq!(sheet_to_variable_name("background", 1, None), "--c");
        assert_eq!(
            sheet_to_variable_name("background", 1, Some("hover")),
            "--d"
        );
    }

    #[test]
    #[serial]
    fn test_debug_sheet_to_variable_name() {
        set_debug(true);
        assert_eq!(
            sheet_to_variable_name("background", 0, None),
            "--background-0-"
        );
        assert_eq!(
            sheet_to_variable_name("background", 0, Some("hover")),
            "--background-0-hover"
        );
        assert_eq!(
            sheet_to_variable_name("background", 1, None),
            "--background-1-"
        );
        assert_eq!(
            sheet_to_variable_name("background", 1, Some("hover")),
            "--background-1-hover"
        );
    }

    #[test]
    #[serial]
    fn test_sheet_to_classname() {
        set_debug(false);
        reset_class_map();
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), Some("hover"), None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("background", 1, None, None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("background", 1, None, Some("hover"), None, None),
            "d"
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname("background", 0, None, None, None, None),
            sheet_to_classname("background", 0, None, None, None, None)
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None, None),
            sheet_to_classname("background", 0, Some("red"), None, None, None),
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None, None),
            sheet_to_classname("  background  ", 0, Some("  red  "), None, None, None),
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None, None),
            sheet_to_classname("  background  ", 0, Some("red;"), None, None, None),
        );
        assert_eq!(
            sheet_to_classname(
                "background",
                0,
                Some("rgba(255, 0, 0,    0.5)"),
                None,
                None,
                None
            ),
            sheet_to_classname("background", 0, Some("rgba(255,0,0,0.5)"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname(
                "background",
                0,
                Some("rgba(255, 0, 0,    0.5)"),
                None,
                None,
                None
            ),
            sheet_to_classname("background", 0, Some("rgba(255,0,0,.5)"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname(
                "background",
                0,
                Some("rgba(255, 0, 0,    0.5)"),
                None,
                None,
                None
            ),
            sheet_to_classname("background", 0, Some("#FF000080"), None, None, None),
        );

        class_map::with_class_map(|map| {
            assert_eq!(
                map.get("").unwrap().get("background-0-#FF000080--255"),
                Some(&2)
            );
        });
        assert_eq!(
            sheet_to_classname("background", 0, Some("#fff"), None, None, None),
            sheet_to_classname("  background  ", 0, Some("#FFF"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffff"), None, None, None),
            sheet_to_classname("background", 0, Some("#FFF"), None, None, None),
        );

        class_map::with_class_map(|map| {
            assert_eq!(map.get("").unwrap().get("background-0-#FFF--255"), Some(&3));
        });

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffff"), None, None, None),
            sheet_to_classname("background", 0, Some("#FFFFFF"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffffAA"), None, None, None),
            sheet_to_classname("background", 0, Some("#FFFFFFaa"), None, None, None),
        );

        class_map::with_class_map(|map| {
            assert_eq!(
                map.get("").unwrap().get("background-0-#FFFA--255"),
                Some(&4)
            );
        });
        assert_eq!(
            sheet_to_classname(
                "background",
                0,
                Some("color-mix(in srgb,var(--primary) 80%,    #000 20%)"),
                None,
                None,
                None
            ),
            sheet_to_classname(
                "background",
                0,
                Some("color-mix(in srgb,    var(--primary) 80%, #000000 20%)"),
                None,
                None,
                None
            ),
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname("background", 0, None, None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("background", 0, None, None, Some(1), None),
            "b"
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname("width", 0, Some("0px"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0em"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0rem"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0vh"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0%"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0dvh"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0dvw"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0vw"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0px red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0% red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0em red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0rem red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0vh red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0vw red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0dvh red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0dvw red"), None, None, None),
            "b"
        );

        assert_eq!(
            sheet_to_classname("test", 0, Some("0px 0"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0em 0"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0rem 0"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0vh 0"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0vw 0"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0dvh 0"), None, None, None),
            "c"
        );

        assert_eq!(
            sheet_to_classname("test", 0, Some("0 0vh"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0 0vw"), None, None, None),
            "c"
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname(
                "transition",
                0,
                Some("all 0.3s ease-in-out"),
                None,
                None,
                None
            ),
            "a"
        );
        assert_eq!(
            sheet_to_classname(
                "transition",
                0,
                Some("all .3s ease-in-out"),
                None,
                None,
                None
            ),
            "a"
        );
    }

    #[test]
    #[serial]
    fn test_debug_sheet_to_classname() {
        set_debug(true);
        assert_eq!(
            sheet_to_classname("background", 0, None, None, None, None),
            "background-0---255"
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), Some("hover"), None, None),
            "background-0-red-hover-255"
        );
        assert_eq!(
            sheet_to_classname("background", 1, None, None, None, None),
            "background-1---255"
        );
        assert_eq!(
            sheet_to_classname("background", 1, Some("red"), Some("hover"), None, None),
            "background-1-red-hover-255"
        );
    }

    #[test]
    #[serial]
    fn test_debug_sheet_to_classname_with_filename() {
        reset_class_map();
        set_debug(true);
        // Debug mode + filename triggers the file_suffix branch (lines 234-235)
        let class_name =
            sheet_to_classname("background", 0, Some("red"), None, None, Some("test.tsx"));
        assert!(class_name.contains("background-0-red--255-"));
        // Should have a file number suffix
        let parts: Vec<&str> = class_name.split('-').collect();
        assert!(
            parts.len() >= 6,
            "Expected file suffix in debug classname: {class_name}"
        );
        set_debug(false);
    }

    #[test]
    fn test_merge_selector() {
        assert_eq!(merge_selector("cls", Some(&"hover".into())), ".cls:hover");
        assert_eq!(
            merge_selector("cls", Some(&"placeholder".into())),
            ".cls::placeholder"
        );
        assert_eq!(
            merge_selector("cls", Some(&"theme-dark".into())),
            ":root[data-theme=dark] .cls"
        );
        assert_eq!(
            merge_selector(
                "cls",
                Some(&StyleSelector::Selector(
                    ":root[data-theme=dark]:hover &".to_string(),
                )),
            ),
            ":root[data-theme=dark]:hover .cls"
        );
        assert_eq!(
            merge_selector(
                "cls",
                Some(&StyleSelector::Selector(
                    ":root[data-theme=dark]::placeholder &".to_string()
                )),
            ),
            ":root[data-theme=dark]::placeholder .cls"
        );

        assert_eq!(
            merge_selector("cls", Some(&["theme-dark", "hover"].into()),),
            ":root[data-theme=dark] .cls:hover"
        );
        assert_eq!(
            merge_selector(
                "cls",
                Some(&StyleSelector::At {
                    kind: AtRuleKind::Media,
                    query: "print".to_string(),
                    selector: None
                })
            ),
            ".cls"
        );

        assert_eq!(
            merge_selector(
                "cls",
                Some(&StyleSelector::At {
                    kind: AtRuleKind::Media,
                    query: "print".to_string(),
                    selector: Some("&:hover".to_string())
                })
            ),
            ".cls:hover"
        );

        assert_eq!(
            merge_selector(
                "cls",
                Some(&StyleSelector::Global(
                    "&".to_string(),
                    "file.ts".to_string()
                ))
            ),
            "&"
        );
    }

    #[test]
    #[serial]
    fn test_set_class_map() {
        let mut map = HashMap::new();
        map.insert("".to_string(), HashMap::new());
        map.get_mut("")
            .unwrap()
            .insert("background-0-rgba(255,0,0,0.5)-".to_string(), 1);
        set_class_map(map);
        assert_eq!(get_class_map().len(), 1);
    }

    #[test]
    #[serial]
    fn test_keyframes_to_keyframes_name() {
        reset_class_map();
        set_debug(false);
        assert_eq!(keyframes_to_keyframes_name("spin", None), num_to_nm_base(0));
        assert_eq!(keyframes_to_keyframes_name("spin", None), num_to_nm_base(0));
        assert_eq!(
            keyframes_to_keyframes_name("spin2", None),
            num_to_nm_base(1)
        );
        reset_class_map();
        set_debug(true);
        assert_eq!(keyframes_to_keyframes_name("spin", None), "k-spin");
        assert_eq!(keyframes_to_keyframes_name("spin1", None), "k-spin1");
    }

    #[test]
    fn test_add_selector_params() {
        assert_eq!(
            add_selector_params(StyleSelector::Selector("hover:is".to_string()), "test"),
            StyleSelector::Selector("hover:is(test)".to_string())
        );
        assert_eq!(
            add_selector_params(
                StyleSelector::Global("&:is".to_string(), "file.ts".to_string()),
                "test"
            ),
            StyleSelector::Global("&:is(test)".to_string(), "file.ts".to_string())
        );
        assert_eq!(
            add_selector_params(
                StyleSelector::At {
                    kind: AtRuleKind::Media,
                    query: "print".to_string(),
                    selector: Some("&:is".to_string())
                },
                "test"
            ),
            StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "print".to_string(),
                selector: Some("&:is(test)".to_string())
            }
        );
    }

    #[test]
    #[serial]
    fn test_sheet_to_classname_with_prefix() {
        set_debug(false);
        reset_class_map();
        set_prefix(Some("app-".to_string()));

        let class1 = sheet_to_classname("background", 0, Some("red"), None, None, None);
        assert!(class1.starts_with("app-"));
        assert_eq!(class1, "app-a");

        let class2 = sheet_to_classname("color", 0, Some("blue"), None, None, None);
        assert!(class2.starts_with("app-"));

        set_prefix(None);
        reset_class_map();
    }

    #[test]
    #[serial]
    fn test_debug_sheet_to_classname_with_prefix() {
        set_debug(true);
        set_prefix(Some("my-".to_string()));

        let class_name = sheet_to_classname("background", 0, Some("red"), None, None, None);
        assert_eq!(class_name, "my-background-0-red--255");

        let with_selector =
            sheet_to_classname("background", 0, Some("red"), Some("hover"), None, None);
        assert!(with_selector.starts_with("my-"));

        set_prefix(None);
    }

    #[test]
    #[serial]
    fn test_sheet_to_variable_name_with_prefix() {
        set_debug(false);
        reset_class_map();
        set_prefix(Some("app-".to_string()));

        assert_eq!(sheet_to_variable_name("background", 0, None), "--app-a");

        set_prefix(None);
        reset_class_map();
    }

    #[test]
    #[serial]
    fn test_keyframes_with_prefix() {
        reset_class_map();
        set_debug(false);
        set_prefix(Some("app-".to_string()));

        let name = keyframes_to_keyframes_name("spin", None);
        assert!(name.starts_with("app-"));

        set_prefix(None);
    }

    #[test]
    #[serial]
    fn test_empty_prefix_is_same_as_none() {
        set_debug(false);
        reset_class_map();

        set_prefix(Some("".to_string()));
        let class1 = sheet_to_classname("background", 0, Some("red"), None, None, None);

        reset_class_map();
        set_prefix(None);
        let class2 = sheet_to_classname("background", 0, Some("red"), None, None, None);

        assert_eq!(class1, class2);
    }

    #[test]
    #[serial]
    fn test_keyframes_to_keyframes_name_with_filename() {
        reset_class_map();
        set_debug(false);
        // Test with filename to cover lines 148-151
        let name = keyframes_to_keyframes_name("spin", Some("test.tsx"));
        // Should include file number prefix
        assert!(name.contains("-"));

        // Same keyframe in same file should return same name
        let name2 = keyframes_to_keyframes_name("spin", Some("test.tsx"));
        assert_eq!(name, name2);

        // Different file should have different prefix
        let name3 = keyframes_to_keyframes_name("spin", Some("other.tsx"));
        assert_ne!(name, name3);
    }

    #[test]
    #[serial]
    fn test_sheet_to_classname_with_filename() {
        reset_class_map();
        set_debug(false);
        // Test with filename to cover the filename branch
        let class1 = sheet_to_classname("background", 0, Some("red"), None, None, Some("test.tsx"));
        // Should include file number prefix
        assert!(class1.contains("-"));

        // Same property in same file should return same classname
        let class2 = sheet_to_classname("background", 0, Some("red"), None, None, Some("test.tsx"));
        assert_eq!(class1, class2);

        // Different file should have different prefix
        let class3 =
            sheet_to_classname("background", 0, Some("red"), None, None, Some("other.tsx"));
        assert_ne!(class1, class3);
    }
}
