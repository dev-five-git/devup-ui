//! The file `@devup-ui/wasm` 1.0.78 - 1.0.81 wrote back as code that no longer
//! parsed, through the functions `codeExtract` and `codeExtractWithoutSourceMap`
//! call. The whole output is compared, so a misplaced element or a lost href
//! fails here; the extractor tests of the same name also parse the output.
use super::{
    Output, code_extract_internal, code_extract_without_source_map_internal, with_style_sheet_mut,
};
use crate::state::TestStateGuard;
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;
use sheet::StyleSheet;
use std::collections::HashMap;

const SOURCE: &str = "import { Center, Flex, Text } from '@devup-ui/react';
import Link from 'next/link';
export const Footer = () => (
  <Flex gap={4}>
    <Center as={Link} props={{ href: '/privacy' }}>Privacy</Center>
    <Text as={Link} props={{ href: '/terms' }}>Terms</Text>
  </Flex>
);
export const after = 42;
";

const EXTRACTED: &str = "import \"@devup-ui/react/devup-ui.css\";
import Link from \"next/link\";
export const Footer = () => <div className=\"a d\">
    <Link className=\"a b c\" href=\"/privacy\">Privacy</Link>
    <Link href=\"/terms\">Terms</Link>
  </div>;
export const after = 42;
";

fn compile(source_map: bool) -> Result<Output, String> {
    with_style_sheet_mut(|sheet| *sheet = StyleSheet::default());
    reset_class_map();
    reset_file_map();
    let extract = if source_map {
        code_extract_internal
    } else {
        code_extract_without_source_map_internal
    };
    extract(
        "footer.tsx",
        SOURCE,
        "@devup-ui/react",
        "@devup-ui/react".to_string(),
        true,
        true,
        false,
        HashMap::new(),
    )
}

#[rstest]
#[serial]
fn sibling_dynamic_as_with_props_object_keeps_each_href(
    #[values(true, false)] source_map: bool,
) -> Result<(), String> {
    // Given
    let _state = TestStateGuard::new();
    // When
    let output = compile(source_map)?;
    // Then: each link keeps its own href, where it stood.
    assert_eq!(output.code(), EXTRACTED);
    assert_eq!(output.map().is_some(), source_map);
    assert!(output.css().is_some());
    Ok(())
}
