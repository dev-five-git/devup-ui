use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::{StylexSource, has_stylex_source};

#[test]
fn sources_are_exact_when_package_is_configured() {
    let cases = [
        ("@stylexjs/stylex", StylexSource::Upstream),
        ("@custom/ui", StylexSource::Root),
        ("@custom/ui/stylex", StylexSource::Dedicated),
        ("@custom/ui/compat/stylex", StylexSource::TypesOnly),
        ("@custom/ui-other", StylexSource::Other),
        ("@custom/ui/stylex/other", StylexSource::Other),
        ("@devup-ui/react/stylex", StylexSource::Other),
        ("stylex", StylexSource::Other),
    ];
    for (source, expected) in cases {
        let classified = StylexSource::classify(source, "@custom/ui");
        assert_eq!(classified, expected);
    }
}

#[test]
fn gate_detects_only_public_value_sources_when_no_aliases_exist() {
    let cases = [
        ("import * as sx from '@stylexjs/stylex';", true),
        (
            "import { positionTry } from '@devup-ui/react/stylex';",
            true,
        ),
        ("import { stylex } from '@devup-ui/react';", true),
        ("const sx = require('@devup-ui/react/stylex');", true),
        ("const { stylex: sx } = require('@devup-ui/react');", true),
        (
            "import type { X } from '@devup-ui/react/compat/stylex';",
            false,
        ),
        ("import { type X } from '@stylexjs/stylex';", false),
        ("import * as sx from 'stylex';", false),
        ("import * as sx from '@devup-ui/react-other';", false),
        ("const sx = require('@devup-ui/react/stylex/other');", false),
        (
            "export { positionTry } from '@devup-ui/react/stylex';",
            true,
        ),
        (
            "export { type PositionTryStyles } from '@devup-ui/react/stylex';",
            false,
        ),
        (
            "export type { PositionTryStyles } from '@devup-ui/react/stylex';",
            false,
        ),
        ("export { other } from 'unrelated';", false),
        ("import('@devup-ui/react/compat/stylex');", true),
        ("type T = import('@devup-ui/react/compat/stylex').T;", false),
    ];
    for (code, expected) in cases {
        let allocator = Allocator::default();
        let program = Parser::new(&allocator, code, SourceType::ts())
            .parse()
            .program;
        let found = has_stylex_source(&program, "@devup-ui/react");
        assert_eq!(found, expected, "{code}");
    }
}
