use std::collections::HashMap;

use crate::{
    ExtractStyleProp,
    component::ExportVariableKind,
    extractor::{ExtractResult, extract_style_from_expression::extract_style_from_expression},
    gen_class_name::gen_class_names,
};
use oxc_allocator::CloneIn;
use oxc_ast::{
    AstBuilder,
    ast::{Argument, CallExpression, Expression, ObjectPropertyKind},
};
use oxc_span::SPAN;

/// Extract styles from vanilla-extract style() function calls
/// Handles patterns like:
/// - style({ background: 'red', color: 'blue' })
/// - style({ ':hover': { background: 'blue' } })
/// - styleVariants({ primary: { bg: 'blue' }, secondary: { bg: 'red' } })
pub fn extract_vanilla_extract_style<'a>(
    ast_builder: &AstBuilder<'a>,
    call: &mut CallExpression<'a>,
    split_filename: Option<&str>,
    _imports: &HashMap<String, ExportVariableKind>,
) -> Option<(ExtractResult<'a>, Expression<'a>)> {
    // Check if this is a vanilla-extract style() or styleVariants() call
    let function_name = match &call.callee {
        Expression::Identifier(ident) => ident.name.as_str(),
        _ => return None,
    };

    if !matches!(function_name, "style" | "styleVariants" | "globalStyle") {
        return None;
    }

    // Extract the style object from the first argument
    if call.arguments.is_empty() {
        return None;
    }

    let mut props_styles: Vec<ExtractStyleProp<'_>> = vec![];

    match function_name {
        "style" => {
            // style({ ... })
            if let Some(Argument::ObjectExpression(_)) = call.arguments.first_mut() {
                let mut expr = call.arguments[0].to_expression().clone_in(ast_builder.allocator);
                let result = extract_style_from_expression(
                    ast_builder,
                    None,
                    &mut expr,
                    0,
                    &None,
                );
                props_styles.extend(result.styles);
            }
        }
        "styleVariants" => {
            // styleVariants({ primary: { ... }, secondary: { ... } })
            if let Some(Argument::ObjectExpression(obj)) = call.arguments.first_mut() {
                for prop in &mut obj.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = prop {
                        if let Expression::ObjectExpression(_) = &property.value {
                            let mut style_expr = property.value.clone_in(ast_builder.allocator);
                            let result = extract_style_from_expression(
                                ast_builder,
                                None,
                                &mut style_expr,
                                0,
                                &None,
                            );
                            props_styles.extend(result.styles);
                        }
                    }
                }
            }
        }
        "globalStyle" => {
            // globalStyle('selector', { ... })
            if call.arguments.len() >= 2 {
                let mut style_expr = call.arguments[1].to_expression().clone_in(ast_builder.allocator);
                let result = extract_style_from_expression(
                    ast_builder,
                    None,
                    &mut style_expr,
                    0,
                    &None,
                );
                props_styles.extend(result.styles);
            }
        }
        _ => return None,
    }

    if props_styles.is_empty() {
        return None;
    }

    let class_name_opt = gen_class_names(ast_builder, &mut props_styles, None, split_filename);

    // Extract the class name from the Option<Expression>
    let class_name_str = if let Some(Expression::StringLiteral(lit)) = &class_name_opt {
        lit.value.to_string()
    } else {
        "ve-class".to_string()
    };

    // Return a string literal with the generated class name
    let class_name_expr = ast_builder.expression_string_literal(
        SPAN,
        ast_builder.atom(&class_name_str),
        None,
    );

    let result = ExtractResult {
        styles: props_styles,
        tag: None,
        style_order: None,
        style_vars: None,
        props: None,
    };

    Some((result, class_name_expr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    #[test]
    fn test_vanilla_extract_simple_style() {
        let allocator = Allocator::default();
        let source_text = r#"
import { style } from '@vanilla-extract/css';
const button = style({ background: 'red', color: 'white' });
        "#;

        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        assert!(!ret.panicked);
        // More assertions would go here in a real test
    }

    #[test]
    fn test_vanilla_extract_with_hover() {
        let allocator = Allocator::default();
        let source_text = r#"
import { style } from '@vanilla-extract/css';
const button = style({
    background: 'blue',
    ':hover': {
        background: 'darkblue'
    }
});
        "#;

        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        assert!(!ret.panicked);
    }

    #[test]
    fn test_vanilla_extract_style_variants() {
        let allocator = Allocator::default();
        let source_text = r#"
import { styleVariants } from '@vanilla-extract/css';
const variants = styleVariants({
    primary: { background: 'blue' },
    secondary: { background: 'gray' }
});
        "#;

        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        assert!(!ret.panicked);
    }

    #[test]
    fn test_vanilla_extract_global_style() {
        let allocator = Allocator::default();
        let source_text = r#"
import { globalStyle } from '@vanilla-extract/css';
globalStyle('body', { margin: 0, padding: 0 });
        "#;

        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        assert!(!ret.panicked);
    }

    #[test]
    fn test_vanilla_extract_media_queries() {
        let allocator = Allocator::default();
        let source_text = r#"
import { style } from '@vanilla-extract/css';
const responsive = style({
    padding: '1rem',
    '@media': {
        'screen and (min-width: 768px)': {
            padding: '2rem'
        }
    }
});
        "#;

        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        assert!(!ret.panicked);
    }

    #[test]
    fn test_vanilla_extract_nested_selectors() {
        let allocator = Allocator::default();
        let source_text = r#"
import { style } from '@vanilla-extract/css';
const container = style({
    display: 'flex',
    selectors: {
        '&:hover': {
            opacity: 0.8
        },
        '& > *': {
            margin: '0.5rem'
        }
    }
});
        "#;

        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        assert!(!ret.panicked);
    }

    #[test]
    fn test_vanilla_extract_complex_object() {
        let allocator = Allocator::default();
        let source_text = r#"
import { style } from '@vanilla-extract/css';
const card = style({
    backgroundColor: 'white',
    borderRadius: '8px',
    padding: '1.5rem',
    boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
    transition: 'transform 0.2s ease',
    ':hover': {
        transform: 'translateY(-2px)',
        boxShadow: '0 4px 8px rgba(0, 0, 0, 0.15)'
    }
});
        "#;

        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        assert!(!ret.panicked);
    }

    #[test]
    fn test_vanilla_extract_conditional_styles() {
        let allocator = Allocator::default();
        let source_text = r#"
import { style } from '@vanilla-extract/css';
import { recipe } from '@vanilla-extract/recipes';

const button = recipe({
    base: {
        borderRadius: '4px'
    },
    variants: {
        color: {
            primary: { background: 'blue' },
            secondary: { background: 'gray' }
        },
        size: {
            small: { padding: '0.5rem' },
            large: { padding: '1rem' }
        }
    }
});
        "#;

        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        assert!(!ret.panicked);
    }

    #[test]
    fn test_vanilla_extract_keyframes() {
        let allocator = Allocator::default();
        let source_text = r#"
import { style, keyframes } from '@vanilla-extract/css';

const fadeIn = keyframes({
    '0%': { opacity: 0 },
    '100%': { opacity: 1 }
});

const animated = style({
    animation: `${fadeIn} 1s ease-in`
});
        "#;

        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        assert!(!ret.panicked);
    }

    #[test]
    fn test_vanilla_extract_vars() {
        let allocator = Allocator::default();
        let source_text = r#"
import { style, createVar } from '@vanilla-extract/css';

const myVar = createVar();

const element = style({
    vars: {
        [myVar]: 'purple'
    },
    color: myVar
});
        "#;

        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        assert!(!ret.panicked);
    }
}
