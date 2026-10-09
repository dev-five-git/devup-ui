pub mod admission;
#[cfg(test)]
mod admission_guard_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod admission_input_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod admission_interleaving_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod admission_root_tests;
pub mod allocation_input;
pub mod at_rule;
pub mod atom_hoist;
pub mod atom_name;
pub mod class_map;
mod constant;
pub mod content_hash;
pub mod content_name;
#[cfg(test)]
mod content_name_tests;
pub mod content_typography;
#[cfg(test)]
mod content_typography_tests;
pub mod content_value;
mod counter_allocation;
#[cfg(test)]
mod counter_context_tests;
pub mod counter_names;
#[cfg(test)]
mod counter_names_tests;
mod counter_owner;
#[cfg(test)]
mod counter_proof_tests;
mod counter_render;
#[cfg(test)]
mod counter_test_helpers;
pub mod debug;
pub mod exact_attempt;
#[cfg(test)]
mod exact_attempt_tests;
pub mod file_map;
pub mod file_routes;
pub mod is_special_property;
mod legacy_variable_names;
pub mod naming;
#[cfg(test)]
mod naming_coverage_tests;
pub mod naming_root;
pub mod naming_scope;
mod num_to_nm_base;
pub mod numeric_value;
pub mod optimize_multi_css_value;
pub mod optimize_value;
pub mod rm_css_comment;
mod root_held;
#[cfg(test)]
mod scoped_name_tests;
mod selector_separator;
pub mod sparse_site;
pub mod style_origin;
pub mod style_selector;
pub mod theme_tokens;
pub mod utils;
use legacy_variable_names::encode_selector;
use legacy_variable_names::write_u8;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, RwLock};

use crate::constant::{GLOBAL_ENUM_STYLE_PROPERTY, GLOBAL_STYLE_PROPERTY};
pub use crate::counter_allocation::CounterSlot;
pub use crate::counter_owner::CounterOwner;
use crate::debug::is_debug;

pub use crate::naming::{Naming, Site};
use crate::num_to_nm_base::num_to_nm_base;
pub use crate::sparse_site::{sheet_to_variable_name, sheet_to_variable_name_at};
use crate::style_selector::StyleSelector;
use crate::utils::to_kebab_case;

