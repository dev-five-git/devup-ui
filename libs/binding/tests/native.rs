use devup_ui_binding_core::code_extract_native;

const SOURCE: &str =
    "import { Box } from '@devup-ui/react'\nexport const view = <Box bg=\"red\" p={1} />\n";

#[test]
fn native_extraction_gives_code_and_a_map_without_css() -> Result<(), String> {
    // When
    let output = code_extract_native(("view.tsx", SOURCE), "@devup-ui/react", "android", true)?;
    // Then: React Native's View with a hoisted style, and nothing for a stylesheet.
    assert!(output.code.contains("View as Box"));
    assert!(output.code.contains("backgroundColor: \"red\""));
    assert!(output.code.contains("padding: 4"));
    assert!(output.map.is_some());
    assert!(output.css.is_none());
    assert!(output.css_file.is_none());
    assert!(!output.updated_base_style);
    assert_eq!(output.dependencies, Vec::<String>::new());
    Ok(())
}

#[test]
fn native_extraction_rejects_an_unknown_platform() {
    // When
    let error = code_extract_native(("view.tsx", SOURCE), "@devup-ui/react", "windows", true)
        .err()
        .unwrap_or_default();
    // Then
    assert_eq!(
        error,
        "unknown React Native platform `windows`: use `ios`, `android` or `web`"
    );
}

#[test]
fn native_extraction_reports_build_errors() {
    // When
    let error = code_extract_native(
        (
            "view.tsx",
            "import { Box } from '@devup-ui/react'\n<Box p={[1, 2]} />\n",
        ),
        "@devup-ui/react",
        "ios",
        false,
    )
    .err()
    .unwrap_or_default();
    // Then
    assert!(error.starts_with("view.tsx:2:6: `<Box>` cannot use `p` on React Native"));
    assert!(error.contains("`runtime: false`"));
}

/// A theme as devup.json writes it
fn theme(json: &str) -> sheet::theme::Theme {
    serde_json::from_str(json).unwrap_or_default()
}

#[test]
fn native_theme_reads_every_token_for_each_theme() {
    // Given: colors in two themes, a renamed dark scheme, and length, shadow
    // and typography tokens whose variants do not all match the colors
    let theme = theme(
        r##"{
            "colors": {
                "light": { "primary": "#000", "gray": { "100": "#111" }, "lightOnly": "#222" },
                "dark": { "primary": "#fff" },
                "dim": { "primary": "#888" }
            },
            "colorScheme": { "dim": "dark", "dark": "light" },
            "length": { "default": { "gap.x": 2, "pad": ["1px", null, "2px"] }, "dim": { "gap.x": "3px" } },
            "shadow": { "light": { "card": "0 1px #000" } },
            "typography": {
                "body": { "fontSize": "16px", "lineHeight": 1.5, "fontFamily": " " },
                "title": [null, { "fontSize": "20px" }]
            }
        }"##,
    );
    // When
    let native = devup_ui_binding_core::native_theme(&theme);
    // Then: the default first, `dim` is the dark one as `dark` renders light
    assert_eq!(native.names, ["light", "dark", "dim"]);
    assert_eq!(native.dark, Some(2));
    let one = |value: &str| Some(vec![Some(value.to_string())]);
    assert_eq!(
        native.tokens["primary"],
        [one("#000"), one("#fff"), one("#888")]
    );
    assert_eq!(
        native.tokens["gray-100"],
        [one("#111"), one("#111"), one("#111")]
    );
    assert_eq!(native.tokens["gap-x"], [one("8px"), one("8px"), one("3px")]);
    assert_eq!(
        native.tokens["card"],
        [one("0 1px #000"), one("0 1px #000"), one("0 1px #000")]
    );
    assert_eq!(
        native.tokens["pad"][0],
        Some(vec![Some("1px".to_string()), None, Some("2px".to_string())])
    );
    assert_eq!(
        native.typography["body"],
        [Some(vec![
            ("font-size".to_string(), "16px".to_string()),
            ("line-height".to_string(), "1.5".to_string()),
        ])]
    );
    assert_eq!(native.typography["title"][0], None);
}

#[test]
fn native_theme_without_colors_names_the_length_and_shadow_variants() {
    let native = devup_ui_binding_core::native_theme(&theme(
        r#"{ "length": { "dark": { "a": "1px" } }, "shadow": { "default": { "b": "0 0 1px red" } } }"#,
    ));
    assert_eq!(native.names, ["default", "dark"]);
    assert_eq!(native.dark, Some(1));
    // The only length variant is the one the page shows without a theme
    let one = Some(vec![Some("1px".to_string())]);
    assert_eq!(native.tokens["a"], [one.clone(), one]);
}

#[test]
fn native_theme_without_a_dark_theme_has_none() {
    let native = devup_ui_binding_core::native_theme(&theme(
        r#"{ "colors": { "default": { "a": "red" } } }"#,
    ));
    assert_eq!(
        (native.names.as_slice(), native.dark),
        (["default".to_string()].as_slice(), None)
    );
}

#[test]
#[serial_test::serial]
fn native_extraction_resolves_the_registered_theme() -> Result<(), String> {
    // Given
    devup_ui_binding_core::register_theme_internal(theme(
        r##"{ "colors": { "light": { "primary": "#000" }, "dark": { "primary": "#fff" } } }"##,
    ));
    // When
    let output = code_extract_native(
        (
            "view.tsx",
            "import { Box } from '@devup-ui/react'\nexport const V = () => <Box bg=\"$primary\" />\n",
        ),
        "@devup-ui/react",
        "ios",
        true,
    );
    devup_ui_binding_core::register_theme_internal(sheet::theme::Theme::default());
    // Then
    let code = output?.code;
    assert!(
        code.contains("__defineThemes([\"light\", \"dark\"], 1)"),
        "{code}"
    );
    assert!(code.contains("backgroundColor: \"#fff\""), "{code}");
    Ok(())
}

#[test]
fn native_theme_keeps_the_breakpoints() {
    // The web's breakpoints by default, and those devup.json sets
    let default = devup_ui_binding_core::native_theme(&theme("{}"));
    let custom = devup_ui_binding_core::native_theme(&theme(r#"{ "breakpoints": [0, 600] }"#));
    assert_eq!(default.breakpoints, [0, 480, 768, 992, 1280, 1600]);
    assert_eq!(custom.breakpoints, [0, 600]);
}
