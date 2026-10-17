//! Negative cases paired with legacy snapshots. Unknown spreads must precede
//! explicit selector/typography props; lexical self-initialization is a TDZ error,
//! unlike the hoisted `var` self-read retained as the positive dynamic control.

use crate::{ExtractOption, extract};
use css::class_map::reset_class_map;
use css::file_map::reset_file_map;

fn error(filename: &str, source: &str, option: ExtractOption) -> String {
    reset_class_map();
    reset_file_map();
    match extract(filename, source, option) {
        Ok(output) => panic!("expected a build error, got {}", output.code),
        Err(error) => error.to_string(),
    }
}

fn core_option() -> ExtractOption {
    ExtractOption {
        package: "@devup-ui/core".to_string(),
        css_dir: "@devup-ui/core".to_string(),
        single_css: true,
        import_main_css: false,
        import_aliases: std::collections::HashMap::new(),
    }
}

pub(super) fn extract_style_props_with_class_name() {
    let source = r#"import { Box, Button as DevupButton, Center, css } from '@devup-ui/core'
import clsx from 'clsx'

<DevupButton
      boxSizing="border-box"
      className={clsx(
        variants[variant],
        isError && variant === 'default' && errorClassNames,
        className,
      )}
      typography={
        isPrimary
          ? {
              sm: 'buttonS',
              md: 'buttonM',
            }[size]
          : undefined
      }
      {...props}
    />
"#;

    let message = error("test.tsx", source, core_option());

    assert!(
        message.starts_with("test.tsx:11:7: `<DevupButton>` cannot use `typography`"),
        "{message}"
    );
    assert!(
        message.contains("write the spread before `typography`"),
        "{message}"
    );
}

pub(super) fn extract_selector() {
    let source = r"import {Center} from '@devup-ui/core'
    <Center
      _active={
        variant !== 'disabled' && {
          boxShadow: 'none',
          transform: 'scale(0.95)',
        }
      }
      _hover={
        variant !== 'disabled' && {
          boxShadow: [
            '0px 1px 3px 0px rgba(0, 0, 0, 0.25)',
            null,
            '0px 0px 15px 0px rgba(0, 0, 0, 0.25)',
          ],
        }
      }
      {...props}
    >
      {children}
    </Center>
        ";

    let message = error("test.tsx", source, core_option());

    for (location, key) in [("test.tsx:3:7", "_active"), ("test.tsx:9:7", "_hover")] {
        assert!(
            message.contains(&format!("{location}: `<Center>` cannot use `{key}`")),
            "{message}"
        );
        assert!(
            message.contains(&format!("replace the whole `{key}` object")),
            "{message}"
        );
        assert!(
            message.contains(&format!("write the spread before `{key}`")),
            "{message}"
        );
    }
}

pub(super) fn test_rest_props() {
    let source = r"import { VStack } from '@devup-ui/core'

export default function Card({
  children,
  className,
  ...props
}) {
  return (
    <VStack
      _active={{
        boxShadow: 'none',
        transform: 'scale(0.95)',
      }}
      className={className}
      {...props}
    >
      {children}
    </VStack>
  )
}

        ";

    let message = error("test.jsx", source, core_option());

    assert!(
        message.starts_with("test.jsx:10:7: `<VStack>` cannot use `_active`"),
        "{message}"
    );
    assert!(
        message.contains("replace the whole `_active` object"),
        "{message}"
    );
    assert!(
        message.contains("write the spread before `_active`"),
        "{message}"
    );
}

pub(super) fn test_inline_local_constants() {
    let source = r"import { Box, css, keyframes, globalCss } from '@devup-ui/react';
const SIZE = 4;
const UNIT = `${SIZE}px`;
const DOUBLE = SIZE * 2;
const HALF = SIZE / 2 - 1;
const LABEL = 'a' + SIZE;
const TINY = 0.0000001;
const HUGE = 1e21;
const ZERO = -0;
const PX = 'px';
const SELF = SELF;
let mutable = 1;
export const a = <Box p={DOUBLE} m={UNIT} w={SIZE + 1} h={HALF} content={LABEL} top={`${TINY}px`} left={`${HUGE}px`} right={ZERO + PX} bottom={mutable} zIndex={SELF} />;
export const b = css({ padding: `${SIZE * 2}px`, margin: SIZE - 1, width: SIZE * SIZE, height: (SIZE) });
export const k = keyframes({ from: { opacity: SIZE / 8 } });
globalCss({ body: { padding: UNIT } });
export const c = <Box p={SIZE / 0} m={PX - 1} w={`${PX}${other}`} h={SIZE % 3} />;";

    let message = error("test.tsx", source, ExtractOption::default());

    assert_eq!(
        message,
        "test.tsx:11:14: style value cannot use `SELF` at build time: it is read before its lexical declaration is initialized, so the original JavaScript throws ReferenceError; move the declaration of `SELF` above this use"
    );
}
