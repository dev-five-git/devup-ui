use std::collections::BTreeMap;

use oxc_ast::ast::{JSXAttribute, JSXAttributeValue, ObjectProperty};
use oxc_ast_visit::{Visit, walk};
use oxc_span::ContentEq;

use crate::extract_style::{
    ExtractStyleProperty, extract_dynamic_style::ExtractDynamicStyle, style_property::StyleProperty,
};
use crate::{ExtractOption, ExtractOutput, ExtractStyleValue};

/// Compare complete expressions after the same parse/codegen/reparse roundtrip.
/// This retains the full callee and arguments, including numeric zero/px guards.
fn expression_eq(left: &str, right: &str) -> bool {
    let arena = oxc_allocator::Allocator::default();
    let canonical = |source: &str| {
        let source = format!("({source});");
        let parsed = oxc_parser::Parser::new(&arena, &source, oxc_span::SourceType::tsx()).parse();
        assert!(
            !parsed.fatal_error && parsed.diagnostics.is_empty(),
            "{source}"
        );
        oxc_codegen::Codegen::new().build(&parsed.program).code
    };
    let left = canonical(left);
    let right = canonical(right);
    let left = oxc_parser::Parser::new(&arena, &left, oxc_span::SourceType::tsx()).parse();
    let right = oxc_parser::Parser::new(&arena, &right, oxc_span::SourceType::tsx()).parse();
    assert!(left.diagnostics.is_empty() && right.diagnostics.is_empty());
    left.program.content_eq(&right.program)
}

pub(crate) fn numeric(read: &str) -> String {
    numeric_with_capture(read, "__devupValue0")
}

pub(crate) fn numeric_with_capture(read: &str, name: &str) -> String {
    format!(
        "(({name}) => typeof {name} === \"number\" && {name} !== 0 ? {name} + \"px\" : {name})({read})"
    )
}

pub(crate) fn identifier_reads(code: &str, name: &str) -> usize {
    struct Reads<'s> {
        name: &'s str,
        count: usize,
    }
    impl<'a> Visit<'a> for Reads<'_> {
        fn visit_identifier_reference(
            &mut self,
            identifier: &oxc_ast::ast::IdentifierReference<'a>,
        ) {
            self.count += usize::from(identifier.name == self.name);
        }
    }
    let arena = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&arena, code, oxc_span::SourceType::tsx()).parse();
    assert!(!parsed.fatal_error && parsed.diagnostics.is_empty());
    let mut reads = Reads { name, count: 0 };
    reads.visit_program(&parsed.program);
    reads.count
}

/// Check every property, complete public identifier, and actual CSS variable slot.
/// The returned class/variable pairs follow the authored expected-property order.
pub(crate) fn assert_variables(
    output: &ExtractOutput,
    filename: &str,
    options: &ExtractOption,
    expected: &[(&str, String)],
) -> Vec<(String, String)> {
    #[derive(Default)]
    struct Slots {
        variables: BTreeMap<String, Vec<String>>,
        classes: Vec<String>,
    }
    impl<'a> Visit<'a> for Slots {
        fn visit_object_property(&mut self, property: &ObjectProperty<'a>) {
            if let Some(key) = property.key.static_name().map(|key| {
                let Some(key) = key.into_cow_str() else {
                    panic!("UTF-8 CSS key");
                };
                key
            }) && key.starts_with("--")
            {
                self.variables
                    .entry(key.into_owned())
                    .or_default()
                    .push(crate::utils::readable_code(&property.value));
            }
            walk::walk_object_property(self, property);
        }
        fn visit_jsx_attribute(&mut self, attribute: &JSXAttribute<'a>) {
            if attribute
                .name
                .as_identifier()
                .is_some_and(|name| name.name == "className")
                && let Some(JSXAttributeValue::StringLiteral(value)) = &attribute.value
            {
                let Some(class) = value.value.as_str() else {
                    panic!("UTF-8 class");
                };
                self.classes
                    .extend(class.split_whitespace().map(str::to_owned));
            }
            walk::walk_jsx_attribute(self, attribute);
        }
    }
    let arena = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&arena, &output.code, oxc_span::SourceType::tsx()).parse();
    assert!(!parsed.fatal_error && parsed.diagnostics.is_empty());
    let mut slots = Slots::default();
    slots.visit_program(&parsed.program);
    assert_eq!(output.styles.len(), expected.len());
    assert_eq!(slots.variables.len(), expected.len());
    let bucket = crate::css_bucket(filename, options);
    let mut result = Vec::new();
    for (property, expected_expression) in expected {
        let styles: Vec<_> = output
            .styles
            .iter()
            .filter_map(|style| match style {
                ExtractStyleValue::Dynamic(style) if style.property() == *property => Some(style),
                _ => None,
            })
            .collect();
        assert_eq!(styles.len(), 1, "missing/duplicate {property}: {output:?}");
        let style = styles[0];
        assert_eq!(
            *style,
            ExtractDynamicStyle::new(property, 0, style.identifier(), None)
        );
        assert!(
            expression_eq(style.identifier(), expected_expression),
            "{property}: {output:?}"
        );
        let StyleProperty::Variable {
            class_name,
            variable_name,
            identifier,
        } = style.extract(bucket.as_deref())
        else {
            panic!("expected variable metadata");
        };
        assert!(expression_eq(&identifier, expected_expression));
        let Some(supplied) = slots.variables.get(&variable_name) else {
            panic!("public variable absent from JSX");
        };
        assert_eq!(supplied.len(), 1);
        assert!(
            expression_eq(&supplied[0], expected_expression),
            "{property}: {output:?}"
        );
        assert_eq!(
            slots
                .classes
                .iter()
                .filter(|name| **name == class_name)
                .count(),
            1
        );
        result.push((class_name, variable_name));
    }
    result
}
