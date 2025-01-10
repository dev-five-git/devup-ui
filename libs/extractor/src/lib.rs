mod component;
mod gen_class_name;
mod gen_style;
mod prop_extract_utils;
mod prop_modify_utils;
mod utils;
mod visit;

use oxc_codegen::Codegen;

use crate::utils::{convert_value, get_variable_class_name, get_variable_name};
use crate::visit::DevupVisitor;
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_ast::VisitMut;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;
use std::error::Error;
use std::hash::{DefaultHasher, Hasher};

/// result of extracting style properties from props
#[derive(Debug)]
pub enum ExtractStyleProp<'a> {
    Static(ExtractStyleValue),
    Responsive(Vec<ExtractStyleProp<'a>>),
    /// static + static ex) margin={test?"4px":"8px"} --> className={test?"margin-4px-0":"margin-8px-0"}
    /// static + dynamic ex) margin={test?a:"8px"} --> className={test?"margin-0":"margin-8px-0"} style={{ "--margin-0": a }}
    /// dynamic + dynamic ex) margin={test?a:b} --> className="margin-0" style={{ "--margin-0": test?a:b }}
    /// issue case: dynamic + dynamic ex) margin={test?a:(b ? "8px": c)} --> className="margin-0" style={{ "--margin-0": test?a:(b ? "8px": c) }}
    Conditional {
        condition: Expression<'a>,
        consequent: Option<Box<ExtractStyleProp<'a>>>,
        alternate: Option<Box<ExtractStyleProp<'a>>>,
    },
}
impl ExtractStyleProp<'_> {
    pub fn extract(&self) -> Vec<ExtractStyleValue> {
        match self {
            ExtractStyleProp::Static(style) => vec![style.clone()],
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => {
                let mut styles = vec![];
                if let Some(consequent) = consequent {
                    styles.extend(consequent.extract());
                }
                if let Some(alternate) = alternate {
                    styles.extend(alternate.extract());
                }
                styles
            }
            ExtractStyleProp::Responsive(ref array) => {
                array.iter().flat_map(|s| s.extract()).collect()
            }
        }
    }
}
/// Style property for props
pub enum StyleProperty {
    ClassName(String),
    Variable {
        class_name: String,
        variable_name: String,
        identifier: String,
    },
}
pub trait ExtractStyleProperty {
    /// extract style properties
    fn extract(&self) -> StyleProperty;
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExtractStaticStyle {
    /// property
    property: String,
    /// fixed value
    value: String,
    /// responsive level
    level: u8,
}

impl ExtractStyleProperty for ExtractStaticStyle {
    fn extract(&self) -> StyleProperty {
        StyleProperty::ClassName(format!(
            "{}-{}-{}",
            self.property,
            convert_value(self.value.as_str()),
            self.level
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExtractCss {
    /// css code
    css: String,
}

impl ExtractStyleProperty for ExtractCss {
    /// hashing css code to class name
    fn extract(&self) -> StyleProperty {
        let mut hasher = DefaultHasher::new();
        hasher.write(self.css.as_bytes());
        StyleProperty::ClassName(format!("d{}", hasher.finish()))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExtractDynamicStyle {
    /// property
    property: String,
    /// responsive
    level: u8,
    identifier: String,
}

impl ExtractStyleProperty for ExtractDynamicStyle {
    fn extract(&self) -> StyleProperty {
        StyleProperty::Variable {
            class_name: get_variable_class_name(self.property.as_str(), self.level),
            variable_name: get_variable_name(self.property.as_str(), self.level),
            identifier: self.identifier.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExtractStyleValue {
    Static(ExtractStaticStyle),
    Dynamic(ExtractDynamicStyle),
    Css(ExtractCss),
}

impl ExtractStyleValue {
    pub fn extract(&self) -> StyleProperty {
        match self {
            ExtractStyleValue::Static(style) => style.extract(),
            ExtractStyleValue::Dynamic(style) => style.extract(),
            ExtractStyleValue::Css(css) => css.extract(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ExtractOutput {
    // used styles
    pub styles: Vec<ExtractStyleValue>,

    // output source
    pub code: String,
}

pub struct ExtractOption {
    pub package: String,
}

pub fn extract(
    filename: &str,
    code: &str,
    option: ExtractOption,
) -> Result<ExtractOutput, Box<dyn Error>> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)?;

    let ParserReturn {
        mut program, // AST
        errors,      // Syntax errors
        panicked,    // Parser encountered an error it couldn't recover from
        ..
    } = Parser::new(&allocator, code, source_type).parse();
    if panicked {
        return Err("Parser panicked".into());
    }
    if !errors.is_empty() {
        return Err(errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
            .into());
    }
    let mut visitor = DevupVisitor::new(&allocator, &option.package);
    visitor.visit_program(&mut program);

    Ok(ExtractOutput {
        styles: visitor.styles,
        code: Codegen::new().build(&program).code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{DefaultHasher, Hasher};

    #[test]
    fn extract_just_tsx() {
        assert_eq!(
            extract(
                "test.tsx",
                "const a = 1;",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                },
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![],
                code: "const a = 1;\n".to_string(),
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                "<Box gap={1} />",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                },
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![],
                code: "<Box gap={1} />;\n".to_string(),
            }
        );
    }
    #[test]
    fn extract_style_props() {
        assert_eq!(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box padding={1} margin={2} wrong={} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "padding".to_string(),
                        value: "4px".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        value: "8px".to_string(),
                        level: 0
                    })
                ],
                code: r#"import { Box } from "@devup-ui/core";
<Box wrong={} className="padding-4px-0 margin-8px-0" />;
"#
                .to_string(),
            }
        );
        assert_eq!(
            extract(
                "test.tsx",
                r"import {Box as C} from '@devup-ui/core'
                <C padding={1} margin={2} />
                ",
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "padding".to_string(),
                        value: "4px".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        value: "8px".to_string(),
                        level: 0
                    })
                ],
                code: r#"import { Box as C } from "@devup-ui/core";
<C className="padding-4px-0 margin-8px-0" />;
"#
                .to_string(),
            }
        );
    }

    #[test]
    fn extract_style_props_with_class_name() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className="exists class name" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "padding".to_string(),
                        value: "4px".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        value: "8px".to_string(),
                        level: 0
                    })
                ],
                code: r#"import { Box as C } from "@devup-ui/core";
<C className="exists class name padding-4px-0 margin-8px-0" />;
"#
                .to_string(),
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className="  exists class name  " />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "padding".to_string(),
                        value: "4px".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        value: "8px".to_string(),
                        level: 0
                    })
                ],
                code: r#"import { Box as C } from "@devup-ui/core";
