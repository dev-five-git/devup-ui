use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

const GLOBAL: &str = "import {createGlobalStyle} from '@devup-ui/react'; export const Global=createGlobalStyle`body{color:red}`;";
const REQUIRED: &str = "const {css:rules}=require('@devup-ui/react'); export const box=rules({});";

#[rstest::rstest]
#[case(
    "import {style,keyframes} from '@devup-ui/react'; export const spin=keyframes({to:{height:10}}); export const box=style({animationName:spin});",
    false
)]
#[case(
    "import {css,keyframes} from '@devup-ui/react'; const label='css({})'; export const spin=keyframes({to:{height:10}});",
    false
)]
#[case(
    "import {css} from '@devup-ui/react'; const run=(css)=>css({});",
    false
)]
#[case(
    "import * as UI from '@devup-ui/react'; const run=(UI)=>UI.css({});",
    false
)]
#[case(
    "import {css as rules,style} from '@devup-ui/react'; const alias=rules; export const own=alias({}); export const box=style({});",
    true
)]
#[case(
    "import * as UI from '@devup-ui/react'; export const box=UI.css({});",
    true
)]
#[case("import {globalCss} from '@devup-ui/react'; globalCss({});", true)]
#[case(GLOBAL, true)]
#[case(
    "import {styled} from '@devup-ui/react'; export const Card=styled.section`color:red`;",
    true
)]
#[case(REQUIRED, true)]
fn eligibility_when_api_ownership_is_lexical(#[case] source: &str, #[case] expected: bool) {
    // Given
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    // When
    let required = super::required(&parsed.program, "@devup-ui/react");
    // Then
    assert_eq!(required, expected);
}
