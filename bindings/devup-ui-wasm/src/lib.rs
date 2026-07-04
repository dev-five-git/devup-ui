use css::class_map::{set_class_map, with_class_map};
use css::file_map::{
    canonical, is_global, set_canonical_map, set_file_map, with_canonical_map, with_file_map,
};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::{ExtractOption, ImportAlias, extract, has_devup_ui};
use rustc_hash::FxHashSet;
use sheet::StyleSheet;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{LazyLock, Mutex};
use wasm_bindgen::prelude::*;

static GLOBAL_STYLE_SHEET: LazyLock<Mutex<StyleSheet>> =
    LazyLock::new(|| Mutex::new(StyleSheet::default()));

fn with_style_sheet<F, R>(f: F) -> R
where
    F: FnOnce(&StyleSheet) -> R,
{
    let guard = GLOBAL_STYLE_SHEET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&guard)
}

fn with_style_sheet_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut StyleSheet) -> R,
{
    let mut guard = GLOBAL_STYLE_SHEET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut guard)
}

#[cfg(not(tarpaulin_include))]
fn js_error(message: impl Display) -> JsValue {
    js_sys::Error::new(&message.to_string()).into()
}

#[wasm_bindgen]
pub struct Output {
    code: String,
    map: Option<String>,
    css_file: Option<String>,
    updated_base_style: bool,
    css: Option<String>,
}
#[wasm_bindgen]
impl Output {
    fn new(
        code: String,
        styles: FxHashSet<ExtractStyleValue>,
        map: Option<String>,
        single_css: bool,
        filename: String,
        css_file: Option<String>,
        import_main_css: bool,
    ) -> Self {
        // Use the bucket identity (single-importer collapse) so the sheet's CSS
        // naming + property bucket + emitted chunk match the canonical class names
        // the extractor already baked into `code`. Identity when no map is loaded.
        // Global (shared-chunk) files are treated like single-css: emitted into the
        // global bucket (devup-ui.css) with prefix-less naming.
        let canonical_filename = canonical(&filename);
        let global = single_css || is_global(&filename);
        with_style_sheet_mut(|sheet| {
            // globalCss (@font-face / global selectors) is per-SOURCE-file, never
            // collapsed. rm_global_css MUST use the RAW filename so a collapsed
            // member (sharing the bucket-root's canonical) never wipes the root's
            // globalCss. Atom property bucketing still uses canonical_filename.
            let default_collected = sheet.rm_global_css(&filename, global);
            let (collected, updated_base_style) =
                sheet.update_styles(&styles, &canonical_filename, global);
            Self {
                code,
                map,
                css_file,
                updated_base_style: updated_base_style || default_collected,
                css: {
                    if !collected && !default_collected {
                        None
                    } else {
                        Some(sheet.create_css(
                            if global {
                                None
                            } else {
                                Some(&canonical_filename)
                            },
                            import_main_css,
                        ))
                    }
                },
            }
        })
    }

    /// Get the code
    #[wasm_bindgen(getter, js_name = "code")]
    #[must_use]
    pub fn code(&self) -> String {
        self.code.clone()
    }

    #[wasm_bindgen(getter, js_name = "cssFile")]
    #[must_use]
    pub fn css_file(&self) -> Option<String> {
        self.css_file.clone()
    }

    #[wasm_bindgen(getter, js_name = "map")]
    #[must_use]
    pub fn map(&self) -> Option<String> {
        self.map.clone()
    }

    #[wasm_bindgen(getter, js_name = "updatedBaseStyle")]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn updated_base_style(&self) -> bool {
        self.updated_base_style
    }

    /// Get the css
    #[wasm_bindgen(getter, js_name = "css")]
    #[must_use]
    pub fn css(&self) -> Option<String> {
        self.css.clone()
    }
}

#[wasm_bindgen(js_name = "setDebug")]
pub fn set_debug(debug: bool) {
    css::debug::set_debug(debug);
}

#[wasm_bindgen(js_name = "isDebug")]
#[must_use]
pub fn is_debug() -> bool {
    css::debug::is_debug()
}

/// Set the CSS class name prefix
///
/// # Example (Vite Config)
/// ```javascript
/// import init, { setPrefix, codeExtract } from 'devup-ui-wasm';
///
/// export default {
///   plugins: [
///     {
///       name: 'devup-ui',
///       apply: 'pre',
///       async configResolved() {
///         await init();
///         setPrefix('du-'); // Set prefix to 'du-'
///       },
///       // ... other plugin code
///     }
///   ]
/// }
/// ```
///
/// # Example (Next.js Plugin)
/// ```typescript
/// import init, { setPrefix } from 'devup-ui-wasm';
///
/// const withDevupUI = (nextConfig) => {
///   return {
///     ...nextConfig,
///     webpack: (config, options) => {
///       if (!options.isServer && !global.devupUIInitialized) {
///         init().then(() => {
///           setPrefix('du-');
///           global.devupUIInitialized = true;
///         });
///       }
///       return config;
///     }
///   };
/// };
/// ```
#[wasm_bindgen(js_name = "setPrefix")]
pub fn set_prefix(prefix: Option<String>) {
    css::set_prefix(prefix);
}

#[wasm_bindgen(js_name = "getPrefix")]
#[must_use]
pub fn get_prefix() -> Option<String> {
    css::get_prefix()
}

/// Internal function to import a `StyleSheet` (testable without `JsValue`)
pub fn import_sheet_internal(sheet: StyleSheet) {
    with_style_sheet_mut(|global_sheet| *global_sheet = sheet);
}

#[wasm_bindgen(js_name = "importSheet")]
#[cfg(not(tarpaulin_include))]
pub fn import_sheet(sheet_object: JsValue) -> Result<(), JsValue> {
    let sheet: StyleSheet = serde_wasm_bindgen::from_value(sheet_object).map_err(js_error)?;
    import_sheet_internal(sheet);
    Ok(())
}

/// Internal function to export `StyleSheet` as JSON string (testable without `JsValue`)
pub fn export_sheet_internal() -> Result<String, String> {
    with_style_sheet(serde_json::to_string).map_err(|e| e.to_string())
}

#[wasm_bindgen(js_name = "exportSheet")]
#[cfg(not(tarpaulin_include))]
pub fn export_sheet() -> Result<String, JsValue> {
    export_sheet_internal().map_err(js_error)
}

/// Internal function to export class map as JSON string (testable without `JsValue`)
pub fn export_class_map_internal() -> Result<String, String> {
    with_class_map(serde_json::to_string).map_err(|e| e.to_string())
}