<C className="exists class name padding-4px-0 margin-8px-0" />;
"#
                .to_string(),
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={"exists class name"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "padding".to_string(),
                        value: "4px".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        value: "8px".to_string(),
                        level: 0
                    })
                ],
                code: r#"import { Box as C } from "@devup-ui/core";
<C className="exists class name padding-4px-0 margin-8px-0" />;
"#
                .to_string(),
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "padding".to_string(),
                        value: "4px".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        value: "8px".to_string(),
                        level: 0
                    })
                ],
                code: r#"import { Box as C } from "@devup-ui/core";
<C className="padding-4px-0 margin-8px-0" />;
"#
                .to_string(),
            }
        );
        assert_eq!(
            extract(
                "test.tsx",
                r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={"a"+"b"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "padding".to_string(),
                        value: "4px".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        value: "8px".to_string(),
                        level: 0
                    })
                ],
                code: r#"import { Box as C } from "@devup-ui/core";
<C className={`padding-4px-0 margin-8px-0 ${"a" + "b"}`} />;
"#
                .to_string(),
            }
        );
    }

    #[test]
    fn extract_class_name_from_component() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import {VStack as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={"a"+"b"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "display".to_string(),
                        value: "flex".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "flexDirection".to_string(),
                        value: "column".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "padding".to_string(),
                        value: "4px".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        value: "8px".to_string(),
                        level: 0
                    }),
                ],
                code: r#"import { VStack as C } from "@devup-ui/core";
<C className={`display-flex-0 flexDirection-column-0 padding-4px-0 margin-8px-0 ${"a" + "b"}`} />;
"#
                .to_string(),
            }
        );
    }
    #[test]
    fn extract_responsive_style_props() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={[null,1]} margin={[2,null,4]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "padding".to_string(),
                        value: "4px".to_string(),
                        level: 1
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        value: "8px".to_string(),
                        level: 0
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        value: "16px".to_string(),
                        level: 2
                    })
                ],
                code: r#"import { Box } from "@devup-ui/core";