#[cfg(target_arch = "wasm32")]
mod prefix_state {
    use std::cell::RefCell;
    thread_local! {
        static GLOBAL_PREFIX: RefCell<Option<String>> = const { RefCell::new(None) };
    }
    pub fn set_prefix(prefix: Option<String>) {
        let _admission = crate::admission::enter();
        crate::admission::assert_administration_allowed("set_prefix");
        GLOBAL_PREFIX.with(|p| *p.borrow_mut() = prefix);
    }
    pub fn get_prefix() -> Option<String> {
        let _admission = crate::admission::enter();
        GLOBAL_PREFIX.with(|p| p.borrow().clone())
    }
    /// Run `f` with the current prefix as `&str` (empty when unset) without cloning.
    #[cfg(not(tarpaulin_include))]
    pub(crate) fn with_prefix<R>(f: impl FnOnce(&str) -> R) -> R {
        let _admission = crate::admission::enter();
        GLOBAL_PREFIX.with(|p| f(p.borrow().as_deref().unwrap_or_default()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod prefix_state {
    use std::sync::LazyLock;
    use std::sync::Mutex;
    static GLOBAL_PREFIX: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));
    pub fn set_prefix(prefix: Option<String>) {
        let _admission = crate::admission::enter();
        crate::admission::assert_administration_allowed("set_prefix");
        let _root = crate::root_held::RootHeld::enter("prefix");
        *GLOBAL_PREFIX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = prefix;
    }
    pub fn get_prefix() -> Option<String> {
        let _admission = crate::admission::enter();
        let _root = crate::root_held::RootHeld::enter("prefix");
        GLOBAL_PREFIX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    /// Run `f` with the current prefix as `&str` (empty when unset) without cloning.
    pub(crate) fn with_prefix<R>(f: impl FnOnce(&str) -> R) -> R {
        let _admission = crate::admission::enter();
        let _root = crate::root_held::RootHeld::enter("prefix");
        f(GLOBAL_PREFIX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_deref()
            .unwrap_or_default())
    }
}

use prefix_state::with_prefix;
pub use prefix_state::{get_prefix, set_prefix};

/// Write `value` into `out`, replacing every `&` with `".<class_name>"` in place.
///
/// Equivalent to `out.push_str(&value.replace('&', &format!(".{class_name}")))`
/// but avoids both the throwaway `".<class>"` String and the intermediate
/// `str::replace` result String.
fn write_ampersand_expansion(out: &mut String, value: &str, class_name: &str) {
    for (i, seg) in value.split('&').enumerate() {
        if i > 0 {
            out.push('.');
            out.push_str(class_name);
        }
        out.push_str(seg);
    }
}

/// Write the merged selector directly into `out`.
///
/// Avoids the throwaway `String` that [`merge_selector`] returns: the common
/// no-selector path pushes `".<class_name>"` in place, while the `&`-replacement
/// cases expand `&` into `out` in place via [`write_ampersand_expansion`].
pub fn write_merge_selector(out: &mut String, class_name: &str, selector: Option<&StyleSelector>) {
    if let Some(selector) = selector {
        match selector {
            StyleSelector::Selector(value) => {
                write_ampersand_expansion(out, value, class_name);
            }
            StyleSelector::At { selector: s, .. } => {
                if let Some(s) = s {
                    write_ampersand_expansion(out, s, class_name);
                } else {
                    out.push('.');
                    out.push_str(class_name);
                }
            }
            StyleSelector::Global(v, _) => out.push_str(v),
        }
    } else {
        out.push('.');
        out.push_str(class_name);
    }
}

#[must_use]
pub fn merge_selector(class_name: &str, selector: Option<&StyleSelector>) -> String {
    // Presize to avoid grow-reallocs in `write_merge_selector`.
    // The `&`-expansion cases push `".<class_name>"` once per `&` in the selector.
    // The base `class_name.len() + selector-len + 2` covers a SINGLE `&` (the
    // common `.a`, `.a:hover`, themed shapes); a selector with N `&` segments
    // (e.g. `:root[data-theme=dark]:hover &` chained variants) would grow-realloc
    // the buffer, so reserve `class_name.len()` extra per additional `&`.
    // Capacity-only: output stays byte-identical.
    let sel = selector.map(StyleSelector::as_class_str);
    // `extra_amps` is `(&-count - 1)`, nonzero ONLY for the rare multi-`&` selector.
    // Locate the FIRST `&` with a `memchr`-backed `find` (bailing to 0 for the common
    // zero/one-`&` selectors ??`.a`, `.a:hover`, `theme-dark` ??without spinning up the
    // `filter(...).count()` iterator over the whole string), then count only the `&`s in
    // the tail AFTER it. That tail count already excludes the first `&`, so it equals
    // `count - 1` directly, dropping the `saturating_sub`. Byte-identical capacity.
    let extra_amps = sel.as_deref().map_or(0, |s| match s.find('&') {
        Some(first) => s[first + 1..].bytes().filter(|&b| b == b'&').count(),
        None => 0,
    });
    let cap =
        class_name.len() + sel.as_deref().map_or(0, str::len) + class_name.len() * extra_amps + 2;
    let mut result = String::with_capacity(cap);
    write_merge_selector(&mut result, class_name, selector);
    result
}

/// Iterator over the disassembled CSS property names for one style property.
///
/// Yields the same sequence as the former `Vec<Cow<'static, str>>` return of
/// [`disassemble_property`] but WITHOUT the per-prop heap `Vec`: the common
/// mapped case (`GLOBAL_STYLE_PROPERTY` hit) borrows the `&'static str` slice
/// in place, and the fallback yields exactly one owned kebab/`-kebab` `String`.
pub enum DisassembleProperty {
    /// Mapped arm: iterate the borrowed `&'static [&'static str]` slice.
    Mapped(std::slice::Iter<'static, &'static str>),
    /// User-defined shorthand properties registered by a build plugin.
    Custom(std::vec::IntoIter<String>),
    /// Fallback arm: yield a single owned kebab-cased property, then finish.
    Fallback(Option<String>),
}

impl Iterator for DisassembleProperty {
    type Item = Cow<'static, str>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            DisassembleProperty::Mapped(iter) => iter.next().map(|s| Cow::Borrowed(*s)),
            DisassembleProperty::Custom(iter) => iter.next().map(Cow::Owned),
            DisassembleProperty::Fallback(slot) => slot.take().map(Cow::Owned),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            DisassembleProperty::Mapped(iter) => iter.size_hint(),
            DisassembleProperty::Custom(iter) => iter.size_hint(),
            DisassembleProperty::Fallback(slot) => {
                let n = usize::from(slot.is_some());
                (n, Some(n))
            }
        }
    }
}

impl ExactSizeIterator for DisassembleProperty {}

#[must_use]
pub fn disassemble_property(property: &str) -> DisassembleProperty {
    let _admission = crate::admission::enter();
    // Nested selector keys (`&:hover`, `:focus`, `.parent &`) are not properties;
    // keep them verbatim so class names and case survive.
    if property.starts_with(':') || property.contains('&') {
        return DisassembleProperty::Fallback(Some(property.to_string()));
    }
    if let Some(properties) = HAS_CUSTOM_SHORTHANDS
        .load(Ordering::Relaxed)
        .then(|| {
            let _root = crate::root_held::RootHeld::enter("shorthands");
            CUSTOM_SHORTHANDS
                .read()
                .ok()
                .and_then(|shorthands| shorthands.get(property).cloned())
        })
        .flatten()
    {
        return DisassembleProperty::Custom(properties.into_iter());
    }

    GLOBAL_STYLE_PROPERTY.get(property).map_or_else(
        || {
            DisassembleProperty::Fallback(Some(
                // Gate the three vendor-prefix `starts_with` scans behind a
                // single first-byte check: only `W`/`M`/`m` can begin
                // `Webkit`/`Moz`/`ms`, so every other property skips all three.
                if matches!(property.as_bytes().first(), Some(b'W' | b'M' | b'm'))
                    && ((property.starts_with("Webkit")
                        && property.len() > 6
                        && property.as_bytes()[6].is_ascii_uppercase())
                        || (property.starts_with("Moz")
                            && property.len() > 3
                            && property.as_bytes()[3].is_ascii_uppercase())
                        || (property.starts_with("ms")
                            && property.len() > 2
                            && property.as_bytes()[2].is_ascii_uppercase()))
                {
                    // Build `-<kebab>` directly into ONE buffer instead of allocating
                    // a `to_kebab_case(property)` String and copying it into a second
                    // presized buffer. This inlines `to_kebab_case`'s exact conversion
                    // (ASCII-uppercase char ??`-` before it when not first, then its
                    // lowercase; other chars copied verbatim) after the leading `-`.
                    // The `i != 0` guard matches `to_kebab_case`, so the vendor
                    // prefix's uppercase first char (`W`/`M`/`m`→lowercase) gets no
                    // extra `-`. Output byte-identical, one fewer allocation.
                    let mut s = String::with_capacity(property.len() + 5);
                    s.push('-');
                    for (i, c) in property.chars().enumerate() {
                        if c.is_ascii_uppercase() {
                            if i != 0 {
                                s.push('-');
                            }
                            s.push(c.to_ascii_lowercase());
                        } else {
                            s.push(c);
                        }
                    }
                    s
                } else {
                    to_kebab_case(property).into_owned()
                },
            ))
        },
        |v| DisassembleProperty::Mapped(v.iter()),
    )
}

static CUSTOM_SHORTHANDS: LazyLock<RwLock<BTreeMap<String, Vec<String>>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));
static HAS_CUSTOM_SHORTHANDS: AtomicBool = AtomicBool::new(false);

