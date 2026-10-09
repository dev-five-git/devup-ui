use crate::assignment_blocker_support::selected;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(("read('test',true)?read('yes',null):read('no',4)", "[\"test\",\"yes\"]", "[]"))]
#[case(("read('test',false)?read('yes',null):read('no',4)", "[\"test\",\"no\"]", "[\"padding:16px:0\"]"))]
#[case(("read('left',0)&&read('right',4)", "[\"left\"]", "[\"padding:0:0\"]"))]
#[case(("read('left',false)&&read('right',4)", "[\"left\"]", "[]"))]
#[case(("read('left',null)&&read('right',4)", "[\"left\"]", "[]"))]
#[case(("read('left','')&&read('right',4)", "[\"left\"]", "[]"))]
#[case(("read('left',0/0)&&read('right',4)", "[\"left\"]", "[]"))]
#[case(("read('left',true)&&read('right',null)", "[\"left\",\"right\"]", "[]"))]
#[case(("read('left',true)||read('right',4)", "[\"left\"]", "[]"))]
#[case(("read('left',null)||read('right',4)", "[\"left\",\"right\"]", "[\"padding:16px:0\"]"))]
#[case(("read('left',false)??read('right',4)", "[\"left\"]", "[]"))]
#[case(("read('left',null)??read('right',4)", "[\"left\",\"right\"]", "[\"padding:16px:0\"]"))]
#[case(("{a:read('a',null),b:read('b',4)}[read('key','a')]", "[\"a\",\"b\",\"key\"]", "[]"))]
#[case(("[read('a',null),read('b',4)][read('key',1)]", "[\"a\",\"b\",\"key\"]", "[\"padding:16px:0\"]"))]
#[case(("`${read('left','')} !important`||read('right',4)", "[\"left\"]", "[]"))]
#[serial]
fn composite_presence_is_lazy_when_selected_value_is_absent(#[case] fixture: (&str, &str, &str)) {
    // Given: every authored leaf and controller has an observable call.
    let (expression, trace, expected) = fixture;
    let source = format!(
        "import {{Box}}from '@devup-ui/react';let trace=[];function read(name,value){{trace.push(name);return value}}const node=<Box p={{{expression}}}/>;"
    );
    // When: the lowered assignment selects classes and inline values.
    let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
    // Then: JavaScript branch laziness and raw zero semantics remain intact.
    assert_eq!(actual, format!("[{trace},{expected}]"));
}

#[rstest]
#[case("p={[read('first',null),null,read('third',4)]}")]
#[case("_hover={{p:[read('first',null),null,read('third',4)]}}")]
#[case("_hover={[{p:read('first',null)},null,{p:read('third',4)}]}")]
#[case("selectors={{'& > span':{p:[read('first',null),null,read('third',4)]}}}")]
#[serial]
fn responsive_selector_classes_drop_only_absent_slots_when_all_sources_execute(
    #[case] attributes: &str,
) {
    // Given: one absent and one active responsive declaration plus a static rule.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';let trace=[];function read(name,value){{trace.push(name);return value}}const node=<Box color='red' {attributes}/>;"
    );
    // When: all authored values execute independent of viewport or selector state.
    let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
    // Then: only the first dynamic class disappears; metadata and static rules remain.
    assert_eq!(
        actual,
        "[[\"first\",\"third\"],[\"color:red:0\",\"padding:16px:2\"]]"
    );
}

#[rstest]
#[case(("`${read()}`", "''", "[]"))]
#[case(("`${read()}px`", "null", "[\"padding:nullpx:0\"]"))]
#[case(("`${read()} !important`", "''", "[]"))]
#[case(("`${read()};`", "''", "[]"))]
#[case(("`${read()} !important;`", "''", "[]"))]
#[case(("`${read()}\\x70x`", "10", "[\"padding:10px:0\"]"))]
#[case(("`1${read()}2`", "''", "[\"padding:48px:0\"]"))]
#[case(("[`${read()} !important`,null,'8px']", "''", "[\"padding:8px:2\"]"))]
#[serial]
fn normalized_presence_uses_whole_value_when_templates_have_suffixes(
    #[case] fixture: (&str, &str, &str),
) {
    // Given: normalization changes the CSS value, not the authored raw result.
    let (expression, value, expected) = fixture;
    let source = format!(
        "import {{Box}}from '@devup-ui/react';let reads=0;function read(){{reads++;return {value}}}const node=<Box p={{{expression}}}/>;"
    );
    // When: normalized class and variable consumers share the interpolation.
    let actual = selected(&source, "JSON.stringify([reads,assigned(node)]);");
    // Then: formatting suffixes cannot make an empty normalized value present.
    assert_eq!(actual, format!("[1,{expected}]"));
}
