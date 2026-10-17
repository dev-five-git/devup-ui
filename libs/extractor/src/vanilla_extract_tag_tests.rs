use boa_engine::{Context, Source};
use rstest::rstest;
use std::{cell::RefCell, rc::Rc};

use super::{Collector, register_vanilla_extract_apis};

#[rstest]
#[case(
    "function make(){ const card=ui.css`color:${eval(\"var local='red'; local\")};`; return local; } make();",
    "red"
)]
#[case(
    "function* make(){ return ui.css`color:${yield 'first'};`; } const iterator=make(); iterator.next(); iterator.next('blue').value;",
    "color:blue;"
)]
#[case(
    "let output='pending'; async function make(){ output=ui.css`color:${await Promise.resolve('green')};`; } make();",
    "color:green;"
)]
#[case(
    "const events=[]; Object.defineProperty(ui,'css',{ get(){events.push('get');return function(parts,...values){events.push(this===ui?'receiver':'lost');return values.join('');};} }); ui.css`${(events.push('first'),'a')}${(events.push('second'),'b')}`; events.join(',');",
    "get,first,second,receiver"
)]
#[case(r"let seen; let equal=true; ui.css=function(parts){ equal=equal && (seen===undefined || seen===parts) && Object.isFrozen(parts) && Object.isFrozen(parts.raw) && parts[0]===undefined && parts.raw[0]==='\\xg'; seen=parts; }; function make(){ui.css`\xg`;} make();make();String(equal);", "true")]
#[case(
    "ui.css=0;let effect=0;try {ui.css`${++effect}`;}catch(error){}String(effect);",
    "1"
)]
#[case(
    "ui.css=(parts,value)=>value;ui.css`${(Array.prototype[Symbol.iterator]=function(){throw new Error('unexpected iteration');},'red')}`;",
    "red"
)]
fn css_tag_terminal_preserves_tag_language_semantics_when_instrumented(
    #[case] body: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    // Given: the real native provenance adapter, with an observable namespace tag.
    let source = format!("import * as ui from '@devup-ui/react';\n{body}");
    let instrumented =
        crate::dead_properties::instrument(&source, "/semantics.ts", "@devup-ui/react")?;
    let (_, body) = instrumented
        .code
        .split_once('\n')
        .ok_or("missing import boundary")?;
    let code = "const ui=__vanilla_extract__; ui.css=function(parts,...values){return parts.reduce((text,part,index)=>text+part+(values[index]??''),'');};\n".to_string() + body;
    let mut context = Context::default();
    let collector = Rc::new(RefCell::new(Collector::default()));
    register_vanilla_extract_apis(&mut context, &collector)?;
    // When
    let result = context
        .eval(Source::from_bytes(&code))
        .map_err(|error| error.to_string())?;
    context.run_jobs().map_err(|error| error.to_string())?;
    let actual = if source.contains("async function") {
        context
            .eval(Source::from_bytes("output"))
            .map_err(|error| error.to_string())?
    } else {
        result
    };
    // Then
    assert_eq!(
        actual
            .to_string(&mut context)
            .map_err(|error| error.to_string())?
            .to_std_string_escaped(),
        expected
    );
    assert_eq!(collector.borrow().origin, None);
    Ok(())
}

#[rstest]
#[case(
    "import {css} from '@devup-ui/react';\n{ const css=(parts,value)=>value; css`${7}`; }",
    "7"
)]
#[case(
    "import * as ui from '@devup-ui/react';\nfunction make(ui){return ui.css`${8}`;}make({css:(parts,value)=>value});",
    "8"
)]
#[case("const css=(parts,value)=>value; css`${9}`;", "9")]
fn css_tag_terminal_preserves_ordinary_tags_when_import_identity_is_absent(
    #[case] source: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    // Given
    let instrumented = crate::dead_properties::instrument(source, "/shadow.ts", "@devup-ui/react")?;
    let code = if source.starts_with("import") {
        instrumented
            .code
            .split_once('\n')
            .ok_or("missing import boundary")?
            .1
    } else {
        &instrumented.code
    };
    let code = format!("const __vanilla_extract__={{__at:(location,api,thunk)=>thunk()}};\n{code}");
    let mut context = Context::default();
    // When
    let result = context
        .eval(Source::from_bytes(&code))
        .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(instrumented.calls, vec![]);
    assert_eq!(
        result
            .to_string(&mut context)
            .map_err(|error| error.to_string())?
            .to_std_string_escaped(),
        expected
    );
    Ok(())
}
