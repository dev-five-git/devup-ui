use std::{error::Error, fmt::Write};

use rstest::rstest;
use serde_json::json;

use super::tests::facts;

const OBJECT_ASSIGNMENT_SOURCE: &str =
    "function Content(){}; ({value:Content}={value:0}); export default Content;";
const FOR_IN_SOURCE: &str =
    "function Content(){}; for(Content in {x:0}){}; export default Content;";
const LOOP_PATTERN_SOURCE: &str =
    "function Content(){}; for({value:Content} of [{value:0}]){}; export default Content;";

#[rstest]
#[case("function Content(){}; export default Content;", true)]
#[case("export default function Content(){};", true)]
#[case("export default function(){};", true)]
#[case("export default ()=>0;", true)]
#[case("const Content=(()=>0); export default Content;", true)]
#[case("const Content=function Named(){}; export default Content;", true)]
#[case("export const Content=()=>0; export {Content as default};", true)]
#[case("export function Content(){}; export {Content as default};", true)]
#[case(
    "function Content(){}; const A=Content; const B=A; export default B;",
    true
)]
#[case(
    "function Content(){}; function change(Content){Content=0;} export default Content;",
    true
)]
#[case(
    "function Content(){}; {let Content; Content=0;} export default Content;",
    true
)]
#[case(
    "function Content(){}; try{}catch(Content){Content=0;} export default Content;",
    true
)]
#[case(
    "function Content(){}; for(let Content of [0]){Content++;} export default Content;",
    true
)]
#[case(
    "function Content(){}; function change(){var Content; Content=0;} export default Content;",
    true
)]
#[case(
    "function Content(){}; const change=function Content(){Content=0;}; export default Content;",
    true
)]
#[case("function Content(){}; Content.value=0; export default Content;", true)]
#[case(
    "function Content(){}; [Content.value]=[0]; export default Content;",
    true
)]
#[case(
    "function Content(){}; let x; ({[Content]:x}={}); export default Content;",
    true
)]
#[case(
    "function Content(){}; function change(){function Content(){}; Content=0;} export default Content;",
    true
)]
#[case(
    "function Content(){}; function change(Content){eval('Content=0');} export default Content;",
    true
)]
#[case(
    "function Content(){}; {let Content=1; eval('Content=0');} export default Content;",
    true
)]
#[case("const Content=()=>0; eval('void 0'); export default Content;", true)]
#[case(
    "function Content(){}; eval?.('void 0'); export default Content;",
    true
)]
#[case(
    "function Content(){}; (0,eval)('void 0'); export default Content;",
    true
)]
#[case("function Content(){}; Content=0; export default Content;", false)]
#[case(
    "function Content(){}; Content=0; const A=Content; const B=A; export default B;",
    false
)]
#[case("export default function Content(){}; Content=0;", false)]
#[case("function Content(){}; Content=0; export {Content as default};", false)]
#[case(OBJECT_ASSIGNMENT_SOURCE, false)]
#[case("function Content(){}; [Content]=[0]; export default Content;", false)]
#[case(
    "function Content(){}; ({value:[Content=0]}={value:[]}); export default Content;",
    false
)]
#[case(
    "function Content(){}; [...Content]=[]; export default Content;",
    false
)]
#[case(
    "function Content(){}; ({...Content}={}); export default Content;",
    false
)]
#[case("function Content(){}; Content++; export default Content;", false)]
#[case(
    "function Content(){}; for(Content of [0]){}; export default Content;",
    false
)]
#[case(FOR_IN_SOURCE, false)]
#[case("function Content(){}; Content+=0; export default Content;", false)]
#[case(LOOP_PATTERN_SOURCE, false)]
#[case(
    "function Content(){}; function change(){Content=0;} export default Content;",
    false
)]
#[case("function Content(){Content=0;} export default Content;", false)]
#[case(
    "function Content(){}; function change(x=(Content=0)){} export default Content;",
    false
)]
#[case(
    "function Content(){}; let x; ({[Content=0]:x}={}); export default Content;",
    false
)]
#[case(
    "function Content(){}; const holder={}; [holder[Content=0]]=[1]; export default Content;",
    false
)]
#[case(
    "function Content(){}; const holder={}; ({x:holder[Content=0]}={x:1}); export default Content;",
    false
)]
#[case("function Content(){}; export {Content as default}; Content=0;", false)]
#[case(
    "function Content(){}; eval('Content=0'); export default Content;",
    false
)]
#[case(
    "function Content(){}; const code='Content=0'; eval(code); export default Content;",
    false
)]
#[case(
    "function Content(){}; eval('Content=0'); const A=Content; const B=A; export default B;",
    false
)]
#[case("export default function Content(){}; eval('Content=0');", false)]
#[case(
    "function Content(){}; function change(){eval('Content=0');} export default Content;",
    false
)]
#[case(
    "function Content(){eval('Content=0');} export default Content;",
    false
)]
#[case(
    "function Content(){}; function change(Content){eval('Content=0');} eval('Content=0'); export default Content;",
    false
)]
#[case("function Content(){};", false)]
#[case("export default 0;", false)]
#[case("export default missing;", false)]
#[case("const A=B; const B=A; export default A;", false)]
#[case("export {default} from 'content';", false)]
#[case("let Content=()=>0; export default Content;", false)]
#[case("export default class Content{};", false)]
fn resolves_callable_when_local_provenance_is_intact(
    #[case] code: &str,
    #[case] callable: bool,
) -> Result<(), Box<dyn Error>> {
    // Given: a local default proof with lexical writes or shadows.
    // When: observation resolves aliases after collecting all writes.
    let result = facts(code, true)?;
    // Then: only intact local callable provenance is certified.
    assert_eq!(result["observation"]["defaultCallable"], json!(callable));
    Ok(())
}

#[test]
fn resolves_iteratively_when_alias_chain_is_long() -> Result<(), Box<dyn Error>> {
    // Given: a long acyclic chain of local const aliases.
    let mut aliases = String::new();
    for index in 0..5000 {
        write!(aliases, "const A{}=A{index};", index + 1)?;
    }
    let code = format!("function Content(){{}} const A0=Content;{aliases}export default A5000;");
    // When: default proof follows the aliases.
    let result = facts(&code, true)?;
    // Then: the terminal function is reached without recursive alias evaluation.
    assert_eq!(result["observation"]["defaultCallable"], json!(true));
    Ok(())
}
