//! Sibling elements with a non-literal `as` and a `props` object in one file.
//!
//! `@devup-ui/wasm` 1.0.78 - 1.0.81 wrote such a file back as code that no longer
//! parsed: the second element with a dynamic `as` was placed where an earlier
//! one stood. The smallest file that showed it is two siblings, each with
//! `as={Link}` and `props={{ href: '/...' }}`.
use crate::{ExtractOption, extract, extract_without_source_map};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use insta::assert_snapshot;
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, JSXAttributeItem, JSXOpeningElement, ObjectPropertyKind};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use rstest::rstest;
use serial_test::serial;

const IMPORTS: &str = "import { Box, Center, Flex, Text, VStack } from '@devup-ui/react';\nimport Link from 'next/link';\n";

/// Every element of the extracted code, with the `href` it carries.
struct Elements<'s> {
    code: &'s str,
    names: Vec<String>,
    hrefs: Vec<(String, String)>,
    attributes: Vec<String>,
}

impl<'a> Visit<'a> for Elements<'_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'a>) {
        let name = element.name.span().source_text(self.code).to_string();
        for attribute in &element.attributes {
            if let JSXAttributeItem::Attribute(attribute) = attribute {
                self.attributes
                    .push(attribute.name.span().source_text(self.code).to_string());
            }
            // `props={{ href: '/privacy' }}` is written as `{...{ href: "/privacy" }}`.
            if let JSXAttributeItem::SpreadAttribute(spread) = attribute
                && let Expression::ObjectExpression(object) = &spread.argument
            {
                for property in &object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property
                        && property.key.is_specific_static_name("href")
                        && let Expression::StringLiteral(href) = &property.value
                    {
                        self.hrefs
                            .push((name.clone(), href.value.display().to_string()));
                    }
                }
            }
        }
        self.names.push(name);
        walk::walk_jsx_opening_element(self, element);
    }
}

/// Parses the extracted code as TSX and returns its elements. Code that does
/// not parse is the regression itself.
fn elements_of(code: &str) -> Elements<'_> {
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, code, SourceType::tsx()).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "extracted code has syntax errors {:?}:\n{code}",
        parsed.diagnostics
    );
    let mut elements = Elements {
        code,
        names: Vec::new(),
        hrefs: Vec::new(),
        attributes: Vec::new(),
    };
    elements.visit_program(&parsed.program);
    elements
}

fn hrefs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, href)| ((*name).to_string(), (*href).to_string()))
        .collect()
}

