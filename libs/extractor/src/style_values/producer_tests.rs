use crate::{
    ExtractStyleValue, extract_style::extract_static_style::ExtractStaticStyle,
    extract_style::style_property::StyleProperty, vanilla_extract::producer_atoms::ProducerAtoms,
    visit::DevupVisitor,
};
use oxc_allocator::Allocator;
use oxc_ast_visit::VisitMut;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use rustc_hash::FxHashSet;
use serial_test::serial;

#[rstest]
#[case("CLASS || 'unused'", true, "anchor")]
#[case("flag ? CLASS : 'external'", true, "anchor")]
#[case("flag ? CLASS : 'external'", false, "external")]
#[case("flag ? 'external' : CLASS", false, "anchor")]
#[case("flag && CLASS", true, "anchor")]
#[serial]
fn residual_classes_survive_when_producer_literals_enter_conditional_composition(
    #[case] expression: &str,
    #[case] flag: bool,
    #[case] residual: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    css::class_map::reset_class_map();
    let red = ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None));
    let Some(StyleProperty::ClassName(class)) = red.extract(None) else {
        return Err("static declaration has no class".into());
    };
    let atoms = ProducerAtoms::from_styles(&FxHashSet::from_iter([red]), None);
    let expression = expression.replace("CLASS", &format!("'{class} anchor'"));
    let code = format!(
        "import {{css}} from '@devup-ui/react'; export const button=css({expression},{{color:'blue'}});"
    );
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, &code, SourceType::ts())
        .parse()
        .program;
    let mut visitor = DevupVisitor::new(
        &allocator,
        "/literal.css.ts",
        "@devup-ui/react",
        vec![],
        None,
    );
    visitor.import_producer_atoms(atoms);
    visitor.visit_program(&mut program);
    assert_eq!(visitor.errors.len(), 0);
    assert!(
        visitor
            .styles
            .iter()
            .all(|value| matches!(value,ExtractStyleValue::Static(style) if style.value()=="blue"))
    );
    let compiled = oxc_codegen::Codegen::new()
        .build(&program)
        .code
        .replace("export const", "const");
    let mut context = boa_engine::Context::default();
    let result = context
        .eval(boa_engine::Source::from_bytes(
            format!("const flag={flag}; {compiled}\nbutton").as_bytes(),
        ))
        .map_err(|error| error.to_string())?;
    let classes = result
        .as_string()
        .ok_or("missing class string")?
        .to_std_string_escaped();
    assert!(
        classes.split_whitespace().any(|token| token == residual),
        "{classes}"
    );
    assert!(
        !classes.split_whitespace().any(|token| token == class),
        "{classes}"
    );
    Ok(())
}
