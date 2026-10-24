use super::*;
use rstest::rstest;

#[rstest]
#[case("__proto__", "false|red")]
#[case("['__proto__']", "true|red")]
fn lexical_copy_when_proto_is_a_setter_does_not_invent_an_own_property(
    #[case] key: &str,
    #[case] expected: &str,
) {
    let allocator = Allocator::default();
    let source = format!(
        "import {{css}} from '@devup-ui/react'; function run(){{const props={{{key}:null,color:'red'}};return css({{...props}});}} run();"
    );
    let mut parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);

    inline_constants(
        &AstBuilder::new(&allocator),
        &mut parsed.program,
        "proto.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &crate::import_alias_visit::PxMarks::default(),
    );

    parsed
        .program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = oxc_codegen::Codegen::new().build(&parsed.program).code;
    let script = format!(
        "const css = props => [Object.hasOwn(props, '__proto__'), props.color].join('|'); {code}"
    );
    let mut context = boa_engine::Context::default();
    let actual = context
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    assert_eq!(
        actual
            .as_string()
            .unwrap_or_else(|| panic!("string required: {actual:?}"))
            .to_std_string_escaped(),
        expected,
        "{code}"
    );
}

#[rstest]
#[case("tone; const tone='red';", None)]
#[case("const tone='red'; tone;", Some("\"red\""))]
fn inline_read_when_the_binding_initializes_on_either_side_of_it_is_safe(
    #[case] source: &str,
    #[case] expected: Option<&str>,
) {
    let allocator = Allocator::default();
    let ast_builder = AstBuilder::new(&allocator);
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let initialization = initialization::Initialization::new(&parsed.program, &scoping);
    let style = StyleSymbols::new(&scoping);
    let css_props = CssTakers::new(&parsed.program, &scoping, CssProp::Off, "@devup-ui/react");
    let symbol = scoping
        .get_root_binding("tone".into())
        .unwrap_or_else(|| panic!("tone binding"));
    let symbols = FxHashMap::from_iter([(symbol, Constant::String("red".to_string()))]);
    let inline = Inline {
        ast_builder: &ast_builder,
        scoping: &scoping,
        initialization: &initialization,
        symbols: &symbols,
        style: &style,
        css_props: &css_props,
        css_failures: &mut Vec::new(),
        unknown: &mut Unknown::default(),
        objects: false,
        styles: false,
        px: false,
        position: InlinePosition::Rules,
        marks: &crate::import_alias_visit::PxMarks::default(),
        class_names: Vec::new(),
    };
    let expression = parsed
        .program
        .body
        .iter()
        .find_map(|statement| match statement {
            Statement::ExpressionStatement(statement) => Some(&statement.expression),
            _ => None,
        })
        .unwrap_or_else(|| panic!("tone read"));
    assert_eq!(
        inline
            .constant(expression)
            .and_then(|value| value.js_literal())
            .as_deref(),
        expected
    );
}

#[rstest]
#[case(true)]
#[case(false)]
fn namespace_object_when_a_nested_member_is_a_function_is_recognized(#[case] function: bool) {
    let leaf = if function {
        Constant::Function
    } else {
        Constant::Number(2.0)
    };
    let object = Constant::Object(Rc::new(FxHashMap::from_iter([(
        "member".to_string(),
        leaf,
    )])));
    assert_eq!(object.has_function(), function);
}

#[rstest]
#[case(vec![])]
#[case(vec![Constant::Number(2.0)])]
#[case(vec![Constant::String("2".to_string()), Constant::Number(3.0)])]
#[case(vec![Constant::Number(2.0), Constant::String("3".to_string())])]
fn exact_power_when_operands_are_not_two_numbers_does_not_fold(#[case] arguments: Vec<Constant>) {
    assert!(fold_math("pow", &arguments).is_none());
}

#[test]
#[serial_test::serial]
fn nested_math_constant_when_read_by_computed_key_keeps_its_exact_value() {
    let output = super::exact_tests::extracted("import {Box} from '@devup-ui/react'; export function a(){const n=Math['PI'];return <Box zIndex={n}/>;}", "")
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        super::exact_tests::static_values(&output),
        vec!["3.141592653589793".to_string()]
    );
}
