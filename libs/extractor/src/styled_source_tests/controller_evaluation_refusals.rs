use crate::{ExtractOption, ImportAlias, extract, utils::STYLE_OBJECT};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::array("[state.before,state.on?[{color:'red'},{margin:1}]:[],state.after]")]
#[case::comma("state.before,state.on?[{color:'red'},{margin:1}]:[],state.after")]
#[case::distinct_array(
    "[state.before,state.on?[{color:'red'},{margin:1}]:[{color:'blue'}],state.middle,state.on?[{background:'green'},{padding:1}]:[{background:'black'}],state.after]"
)]
#[case::lookup_array(
    "[state.on?[{color:{red:'red',blue:'blue'}[state.key]},{margin:1}]:[],state.after]"
)]
#[serial]
fn module_getter_class_shapes_remain_refused(
    #[case] arguments: &str,
    #[values("emotionCss", "css", "styled.div")] factory: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{css,styled}} from '@devup-ui/react';import {{css as emotionCss}} from '@emotion/react';const trace=[];let n=0;let selectedKey='red';const key={{[Symbol.toPrimitive](){{trace.push('coerce');return selectedKey;}}}};const state={{get before(){{trace.push('before');return 'before';}},get on(){{trace.push('on');return ++n===1;}},get middle(){{trace.push('middle');return 'middle';}},get key(){{trace.push('key');return key;}},get after(){{trace.push('after');selectedKey='blue';return 'opaque';}}}};const f=()=>{factory}({arguments});"
    );
    let option = ExtractOption {
        single_css,
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    // When
    let result = extract("test.ts", &source, option);
    // Then
    let error = match result {
        Ok(output) => panic!("module getter classes must remain refused: {output:?}"),
        Err(error) => error,
    };
    assert!(error.to_string().contains(STYLE_OBJECT), "{error}");
}

#[rstest]
#[serial]
fn module_getter_array_styled_retarget_shape_remains_refused(
    #[values(false, true)] single_css: bool,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let source = "import {styled} from '@devup-ui/react';const trace=[];let n=0;const state={get opaque(){trace.push('class');return 'opaque';},get on(){trace.push('on');return ++n===1;}};const Base=styled.div([state.opaque,state.on?[{color:'red'},{margin:1}]:[{color:'blue'}]]);const With=Base.withComponent('section');const Child=styled(Base)({padding:1});";
    // When
    let result = extract(
        "test.ts",
        source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    // Then
    let error = match result {
        Ok(output) => panic!("module getter array retarget must remain refused: {output:?}"),
        Err(error) => error,
    };
    assert!(error.to_string().contains(STYLE_OBJECT), "{error}");
}