/// Replace the custom shorthand registry used by style extraction.
pub fn set_custom_shorthands(shorthands: BTreeMap<String, Vec<String>>) {
    let _admission = crate::admission::enter();
    crate::admission::assert_administration_allowed("set_custom_shorthands");
    let _root = crate::root_held::RootHeld::enter("shorthands");
    if let Ok(mut registry) = CUSTOM_SHORTHANDS.write() {
        let shorthands: BTreeMap<String, Vec<String>> = shorthands
            .into_iter()
            .map(|(name, properties)| {
                let properties = properties
                    .into_iter()
                    .flat_map(|property| {
                        GLOBAL_STYLE_PROPERTY.get(property.as_str()).map_or_else(
                            || vec![to_kebab_case(&property).into_owned()],
                            |mapped| mapped.iter().map(|value| (*value).to_string()).collect(),
                        )
                    })
                    .collect();
                (name, properties)
            })
            .collect();
        HAS_CUSTOM_SHORTHANDS.store(!shorthands.is_empty(), Ordering::Relaxed);
        *registry = shorthands;
    }
}

#[must_use]
pub fn get_custom_shorthand_names() -> Vec<String> {
    let _admission = crate::admission::enter();
    let _root = crate::root_held::RootHeld::enter("shorthands");
    CUSTOM_SHORTHANDS.read().map_or_else(
        |_| Vec::new(),
        |registry| registry.keys().cloned().collect(),
    )
}

