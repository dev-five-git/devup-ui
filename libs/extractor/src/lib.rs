mod component;
mod gen_class_name;
mod gen_style;
mod media_prop_extract_utils;
mod prop_extract_utils;
mod prop_modify_utils;
mod utils;
mod visit;

use oxc_codegen::Codegen;

use crate::utils::convert_value;
use crate::visit::DevupVisitor;
use css::{sheet_to_classname, sheet_to_variable_name};
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
                    styles.append(&mut consequent.extract());
                }
                if let Some(alternate) = alternate {
                    styles.append(&mut alternate.extract());
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
    pub property: String,
    /// fixed value
    pub value: String,
    /// responsive level
    pub level: u8,
    /// selector
    pub selector: Option<String>,
}

impl ExtractStyleProperty for ExtractStaticStyle {
    fn extract(&self) -> StyleProperty {
        StyleProperty::ClassName(sheet_to_classname(
            self.property.as_str(),
            self.level,
            Some(convert_value(self.value.as_str()).as_str()),
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExtractCss {
    /// css code
    pub css: String,
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
    pub property: String,
    /// responsive
    pub level: u8,
    identifier: String,

    /// selector
    pub selector: Option<String>,
}

impl ExtractStyleProperty for ExtractDynamicStyle {
    fn extract(&self) -> StyleProperty {
        StyleProperty::Variable {
            class_name: sheet_to_classname(self.property.as_str(), self.level, None),
            variable_name: sheet_to_variable_name(self.property.as_str(), self.level),
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
    pub css_file: Option<String>,
}

pub fn extract(
    filename: &str,
    code: &str,
    option: ExtractOption,
) -> Result<ExtractOutput, Box<dyn Error>> {
    if !code.contains(option.package.as_str()) {
        // skip if not using package
        return Ok(ExtractOutput {
            styles: vec![],
            code: code.to_string(),
        });
    }
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
    let mut visitor = DevupVisitor::new(
        &allocator,
        &option.package,
        option
            .css_file
            .unwrap_or(format!("{}/devup-ui.css", option.package))
            .as_str(),
    );
    visitor.visit_program(&mut program);

    Ok(ExtractOutput {
        styles: visitor.styles,
        code: Codegen::new().build(&program).code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use std::hash::{DefaultHasher, Hasher};

    #[test]
    fn extract_just_tsx() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            "const a = 1;",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            },
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            "<Box gap={1} />",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            },
        )
        .unwrap());
    }
    #[test]
    fn ignore_special_props() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r"import {Box} from '@devup-ui/core'
        <Box padding={1} ref={ref} data-test={1} role={2} children={[]} onClick={()=>{}} aria-valuenow={24} key={2} />
        ",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }
    #[test]
    fn extract_style_props() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r"import {Box} from '@devup-ui/core'
        <Box padding={1} margin={2} wrong={} />
        ",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
        assert_debug_snapshot!(extract(
            "test.tsx",
            r"import {Box as C} from '@devup-ui/core'
                <C padding={1} margin={2} />
                ",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_style_props_with_class_name() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className="exists class name" />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className="  exists class name  " />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={"exists class name"} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={"a"+"b"} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_class_name_from_component() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {VStack as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={"a"+"b"} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap(),);
    }
    #[test]
    fn extract_responsive_style_props() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box padding={[null,1]} margin={[2,null,4]} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_dynamic_style_props() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box padding={someStyleVar} margin={someStyleVar2} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_dynamic_responsive_style_props() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box padding={[someStyleVar,null,someStyleVar1]} margin={[null,someStyleVar2]} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_compound_responsive_style_props() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box padding={[someStyleVar,undefined,someStyleVar1]} margin={[null,someStyleVar2]} bg="red" />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_wrong_responsive_style_props() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box padding={[NaN,undefined,null]} margin={Infinity} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_variable_style_props_with_style() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a} style={{ key:value }} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a} style={styles} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_conditional_style_props() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? "4px" : "3px"} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? c : d} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? "4px" : d} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_responsive_conditional_style_props() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b ? "4px" : "3px"]} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
        <Box margin={["6px", a === b ? "4px" : "3px"]} />;
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? c : [d, e, f, "2px"]} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? c : [d, e, f, x === y ? "4px" : "2px"]} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? [d, e, f, x === y ? "4px" : "2px"] : c} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? [d, e, f, x === y ? "4px" : "2px"] : ["1px", "2px", "3px"]} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b && "4px"]} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b && "4px", c === d ? "5px" : null]} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_logical_case() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a===b && "1px"} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a===b || "1px"} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a ?? "1px"} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }
    #[test]
    fn extract_responsive_conditional_style_props_with_class_name() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b ? (q > w ? "4px" : "8px") : "3px"]} className={"exists"} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b || "4px"]} className={"exists"} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_selector() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r"import {Box} from '@devup-ui/core'
        <Box _hover={{
          mx: 1
        }} />
        ",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_selector_with_responsive() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r"import {Box} from '@devup-ui/core'
        <Box _hover={{
          mx: [1, 2]
        }} />
        ",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        assert_debug_snapshot!(extract(
            "test.tsx",
            r"import {Box} from '@devup-ui/core'
        <Box _hover={[{
          mx: 10
        },{
          mx: 20
        }]} />
        ",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_static_css_class_name_props() {
        let mut hasher = DefaultHasher::new();
        hasher.write("background-color: red;".as_bytes());
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { css } from "@devup-ui/core";
<Box className={css`
  background-color: red;
`}/>;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    fn extract_static_css_with_theme() {
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
        <Box color="$nice" />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }
}