<Box className="padding-4px-1 margin-8px-0 margin-16px-2" />;
"#
                .to_string(),
            }
        );
    }

    #[test]
    fn extract_dynamic_style_props() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={someStyleVar} margin={someStyleVar2} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "padding".to_string(),
                        level: 0,
                        identifier: "someStyleVar".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 0,
                        identifier: "someStyleVar2".to_string(),
                    }),
                ],
                code: format!(
                    r#"import {{ Box }} from "@devup-ui/core";
<Box className="padding-0 margin-0" style={{{{
{}"--padding-0": someStyleVar,
{}"--margin-0": someStyleVar2
}}}} />;
"#,
                    "\t", "\t"
                )
                .to_string(),
            }
        );
    }

    #[test]
    fn extract_dynamic_responsive_style_props() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={[someStyleVar,null,someStyleVar1]} margin={[null,someStyleVar2]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "padding".to_string(),
                        level: 0,
                        identifier: "someStyleVar".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "padding".to_string(),
                        level: 2,
                        identifier: "someStyleVar1".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 1,
                        identifier: "someStyleVar2".to_string(),
                    }),
                ],
                code: format!(
                    r#"import {{ Box }} from "@devup-ui/core";
<Box className="padding-0 padding-2 margin-1" style={{{{
{}"--padding-0": someStyleVar,
{}"--padding-2": someStyleVar1,
{}"--margin-1": someStyleVar2
}}}} />;
"#,
                    "\t", "\t", "\t",
                )
                .to_string(),
            }
        );
    }

    #[test]
    fn extract_compound_responsive_style_props() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={[someStyleVar,undefined,someStyleVar1]} margin={[null,someStyleVar2]} bg="red" />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "padding".to_string(),
                        level: 0,
                        identifier: "someStyleVar".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "padding".to_string(),
                        level: 2,
                        identifier: "someStyleVar1".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 1,
                        identifier: "someStyleVar2".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "bg".to_string(),
                        value: "red".to_string(),
                        level: 0,
                    }),
                ],
                code: r#"import { Box } from "@devup-ui/core";
<Box className="padding-0 padding-2 margin-1 bg-red-0" style={{
	"--padding-0": someStyleVar,
	"--padding-2": someStyleVar1,
	"--margin-1": someStyleVar2
}} />;
"#
                .to_string(),
            }
        );
    }

    #[test]
    fn extract_wrong_responsive_style_props() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={[NaN,undefined,null]} margin={Infinity} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![],
                code: format!(
                    r#"import {{ Box }} from "@devup-ui/core";
<Box padding={{[
{}NaN,
{}undefined,
{}null
]}} margin={{Infinity}} />;
"#,
                    "\t", "\t", "\t"
                )
                .to_string(),
            }
        );
    }

    #[test]
    fn extract_variable_style_props_with_style() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a} style={{ key:value }} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                    property: "margin".to_string(),
                    level: 0,
                    identifier: "a".to_string(),
                }),],
                code: format!(
                    r#"import {{ Box }} from "@devup-ui/core";
<Box className="margin-0" style={{{{
{}...{{ key: value }},
{}"--margin-0": a
}}}} />;
"#,
                    "\t", "\t"
                )
                .to_string(),
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a} style={styles} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                    property: "margin".to_string(),
                    level: 0,
                    identifier: "a".to_string(),
                })],
                code: format!(
                    r#"import {{ Box }} from "@devup-ui/core";
<Box className="margin-0" style={{{{
{}...styles,
{}"--margin-0": a
}}}} />;
"#,
                    "\t", "\t"
                )
                .to_string(),
            }
        );
    }

    #[test]
    fn extract_conditional_style_props() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? "4px" : "3px"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 0,
                        value: "4px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 0,
                        value: "3px".to_string(),
                    }),
                ],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={a === b ? "margin-4px-0" : "margin-3px-0"} />;
"#
                .to_string()
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? c : d} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 0,
                        identifier: "c".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 0,
                        identifier: "d".to_string(),
                    }),
                ],
                code: r#"import { Box } from "@devup-ui/core";
<Box className="margin-0" style={{ "--margin-0": a === b ? c : d }} />;
"#
                .to_string()
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? "4px" : d} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 0,
                        value: "4px".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 0,
                        identifier: "d".to_string(),
                    }),
                ],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={a === b ? "margin-4px-0" : "margin-0"} style={{ "--margin-0": d }} />;