#[must_use]
pub fn add_selector_params(selector: StyleSelector, params: &str) -> StyleSelector {
    match selector {
        StyleSelector::Selector(value) => StyleSelector::Selector(format!("{value}({params})")),
        StyleSelector::Global(value, file) => {
            StyleSelector::Global(format!("{value}({params})"), file)
        }
        StyleSelector::At {
            kind,
            query,
            selector,
            outer,
            file,
        } => StyleSelector::At {
            kind,
            query,
            selector: selector.map(|s| format!("{s}({params})")),
            outer,
            file,
        },
    }
}

/// True when `property` is a known enum style property (e.g. `display`).
///
/// Holds regardless of whether a specific value maps to an expansion. Cheap
/// phf `contains_key`; lets callers distinguish "not an enum prop" from
/// "enum prop, value not mapped" without materializing an empty `Vec`.
#[must_use]
pub fn is_enum_property(property: &str) -> bool {
    GLOBAL_ENUM_STYLE_PROPERTY.contains_key(property)
}

/// Borrow the phf expansion map for `(property, value)` without materializing a `Vec`.
///
/// The caller iterates `.entries()`/reads `.len()` directly off the static map, so the
/// enum-property extract hot path (every `display`/`alignItems`/??prop per breakpoint
/// level) no longer allocates a throwaway `Vec` per call.
#[must_use]
pub fn get_enum_property_value(
    property: &str,
    value: &str,
) -> Option<&'static phf::Map<&'static str, &'static str>> {
    GLOBAL_ENUM_STYLE_PROPERTY
        .get(property)
        .and_then(|map| map.get(value))
}

#[must_use]
pub fn get_enum_property_map(property: &str) -> Option<BTreeMap<&str, BTreeMap<&str, &str>>> {
    GLOBAL_ENUM_STYLE_PROPERTY.get(property).map(|map| {
        map.entries()
            .map(|(k, v)| (*k, v.entries().map(|(k, v)| (*k, *v)).collect()))
            .collect()
    })
}

