use rstest::rstest;

#[rstest]
#[case("export const broken = ;", false)]
#[case(
    "export type Color = 'red'; export const box=style({color:'red'});",
    true
)]
#[case(
    "export interface Color { value: string } export const box=style({color:'red'});",
    true
)]
#[case(
    "export enum Color { Red='red' } export const box=style({color:Color.Red});",
    true
)]
#[case("Math.random(); export const box=style({color:'red'});", false)]
#[case("42; export const box=style({color:'red'});", false)]
#[case("{ const box=style({color:'red'}); }", false)]
#[case("for(;;) { style({color:'red'}); break; }", false)]
#[case(
    "export default function render(){return style({color:'red'});}",
    false
)]
#[case(
    "const values={read(){return 'red'}}; export const box=style({color:values.read()});",
    false
)]
#[case(
    "export const variants=styleVariants({red:'red'},function(color){return {color}});",
    true
)]
#[case(
    "export const box=style({color:'red'}); export const view=<><div/></>;",
    false
)]
fn full_evaluation_routing_when_authored_syntax_requires_a_mixed_module(
    #[case] body: &str,
    #[case] expected: bool,
) {
    let source = format!("import {{style,styleVariants}} from '@vanilla-extract/css';{body}");
    let actual = super::super::is_module("/detection.tsx", &source);
    assert_eq!(actual, expected, "{source}");
}