"#
                .to_string()
            }
        );
    }

    #[test]
    fn extract_responsive_conditional_style_props() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b ? "4px" : "3px"]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 1,
                        value: "4px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 1,
                        value: "3px".to_string(),
                    }),
                ],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={a === b ? "margin-4px-1" : "margin-3px-1"} />;
"#
                .to_string()
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
        <Box margin={["6px", a === b ? "4px" : "3px"]} />;
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 0,
                        value: "6px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 1,
                        value: "4px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 1,
                        value: "3px".to_string(),
                    }),
                ],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={`margin-6px-0 ${a === b ? "margin-4px-1" : "margin-3px-1"}`} />;
"#
                .to_string()
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? c : [d, e, f, "2px"]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 0,
                        identifier: "c".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 0,
                        identifier: "d".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 1,
                        identifier: "e".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 2,
                        identifier: "f".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 3,
                        value: "2px".to_string(),
                    }),
                ],
                code: format!(
                    r#"import {{ Box }} from "@devup-ui/core";
<Box className={{a === b ? "margin-0" : "margin-0 margin-1 margin-2 margin-2px-3"}} style={{{{
{}"--margin-0": a === b ? c : d,
{}"--margin-1": e,
{}"--margin-2": f
}}}} />;
"#,
                    "\t", "\t", "\t"
                )
                .to_string()
            }
        );

        assert_eq!(
                    extract(
                        "test.tsx",
                        r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? c : [d, e, f, x === y ? "4px" : "2px"]} />;
"#,
                        ExtractOption {
                            package: "@devup-ui/core".to_string()
                        }
                    )
                    .unwrap(),
                    ExtractOutput {
                        styles: vec![
                            ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                                property: "margin".to_string(),
                                level: 0,
                                identifier: "c".to_string(),
                            }),
                            ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                                property: "margin".to_string(),
                                level: 0,
                                identifier: "d".to_string(),
                            }),
                            ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                                property: "margin".to_string(),
                                level: 1,
                                identifier: "e".to_string(),
                            }),
                            ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                                property: "margin".to_string(),
                                level: 2,
                                identifier: "f".to_string(),
                            }),
                            ExtractStyleValue::Static(ExtractStaticStyle {
                                property: "margin".to_string(),
                                level: 3,
                                value: "4px".to_string(),
                            }),
                            ExtractStyleValue::Static(ExtractStaticStyle {
                                property: "margin".to_string(),
                                level: 3,
                                value: "2px".to_string(),
                            }),
                        ],
                        code: format!(r#"import {{ Box }} from "@devup-ui/core";
<Box className={{a === b ? "margin-0" : `margin-0 margin-1 margin-2 ${{x === y ? "margin-4px-3" : "margin-2px-3"}}`}} style={{{{
{}"--margin-0": a === b ? c : d,
{}"--margin-1": e,
{}"--margin-2": f
}}}} />;
"#, "\t", "\t", "\t")
                        .to_string()
                    }
                );

        assert_eq!(
                    extract(
                        "test.tsx",
                        r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? [d, e, f, x === y ? "4px" : "2px"] : c} />;
"#,
                        ExtractOption {
                            package: "@devup-ui/core".to_string()
                        }
                    )
                        .unwrap(),
                    ExtractOutput {
                        styles: vec![
                            ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                                property: "margin".to_string(),
                                level: 0,
                                identifier: "d".to_string(),
                            }),
                            ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                                property: "margin".to_string(),
                                level: 1,
                                identifier: "e".to_string(),
                            }),
                            ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                                property: "margin".to_string(),
                                level: 2,
                                identifier: "f".to_string(),
                            }),
                            ExtractStyleValue::Static(ExtractStaticStyle {
                                property: "margin".to_string(),
                                level: 3,
                                value: "4px".to_string(),
                            }),
                            ExtractStyleValue::Static(ExtractStaticStyle {
                                property: "margin".to_string(),
                                level: 3,
                                value: "2px".to_string(),
                            }),
                            ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                                property: "margin".to_string(),
                                level: 0,
                                identifier: "c".to_string(),
                            }),
                        ],
                        code: format!(r#"import {{ Box }} from "@devup-ui/core";
<Box className={{a === b ? `margin-0 margin-1 margin-2 ${{x === y ? "margin-4px-3" : "margin-2px-3"}}` : "margin-0"}} style={{{{
{}"--margin-0": a === b ? d : c,
{}"--margin-1": e,
{}"--margin-2": f
}}}} />;
"#, "\t", "\t", "\t")
                            .to_string()
                    }
                );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? [d, e, f, x === y ? "4px" : "2px"] : ["1px", "2px", "3px"]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
                .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 0,
                        identifier: "d".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 1,
                        identifier: "e".to_string(),
                    }),
                    ExtractStyleValue::Dynamic(ExtractDynamicStyle {
                        property: "margin".to_string(),
                        level: 2,
                        identifier: "f".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 3,
                        value: "4px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 3,
                        value: "2px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 0,
                        value: "1px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 1,
                        value: "2px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 2,
                        value: "3px".to_string(),
                    }),
                ],
                code: format!(r#"import {{ Box }} from "@devup-ui/core";
<Box className={{a === b ? `margin-0 margin-1 margin-2 ${{x === y ? "margin-4px-3" : "margin-2px-3"}}` : "margin-1px-0 margin-2px-1 margin-3px-2"}} style={{{{
{}"--margin-0": d,
{}"--margin-1": e,
{}"--margin-2": f
}}}} />;
"#, "\t", "\t", "\t")
                    .to_string()
            }
        );
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b && "4px"]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![ExtractStyleValue::Static(ExtractStaticStyle {
                    property: "margin".to_string(),
                    level: 1,
                    value: "4px".to_string(),
                }),],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={a === b ? "margin-4px-1" : ""} />;