thread_local! {
    /// Reusable scratch buffer for building a class-map key. Because the common
    /// path only PROBES the map with a borrowed `&str`, the key never needs its
    /// own heap allocation per call ??it is built into this buffer, borrowed for
    /// the probe, and only `.to_string()`-cloned on a genuine insert. Reusing
    /// one buffer removes the per-generated-name key `String` allocation on the
    /// hot repeat-property path.
    static KEY_BUF: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Get-or-insert a key in the per-file class map and return its numeric slot.
/// `build_key` fills the supplied reusable buffer with the key bytes; the buffer
/// is borrowed for the probe so the common already-present path allocates
/// nothing, and only a real insert clones the key into an owned `String`.
/// Single home for the class naming algorithm shared by keyframes, classname
/// and variable-name generation.
fn class_slot_for_key(filename_key: &str, build_key: impl FnOnce(&mut String)) -> CounterSlot {
    let _admission = crate::admission::enter();
    KEY_BUF.with(|buf| {
        let _root = crate::root_held::RootHeld::enter("key_buf");
        let mut key = buf.borrow_mut();
        key.clear();
        build_key(&mut key);
        counter_allocation::reserve_counter(filename_key, key.as_str())
    })
}

/// Render the existing name from the slot returned by the single allocation request.
fn class_num_for_key(filename_key: &str, build_key: impl FnOnce(&mut String)) -> String {
    class_slot_for_key(filename_key, build_key).name()
}

/// An animation's name from its escaped content: equal animations share one
/// name wherever they are met.
#[must_use]
pub fn keyframes_name_of_escaped(escaped: &str) -> String {
    let _admission = crate::admission::enter();
    with_prefix(|prefix| format!("{prefix}{}{escaped}", if is_debug() { "k-" } else { "K" }))
}

#[must_use]
pub fn keyframes_to_keyframes_name(keyframes: &str, filename: Option<&str>) -> String {
    let _admission = crate::admission::enter();
    if atom_hoist::is_atom_hoist() {
        let scope = filename.map_or_else(
            || "g".to_string(),
            |file| format!("l-{}", atom_name::hex(&file_map::canonical(file))),
        );
        return with_prefix(|prefix| format!("{prefix}k1-{scope}-{}", atom_name::hex(keyframes)));
    }
    let mut escaped = String::with_capacity(keyframes.len() + 8);
    naming::escape_into(&mut escaped, keyframes);
    keyframes_name_of_escaped(&escaped)
}

pub fn sheet_to_classname(
    property: &str,
    level: u8,
    value: Option<&str>,
    selector: Option<&str>,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> String {
    sheet_to_classname_named(
        property,
        level,
        value,
        selector,
        style_order,
        filename,
        Naming::Own,
    )
}

pub fn sheet_to_classname_named(
    property: &str,
    level: u8,
    value: Option<&str>,
    selector: Option<&str>,
    style_order: Option<u8>,
    filename: Option<&str>,
    naming: Naming,
) -> String {
    let selector = selector.map(|text| StyleSelector::Selector(text.trim().to_string()));
    sheet_to_classname_content(
        &content_name::AtomContent {
            property: property.trim(),
            value,
            level,
            order: style_order.unwrap_or(255),
            naming,
            selector: selector.as_ref(),
            layer: None,
            dynamic: false,
        },
        filename,
    )
}

/// Name the same structured content that the sheet validates before insertion.
#[must_use]
pub fn sheet_to_classname_content(
    content: &content_name::AtomContent<'_>,
    filename: Option<&str>,
) -> String {
    sheet_to_classname_owned(content, filename, CounterOwner::Inactive)
}

/// Keep original counter ownership separate from canonical content and delivery scope.
#[must_use]
pub fn sheet_to_classname_owned(
    content: &content_name::AtomContent<'_>,
    filename: Option<&str>,
    owner: CounterOwner,
) -> String {
    let _admission = crate::admission::enter();
    let descriptor = content.content();
    match naming::owned_private_counter(owner, (filename, content.order), content.naming) {
        Some(id) => {
            let scope = format!("D9-{id}");
            let number = class_num_for_key(&scope, |key| {
                key.push_str(&atom_name::hex(&descriptor.lossless));
            });
            let file = num_to_nm_base(usize::try_from(id).unwrap_or_default());
            with_prefix(|prefix| format!("{prefix}{file}-{number}"))
        }
        None => naming_scope::name(
            &descriptor,
            (filename, content.order),
            content_hash::FingerprintBits::PRODUCTION,
        ),
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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[serial]
    fn test_with_prefix_borrows_host_prefix() {
        set_prefix(Some("host-".to_string()));

        let prefix = with_prefix(str::to_string);

        assert_eq!(prefix, "host-");
        set_prefix(None);
    }

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
            "OLbackground-vred"
        );
        let hover = sheet_to_classname("background", 0, Some("red"), Some("hover"), None, None);
        assert_eq!(hover.len(), 18);
        assert!(hover.starts_with("OH"));
        assert_eq!(
            sheet_to_classname("background", 1, None, None, None, None),
            "OLbackground-l1"
        );
        let responsive_hover = sheet_to_classname("background", 1, None, Some("hover"), None, None);
        assert_eq!(responsive_hover.len(), 18);
        assert_ne!(responsive_hover, hover);

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

        assert_eq!(get_class_map().len(), 0, "the shared sheet has no counter");
        assert_eq!(
            sheet_to_classname("background", 0, Some("#fff"), None, None, None),
            sheet_to_classname("  background  ", 0, Some("#FFF"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffff"), None, None, None),
            sheet_to_classname("background", 0, Some("#FFF"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffff"), None, None, None),
            sheet_to_classname("background", 0, Some("#FFFFFF"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffffAA"), None, None, None),
            sheet_to_classname("background", 0, Some("#FFFFFFaa"), None, None, None),
        );

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
            "OLbackground"
        );
        assert_eq!(
            sheet_to_classname("background", 0, None, None, Some(1), None),
            "OLbackground-o1"
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname("width", 0, Some("0px"), None, None, None),
            "OLwidth-v0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0em"), None, None, None),
            "OLwidth-v0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0rem"), None, None, None),
            "OLwidth-v0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0vh"), None, None, None),
            "OLwidth-v0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0%"), None, None, None),
            "OLwidth-v0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0dvh"), None, None, None),
            "OLwidth-v0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0dvw"), None, None, None),
            "OLwidth-v0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0vw"), None, None, None),
            "OLwidth-v0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0"), None, None, None),
            "OLwidth-v0"
        );
        let border = sheet_to_classname("border", 0, Some("solid 0 red"), None, None, None);
        assert_eq!(border.len(), 18);
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0px red"), None, None, None),
            border
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0% red"), None, None, None),
            border
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0em red"), None, None, None),
            border
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0rem red"), None, None, None),
            border
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0vh red"), None, None, None),
            border
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0vw red"), None, None, None),
            border
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0dvh red"), None, None, None),
            border
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0dvw red"), None, None, None),
            border
        );

        assert_eq!(
            sheet_to_classname("test", 0, Some("0px 0"), None, None, None),
            "OLtest-v0_s0"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0em 0"), None, None, None),
            "OLtest-v0_s0"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0rem 0"), None, None, None),
            "OLtest-v0_s0"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0vh 0"), None, None, None),
            "OLtest-v0_s0"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0vw 0"), None, None, None),
            "OLtest-v0_s0"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0dvh 0"), None, None, None),
            "OLtest-v0_s0"
        );

        assert_eq!(
            sheet_to_classname("test", 0, Some("0 0vh"), None, None, None),
            "OLtest-v0_s0"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0 0vw"), None, None, None),
            "OLtest-v0_s0"
        );

        reset_class_map();
        let transition = sheet_to_classname(
            "transition",
            0,
            Some("all .3s ease-in-out"),
            None,
            None,
            None,
        );
        assert_eq!(transition.len(), 18);
        assert_eq!(
            sheet_to_classname(
                "transition",
                0,
                Some("all 0.3s ease-in-out"),
                None,
                None,
                None
            ),
            transition
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
            transition
        );
    }

    #[test]
    #[serial]
    fn test_debug_sheet_to_classname() {
        set_debug(true);
        assert_eq!(
            sheet_to_classname("background", 0, None, None, None, None),
            "OLbackground"
        );
        let hover = sheet_to_classname("background", 0, Some("red"), Some("hover"), None, None);
        assert!(hover.starts_with("OH"));
        assert_eq!(hover.len(), 18);
        assert_eq!(
            sheet_to_classname("background", 1, None, None, None, None),
            "OLbackground-l1"
        );
        assert_ne!(
            sheet_to_classname("background", 1, Some("red"), Some("hover"), None, None),
            hover
        );
    }

    #[test]
    #[serial]
    fn test_debug_sheet_to_classname_encodes_value() {
        set_debug(true);
        for (property, value) in [
            ("scale", "0.8"),
            ("background", "$text"),
            ("color", "#fff"),
            ("height", "50%"),
            ("transition", "all .2s ease-in-out"),
        ] {
            let name = sheet_to_classname(property, 0, Some(value), None, None, None);
            assert!(name.len() <= 18, "{name}");
            assert!(
                name.bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')),
                "{name}"
            );
        }
        set_debug(false);
    }

    #[test]
    #[serial]
    fn test_debug_sheet_to_classname_with_filename() {
        reset_class_map();
        file_map::reset_file_map();
        set_debug(true);
        let class_name =
            sheet_to_classname("background", 0, Some("red"), None, None, Some("test.tsx"));
        assert_eq!(class_name, "FLtest_ptsx-OLbackground-vred");
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
                    selector: None,
                    outer: vec![],
                    file: None,
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
                    selector: Some("&:hover".to_string()),
                    outer: vec![],
                    file: None,
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
        map.insert(String::new(), HashMap::new());
        map.entry(String::new())
            .or_default()
            .insert("background-0-rgba(255,0,0,0.5)-".to_string(), 1);
        set_class_map(map);
        assert_eq!(get_class_map().len(), 1);
    }

    #[test]
    #[serial]
    fn test_keyframes_to_keyframes_name() {
        reset_class_map();
        set_debug(false);
        assert_eq!(keyframes_to_keyframes_name("spin", None), "Kspin");
        assert_eq!(keyframes_to_keyframes_name("spin", None), "Kspin");
        assert_eq!(keyframes_to_keyframes_name("spin2", None), "Kspin2");
        assert_eq!(
            keyframes_to_keyframes_name("spin", Some("a.tsx")),
            "Kspin",
            "equal animations share one name in every file"
        );
        assert_eq!(get_class_map().len(), 0, "no counter is used");
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
                    selector: Some("&:is".to_string()),
                    outer: vec![],
                    file: None,
                },
                "test"
            ),
            StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "print".to_string(),
                selector: Some("&:is(test)".to_string()),
                outer: vec![],
                file: None,
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
        assert_eq!(class1, "app-OLbackground-vred");

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
        assert_eq!(class_name, "my-OLbackground-vred");

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

        set_prefix(Some(String::new()));
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
        let name = keyframes_to_keyframes_name("spin", Some("test.tsx"));

        let name2 = keyframes_to_keyframes_name("spin", Some("test.tsx"));
        assert_eq!(name, name2);

        let name3 = keyframes_to_keyframes_name("spin", Some("other.tsx"));
        assert_eq!(name, name3, "the name is the content, not the file");
        assert_ne!(name, keyframes_to_keyframes_name("spin2", Some("test.tsx")));
    }

    #[test]
    #[serial]
    fn test_sheet_to_classname_with_filename() {
        reset_class_map();
        file_map::reset_file_map();
        set_debug(false);
        // Test with filename to cover the filename branch
        let class1 = sheet_to_classname("background", 0, Some("red"), None, None, Some("test.tsx"));
        // Should include file number prefix
        assert!(class1.contains('-'));

        // Same property in same file should return same classname
        let class2 = sheet_to_classname("background", 0, Some("red"), None, None, Some("test.tsx"));
        assert_eq!(class1, class2);

        let class3 =
            sheet_to_classname("background", 0, Some("red"), None, None, Some("other.tsx"));
        assert_ne!(
            class1, class3,
            "independently delivered sheets must retain distinct identities without D9"
        );
    }

    #[test]
    #[serial]
    fn test_disassemble_property_size_hint() {
        // Mapped arm: the hint comes straight from the borrowed slice iterator.
        let mapped = disassemble_property("bg");
        assert_eq!(mapped.size_hint(), (1, Some(1)));
        assert_eq!(mapped.count(), 1);

        // Fallback arm: one pending kebab-cased property, then nothing.
        let mut fallback = disassemble_property("someUnmappedProperty");
        assert_eq!(fallback.size_hint(), (1, Some(1)));
        assert_eq!(fallback.next().as_deref(), Some("some-unmapped-property"));
        assert_eq!(fallback.size_hint(), (0, Some(0)));
    }

    #[test]
    #[serial]
    fn test_custom_shorthand() {
        set_custom_shorthands(BTreeMap::from([(
            "insetX".to_string(),
            vec![
                "left".to_string(),
                "marginRight".to_string(),
                "py".to_string(),
            ],
        )]));

        assert_eq!(
            disassemble_property("insetX").collect::<Vec<_>>(),
            ["left", "margin-right", "padding-top", "padding-bottom"]
        );

        set_custom_shorthands(BTreeMap::new());
        assert_eq!(
            disassemble_property("insetX").collect::<Vec<_>>(),
            ["inset-x"]
        );
    }
}
