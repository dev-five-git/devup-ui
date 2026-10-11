use crate::{
    ExtractOption, ExtractOutput, ExtractStyleValue, ResolvedModule, extract_with_modules,
};
use css::file_map::reset_file_map;
use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

pub(super) fn exported(
    output: &ExtractOutput,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::ts()).parse();
    for statement in &parsed.program.body {
        if let Statement::ExportDeclaration(export) = statement
            && let Declaration::VariableDeclaration(declaration) = &export.declaration
        {
            for binding in &declaration.declarations {
                if binding
                    .id
                    .get_binding_identifier()
                    .is_some_and(|id| id.name == name)
                    && let Some(value) = &binding.init
                {
                    if let Some(class) = literal_class(value) {
                        return Ok(class);
                    }
                    let code = output
                        .code
                        .lines()
                        .filter(|line| !line.starts_with("import "))
                        .collect::<Vec<_>>()
                        .join("\n")
                        .replace("export const", "const");
                    let mut context = boa_engine::Context::default();
                    let value = context
                        .eval(boa_engine::Source::from_bytes(
                            format!("{code}\n{name}").as_bytes(),
                        ))
                        .map_err(|error| error.to_string())?;
                    return value
                        .as_string()
                        .map(|value| value.to_std_string_escaped())
                        .ok_or_else(|| format!("nonstring export {name}").into());
                }
            }
        }
    }
    Err(format!("missing literal export {name}: {}", output.code).into())
}

fn literal_class(expression: &Expression<'_>) -> Option<String> {
    match expression {
        Expression::StringLiteral(value) => Some(value.value.to_string()),
        Expression::BinaryExpression(binary)
            if binary.operator == oxc_syntax::operator::BinaryOperator::Addition =>
        {
            Some(literal_class(&binary.left)? + &literal_class(&binary.right)?)
        }
        _ => None,
    }
}

fn extract_case(
    operands: &str,
    extension: &str,
) -> Result<ExtractOutput, Box<dyn std::error::Error>> {
    let path = format!("/producer.css.{extension}");
    let resolver = move |specifier: &str, _: &str| {
        match specifier {
        "./producer.css" => Some(ResolvedModule {
            path: path.clone(),
            code: "import {style,createVar} from '@devup-ui/react'; export const space=createVar('space'); export const base=style({color:'red',padding:8}); export const nested={base};".into(),
        }),
        "./forward.css" => Some(ResolvedModule {
            path: "/forward.css.ts".into(),
            code: "import {nested,space} from './producer.css'; export const forward={nested}; export {space};".into(),
        }),
        "./barrel" => Some(ResolvedModule {
            path: "/barrel.ts".into(),
            code: "export * as all from './forward.css';".into(),
        }),
        _ => None,
    }
    };
    extract_with_modules(
        "/consumer.css.ts",
        &format!(
            "import {{style}} from '@devup-ui/react'; import * as p from './producer.css'; import {{forward,space}} from './forward.css'; import * as barrel from './barrel'; export const button=style({operands});"
        ),
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )
}

pub(super) fn active_values(output: &ExtractOutput, classes: &str) -> Vec<(String, String)> {
    let mut values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|value| {
            let ExtractStyleValue::Static(style) = value else {
                return None;
            };
            let Some(crate::extract_style::style_property::StyleProperty::ClassName(class)) =
                value.extract(None)
            else {
                return None;
            };
            classes
                .split_whitespace()
                .any(|token| token == class)
                .then(|| (style.property().to_string(), style.value().to_string()))
        })
        .collect();
    values.sort();
    values
}

#[rstest]
#[case("ts")]
#[case("js")]
#[serial]
fn imported_atoms_are_overridden_when_later_rules_match(
    #[case] extension: &str,
    #[values("forward.nested['base']", "barrel.all.forward.nested.base")] base: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    reset_file_map();
    let output = extract_case(
        &format!("[{base},{{color:'blue',margin:space}}]"),
        extension,
    )?;
    let classes = exported(&output, "button")?;
    assert_eq!(
        active_values(&output, &classes),
        vec![
            ("color".into(), "blue".into()),
            ("margin".into(), "var(--space-0-0)".into()),
            ("padding".into(), "8px".into())
        ]
    );
    let red = ExtractStyleValue::Static(
        crate::extract_style::extract_static_style::ExtractStaticStyle::new(
            "color", "red", 0, None,
        ),
    );
    if let Some(crate::extract_style::style_property::StyleProperty::ClassName(red)) =
        red.extract(None)
    {
        assert!(!classes.split_whitespace().any(|class| class == red));
    }
    assert!(output.code.contains("import \"./forward.css\""));
    assert!(
        output
            .dependencies
            .contains(&format!("/producer.css.{extension}"))
    );
    Ok(())
}

#[rstest]
#[case("[{color:'blue'},p.base]", "red")]
#[case("[p.base,{color:'blue'},p.base]", "red")]
#[case("[p.base,{color:'blue'},'external']", "blue")]
#[serial]
fn imported_order_is_preserved_when_operands_repeat(
    #[case] operands: &str,
    #[case] color: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    reset_file_map();
    let output = extract_case(operands, "ts")?;
    let classes = exported(&output, "button")?;
    assert_eq!(
        active_values(&output, &classes),
        vec![
            ("color".into(), color.into()),
            ("padding".into(), "8px".into())
        ]
    );
    assert_eq!(
        classes.split_whitespace().any(|class| class == "external"),
        operands.contains("external")
    );
    Ok(())
}

#[rstest]
#[case("[{color:'red'},base]", "blue")]
#[case("[base,{color:'red'}]", "red")]
#[case("[base,[{color:'red'},base]]", "blue")]
#[serial]
fn local_bases_expand_in_place_when_inline_rules_precede_them(
    #[case] operands: &str,
    #[case] color: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    reset_file_map();
    let resolver = |_: &str, _: &str| None;
    let output = extract_with_modules(
        "/local.css.ts",
        &format!(
            "import {{style}} from '@devup-ui/react'; const base=style({{color:'blue'}}); export const button=style({operands});"
        ),
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    let classes = exported(&output, "button")?;
    assert_eq!(
        active_values(&output, &classes),
        vec![("color".into(), color.into())]
    );
    Ok(())
}
