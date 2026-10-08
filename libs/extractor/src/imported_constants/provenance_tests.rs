use super::provenance::Proof;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("const value=1;", vec![], (true,true,true))]
#[case("const value=null;", vec![None], (true,true,true))]
#[case("const value=true;", vec![Some("x")], (true,true,true))]
#[case("const value='x';", vec![], (true,true,true))]
#[case("const value={ p: 1 };", vec![Some("p")], (true,true,true))]
#[case("const value={p:{q:1}};", vec![Some("p"),Some("q")], (true,true,false))]
#[case("const value={ p: 1 };", vec![Some("missing")], (true,true,true))]
#[case("const value=[1];", vec![Some("0")], (true,true,true))]
#[case("const value=[1];", vec![Some("99")], (true,true,true))]
#[case("const value=[1];", vec![Some("length")], (true,true,true))]
#[case("const value=[1];", vec![Some("custom")], (false,true,true))]
#[case("const value=[{ p: 1 }];", vec![None], (false,true,false))]
#[case("const value=()=>1;", vec![None], (false,false,false))]
#[case("const value=function(){};", vec![Some("p")], (false,false,false))]
#[case("const value=unknown;", vec![], (false,false,false))]
#[case("const value=unknown;", vec![None], (false,false,false))]
#[case("const value=unknown;", vec![Some("p")], (false,false,false))]
#[case("const value={get p(){return 1}};", vec![], (false,false,false))]
#[case("const value={1n:1};", vec![Some("1")], (true,true,true))]
#[case("const value={ valueOf: 1 };", vec![], (false,false,false))]
#[case("const value={...unknown};", vec![], (false,false,false))]
#[case("const value=[...unknown];", vec![], (false,false,false))]
#[case("const value={...{ p: 1 }};", vec![None], (true,true,true))]
#[case("const value=[,...[1]];", vec![None], (true,true,true))]
#[case("const value=[1].length;", vec![], (true,true,true))]
#[case("const value=[1]['0'];", vec![], (true,true,true))]
#[case("const value=[1].custom;", vec![], (false,false,false))]
#[case("const value={ p: 1 }.p;", vec![], (true,true,true))]
#[case("const value={ p: 1 }.missing;", vec![], (true,true,true))]
#[case("const value={ p: 1 }[key];", vec![], (false,false,false))]
#[case("const value=(1).x;", vec![], (false,false,false))]
#[case("const value=Object.freeze();", vec![], (false,false,false))]
#[case("const make=()=>({ p: 1 });const value=make(1);", vec![], (false,false,false))]
#[case("const make=()=>make();const value=make();", vec![], (false,false,false))]
#[case("const value=unknown();", vec![], (false,false,false))]
#[case("const value=unknown.make();", vec![], (false,false,false))]
#[case("const make=1;const value=make();", vec![], (false,false,false))]
#[case("let make;const value=make();", vec![], (false,false,false))]
#[case("const make=()=>{};const value=make();", vec![], (false,false,false))]
#[case("const make=function(){1;};const value=make();", vec![], (false,false,false))]
#[case("function make(x){return 1}const value=make();", vec![], (false,false,false))]
#[case("const make=async()=>1;const value=make();", vec![], (false,false,false))]
#[case("const value=next;const next=value;", vec![], (false,false,false))]
#[case("class value{}", vec![], (false,false,false))]
#[case("function value(){}", vec![], (false,false,false))]
fn syntactic_proof_when_values_have_known_or_uncertain_provenance(
    #[case] source: &str,
    #[case] path: Vec<Option<&str>>,
    #[case] expected: (bool, bool, bool),
) {
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, source, SourceType::ts())
        .parse()
        .program;
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&program)
        .semantic;
    let proof = Proof {
        nodes: semantic.nodes(),
        scoping: semantic.scoping(),
    };
    let symbol = proof
        .scoping
        .get_root_binding("value".into())
        .unwrap_or_else(|| panic!("fixture binding"));
    let shape = proof.binding(symbol);
    let path: Vec<_> = path
        .into_iter()
        .map(|key| key.map(str::to_string))
        .collect();
    assert_eq!(
        (
            shape.primitive_path(&path),
            shape.plain(),
            shape.shallow_primitives()
        ),
        expected,
        "{source}"
    );
}