#[wasm_bindgen(js_name = "importClassMap")]
#[cfg(not(tarpaulin_include))]
pub fn import_class_map(sheet_object: JsValue) -> Result<(), JsValue> {
    set_class_map(serde_wasm_bindgen::from_value(sheet_object).map_err(js_error)?);
    Ok(())
}

#[wasm_bindgen(js_name = "exportClassMap")]
#[cfg(not(tarpaulin_include))]
pub fn export_class_map() -> Result<String, JsValue> {
    export_class_map_internal().map_err(js_error)
}

/// Internal function to export file map as JSON string (testable without `JsValue`)
pub fn export_file_map_internal() -> Result<String, String> {
    with_file_map(serde_json::to_string).map_err(|e| e.to_string())
}

#[wasm_bindgen(js_name = "importFileMap")]
#[cfg(not(tarpaulin_include))]
pub fn import_file_map(sheet_object: JsValue) -> Result<(), JsValue> {
    set_file_map(serde_wasm_bindgen::from_value(sheet_object).map_err(js_error)?);
    Ok(())
}

#[wasm_bindgen(js_name = "exportFileMap")]
#[cfg(not(tarpaulin_include))]
pub fn export_file_map() -> Result<String, JsValue> {
    export_file_map_internal().map_err(js_error)
}

/// Internal function to import the canonical (bucket) map (testable without `JsValue`)
pub fn import_canonical_map_internal(map: HashMap<String, String>) {
    set_canonical_map(map);
}

/// Internal function to export the canonical map as JSON string (testable without `JsValue`)
pub fn export_canonical_map_internal() -> Result<String, String> {
    with_canonical_map(serde_json::to_string).map_err(|e| e.to_string())
}

#[wasm_bindgen(js_name = "importCanonicalMap")]
#[cfg(not(tarpaulin_include))]
pub fn import_canonical_map(map_object: JsValue) -> Result<(), JsValue> {
    set_canonical_map(serde_wasm_bindgen::from_value(map_object).map_err(js_error)?);
    Ok(())
}

#[wasm_bindgen(js_name = "exportCanonicalMap")]
#[cfg(not(tarpaulin_include))]
pub fn export_canonical_map() -> Result<String, JsValue> {
    export_canonical_map_internal().map_err(js_error)
}

/// Set the atom-level hoist threshold.
///
/// When set to `Some(n)`, a style atom whose content is used by `>= n` distinct
/// routes is emitted into the shared global `devup-ui.css` (shipped once) instead
/// of duplicated into each per-route chunk. `None` (the default) disables atom
/// hoisting entirely (identity behavior).
///
/// MUST be called BEFORE `codeExtract` so atoms receive global (shared) class
/// names; enabling it afterwards leaves per-file names and nothing hoists.
/// Pair with `importFileRoutes` to provide the file -> routes mapping.
#[wasm_bindgen(js_name = "setAtomHoist")]
pub fn set_atom_hoist(threshold: Option<usize>) {
    css::atom_hoist::set_atom_hoist(threshold);
}

/// Internal function to import the file -> routes map (testable without `JsValue`)
pub fn import_file_routes_internal(map: HashMap<String, std::collections::HashSet<u32>>) {
    css::file_routes::set_file_routes(map);
}

/// Import the file -> set-of-route-ids mapping used to decide atom hoisting.
///
/// Accepts a JS object like `{ "src/page.tsx": [0, 3], "src/card.tsx": [0] }`
/// where each value is the set of leaf-route ids whose render closure includes
/// that file. Populated by the build-time pre-pass.
#[wasm_bindgen(js_name = "importFileRoutes")]
#[cfg(not(tarpaulin_include))]
pub fn import_file_routes(map_object: JsValue) -> Result<(), JsValue> {
    css::file_routes::set_file_routes(
        serde_wasm_bindgen::from_value(map_object).map_err(js_error)?,
    );
    Ok(())
}

/// Internal function to extract code (testable without `JsValue`)
#[allow(clippy::too_many_arguments)]
pub fn code_extract_internal(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: HashMap<String, ImportAlias>,
) -> Result<Output, String> {
    match extract(
        filename,
        code,
        ExtractOption {
            package: package.to_string(),
            css_dir,
            single_css,
            import_main_css: import_main_css_in_code,
            import_aliases,
        },
    ) {
        Ok(output) => Ok(Output::new(
            output.code,
            output.styles,
            output.map,
            single_css,
            filename.to_string(),
            output.css_file,
            import_main_css_in_css,
        )),
        Err(error) => Err(error.to_string()),
    }
}

#[wasm_bindgen(js_name = "codeExtract")]
#[allow(clippy::too_many_arguments)]
#[cfg(not(tarpaulin_include))]
pub fn code_extract(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: JsValue,
) -> Result<Output, JsValue> {
    // Deserialize import_aliases from JsValue
    // Format: { "package": "namedExport" } or { "package": null } for named exports
    let aliases: HashMap<String, Option<String>> =
        serde_wasm_bindgen::from_value(import_aliases).map_err(js_error)?;

    // Convert to ImportAlias enum
    let import_aliases: HashMap<String, ImportAlias> = aliases
        .into_iter()
        .map(|(k, v)| {
            let alias = match v {
                Some(name) => ImportAlias::DefaultToNamed(name),
                None => ImportAlias::NamedToNamed,
            };
            (k, alias)
        })
        .collect();

    code_extract_internal(
        filename,
        code,
        package,
        css_dir,
        single_css,
        import_main_css_in_code,
        import_main_css_in_css,
        import_aliases,
    )
    .map_err(js_error)
}

/// Internal function to register theme (testable without `JsValue`)
pub fn register_theme_internal(theme: sheet::theme::Theme) {
    with_style_sheet_mut(|sheet| sheet.set_theme(theme));
}

#[wasm_bindgen(js_name = "registerTheme")]
#[cfg(not(tarpaulin_include))]
pub fn register_theme(theme_object: JsValue) -> Result<(), JsValue> {
    let theme: sheet::theme::Theme =
        serde_wasm_bindgen::from_value(theme_object).map_err(js_error)?;
    register_theme_internal(theme);
    Ok(())
}

#[wasm_bindgen(js_name = "getDefaultTheme")]
#[cfg(not(tarpaulin_include))]
pub fn get_default_theme() -> Result<Option<String>, JsValue> {
    Ok(with_style_sheet(|sheet| sheet.theme.get_default_theme()))
}