#[rstest]
// The file of the report: two siblings, each a link.
#[case::two_siblings(
    "two_siblings",
    "export const Footer = () => (
  <Flex gap={4}>
    <Center as={Link} props={{ href: '/privacy' }}>Privacy</Center>
    <Text as={Link} props={{ href: '/terms' }}>Terms</Text>
  </Flex>
);",
    &[("Link", "/privacy"), ("Link", "/terms")]
)]
#[case::self_closing_siblings(
    "self_closing_siblings",
    "export const Footer = () => (
  <Flex>
    <Center as={Link} props={{ href: '/privacy' }} />
    <Text as={Link} props={{ href: '/terms' }} />
  </Flex>
);",
    &[("Link", "/privacy"), ("Link", "/terms")]
)]
#[case::three_siblings_with_styles(
    "three_siblings_with_styles",
    "export const Footer = () => (
  <VStack gap={2}>
    <Center as={Link} bg=\"$primary\" p={2} props={{ href: '/privacy' }}>Privacy</Center>
    <Text as={Link} color=\"red\" props={{ href: '/terms' }}>Terms</Text>
    <Box as={Link} _hover={{ opacity: 0.5 }} props={{ href: '/legal/cookies' }}>Cookies</Box>
  </VStack>
);",
    &[("Link", "/privacy"), ("Link", "/terms"), ("Link", "/legal/cookies")]
)]
#[case::siblings_in_a_fragment(
    "siblings_in_a_fragment",
    "export const Footer = () => (
  <>
    <Center as={Link} props={{ href: '/privacy' }}>Privacy</Center>
    <Text as={Link} props={{ href: '/terms' }}>Terms</Text>
  </>
);",
    &[("Link", "/privacy"), ("Link", "/terms")]
)]
#[case::siblings_of_a_plain_element(
    "siblings_of_a_plain_element",
    "export const Footer = () => (
  <footer>
    <span>before</span>
    <Center as={Link} props={{ href: '/privacy' }}>Privacy</Center>
    {' / '}
    <Text as={Link} props={{ href: '/terms' }}>Terms</Text>
    <span>after</span>
  </footer>
);",
    &[("Link", "/privacy"), ("Link", "/terms")]
)]
#[case::nested_dynamic_as(
    "nested_dynamic_as",
    "export const Footer = () => (
  <Flex as={Link} props={{ href: '/' }}>
    <Center as={Link} props={{ href: '/privacy' }}>
      <Text as={Link} props={{ href: '/privacy/detail' }}>Detail</Text>
    </Center>
    <Text as={Link} props={{ href: '/terms' }}>Terms</Text>
  </Flex>
);",
    &[
        ("Link", "/"),
        ("Link", "/privacy"),
        ("Link", "/privacy/detail"),
        ("Link", "/terms")
    ]
)]
#[case::conditional_as(
    "conditional_as",
    "export const Footer = ({ external }) => (
  <Flex>
    <Center as={external ? 'a' : Link} props={{ href: '/privacy' }}>Privacy</Center>
    <Text as={external ? 'a' : Link} props={{ href: '/terms' }}>Terms</Text>
  </Flex>
);",
    &[
        ("a", "/privacy"),
        ("Link", "/privacy"),
        ("a", "/terms"),
        ("Link", "/terms")
    ]
)]
#[case::urls_and_more_props(
    "urls_and_more_props",
    "export const Footer = () => (
  <Flex>
    <Center as={Link} props={{ href: 'https://devup-ui.com/docs/?next=/a/b#c', target: '_blank', rel: 'noopener' }}>Docs</Center>
    <Text as={Link} props={{ href: '//cdn.example.com/a/', prefetch: false }}>Cdn</Text>
  </Flex>
);",
    &[
        ("Link", "https://devup-ui.com/docs/?next=/a/b#c"),
        ("Link", "//cdn.example.com/a/")
    ]
)]
#[case::siblings_in_a_list(
    "siblings_in_a_list",
    "export const Footer = ({ items }) => (
  <Flex>
    {items.map((item) => (
      <Center key={item} as={Link} props={{ href: '/items' }}>{item}</Center>
    ))}
    <Text as={Link} props={{ href: '/terms' }}>Terms</Text>
    {items.length > 0 && <Box as={Link} props={{ href: '/more' }}>More</Box>}
  </Flex>
);",
    &[("Link", "/items"), ("Link", "/terms"), ("Link", "/more")]
)]
#[case::two_components_in_one_file(
    "two_components_in_one_file",
    "export const Privacy = () => <Center as={Link} props={{ href: '/privacy' }}>Privacy</Center>;
export const Terms = () => <Text as={Link} props={{ href: '/terms' }}>Terms</Text>;
export const Both = () => (
  <Flex>
    <Privacy />
    <Text as={Link} props={{ href: '/both' }} />
    <Terms />
  </Flex>
);",
    &[("Link", "/privacy"), ("Link", "/terms"), ("Link", "/both")]
)]
#[case::member_and_attribute_as(
    "member_and_attribute_as",
    "export const Footer = ({ ui }) => (
  <Flex title={<Text as={ui.Link} props={{ href: '/title' }} />}>
    <Center as={ui.Link} props={{ href: '/privacy' }}>Privacy</Center>
    <Text as={ui.Link} props={{ href: '/terms' }}>Terms</Text>
  </Flex>
);",
    &[
        ("ui.Link", "/title"),
        ("ui.Link", "/privacy"),
        ("ui.Link", "/terms")
    ]
)]
#[serial]
fn sibling_dynamic_as_with_props_object_stays_valid(
    #[case] name: &str,
    #[case] body: &str,
    #[case] expected: &[(&str, &str)],
    #[values(true, false)] source_map: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!("{IMPORTS}{body}\nexport const after = 42;\n");
    // When
    let output = if source_map {
        extract("footer.tsx", &source, ExtractOption::default())
    } else {
        extract_without_source_map("footer.tsx", &source, ExtractOption::default())
    }?;
    // Then: the code parses, no component or `as`/`props` is left, every href
    // stays on its own element, and the statement after the component remains.
    let elements = elements_of(&output.code);
    for component in ["Box", "Center", "Flex", "Text", "VStack"] {
        assert!(
            !elements.names.iter().any(|element| element == component),
            "<{component}> is left in:\n{}",
            output.code
        );
    }
    for attribute in ["as", "props"] {
        assert!(
            !elements.attributes.iter().any(|left| left == attribute),
            "`{attribute}` is left in:\n{}",
            output.code
        );
    }
    assert_eq!(elements.hrefs, hrefs(expected), "{}", output.code);
    assert!(output.code.contains("export const after = 42;"));
    if source_map {
        assert_snapshot!(name, output.code);
    }
    Ok(())
}
