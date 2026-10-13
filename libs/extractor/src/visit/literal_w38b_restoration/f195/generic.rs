use super::*;
use oxc_ast_visit::{Visit, walk};

#[derive(Clone, Copy)]
enum Emission {
    Generator,
    Merger,
}

impl Emission {
    fn emit<'a>(self, ast: &AstBuilder<'a>, value: &Expression<'a>) -> Expression<'a> {
        match self {
            Self::Generator => {
                let mut props: Vec<ExtractStyleProp<'a>> = vec![ExtractStyleProp::Expression {
                    styles: vec![],
                    expression: value.clone_in(ast.allocator()),
                }];
                gen_class_names(ast, &mut props, None, None)
                    .unwrap_or_else(|| panic!("generic singleton"))
            }
            Self::Merger => merge_expression_for_class_name(ast, [value.clone_in(ast.allocator())])
                .unwrap_or_else(|| panic!("arbitrary singleton")),
        }
    }
}

#[derive(Default)]
struct Literals(Vec<String>);

impl<'a> Visit<'a> for Literals {
    fn visit_expression(&mut self, value: &Expression<'a>) {
        match value {
            Expression::StringLiteral(value) => self.0.push(format!("{value:?}")),
            Expression::NumericLiteral(value) => self.0.push(format!("{value:?}")),
            Expression::TemplateLiteral(value) => self.0.push(format!("{:?}", value.quasis)),
            _ => {}
        }
        walk::walk_expression(self, value);
    }
}

#[rstest]
#[case("external;", ("identifier", r#"["identifier",[]]"#))]
#[case("take();", ("call", r#"["call",["call"]]"#))]
#[case("external || take();", ("logical", r#"["identifier",[]]"#))]
#[case("state[key];", ("member", r#"["member",["key","member"]]"#))]
#[case("'binary-a' + '-b';", ("other", r#"["binary-a-b",[]]"#))]
#[case("({color:'blue'});", ("other", r#"[{"color":"blue"},[]]"#))]
#[case("['array-a','array-b'];", ("other", r#"[["array-a","array-b"],[]]"#))]
#[case("(trace.push('sequence'),'sequence');", ("other", r#"["sequence",["sequence"]]"#))]
#[case("`raw\\n${external}`;", ("template", r#"["raw\nidentifier",[]]"#))]
#[serial]
fn generic_singleton_when_payload_is_arbitrary_preserves_source_contract(
    #[case] source: &str,
    #[case] contract: (&str, &str),
    #[values(Emission::Generator, Emission::Merger)] emission: Emission,
) {
    // Given: real parsed/semantic payloads, never asserted as local ClassNames outputs.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let payload = parsed(&allocator, source);
    let span = payload.span();
    let code = readable_code(&payload);
    let mut literals = Literals::default();
    literals.visit_expression(&payload);
    // When: the unchanged default carrier or merger emits one arbitrary payload.
    let actual = emission.emit(&ast, &payload);
    let raw_code = readable_code(&actual);
    println!(
        "W38H_F195_RAW {}",
        serde_json::json!({
            "schema": 1,
            "kind": "singleton",
            "source": source,
            "mode": match emission {
                Emission::Generator => "generator",
                Emission::Merger => "merger",
            },
            "code": raw_code,
            "code_utf8_bytes": raw_code.len(),
            "span": [actual.span().start, actual.span().end],
            "stylesheet": null,
        })
    );
    // Then: root, outer span, code, descendant literal metadata and value/trace agree.
    assert_eq!(root(&actual), contract.0);
    assert_eq!(actual.span(), span);
    assert_eq!(readable_code(&actual), code);
    let mut emitted_literals = Literals::default();
    emitted_literals.visit_expression(&actual);
    assert_eq!(emitted_literals.0, literals.0);
    let setup = "const external='identifier';function take(){trace.push('call');return 'call';}const key={toString(){trace.push('key');return 'className';}};const state={get className(){trace.push('member');return 'member';}}";
    let expected: serde_json::Value =
        serde_json::from_str(contract.1).unwrap_or_else(|error| panic!("fixture JSON: {error}"));
    assert_eq!(json_evaluated(&actual, setup), expected);
}

#[test]
#[serial]
fn generic_generator_when_payload_list_is_empty_preserves_default_inference() {
    // Given: the original bare carrier annotation and empty call site.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut props: Vec<ExtractStyleProp<'_>> = vec![];
    // When: default generation receives no declarations.
    let actual = gen_class_names(&ast, &mut props, None, None);
    let raw_code = actual.as_ref().map(readable_code);
    println!(
        "W38H_F195_RAW {}",
        serde_json::json!({
            "schema": 1,
            "kind": "empty",
            "source": "",
            "mode": "generator",
            "code": raw_code,
            "code_utf8_bytes": raw_code.as_ref().map(String::len),
            "stylesheet": null,
        })
    );
    // Then: there is no class expression, rather than an invented empty payload.
    assert!(actual.is_none());
}
