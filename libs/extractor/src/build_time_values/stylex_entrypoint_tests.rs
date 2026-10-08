use serial_test::serial;

use crate::{ExtractOption, ExtractStyleValue};

#[test]
#[serial]
fn exact_local_computation_is_folded_when_public_stylex_bindings_take_values() {
    let entries = [
        ("import * as sx from '@stylexjs/stylex';", "sx.positionTry"),
        (
            "import { positionTry as pt } from '@devup-ui/react/stylex';",
            "pt",
        ),
        (
            "import { stylex as sx } from '@devup-ui/react';",
            "sx.positionTry",
        ),
        (
            "import * as devup from '@devup-ui/react';",
            "devup.stylex.positionTry",
        ),
        (
            "const sx = require('@devup-ui/react/stylex');",
            "sx.positionTry",
        ),
        (
            "const { positionTry: pt } = require('@devup-ui/react/stylex');",
            "pt",
        ),
        (
            "const { stylex: sx } = require('@devup-ui/react');",
            "sx.positionTry",
        ),
    ];
    for (import, call) in entries {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        let source = format!(
            "{import}\nfunction double(x) {{ return x * 2; }}\nexport const name = {call}({{ width: double(5) }});"
        );
        let output = crate::extract(
            "computed.ts",
            &source,
            ExtractOption {
                import_main_css: false,
                ..ExtractOption::default()
            },
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Css(css) if css.css.contains("width:10px;"))), "{}", output.code);
        assert!(!output.code.contains("positionTry("), "{}", output.code);
    }
}