"#
                .to_string()
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b && "4px", c === d ? "5px" : null]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 1,
                        value: "4px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 2,
                        value: "5px".to_string(),
                    })
                ],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={`${a === b ? "margin-4px-1" : ""} ${c === d ? "margin-5px-2" : ""}`} />;
"#
                .to_string()
            }
        );
    }

    #[test]
    fn extract_logical_case() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a===b && "1px"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![ExtractStyleValue::Static(ExtractStaticStyle {
                    property: "margin".to_string(),
                    level: 0,
                    value: "1px".to_string(),
                }),],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={a === b ? "margin-1px-0" : ""} />;
"#
                .to_string()
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a===b || "1px"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![ExtractStyleValue::Static(ExtractStaticStyle {
                    property: "margin".to_string(),
                    level: 0,
                    value: "1px".to_string(),
                }),],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={a === b ? "" : "margin-1px-0"} />;
"#
                .to_string()
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a ?? "1px"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![ExtractStyleValue::Static(ExtractStaticStyle {
                    property: "margin".to_string(),
                    level: 0,
                    value: "1px".to_string(),
                }),],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={a !== null && a !== undefined ? "" : "margin-1px-0"} />;
"#
                .to_string()
            }
        );
    }
    #[test]
    fn extract_responsive_conditional_style_props_with_class_name() {
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b ? (q > w ? "4px" : "8px") : "3px"]} className={"exists"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 1,
                        value: "4px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 1,
                        value: "8px".to_string(),
                    }),
                    ExtractStyleValue::Static(ExtractStaticStyle {
                        property: "margin".to_string(),
                        level: 1,
                        value: "3px".to_string(),
                    }),
                ],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={`exists ${a === b ? q > w ? "margin-4px-1" : "margin-8px-1" : "margin-3px-1"}`} />;
"#
                .to_string()
            }
        );

        assert_eq!(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b || "4px"]} className={"exists"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![ExtractStyleValue::Static(ExtractStaticStyle {
                    property: "margin".to_string(),
                    level: 1,
                    value: "4px".to_string(),
                }),],
                code: r#"import { Box } from "@devup-ui/core";
<Box className={`exists ${a === b ? "" : "margin-4px-1"}`} />;
"#
                .to_string()
            }
        );
    }
    #[test]
    fn extract_static_css_class_name_props() {
        let mut hasher = DefaultHasher::new();
        hasher.write("background-color: red;".as_bytes());
        assert_eq!(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css`
  background-color: red;
`}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string()
                }
            )
            .unwrap(),
            ExtractOutput {
                styles: vec![ExtractStyleValue::Css(ExtractCss {
                    css: "background-color: red;".to_string(),
                }),],
                code: format!(
                    r#"import {{ css }} from "@devup-ui/core";
<Box className={{css`{}`}} />;
"#,
                    format!("d{}", hasher.finish())
                )
            }
        );
    }
}
