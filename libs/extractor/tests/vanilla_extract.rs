#![allow(clippy::unwrap_used)]

use std::collections::HashMap;

use extractor::{ExtractOption, extract};

#[test]
fn font_faces_extract_through_the_library() {
    let output = extract(
        "fonts.css.ts",
        r"import { fontFace, globalFontFace, style } from '@devup-ui/react'
const body = fontFace({ src: 'local(a)' }, 'Body')
const icons = fontFace([{ src: 'local(b)' }, {}])
const cyclic = {}; cyclic.self = cyclic
fontFace(cyclic)
globalFontFace('Inter', { src: 'local(Inter)' })
export const text = style({ fontFamily: body, content: icons })",
        ExtractOption {
            package: "@devup-ui/react".to_string(),
            css_dir: "@devup-ui/react".to_string(),
            single_css: true,
            import_main_css: false,
            import_aliases: HashMap::new(),
        },
    )
    .unwrap();
    let styles = format!("{:?}", output.styles);
    for family in ["Body-", "font-", "Inter"] {
        assert!(styles.contains(family), "{family} missing from {styles}");
    }
    assert!(styles.contains("local(b)"), "{styles}");
}