#[wasm_bindgen(js_name = "getCss")]
#[cfg(not(tarpaulin_include))]
pub fn get_css(file_num: Option<usize>, import_main_css: bool) -> Result<String, JsValue> {
    Ok(with_style_sheet(|sheet| {
        if let Some(file_num) = file_num {
            with_file_map(|map| {
                sheet.create_css(
                    map.get_by_right(&file_num).map(String::as_str),
                    import_main_css,
                )
            })
        } else {
            sheet.create_css(None, import_main_css)
        }
    }))
}

#[wasm_bindgen(js_name = "getThemeInterface")]
#[cfg(not(tarpaulin_include))]
pub fn get_theme_interface(
    package_name: &str,
    color_interface_name: &str,
    typography_interface_name: &str,
    length_interface_name: &str,
    shadows_interface_name: &str,
    theme_interface_name: &str,
) -> String {
    with_style_sheet(|sheet| {
        sheet.create_interface(
            package_name,
            color_interface_name,
            typography_interface_name,
            length_interface_name,
            shadows_interface_name,
            theme_interface_name,
        )
    })
}

#[wasm_bindgen(js_name = "hasDevupUI")]
#[cfg(not(tarpaulin_include))]
#[must_use]
pub fn has_devup_ui_wasm(filename: &str, code: &str, package: &str) -> bool {
    has_devup_ui(filename, code, package)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use serial_test::serial;
    use sheet::theme::{ColorTheme, Theme, Typography};

    fn make_named_color_theme(name: &str, value: &str) -> ColorTheme {
        let mut ct = ColorTheme::default();
        ct.add_color(name, value);
        ct
    }

    #[test]
    #[serial]
    fn atom_hoist_splits_global_and_private() {
        use css::atom_hoist::set_atom_hoist;
        use css::class_map::reset_class_map;
        use css::file_map::reset_file_map;
        use css::file_routes::{reset_file_routes, set_file_routes};
        use std::collections::{HashMap, HashSet};

        {
            let mut s = GLOBAL_STYLE_SHEET.lock().unwrap();
            *s = StyleSheet::default();
        }
        reset_class_map();
        reset_file_map();
        reset_file_routes();
        register_theme_internal(sheet::theme::Theme::default());

        // a.tsx -> route 0, b.tsx -> route 1. bg:red is in BOTH (routes {0,1}, count 2 => HOIST).
        // width:11px only in a (route {0}, private). width:22px only in b (private).
        let mut fr = HashMap::new();
        fr.insert("a.tsx".to_string(), HashSet::from([0u32]));
        fr.insert("b.tsx".to_string(), HashSet::from([1u32]));
        set_file_routes(fr);
        set_atom_hoist(Some(2));

        let srca = r#"import { Box } from "@devup-ui/react"; const x = <Box bg="red" w="11px" />;"#;
        let srcb = r#"import { Box } from "@devup-ui/react"; const x = <Box bg="red" w="22px" />;"#;
        code_extract_internal(
            "a.tsx",
            srca,
            "@devup-ui/react",
            "df".to_string(),
            false,
            false,
            false,
            HashMap::new(),
        )
        .unwrap();
        code_extract_internal(
            "b.tsx",
            srcb,
            "@devup-ui/react",
            "df".to_string(),
            false,
            false,
            false,
            HashMap::new(),
        )
        .unwrap();

        let global = with_style_sheet(|s| s.create_css(None, false));
        let chunk_a = with_style_sheet(|s| s.create_css(Some("a.tsx"), false));
        let chunk_b = with_style_sheet(|s| s.create_css(Some("b.tsx"), false));

        // hoisted shared atom in the global file, ONCE; not the private ones
        assert!(
            global.contains("background:red"),
            "global must have hoisted bg:red"
        );
        assert_eq!(
            global.matches("background:red").count(),
            1,
            "bg:red deduped once in global"
        );
        assert!(
            !global.contains("width:11px") && !global.contains("width:22px"),
            "private atoms not in global"
        );

        // private atoms in their route chunk; hoisted atom NOT duplicated there
        assert!(
            chunk_a.contains("width:11px") && !chunk_a.contains("background:red"),
            "chunk a: private only"
        );
        assert!(
            chunk_b.contains("width:22px") && !chunk_b.contains("background:red"),
            "chunk b: private only"
        );

        set_atom_hoist(None);
        reset_file_routes();
    }

    /// Env-gated artifact emitter for split-native measurement. Writes REAL
    /// devup CSS output (header, `@layer`, naming, dedup all authentic) for three
    /// delivery models across several workloads, so an external script can
    /// measure gzip/brotli + multi-route session + incremental-invalidation
    /// bytes. Set `DEVUP_EMIT_MEASURE=1` to run; no-op (and zero cost) otherwise
    /// so the normal test suite stays clean.
    #[test]
    #[serial]
    #[allow(clippy::items_after_statements, clippy::format_push_string)]
    fn emit_split_measurement_artifacts() {
        use css::atom_hoist::set_atom_hoist;
        use css::class_map::reset_class_map;
        use css::file_map::reset_file_map;
        use css::file_routes::{reset_file_routes, set_file_routes};
        use std::collections::{HashMap, HashSet};
        use std::fs;

        if std::env::var("DEVUP_EMIT_MEASURE").is_err() {
            return;
        }

        let props = [
            "w",
            "h",
            "p",
            "m",
            "minW",
            "minH",
            "maxW",
            "maxH",
            "fontSize",
            "lineHeight",
            "borderRadius",
            "gap",
        ];
        let atom = |key: &str, px: usize| format!("<Box {key}=\"{px}px\" />");
        let build = |els: &[String]| {
            format!(
                "import {{ Box }} from \"@devup-ui/react\"; const x = <>{}</>;",
                els.join("")
            )
        };
        let reset = || {
            {
                let mut s = GLOBAL_STYLE_SHEET.lock().unwrap();
                *s = StyleSheet::default();
            }
            reset_class_map();
            reset_file_map();
            reset_file_routes();
            register_theme_internal(sheet::theme::Theme::default());
        };

        let out = std::env::temp_dir().join("devup-split-measure");
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(&out).unwrap();

        // (name, routes, universal atoms, private atoms/route)
        let workloads = [
            ("shared_heavy", 8usize, 80usize, 25usize),
            ("balanced", 8usize, 50usize, 50usize),
            ("disjoint", 8usize, 20usize, 60usize),
        ];
        let mut manifest = String::from("[");
        for (wi, (name, n, u, p)) in workloads.iter().enumerate() {
            let (n, u, p) = (*n, *u, *p);
            let universal: Vec<String> = (0..u)
                .map(|i| atom(props[i % props.len()], 100_000 + i))
                .collect();
            let make_priv = |r: usize| -> Vec<String> {
                (0..p)
                    .map(|i| {
                        atom(
                            props[i % props.len()],
                            1_000_000 + wi * 1_000_000 + r * p + i,
                        )
                    })
                    .collect()
            };
            let sources: Vec<String> = (0..n)
                .map(|r| {
                    let mut e = universal.clone();
                    e.extend(make_priv(r));
                    build(&e)
                })
                .collect();
            let run = |single: bool| {
                reset();
                for (r, src) in sources.iter().enumerate() {
                    code_extract_internal(
                        &format!("r{r}.tsx"),
                        src,
                        "@devup-ui/react",
                        "df".to_string(),
                        single,
                        false,
                        false,
                        HashMap::new(),
                    )
                    .unwrap();
                }
            };

            // single-css: one shared file with every atom.
            run(true);
            fs::write(
                out.join(format!("{name}_single.css")),
                with_style_sheet(|s| s.create_css(None, false)),
            )
            .unwrap();

            // per-file: shared base (theme/base only) + one full chunk per route.
            run(false);
            fs::write(
                out.join(format!("{name}_perfile_base.css")),
                with_style_sheet(|s| s.create_css(None, false)),
            )
            .unwrap();
            for r in 0..n {
                fs::write(
                    out.join(format!("{name}_perfile_r{r}.css")),
                    with_style_sheet(|s| s.create_css(Some(&format!("r{r}.tsx")), false)),
                )
                .unwrap();
            }

            // atom-B: hoisted shared base (universal atoms) + per-route delta.
            // CRITICAL: atom_hoist must be enabled BEFORE extraction so atoms get
            // GLOBAL names (shared identity across files). Enabling it only at
            // create_css time leaves per-file names, so the same universal atom
            // looks like N distinct atoms (one per file) and never hoists.
            reset();
            let mut fr = HashMap::new();
            for r in 0..n {
                fr.insert(format!("r{r}.tsx"), HashSet::from([r as u32]));
            }
            set_file_routes(fr);
            set_atom_hoist(Some(n));
            for (r, src) in sources.iter().enumerate() {
                code_extract_internal(
                    &format!("r{r}.tsx"),
                    src,
                    "@devup-ui/react",
                    "df".to_string(),
                    false,
                    false,
                    false,
                    HashMap::new(),
                )
                .unwrap();
            }
            fs::write(
                out.join(format!("{name}_atomb_base.css")),
                with_style_sheet(|s| s.create_css(None, false)),
            )
            .unwrap();
            for r in 0..n {
                fs::write(
                    out.join(format!("{name}_atomb_r{r}.css")),
                    with_style_sheet(|s| s.create_css(Some(&format!("r{r}.tsx")), false)),
                )
                .unwrap();
            }
            set_atom_hoist(None);
            reset_file_routes();

            manifest.push_str(&format!(
                "{}{{\"name\":\"{name}\",\"n\":{n},\"u\":{u},\"p\":{p}}}",
                if wi > 0 { "," } else { "" }
            ));
        }
        manifest.push(']');
        fs::write(out.join("manifest.json"), manifest).unwrap();
        reset();
        set_atom_hoist(None);
        println!("[EMIT] artifacts -> {}", out.display());
    }

    /// SPLIT-NATIVE LOCK: atom-level route-aware hoisting (global-named
    /// shared-base + per-route delta) is a STRICT upgrade over the per-file mode
    /// on the metrics that split actually competes on -- multi-route SESSION
    /// bytes and incremental-deploy INVALIDATION bytes -- NOT on fresh-single-
    /// route bytes (where per-file already hits the theoretical floor).
    ///
    /// This test supersedes an earlier "no win" lock that was built on a
    /// measurement bug: enabling atom_hoist AFTER extraction left per-file class
    /// names, so the same universal atom looked like N distinct atoms and never
    /// hoisted -- making atom-B byte-identical to per-file (a no-op, not a
    /// truth). The fix, asserted here, is that atom_hoist MUST be enabled BEFORE
    /// extraction so atoms get GLOBAL (shared) names.
    #[test]
    #[serial]
    // byte sizes are tiny so ratios are exact; doc prose names models literally
    #[allow(clippy::cast_precision_loss, clippy::doc_markdown)]
    fn atom_b_beats_per_file_on_session_and_invalidation() {
        use css::atom_hoist::set_atom_hoist;
        use css::class_map::reset_class_map;
        use css::file_map::reset_file_map;
        use css::file_routes::{reset_file_routes, set_file_routes};
        use std::collections::{HashMap, HashSet};

        // Realistic design-system workload: many shared primitives, fewer
        // route-private atoms. Routes are disjoint on private atoms.
        const ROUTES: usize = 8;
        const UNIVERSAL: usize = 80;
        const PRIVATE: usize = 25;

        let props = ["w", "h", "p", "m", "minW", "minH", "maxW", "maxH"];
        let atom = |key: &str, px: usize| format!("<Box {key}=\"{px}px\" />");
        let build_source = |elements: &[String]| -> String {
            let body = elements.join("");
            format!("import {{ Box }} from \"@devup-ui/react\"; const x = <>{body}</>;")
        };
        let reset_engine = || {
            {
                let mut s = GLOBAL_STYLE_SHEET.lock().unwrap();
                *s = StyleSheet::default();
            }
            reset_class_map();
            reset_file_map();
            reset_file_routes();
            register_theme_internal(sheet::theme::Theme::default());
        };

        let universal_atoms: Vec<String> = (0..UNIVERSAL)
            .map(|i| atom(props[i % props.len()], 100_000 + i))
            .collect();
        let make_private = |route: usize| -> Vec<String> {
            (0..PRIVATE)
                .map(|i| atom(props[i % props.len()], 1_000_000 + route * PRIVATE + i))
                .collect()
        };
        let sources: Vec<String> = (0..ROUTES)
            .map(|r| {
                let mut e = universal_atoms.clone();
                e.extend(make_private(r));
                build_source(&e)
            })
            .collect();
        let extract_all = |single_css: bool| {
            for (r, src) in sources.iter().enumerate() {
                code_extract_internal(
                    &format!("r{r}.tsx"),
                    src,
                    "@devup-ui/react",
                    "df".to_string(),
                    single_css,
                    false,
                    false,
                    HashMap::new(),
                )
                .unwrap();
            }
        };

        // ---- per-file: atom_hoist OFF, multi-css. Each chunk carries all of
        // its route's atoms (universals duplicated into every chunk). ----
        reset_engine();
        extract_all(false);
        let pf_base = with_style_sheet(|s| s.create_css(None, false)).len();
        let pf_chunks: Vec<usize> = (0..ROUTES)
            .map(|r| with_style_sheet(|s| s.create_css(Some(&format!("r{r}.tsx")), false)).len())
            .collect();

        // ---- atom-B: enable hoist + routes BEFORE extraction so atoms get
        // GLOBAL names; universals (used by all ROUTES) hoist into the base,
        // privates stay in their per-route delta. ----
        reset_engine();
        let mut fr = HashMap::new();
        for r in 0..ROUTES {
            fr.insert(format!("r{r}.tsx"), HashSet::from([r as u32]));
        }
        set_file_routes(fr);
        set_atom_hoist(Some(ROUTES));
        extract_all(false);
        let ab_base = with_style_sheet(|s| s.create_css(None, false)).len();
        let ab_deltas: Vec<usize> = (0..ROUTES)
            .map(|r| with_style_sheet(|s| s.create_css(Some(&format!("r{r}.tsx")), false)).len())
            .collect();
        set_atom_hoist(None);
        reset_file_routes();
        reset_engine();

        // Session = visit every route once (base cached after the first route).
        let pf_session = pf_base + pf_chunks.iter().sum::<usize>();
        let ab_session = ab_base + ab_deltas.iter().sum::<usize>();
        // Invalidation = one route's styles change; returning user re-downloads
        // only the file(s) whose hash changed.
        let pf_invalidation = pf_chunks[0];
        let ab_invalidation = ab_deltas[0];

        let session_margin = (pf_session as f64 - ab_session as f64) / pf_session as f64 * 100.0;
        let invalidation_margin =
            (pf_invalidation as f64 - ab_invalidation as f64) / pf_invalidation as f64 * 100.0;
        println!(
            "[SPLIT] base: per-file={pf_base}B atom-B={ab_base}B | chunk: per-file={}B atom-B-delta={}B",
            pf_chunks[0], ab_deltas[0]
        );
        println!(
            "[SPLIT] session: per-file={pf_session}B atom-B={ab_session}B ({session_margin:.1}% smaller) | invalidation: per-file={pf_invalidation}B atom-B={ab_invalidation}B ({invalidation_margin:.1}% smaller)"
        );

        // Regression guard against the no-op-hoist bug: hoisting MUST have moved
        // the universal atoms into the base, so the base is large and the delta
        // is much smaller than a full per-file chunk.
        assert!(
            ab_base > pf_base + 500,
            "hoist no-op: atom-B base ({ab_base}B) should hold the universal atoms, \
             but is barely larger than the empty per-file base ({pf_base}B). \
             atom_hoist was likely enabled AFTER extraction."
        );
        assert!(
            (ab_deltas[0] as f64) < (pf_chunks[0] as f64) * 0.6,
            "hoist no-op: atom-B delta ({}B) should be far smaller than the full \
             per-file chunk ({}B) once universals are hoisted out",
            ab_deltas[0],
            pf_chunks[0]
        );
        // The split-native wins this whole investigation hinges on.
        assert!(
            session_margin >= 15.0,
            "atom-B should beat per-file on multi-route session bytes by >=15% \
             (got {session_margin:.1}%)"
        );
        assert!(
            invalidation_margin >= 30.0,
            "atom-B should beat per-file on incremental-deploy invalidation by \
             >=30% (got {invalidation_margin:.1}%)"
        );
    }

    #[test]
    #[serial]
    fn test_atom_hoist_and_file_routes_bindings() {
        use css::atom_hoist::atom_hoist_threshold;
        use css::file_routes::{get_file_routes, reset_file_routes};
        use std::collections::{HashMap, HashSet};

        // setAtomHoist binding controls the global threshold.
        set_atom_hoist(None);
        assert_eq!(atom_hoist_threshold(), None);
        set_atom_hoist(Some(4));
        assert_eq!(atom_hoist_threshold(), Some(4));
        set_atom_hoist(None);
        assert_eq!(atom_hoist_threshold(), None);

        // importFileRoutes binding populates the file->routes map.
        reset_file_routes();
        let mut m = HashMap::new();
        m.insert("a.tsx".to_string(), HashSet::from([0u32, 1]));
        m.insert("b.tsx".to_string(), HashSet::from([2u32]));
        import_file_routes_internal(m.clone());
        assert_eq!(get_file_routes(), m);
        reset_file_routes();
    }

    #[test]
    #[serial]
    fn test_canonical_map_import_export_roundtrip() {
        use css::file_map::{get_canonical_map, reset_canonical_map};
        reset_canonical_map();
        let mut m = HashMap::new();
        m.insert("src/child.tsx".to_string(), "src/parent.tsx".to_string());
        import_canonical_map_internal(m.clone());
        assert_eq!(get_canonical_map(), m);
        let json = export_canonical_map_internal().expect("export canonical map");
        assert!(json.contains("src/child.tsx"));
        assert!(json.contains("src/parent.tsx"));
        // canonical() resolves via the imported map; unmapped is identity.
        assert_eq!(canonical("src/child.tsx"), "src/parent.tsx");
        assert_eq!(canonical("src/other.tsx"), "src/other.tsx");
        reset_canonical_map();
    }

    #[test]
    #[serial]
    fn test_code_extract() {
        {
            let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
            *sheet = StyleSheet::default();
        }
        assert_eq!(
            get_css(None, false).unwrap().split("*/").nth(1).unwrap(),
            ""
        );

        {
            let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
            let mut theme = Theme::default();
            let mut color_theme = ColorTheme::default();
            color_theme.add_color("primary", "#000");
            theme.add_color_theme("dark", color_theme);

            let mut color_theme = ColorTheme::default();
            color_theme.add_color("primary", "#FFF");
            theme.add_color_theme("default", color_theme);
            sheet.set_theme(theme);
        }

        assert_debug_snapshot!(get_css(None, false).unwrap().split("*/").nth(1).unwrap());
    }

    #[test]
    #[serial]
    fn deserialize_theme() {
        {
            let theme: Theme = serde_json::from_str(
                r##"{
            "colors": {
                "default": {
                    "primary": "#000"
                },
                "dark": {
                    "primary": "#fff"
                }
            },
            "typography": {
                "default": [
                    {
                        "fontFamily": "Arial",
                        "fontSize": "16px",
                        "fontWeight": 400,
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    },
                    {
                        "fontFamily": "Arial",
                        "fontSize": "24px",
                        "fontWeight": "400",
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    },
                    {
                        "fontFamily": "Arial",
                        "fontSize": "24px",
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    }
                ]
            }
        }"##,
            )
            .unwrap();
            assert_eq!(theme.breakpoints, vec![0, 480, 768, 992, 1280, 1600]);
            assert_debug_snapshot!(theme.to_css());
        }
        {
            let theme: Theme = serde_json::from_str(
                r##"{
            "colors": {
                "default": {
                    "primary": "#000"
                },
                "dark": {
                    "primary": "#fff"
                }
            },
            "typography": {
                "default":
                    {
                        "fontFamily": "Arial",
                        "fontSize": "16px",
                        "fontWeight": "400",
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    }
            }
        }"##,
            )
            .unwrap();
            assert_debug_snapshot!(theme);
        }

        {
            let theme: Theme = serde_json::from_str(
                r#"{
"typography":{"noticeButton":{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.02em"},"button":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"title":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"20px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"text":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"15px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"caption":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"12px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"14px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"noticeTitle":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"15px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"noticeText":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"14px","lineHeight":1.5,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"16px","lineHeight":1.5,"letterSpacing":"-0.02em"}],"h3":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"24px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"h1":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"28px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"36px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"body":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"20px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"noticeBold":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"14px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"notice":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"13px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"h2":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"20px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"28px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"result":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"24px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"32px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"resultPoint":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":800,"fontSize":"24px","lineHeight":1.4,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":800,"fontSize":"28px","lineHeight":1.4,"letterSpacing":"-0.01em"}],"resultText":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"18px","lineHeight":1.4,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"22px","lineHeight":1.4,"letterSpacing":"-0.01em"}],"resultList":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"16px","lineHeight":1.4,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"20px","lineHeight":1.4,"letterSpacing":"-0.01em"}]}
        }"#,
            )
            .unwrap();
            assert_debug_snapshot!(theme);
        }
    }

    #[test]
    #[serial]
    fn to_css_from_theme() {
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");

        assert_eq!(color_theme.css_entries().count(), 1);

        theme.add_color_theme("default", color_theme);
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#fff");
        theme.add_color_theme("dark", color_theme);
        theme.add_typography(
            "default",
            vec![
                Some(Typography::new(
                    Some("Arial".to_string()),
                    Some("16px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                )),
                Some(Typography::new(
                    Some("Arial".to_string()),
                    Some("24px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                )),
            ],
        );

        theme.add_typography(
            "default1",
            vec![
                None,
                Some(Typography::new(
                    Some("Arial".to_string()),
                    Some("24px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                )),
            ],
        );
        let css = theme.to_css();
        assert_debug_snapshot!(css);

        assert_eq!(Theme::default().to_css(), "");
        let mut theme = Theme::default();
        theme.add_typography(
            "default",
            vec![Some(Typography::new(None, None, None, None, None))],
        );
        assert_eq!(theme.to_css(), "");

        let mut theme = Theme::default();
        theme.add_color_theme("default", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("dark", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("dark", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("a", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("a", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("c", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_named_color_theme("primary", "#001"));
        theme.add_color_theme("a", make_named_color_theme("primary", "#002"));
        theme.add_color_theme("c", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());
    }

    #[rstest]
    #[case(
        vec![0, 480, 768, 992, 1280],
        vec![0, 480, 768, 992, 1280, 1600]
    )]
    #[case(
        vec![0, 480, 768, 992, 1280, 1600],
        vec![0, 480, 768, 992, 1280, 1600]
    )]
    #[case(
        vec![0, 480, 768, 992, 1280, 1600, 1920],
        vec![0, 480, 768, 992, 1280, 1600, 1920]
    )]
    fn update_breakpoints(#[case] input: Vec<u16>, #[case] expected: Vec<u16>) {
        let mut theme = Theme::default();
        theme.update_breakpoints(input);
        assert_eq!(theme.breakpoints, expected);
    }

    #[test]
    #[serial]
    fn test_get_theme_interface() {
        let sheet = StyleSheet::default();
        assert_eq!(
            sheet.create_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "LengthInterface",
                "ShadowsInterface",
                "ThemeInterface"
            ),
            ""
        );

        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");
        theme.add_color_theme("dark", color_theme);
        GLOBAL_STYLE_SHEET.lock().unwrap().set_theme(theme);
        assert_eq!(
            get_theme_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "LengthInterface",
                "ShadowsInterface",
                "ThemeInterface"
            ),
            "import \"package\";declare module \"package\"{interface ColorInterface{$primary:null}interface TypographyInterface{}interface LengthInterface{}interface ShadowsInterface{}interface ThemeInterface{dark:null}}"
        );

        // test wrong case
        let mut sheet = StyleSheet::default();
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("(primary)", "#000");
        theme.add_color_theme("dark", color_theme);
        theme.add_typography(
            "prim``ary",
            vec![Some(Typography::new(
                Some("Arial".to_string()),
                Some("16px".to_string()),
                Some("400".to_string()),
                Some("1.5".to_string()),
                Some("0.5".to_string()),
            ))],
        );
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(
            get_theme_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "LengthInterface",
                "ShadowsInterface",
                "ThemeInterface"
            ),
            "import \"package\";declare module \"package\"{interface ColorInterface{[`$(primary)`]:null}interface TypographyInterface{[`prim\\`\\`ary`]:null}interface LengthInterface{}interface ShadowsInterface{}interface ThemeInterface{dark:null}}"
        );
    }

    #[test]
    #[serial]
    fn test_debug() {
        assert!(!is_debug());
        set_debug(true);
        assert!(is_debug());
        set_debug(false);
        assert!(!is_debug());
    }

    #[test]
    #[serial]
    fn test_prefix() {
        assert_eq!(get_prefix(), None);
        set_prefix(Some("du-".to_string()));
        assert_eq!(get_prefix(), Some("du-".to_string()));
        set_prefix(None);
        assert_eq!(get_prefix(), None);
    }

    #[test]
    #[serial]
    fn test_default_theme() {
        let mut theme = Theme::default();
        theme.add_color_theme("light", ColorTheme::default());
        theme.add_color_theme("dark", ColorTheme::default());
        let mut sheet = StyleSheet::default();
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(get_default_theme().unwrap(), Some("light".to_string()));

        let mut theme = Theme::default();
        theme.add_color_theme("default", ColorTheme::default());
        theme.add_color_theme("dark", ColorTheme::default());

        let mut sheet = StyleSheet::default();
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(get_default_theme().unwrap(), Some("default".to_string()));

        let mut theme = Theme::default();
        theme.add_color_theme("dark", ColorTheme::default());

        let mut sheet = StyleSheet::default();
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(get_default_theme().unwrap(), Some("dark".to_string()));
    }

    #[test]
    #[serial]
    fn test_output_new_and_getters() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Use extract to get real styles
        let result = extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
<Box bg="red" />"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_dir: "@devup-ui/core".to_string(),
                single_css: false,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
        )
        .unwrap();

        let output = Output::new(
            result.code.clone(),
            result.styles,
            Some("//# sourceMappingURL=test".to_string()),
            false,
            "test.tsx".to_string(),
            Some("devup-ui-0.css".to_string()),
            false,
        );

        // Test getters
        assert!(!output.code().is_empty());
        assert_eq!(output.css_file(), Some("devup-ui-0.css".to_string()));
        assert_eq!(output.map(), Some("//# sourceMappingURL=test".to_string()));
        assert!(output.css().is_some());
    }

    #[test]
    #[serial]
    fn test_output_updated_base_style() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Create output with empty styles
        let styles = FxHashSet::default();
        let output = Output::new(
            "code".to_string(),
            styles,
            None,
            true,
            "test.tsx".to_string(),
            None,
            false,
        );

        // Test updated_base_style getter
        let _ = output.updated_base_style();
        assert!(output.css().is_none()); // No styles = no CSS
    }

    #[test]
    #[serial]
    fn test_has_devup_ui_wasm_function() {
        // Test positive case
        assert!(has_devup_ui_wasm(
            "test.tsx",
            "import { Box } from '@devup-ui/react';",
            "@devup-ui/react"
        ));

        // Test negative case
        assert!(!has_devup_ui_wasm(
            "test.tsx",
            "const x = 1;",
            "@devup-ui/react"
        ));

        // Test invalid extension
        assert!(!has_devup_ui_wasm(
            "test.invalid",
            "import { Box } from '@devup-ui/react';",
            "@devup-ui/react"
        ));
    }

    #[test]
    #[serial]
    fn test_output_single_css_mode() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Use extract to get real styles in single_css mode
        let result = extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
<Box color="blue" />"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_dir: "@devup-ui/core".to_string(),
                single_css: true,
                import_main_css: true,
                import_aliases: HashMap::new(),
            },
        )
        .unwrap();

        let output = Output::new(
            result.code,
            result.styles,
            None,
            true, // single_css = true
            "test.tsx".to_string(),
            Some("devup-ui.css".to_string()),
            true, // import_main_css = true
        );

        assert!(output.css().is_some());
    }

    #[test]
    #[serial]
    fn test_output_with_global_css_removal() {
        // Reset global state
        let mut sheet = StyleSheet::default();

        // Add some global CSS first
        sheet.add_property(
            "test.tsx",
            "margin",
            0,
            "0",
            Some(&css::style_selector::StyleSelector::Global(
                "body".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
            None,
        );

        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        css::class_map::reset_class_map();

        // Now create output which should trigger rm_global_css
        let styles = FxHashSet::default();
        let output = Output::new(
            "new code".to_string(),
            styles,
            None,
            false,
            "test.tsx".to_string(),
            None,
            false,
        );

        // The updated_base_style should be true because global CSS was removed
        assert!(output.updated_base_style());
    }

    // Regression: single-importer collapse must NOT wipe a bucket-root file's
    // globalCss (@font-face / global selectors). When child.tsx collapses into
    // parent.tsx, extracting child must not delete parent's globalCss.
    fn collapse_setup() {
        use css::class_map::reset_class_map;
        use css::file_map::{reset_canonical_map, reset_file_map};
        {
            let mut s = GLOBAL_STYLE_SHEET.lock().unwrap();
            *s = StyleSheet::default();
        }
        reset_class_map();
        reset_file_map();
        reset_canonical_map();
        register_theme_internal(sheet::theme::Theme::default());
    }

    fn extract_for_collapse(filename: &str, code: &str) {
        code_extract_internal(
            filename,
            code,
            "@devup-ui/react",
            "df".to_string(),
            false,
            false,
            false,
            HashMap::new(),
        )
        .unwrap();
    }

    const LAYOUT_GLOBAL: &str = r#"import { globalCss } from "@devup-ui/react"; globalCss({ pre: { borderRadius: "10px" }, fontFaces: [{ fontFamily: "Pretendard", src: "url(/p.woff2)", fontWeight: 800 }] });"#;
    const MEMBER_BOX: &str =
        r#"import { Box } from "@devup-ui/react"; const x = <Box bg="red" />;"#;

    #[test]
    #[serial]
    fn collapse_member_after_root_keeps_global_css() {
        collapse_setup();
        let mut m = HashMap::new();
        m.insert("footer.tsx".to_string(), "layout.tsx".to_string());
        import_canonical_map_internal(m);

        extract_for_collapse("layout.tsx", LAYOUT_GLOBAL);
        // member collapses into layout.tsx, extracted AFTER the root -> must NOT
        // wipe layout's @font-face / pre{} globalCss.
        extract_for_collapse("footer.tsx", MEMBER_BOX);

        let css = with_style_sheet(|s| s.create_css(None, false));
        css::file_map::reset_canonical_map();
        assert!(
            css.contains("@font-face"),
            "collapse wiped @font-face. css=\n{css}"
        );
        assert!(
            css.contains("Pretendard"),
            "collapse wiped Pretendard font-family. css=\n{css}"
        );
        assert!(
            css.contains("border-radius:10px"),
            "collapse wiped pre{{}} global selector. css=\n{css}"
        );
    }

    #[test]
    #[serial]
    fn collapse_member_before_root_keeps_global_css() {
        collapse_setup();
        let mut m = HashMap::new();
        m.insert("footer.tsx".to_string(), "layout.tsx".to_string());
        import_canonical_map_internal(m);

        // member first, then root -> root still re-adds its globalCss.
        extract_for_collapse("footer.tsx", MEMBER_BOX);
        extract_for_collapse("layout.tsx", LAYOUT_GLOBAL);

        let css = with_style_sheet(|s| s.create_css(None, false));
        css::file_map::reset_canonical_map();
        assert!(
            css.contains("@font-face"),
            "missing @font-face. css=\n{css}"
        );
        assert!(
            css.contains("Pretendard"),
            "missing Pretendard. css=\n{css}"
        );
    }

    #[test]
    #[serial]
    fn collapse_member_with_own_global_css_preserves_both() {
        collapse_setup();
        let mut m = HashMap::new();
        m.insert("footer.tsx".to_string(), "layout.tsx".to_string());
        import_canonical_map_internal(m);

        extract_for_collapse("layout.tsx", LAYOUT_GLOBAL);
        // member has its OWN globalCss with a distinct font family.
        extract_for_collapse(
            "footer.tsx",
            r#"import { globalCss } from "@devup-ui/react"; globalCss({ fontFaces: [{ fontFamily: "D2Coding", src: "url(/d.woff2)" }] });"#,
        );

        let css = with_style_sheet(|s| s.create_css(None, false));
        css::file_map::reset_canonical_map();
        assert!(
            css.contains("Pretendard"),
            "collapse wiped root's Pretendard. css=\n{css}"
        );
        assert!(
            css.contains("D2Coding"),
            "member's own font-family missing. css=\n{css}"
        );
    }

    #[test]
    #[serial]
    fn collapse_multiple_members_keep_root_global_css() {
        collapse_setup();
        let mut m = HashMap::new();
        m.insert("footer.tsx".to_string(), "layout.tsx".to_string());
        m.insert("header.tsx".to_string(), "layout.tsx".to_string());
        import_canonical_map_internal(m);

        extract_for_collapse("layout.tsx", LAYOUT_GLOBAL);
        // multiple members collapse into the same root; none may wipe its globalCss.
        extract_for_collapse("footer.tsx", MEMBER_BOX);
        extract_for_collapse("header.tsx", MEMBER_BOX);

        let css = with_style_sheet(|s| s.create_css(None, false));
        css::file_map::reset_canonical_map();
        assert!(
            css.contains("Pretendard") && css.contains("border-radius:10px"),
            "multiple collapsed members wiped root globalCss. css=\n{css}"
        );
    }

    #[test]
    #[serial]
    fn collapse_member_reextract_clears_only_its_own_global_css() {
        collapse_setup();
        let mut m = HashMap::new();
        m.insert("footer.tsx".to_string(), "layout.tsx".to_string());
        import_canonical_map_internal(m);

        extract_for_collapse("layout.tsx", LAYOUT_GLOBAL);
        // member's OWN globalCss v1: a global selector + a distinct font.
        extract_for_collapse(
            "footer.tsx",
            r#"import { globalCss } from "@devup-ui/react"; globalCss({ code: { color: "red" }, fontFaces: [{ fontFamily: "D2Coding", src: "url(/d.woff2)" }] });"#,
        );
        // member re-extracted (HMR) with DIFFERENT globalCss.
        extract_for_collapse(
            "footer.tsx",
            r#"import { globalCss } from "@devup-ui/react"; globalCss({ samp: { color: "blue" } });"#,
        );

        let css = with_style_sheet(|s| s.create_css(None, false));
        css::file_map::reset_canonical_map();
        // member's NEW globalCss present; its STALE globalCss cleared from the
        // canonical bucket (selector) and raw maps (font); root untouched.
        assert!(
            css.contains("color:blue"),
            "member new globalCss missing. css=\n{css}"
        );
        assert!(
            !css.contains("color:red"),
            "stale member global selector not cleared from canonical bucket. css=\n{css}"
        );
        assert!(
            !css.contains("D2Coding"),
            "stale member @font-face not cleared. css=\n{css}"
        );
        assert!(
            css.contains("Pretendard"),
            "root globalCss wiped by member re-extract. css=\n{css}"
        );
    }

    #[test]
    #[serial]
    fn test_import_sheet_internal() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Create a custom sheet with a property
        let mut custom_sheet = StyleSheet::default();
        custom_sheet.add_property("custom.tsx", "color", 0, "red", None, Some(0), None);

        // Import the custom sheet
        import_sheet_internal(custom_sheet);

        // Verify the sheet was imported by exporting it
        let result = export_sheet_internal();
        assert!(result.is_ok());
        // The exported JSON should contain the property we added
        let json = result.unwrap();
        assert!(json.contains("color") || json.contains("red"));
    }

    #[test]
    #[serial]
    fn test_export_sheet_internal() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();

        // Export the sheet
        let result = export_sheet_internal();
        assert!(result.is_ok());

        // The result should be valid JSON
        let json_str = result.unwrap();
        assert!(json_str.starts_with('{'));
        assert!(json_str.ends_with('}'));
    }

    #[test]
    #[serial]
    fn test_export_class_map_internal() {
        // Reset class map
        css::class_map::reset_class_map();

        // Export the class map
        let result = export_class_map_internal();
        assert!(result.is_ok());

        // The result should be valid JSON (empty map)
        let json_str = result.unwrap();
        assert_eq!(json_str, "{}");
    }

    #[test]
    #[serial]
    fn test_export_file_map_internal() {
        // Reset file map
        css::file_map::reset_file_map();

        // Export the file map
        let result = export_file_map_internal();
        assert!(result.is_ok());

        // The result should be valid JSON (empty map)
        let json_str = result.unwrap();
        assert!(json_str.starts_with('{') || json_str.starts_with("[]"));
    }

    #[test]
    #[serial]
    fn test_code_extract_internal_success() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Test successful extraction
        let result = code_extract_internal(
            "test.tsx",
            r#"import {Box} from '@devup-ui/react'
<Box color="red" />"#,
            "@devup-ui/react",
            "@devup-ui/react".to_string(),
            false,
            false,
            false,
            HashMap::new(),
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.code().is_empty());
    }

    #[test]
    #[serial]
    fn test_code_extract_internal_error() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Test extraction with invalid file extension (should fail)
        let result = code_extract_internal(
            "test.invalid_extension", // Invalid extension will cause SourceType::from_path to fail
            "import { Box } from '@devup-ui/react'; <Box />",
            "@devup-ui/react",
            "@devup-ui/react".to_string(),
            false,
            false,
            false,
            HashMap::new(),
        );

        assert!(result.is_err());
        if let Err(error) = result {
            assert!(!error.is_empty());
        }
    }

    #[test]
    #[serial]
    fn test_register_theme_internal() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();

        // Create and register a theme
        let mut theme = sheet::theme::Theme::default();
        let mut color_theme = sheet::theme::ColorTheme::default();
        color_theme.add_color("primary", "#ff0000");
        theme.add_color_theme("default", color_theme);

        register_theme_internal(theme);

        // Verify the theme was registered
        let default_theme = GLOBAL_STYLE_SHEET.lock().unwrap().theme.get_default_theme();
        assert_eq!(default_theme, Some("default".to_string()));
    }
}
