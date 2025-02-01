mod component;
pub mod extract_style;
mod gen_class_name;
mod gen_style;
mod prop_modify_utils;
mod style_extractor;
mod utils;
mod visit;

use oxc_codegen::Codegen;
use std::collections::BTreeMap;

use crate::extract_style::ExtractStyleValue;
use crate::visit::DevupVisitor;
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_ast::VisitMut;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;
use std::error::Error;

/// result of extracting style properties from props
#[derive(Debug)]
pub enum ExtractStyleProp<'a> {
    Static(ExtractStyleValue),
    StaticArray(Vec<ExtractStyleProp<'a>>),
    /// static + static ex) margin={test?"4px":"8px"} --> className={test?"margin-4px-0":"margin-8px-0"}
    /// static + dynamic ex) margin={test?a:"8px"} --> className={test?"margin-0":"margin-8px-0"} style={{ "--margin-0": a }}
    /// dynamic + dynamic ex) margin={test?a:b} --> className="margin-0" style={{ "--margin-0": test?a:b }}
    /// issue case: dynamic + dynamic ex) margin={test?a:(b ? "8px": c)} --> className="margin-0" style={{ "--margin-0": test?a:(b ? "8px": c) }}
    Conditional {
        condition: Expression<'a>,
        consequent: Option<Box<ExtractStyleProp<'a>>>,
        alternate: Option<Box<ExtractStyleProp<'a>>>,
    },
    Expression {
        styles: Vec<ExtractStyleValue>,
        expression: Expression<'a>,
    },
    MemberExpression {
        map: BTreeMap<String, Box<ExtractStyleProp<'a>>>,
        expression: Expression<'a>,
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
            ExtractStyleProp::StaticArray(ref array) => {
                array.iter().flat_map(|s| s.extract()).collect()
            }
            ExtractStyleProp::Expression { styles, .. } => styles.to_vec(),
            ExtractStyleProp::MemberExpression { ref map, .. } => {
                map.values().flat_map(|s| s.extract()).collect()
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
    let source_type = SourceType::from_path(filename)?;
    if !code.contains(option.package.as_str()) {
        // skip if not using package
        return Ok(ExtractOutput {
            styles: vec![],
            code: code.to_string(),
        });
    }
    let allocator = Allocator::default();

    let ParserReturn {
        mut program, // AST
        panicked,    // Parser encountered an error it couldn't recover from
        ..
    } = Parser::new(&allocator, code, source_type).parse();
    if panicked {
        return Err("Parser panicked".into());
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
    use css::reset_class_map;
    use insta::assert_debug_snapshot;
    use serial_test::serial;

    #[test]
    #[serial]
    fn extract_just_tsx() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            "const a = 1;",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            },
        )
        .unwrap());

        reset_class_map();
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
    #[serial]
    fn ignore_special_props() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
        <Box padding={1} ref={ref} data-test={1} role={2} children={[]} onClick={()=>{}} aria-valuenow={24} key={2} tabIndex={1} id="id" />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Input} from '@devup-ui/core'
        <Input placeholder="a" maxLength="b" minLength="c" />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn convert_tag() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
        <Box as="secton" />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
        <Box as={"secton"} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
        <Box as={`secton`} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
        // assert_debug_snapshot!(extract(
        //     "test.tsx",
        //     r#"import {Box} from '@devup-ui/core'
        // <Box as={b ? "div":"secton"} />
        // "#,
        //     ExtractOption {
        //         package: "@devup-ui/core".to_string(),
        //         css_file: None
        //     }
        // )
        // .unwrap());
        // assert_debug_snapshot!(extract(
        //     "test.tsx",
        //     r#"import {Box} from '@devup-ui/core'
        // <Box as={b ? undefined:"secton"} />
        // "#,
        //     ExtractOption {
        //         package: "@devup-ui/core".to_string(),
        //         css_file: None
        //     }
        // )
        // .unwrap());
        //
        // assert_debug_snapshot!(extract(
        //     "test.tsx",
        //     r#"import {Box} from '@devup-ui/core'
        // <Box as={b ? null:"secton"} />
        // "#,
        //     ExtractOption {
        //         package: "@devup-ui/core".to_string(),
        //         css_file: None
        //     }
        // )
        // .unwrap());
        // assert_debug_snapshot!(extract(
        //     "test.tsx",
        //     r#"import {Box} from '@devup-ui/core'
        // <Box as={b ? null:undefined} />
        // "#,
        //     ExtractOption {
        //         package: "@devup-ui/core".to_string(),
        //         css_file: None
        //     }
        // )
        // .unwrap());
    }
    #[test]
    #[serial]
    fn extract_style_props() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r"import {Box} from '@devup-ui/core'
        <Box padding={1} margin={2} wrong={} wrong2=<></> />
        ",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
        reset_class_map();
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
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r"import {Input} from '@devup-ui/core'
        <Input padding={1} margin={2} />
        ",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r"import {Button} from '@devup-ui/core'
        <Button padding={1} margin={2} />
        ",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r"import {Flex} from '@devup-ui/core'
        <Flex padding={1} margin={2} />
        ",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn extract_style_props_with_class_name() {
        reset_class_map();
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

        reset_class_map();
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

        reset_class_map();
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

        reset_class_map();
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
        reset_class_map();
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

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Image} from '@devup-ui/core'
        <Image
          className={styles.logo}
          src="/next.svg"
          alt="Next.js logo"
          width={180}
          height={38}
        />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn extract_class_name_from_component() {
        reset_class_map();
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
        .unwrap());
    }
    #[test]
    #[serial]
    fn extract_responsive_style_props() {
        reset_class_map();
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
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Flex } from "@devup-ui/core";
<Flex display={['none', null, "flex"]}/>;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn extract_dynamic_style_props() {
        reset_class_map();
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
    #[serial]
    fn extract_dynamic_responsive_style_props() {
        reset_class_map();
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
    #[serial]
    fn extract_compound_responsive_style_props() {
        reset_class_map();
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
    #[serial]
    fn extract_wrong_responsive_style_props() {
        reset_class_map();
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
    #[serial]
    fn extract_variable_style_props_with_style() {
        reset_class_map();
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

        reset_class_map();
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
    #[serial]
    fn extract_conditional_style_props() {
        reset_class_map();
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

        reset_class_map();
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

        reset_class_map();
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

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? null : undefined} />;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn extract_responsive_conditional_style_props() {
        reset_class_map();
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

        reset_class_map();
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

        reset_class_map();
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

        reset_class_map();
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

        reset_class_map();
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

        reset_class_map();
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
        reset_class_map();
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

        reset_class_map();
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
    #[serial]
    fn extract_logical_case() {
        reset_class_map();
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

        reset_class_map();
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

        reset_class_map();
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
    #[serial]
    fn extract_responsive_conditional_style_props_with_class_name() {
        reset_class_map();
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

        reset_class_map();
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
    #[serial]
    fn extract_selector() {
        reset_class_map();
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
    #[serial]
    fn extract_selector_with_responsive() {
        reset_class_map();
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

        reset_class_map();
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
    #[serial]
    fn extract_static_css_class_name_props() {
        reset_class_map();
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

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { css as c } from "@devup-ui/core";
<Box className={c`
  background-color: red;
`}/>;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { css } from "@devup-ui/core";
<Box className={css({
  bg:"red",
  color:"blue"
})}/>;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { css as c } from "@devup-ui/core";
<Box className={c({
  bg:"red"
})}/>;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { css } from "@devup-ui/core";
<Box className={css({
  _hover: {
    bg:"red",
    color:"blue"
  }
})}/>;
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import { css } from "@devup-ui/core";
        <div className={css(a?{bg:"red"}:{bg:"blue"})}/>;
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn extract_static_css_with_theme() {
        reset_class_map();
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

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
        <Box color={`$nice`} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
        <Box color={("$nice")} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn apply_typography() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Text} from '@devup-ui/core'
        <Text typography="bold" />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Text} from '@devup-ui/core'
        <Text typography={`bold`} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Text} from '@devup-ui/core'
        <Text typography={a ? "bold" : "bold2"} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }
    #[test]
    #[serial]
    fn apply_var_typography() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Text} from '@devup-ui/core'
        <Text typography={variable} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Text} from '@devup-ui/core'
        <Text typography={bo ? a : b} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {Text} from '@devup-ui/core'
        <Text typography={`${bo ? a : b}`} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn raise_error() {
        reset_class_map();
        assert!(extract(
            "test.wrong",
            "const a = 1;",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            },
        )
        .unwrap_err()
        .to_string()
        .starts_with("Unknown file extension"));

        reset_class_map();
        assert_eq!(
            extract(
                "test.tsx",
                "import {} '@devup-ui/core';\na a = 1;",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_file: None
                },
            )
            .unwrap_err()
            .to_string(),
            "Parser panicked"
        );
    }

    #[test]
    #[serial]
    fn import_wrong_component() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.tsx",
            r#"import {W} from '@devup-ui/core'
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn support_transpile_mjs() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.mjs",
            r#"import { jsxs as r, jsx as e } from "react/jsx-runtime";
import { Box as o, Text as t, Flex as i } from "@devup-ui/react";
function c() {
  return /* @__PURE__ */ r("div", { children: [
    /* @__PURE__ */ e(
      o,
      {
        _hover: {
          bg: "blue"
        },
        bg: "$text",
        color: "red",
        children: "hello"
      }
    ),
    /* @__PURE__ */ e(t, { typography: "header", children: "typo" }),
    /* @__PURE__ */ e(i, { as: "section", mt: 2, children: "section" })
  ] });
}
export {
  c as Lib
};"#,
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import { jsxs as r, jsx as e } from "react/jsx-runtime";
import { Box as o, Text as t, Flex as i } from "@devup-ui/react";
function c() {
  return /* @__PURE__ */ r("div", { children: [
    /* @__PURE__ */ e(
      o,
      {
        _hover: {
          bg: "blue"
        },
        bg: "$text",
        color: "red",
        children: "hello"
      }
    ),
    /* @__PURE__ */ e(t, { typography: "header", children: "typo" }),
    /* @__PURE__ */ e(i, { as: "section", mt: 2, children: "section" })
  ] });
}
export {
  c as Lib
};"#,
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn support_transpile_cjs() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.cjs",
            r#""use strict";Object.defineProperty(exports,Symbol.toStringTag,{value:"Module"});const e=require("react/jsx-runtime"),r=require("@devup-ui/react");function t(){return e.jsxs("div",{children:[e.jsx(r.Box,{_hover:{bg:"blue"},bg:"$text",color:"red",children:"hello"}),e.jsx(r.Text,{typography:"header",children:"typo"}),e.jsx(r.Flex,{as:"section",mt:2,children:"section"})]})}exports.Lib=t;"#,
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.cjs",
            r#""use strict";Object.defineProperty(exports,Symbol.toStringTag,{value:"Module"});const {jsx:e1, jsxs:e2}=require("react/jsx-runtime"),r=require("@devup-ui/react");function t(){return e2("div",{children:[e1(r.Box,{_hover:{bg:"blue"},bg:"$text",color:"red",children:"hello"}),e1(r.Text,{typography:"header",children:"typo"}),e1(r.Flex,{as:"section",mt:2,children:"section"})]})}exports.Lib=t;"#,
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#""use strict";Object.defineProperty(exports,Symbol.toStringTag,{value:"Module"});const e=require("react/jsx-runtime"),r=require("@devup-ui/react");function t(){return e.jsxs("div",{children:[e.jsx(r.Box,{_hover:{bg:"blue"},bg:"$text",color:"red",children:"hello"}),e.jsx(r.Text,{typography:"header",children:"typo"}),e.jsx(r.Flex,{as:"section",mt:2,children:"section"})]})}exports.Lib=t;"#,
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn maintain_value() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={1} zIndex={2} fontWeight={900} scale={2} flex={1} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn ternary_operator_in_selector() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex _hover={a ? { bg: "red" } : undefined} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex _hover={a ? { bg: "red" } : {}} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex _hover={a ? { bg: "red",color:"blue" } : { fontWeight:"bold", color:"red" }} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn test_rest_props() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={0.5} {...props} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn props_wrong_direct_array_select() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={[][0]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][-10]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][+10]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][100]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn props_wrong_direct_object_select() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Box} from '@devup-ui/core'
        <Box opacity={{}[1]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5}["wrong"]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5}[`wrong`]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5}[1]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn props_direct_array_select() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][0]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][a]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex bg={["$red", "$blue"][idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex bg={[`$red`, `${variable}`][idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Center} from '@devup-ui/core'
<Center
            bg={['$webBg', '$appBg', '$solutionBg'][categoryId - 1]}
          >
          </Center>
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5, ...some][100]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5, ...some][a]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn props_direct_object_select() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5}["a"]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5, ...any}["b"]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5, ...any}["some"]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex bg={{a:"$red", b:"$blue"}[idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn props_direct_variable_object_select() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5}[a]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Box} from '@devup-ui/core'
<Box bg={SOME_VAR[idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn props_direct_object_responsive_select() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
;<Flex gap={{ 0: [1, 2, 3], 1: [4, 5, 6] }[idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
;<Flex gap={{ "a": [1, 2, 3], "b": [4, 5, 6] }[idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }
    #[test]
    #[serial]
    fn props_direct_variable_object_responsive_select() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
;<Flex gap={{ 0: [a, b, c], "1": [d, e, f] }[idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn props_direct_array_responsive_select() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
;<Flex gap={[[1, 2, 3], [4, 5, 6]][idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
;<Flex gap={[[1, 2, 3],[4, 5, 6]][idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }
    #[test]
    #[serial]
    fn props_direct_variable_array_responsive_select() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
;<Flex gap={[[a, b, c], [d, e, f]][idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn props_direct_hybrid_responsive_select() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
;<Flex gap={[[a, 1, c], [d, e, 2]][idx]} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }
    #[test]
    #[serial]
    fn test_component_in_func() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Flex} from '@devup-ui/core'
PROCESS_DATA.map(({ id, title, content }, idx) => (
          <MotionDiv key={idx}>
            <Flex alignItems="center" gap={[3, null, 5, null, 10]}>
            </Flex>
          </MotionDiv>
        ))
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn backtick_prop() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Box} from '@devup-ui/core'
            <Box bg={`black`} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Box} from '@devup-ui/core'
            <Box bg={`${variable}`} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn group_selector_props() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Box} from '@devup-ui/core'
            <Box _groupHover={{ bg: "red" }} />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn test_duplicate_style_props() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Box} from '@devup-ui/core'
            <Box bg="red" background="red" />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn avoid_same_name_component() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Box} from '@devup-ui/core'
import {Button} from '@devup/ui'
            ;<Box bg="red" background="red" />
            ;<Button bg="red" background="red" />
        "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn css_props_destructuring_assignment() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {css} from '@devup-ui/core'
    <div className={css({
       ...(a ? { bg: 'red' } : { bg: 'blue' }),
       ...({ p: 1 }),
     })} />
            "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());

        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {css} from '@devup-ui/core'
    <div className={css({
       ...(a ? { bg: 'red', border: "solid 1px red" } : { bg: 'blue' }),
       ...({ p: 1,m: 1 }),
     })} />
            "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }

    #[test]
    #[serial]
    fn theme_props() {
        reset_class_map();
        assert_debug_snapshot!(extract(
            "test.js",
            r#"import {Box} from '@devup-ui/core'
    <Box _themeDark={{ display:"none" }} _themeLight={{ display: "flex" }} />
            "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_file: None
            }
        )
        .unwrap());
    }
}
