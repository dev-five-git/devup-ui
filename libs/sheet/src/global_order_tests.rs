use super::{StyleSheet, Theme};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use extractor::{ExtractOption, extract};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(false, true)]
#[case(true, true)]
#[case(false, false)]
#[case(true, false)]
#[serial]
fn shared_tailwind_registrations_survive_source_updates(
    #[case] reverse: bool,
    #[case] single_css: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given two source owners contributing different shared registrations.
    css::debug::set_debug(false);
    css::set_prefix(None);
    css::atom_hoist::set_atom_hoist(None);
    reset_class_map();
    reset_file_map();
    let mut sheet = StyleSheet::default();
    sheet.set_theme(Theme::default());
    let option = ExtractOption {
        package: "@devup-ui/react".to_string(),
        css_dir: "@devup-ui/react".to_string(),
        single_css,
        import_main_css: false,
        import_aliases: std::collections::HashMap::new(),
    };
    let mut sources = [
        (
            "a.tsx",
            "import {Box} from '@devup-ui/react';<Box className=\"before:block\"/>",
        ),
        (
            "b.tsx",
            "import {Box} from '@devup-ui/react';<Box className=\"leading-tight\"/>",
        ),
    ];
    if reverse {
        sources.reverse();
    }
    let expected = [
        "@property --tw-content{syntax:\"*\";inherits:false;initial-value:\"\"}",
        "@property --tw-leading{syntax:\"*\";inherits:false}",
    ];

    // When extraction repeats, then an authored owner changes and becomes empty,
    // following the same remove/update sequence as the public WASM entry point.
    for (step, (file, source)) in sources
        .into_iter()
        .chain(sources)
        .chain([
            (
                "a.tsx",
                concat!(
                    "import {globalCss} from '@devup-ui/react';globalCss`body{color:",
                    "red}`;"
                ),
            ),
            (
                "a.tsx",
                concat!(
                    "import {globalCss} from '@devup-ui/react';globalCss`body{color:",
                    "blue}`;"
                ),
            ),
            ("a.tsx", "export {};"),
        ])
        .enumerate()
    {
        let output = extract(file, source, option.clone())?;
        sheet.rm_global_css(file, single_css);
        let (_, updated_base_style) = sheet.update_styles(&output.styles, file, single_css);
        assert_eq!(updated_base_style, step < 2 || step == 4 || step == 5);
        if step == 0 {
            continue;
        }

        // Then the emitted registration prefix is main's literal sorted union,
        // byte for byte, once each; authored rules still replace rather than union.
        let emitted = sheet.create_css(None, false);
        let body = emitted
            .strip_prefix(StyleSheet::create_header())
            .ok_or("missing CSS header")?;
        let registrations: Vec<_> = body
            .split_inclusive('}')
            .filter(|rule| rule.starts_with("@property "))
            .collect();
        assert_eq!(registrations, expected, "step {step}: {body}");
        assert!(body.starts_with(&expected.concat()), "step {step}: {body}");
        match step {
            4 => assert!(body.contains(concat!("body{color:", "red}"))),
            5 => {
                assert!(body.contains(concat!("body{color:", "blue}")));
                assert!(!body.contains(concat!("body{color:", "red}")));
            }
            6 => assert!(!body.contains("body{")),
            _ => {}
        }
    }
    Ok(())
}

#[rstest]
#[case(
    concat!("globalCss`body{color:", "red}`;globalCss`body{color:", "blue}`;"),
    concat!("body{color:", "red}body{color:", "blue}")
)]
#[case(
    "globalCss({imports:['data:text/css,body%7Bcolor%3Ared%7D','data:text/css,body%7Bcolor%3Ablue%7D']});",
    "@import \"data:text/css,body%7Bcolor%3Ared%7D\";@import \"data:text/css,body%7Bcolor%3Ablue%7D\";"
)]
#[case(
    "globalCss({fontFaces:[{fontFamily:'Audit',src:'local(\"Arial\")',sizeAdjust:'200%'},{fontFamily:'Audit',src:'local(\"Arial\")',sizeAdjust:'100%'}]});",
    "@font-face{font-family:Audit;size-adjust:200%;src:local(\"Arial\")}@font-face{font-family:Audit;size-adjust:100%;src:local(\"Arial\")}"
)]
#[case(
    "globalCss({body:{color:'red'}});globalCss({body:{color:'blue'}});",
    "@layer b;@layer b{body{color:red;color:blue}}"
)]
#[case(
    "globalCss({body:{marginTop:'10px',m:0}});",
    "@layer b;@layer b{body{margin-top:10px;margin:0}}"
)]
#[case(
    "globalCss({'@layer zz':{p:{m:0}},'@layer aa':{p:{m:1}}});",
    "@layer b;@layer zz,aa;@layer aa{p{margin:4px}}@layer zz{p{margin:0}}"
)]
#[serial]
fn global_css_keeps_authored_cascade_order(
    #[case] declarations: &str,
    #[case] expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given an isolated file, without class-name normalization that could alter URLs.
    css::debug::set_debug(false);
    css::set_prefix(None);
    css::atom_hoist::set_atom_hoist(None);
    reset_class_map();
    reset_file_map();
    let mut sheet = StyleSheet::default();
    sheet.set_theme(Theme::default());
    let source = format!("import {{globalCss}} from '@devup-ui/react';{declarations}");

    // When its written declarations pass through extraction and sheet emission.
    let output = extract(
        "global-order.tsx",
        &source,
        ExtractOption {
            package: "@devup-ui/react".to_string(),
            css_dir: "@devup-ui/react".to_string(),
            single_css: true,
            import_main_css: false,
            import_aliases: std::collections::HashMap::new(),
        },
    )?;
    sheet.update_styles(&output.styles, "global-order.tsx", true);
    let css = sheet.create_css(None, false);

    // Then the complete emitted cascade retains the order that determines its winner.
    assert_eq!(
        css.strip_prefix(StyleSheet::create_header()),
        Some(expected)
    );
    Ok(())
}
