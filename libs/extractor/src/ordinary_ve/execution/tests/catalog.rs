use super::*;

#[test]
#[serial]
fn uses_the_registered_native_catalog_when_all_sixteen_apis_share_a_selected_transaction()
-> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = r"
import * as ve from '@vanilla-extract/css';
const variable=ve.createVar({syntax:'<length>',inherits:false,initialValue:'0px'});
const fallback=ve.fallbackVar(variable,'1px');
const container=ve.createContainer('box');
const layer=ve.layer('layout');
ve.globalLayer('reset');
const font=ve.fontFace({src:'url(local.woff2)'});
ve.globalFontFace('Global',{src:'url(global.woff2)'});
const contract=ve.createThemeContract({space:null});
const global=ve.createGlobalThemeContract({color:'app-color'});
const assigned=ve.assignVars(contract,{space:'12px'});
const theme=ve.createTheme(contract,{space:'8px'});
const effect=ve.createGlobalTheme(':root',global,{color:'black'});
const animation=ve.keyframes({from:{opacity:0},to:{opacity:1}});
const box=ve.style({fontFamily:font,animationName:animation,margin:fallback});
const variants=ve.styleVariants({small:{padding:0},large:{padding:4}});
ve.globalStyle('body',{vars:assigned});
const host=window.document;
throw new Error('only runtime');
";
    // When
    let result = run(written("/catalog.ts", code), None)?;
    // Then
    assert_eq!(result.collected.styles.len(), 3);
    assert_eq!(result.collected.keyframes.len(), 1);
    assert_eq!(result.collected.font_faces.len(), 2);
    assert_eq!(result.collected.layers, ["layout-0-2", "reset"]);
    assert_eq!(result.collected.property_rules.len(), 1);
    assert_eq!(result.collected.global_styles.len(), 3);
    assert_eq!(binding(&result, "effect").expression, "undefined");
    assert_eq!(
        binding(&result, "global").expression,
        "{ \"color\": \"var(--app-color)\" }"
    );
    assert_eq!(
        binding(&result, "fallback").expression,
        "\"var(--var-0-0, 1px)\""
    );
    assert_eq!(binding(&result, "container").expression, "\"box-0-1\"");
    assert!(!binding(&result, "variants").expression.contains("__style_"));
    Ok(())
}

#[test]
#[serial]
fn retains_exact_mutation_effects_when_the_selected_initializer_performs_them() -> Result<(), String>
{
    // Given
    css::file_map::reset_file_map();
    let code = "import {style,createVar} from '@vanilla-extract/css';const result=(()=>{const data={color:'red'};data.color='blue';const values=[];values.push(createVar());return {box:style(data),token:values[0]}})();const after=createVar();";
    // When
    let result = run(written("/exact-mutation.ts", code), None)?;
    // Then
    let lowered = crate::vanilla_extract::collected_styles_to_code_with_keyframes(
        &result.collected,
        "@devup-ui/react",
        &rustc_hash::FxHashMap::default(),
    );
    assert!(lowered.contains("blue"));
    assert!(!lowered.contains("red"));
    assert!(
        binding(&result, "result")
            .expression
            .contains("var(--var-0-0)")
    );
    assert_eq!(binding(&result, "after").expression, "\"var(--var-0-1)\"");
    Ok(())
}
