use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, Statement, TSType, TSTypeName, TSTypeQueryExprName};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

use super::{
    Scope, ValueType, annotation, classify,
    graph::Graph,
    model::{Model, Node, Scalar, Shape},
    syntax,
};

#[test]
fn receiver_type_name_is_unproven_when_taken_from_a_parsed_type_query() {
    // Given: an actual parser-produced receiver name, not a fabricated type
    // reference to `this`. Type queries are not resolved by the source prover.
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, "type Receiver = typeof this;", SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{:?}", parsed.diagnostics);
    let Statement::TSTypeAliasDeclaration(alias) = &parsed.program.body[0] else {
        panic!("type alias fixture")
    };
    let TSType::TSTypeQuery(query) = &alias.type_annotation else {
        panic!("receiver query fixture")
    };
    let TSTypeQueryExprName::ThisExpression(receiver) = &query.expr_name else {
        panic!("parser-produced receiver fixture")
    };
    let name = TSTypeName::ThisExpression(receiver.clone_in(&allocator));
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let model = Model::default();
    let mut graph = Graph::new(None, "@devup-ui/react");

    // When: the type-name API resolves that valid receiver-name variant.
    let proof = graph.resolve(&model, "/receiver.ts", &annotation::name(&name, &scoping));

    // Then: it supplies no scalar/object contract inferred from `this` spelling.
    assert_eq!(proof, Shape::UNKNOWN);
}

struct SelectedProofs(Vec<ValueType>);

impl<'a> Visit<'a> for SelectedProofs {
    fn visit_expression(&mut self, value: &Expression<'a>) {
        if let Expression::CallExpression(call) = value
            && matches!(&call.callee, Expression::Identifier(name) if name.name == "useValue")
        {
            self.0.extend(
                call.arguments
                    .iter()
                    .filter_map(|argument| argument.as_expression().map(classify)),
            );
        }
        walk::walk_expression(self, value);
    }
}

#[rstest]
#[case("{ size: number } & number", ValueType::Unproven)]
#[case("number & { size: number }", ValueType::Unproven)]
#[case("{ size: number } & unknown", ValueType::Unproven)]
#[case("{ size: number } & { label: string }", ValueType::Number)]
fn intersection_member_proof_is_conservative_when_a_constituent_is_not_an_object(
    #[case] intersection: &str,
    #[case] expected: ValueType,
) {
    // Given: supported TS intersections, with a known field before or after
    // a nonobject constituent, plus an all-object control.
    let source = format!(
        "type Value = {intersection}; function render(value: Value) {{ useValue(value.size); }}"
    );
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{:?}", parsed.diagnostics);

    // When: source-only collection and resolution inspect the selected field.
    let (_scope, _) = Scope::enter(&parsed.program, "/intersection.ts", None, "@devup-ui/react");
    let mut proofs = SelectedProofs(Vec::new());
    proofs.visit_program(&parsed.program);

    // Then: partial fields do not survive a nonobject merge, in either order;
    // an entirely object-shaped intersection still proves its numeric field.
    assert_eq!(proofs.0, vec![expected]);
}

#[rstest]
#[case(r"tag`\unicodepx`")]
#[case(r"tag`${read()}\unicodepx`")]
fn template_proof_is_general_string_when_a_valid_tagged_template_has_no_cooked_tail(
    #[case] source: &str,
) {
    // Given: invalid escapes are legal in tagged templates; Oxc leaves the
    // cooked tail absent. The raw `px` suffix cannot prove a cooked CSS unit.
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{:?}", parsed.diagnostics);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("tagged expression fixture")
    };
    let Expression::TaggedTemplateExpression(tagged) = &statement.expression else {
        panic!("tagged template fixture")
    };
    assert!(
        tagged
            .quasi
            .quasis
            .last()
            .is_some_and(|tail| { tail.value.cooked.is_none() && tail.value.raw.ends_with("px") })
    );
    let model = Model::default();
    let mut graph = Graph::new(None, "@devup-ui/react");

    // When: the template API classifies the parser-produced quasis, without
    // pretending the enclosing tag's result has a proven return contract.
    let node = Node::Scalar(syntax::template(
        &tagged.quasi.quasis,
        !tagged.quasi.expressions.is_empty(),
    ));
    let proof = graph.resolve(&model, "/tagged.ts", &node);

    // Then: neither a raw CSS-unit suffix nor a missing cooked value proves
    // NonNumericString or Unproven: the template seam retains general String.
    assert_eq!(proof, Shape::Scalar(Scalar::String));
}
