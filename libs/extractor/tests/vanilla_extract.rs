#![allow(clippy::unwrap_used)]

use std::collections::HashMap;

use extractor::{ExtractOption, extract};

fn extract_stylesheet(code: &str) -> String {
    let output = extract(
        "stylesheet.css.ts",
        &format!("import {{ style, globalStyle, styleVariants, keyframes, fontFace, globalFontFace, createVar, fallbackVar, createContainer, layer, globalLayer, createTheme, createGlobalTheme, createThemeContract, createGlobalThemeContract, assignVars }} from '@devup-ui/react'\n{code}"),
        ExtractOption {
            package: "@devup-ui/react".to_string(),
            css_dir: "@devup-ui/react".to_string(),
            single_css: true,
            import_main_css: false,
            import_aliases: HashMap::new(),
        },
    )
    .unwrap();
    format!("{:?}\n{}", output.styles, output.code)
}

// The library build other targets link is a separate copy from the unit-test
// build; these run every vanilla-extract API through it.
#[test]
fn font_faces_extract_through_the_library() {
    let output = extract_stylesheet(
        r"const body = fontFace({ src: 'local(a)' }, 'Body')
const icons = fontFace([{ src: 'local(b)' }, {}])
const cyclic = {}; cyclic.self = cyclic
fontFace(cyclic)
globalFontFace('Inter', { src: 'local(Inter)' })
export const text = style({ fontFamily: body, content: icons })",
    );
    for family in ["Body-", "font-", "Inter", "local(b)"] {
        assert!(output.contains(family), "{family} missing from {output}");
    }
}

#[test]
fn an_unreadable_var_declaration_leaves_the_stylesheet_alone() {
    let code = "import { createVar } from '@devup-ui/react'\nexport const v = createVar({ syntax: Symbol() })";
    let output = extract(
        "broken.css.ts",
        code,
        ExtractOption {
            package: "@devup-ui/react".to_string(),
            css_dir: "@devup-ui/react".to_string(),
            single_css: true,
            import_main_css: false,
            import_aliases: HashMap::new(),
        },
    )
    .unwrap();
    assert!(output.styles.is_empty());
}

#[test]
fn vars_themes_and_layers_extract_through_the_library() {
    let output = extract_stylesheet(
        r"const plain = createVar()
const typed = createVar({ syntax: ['<length>', '<percentage>'], inherits: false, initialValue: '0px' }, 'size')
export const box = style({ vars: { [plain]: '1px' }, width: fallbackVar(typed, plain, '2px'), containerName: createContainer('side') })
const contract = createThemeContract({ color: null })
export const theme = createTheme(contract, { color: 'red' })
export const [light, lightVars] = createTheme({ '@layer': layer('theme'), space: 4 })
createGlobalTheme(':root', createGlobalThemeContract({ gap: 'gap' }), { gap: '1px' })
export const assigned = style({ vars: assignVars(contract, { color: 'blue' }) })
export const variants = styleVariants({ a: { color: 'red' } }, (rule) => [box, rule])
globalStyle(`${variants.a} span`, { '@layer': { [globalLayer({ parent: 'base' }, 'x')]: { margin: 1 } } })
export const spin = keyframes({ to: { opacity: 1 } })",
    );
    for expected in [
        "@property --size-",
        "var(--size-",
        "container-name",
        "theme-",
        "property: \"--gap\", value: \"1px\"",
        "value: \"4\", level: 0, selector: Some(Global(\".theme-",
        "base.x",
    ] {
        assert!(
            output.contains(expected),
            "{expected} missing from {output}"
        );
    }
}
