use super::super::plan::{EscapeKind, MemberDemand};
use super::{names, selected, text};

#[rstest::rstest]
#[case("make.call(null)")]
#[case("make.apply(null,[])")]
fn selects_native_helper_when_call_or_apply_invokes_it(#[case] call: &str) {
    // Given
    let source = format!(
        "import {{createVar}} from '@vanilla-extract/css';\nfunction make(){{return createVar()}}\nconst host=window.document;\nconst token={call};"
    );
    // When
    let plan = selected(&source);
    // Then
    assert_eq!(names(&plan.units), vec!["make", "token"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(text(&source, plan.roots[0].span), format!("token={call}"));
    assert_eq!(plan.native_calls.len(), 1);
    assert_eq!(text(&source, plan.native_calls[0].span), "createVar()");
    assert_eq!(plan.roots[0].native_calls, vec![plan.native_calls[0].node]);
    assert_eq!(plan.helper_calls.len(), 1);
    assert_eq!(text(&source, plan.helper_calls[0].span), call);
    assert_eq!(plan.helper_calls[0].caller, plan.roots[0].owner);
    assert_eq!(plan.helper_calls[0].callable, plan.units[0].node);
    assert_eq!(
        text(&source, plan.units[0].span),
        "function make(){return createVar()}"
    );
    assert!(
        plan.reads
            .iter()
            .all(|read| !matches!(read.name.as_str(), "host" | "window"))
    );
    assert_eq!((plan.escapes.len(), plan.checks.len()), (0, 0));
}

#[test]
fn selects_nested_helper_when_computed_members_call_it() {
    // Given
    let source = "import {createVar} from '@vanilla-extract/css';\nconst helpers={nested:{make:()=>createVar()},unused:()=>window.name};\nconst token=helpers['nested']['make']();";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["helpers", "token"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(
        text(source, plan.roots[0].span),
        "token=helpers['nested']['make']()"
    );
    assert_eq!(plan.native_calls.len(), 1);
    assert_eq!(text(source, plan.native_calls[0].span), "createVar()");
    assert_eq!(plan.roots[0].native_calls, vec![plan.native_calls[0].node]);
    assert_eq!(plan.helper_calls.len(), 1);
    assert_eq!(
        text(source, plan.helper_calls[0].span),
        "helpers['nested']['make']()"
    );
    assert_eq!(plan.helper_calls[0].caller, plan.roots[0].owner);
    assert!(plan.reads.iter().all(|read| read.name != "window"));
    assert_eq!((plan.escapes.len(), plan.checks.len()), (0, 0));
}

#[test]
fn selects_callback_targets_in_source_order_when_arguments_are_spread() {
    // Given
    let source = "import {createVar} from '@vanilla-extract/css';\nfunction invoke(...callbacks){return callbacks[0]()}\nfunction make(){return createVar()}\nconst token=invoke(...[make]);\nconst ignored=()=>window.name;";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["invoke", "make", "token"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(text(source, plan.roots[0].span), "token=invoke(...[make])");
    assert_eq!(plan.native_calls.len(), 1);
    assert_eq!(text(source, plan.native_calls[0].span), "createVar()");
    assert_eq!(plan.roots[0].native_calls, vec![plan.native_calls[0].node]);
    let calls = &plan.helper_calls;
    assert_eq!(
        calls.iter().map(|call| call.callable).collect::<Vec<_>>(),
        vec![plan.units[0].node, plan.units[1].node]
    );
    let spans = calls.iter().map(|call| text(source, call.span));
    assert_eq!(
        spans.collect::<Vec<_>>(),
        vec!["invoke(...[make])", "invoke(...[make])"]
    );
    assert!(calls.iter().all(|call| call.caller == plan.roots[0].owner));
    assert_eq!(
        text(source, plan.units[0].span),
        "function invoke(...callbacks){return callbacks[0]()}"
    );
    assert_eq!(
        text(source, plan.units[1].span),
        "function make(){return createVar()}"
    );
    assert!(
        plan.reads
            .iter()
            .all(|read| !matches!(read.name.as_str(), "ignored" | "window"))
    );
    assert_eq!((plan.escapes.len(), plan.checks.len()), (0, 0));
}

#[test]
fn excludes_type_inputs_when_control_statements_remain_lexical_roots() {
    // Given
    let source = "import {style} from '@vanilla-extract/css';\nconst host=window.name;\nexport type Alias=typeof host;\nexport interface Marker {value:typeof host}\nfunction make(value:typeof host){return style({color:'blue'})}\nconst box=make(undefined);\nif(false){style({color:'red'})}";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["make", "box"]);
    assert_eq!(
        plan.units
            .iter()
            .map(|unit| text(source, unit.span))
            .collect::<Vec<_>>(),
        vec![
            "function make(value:typeof host){return style({color:'blue'})}",
            "box=make(undefined)",
            "if(false){style({color:'red'})}"
        ]
    );
    assert_eq!(
        plan.roots
            .iter()
            .map(|root| text(source, root.span))
            .collect::<Vec<_>>(),
        vec!["box=make(undefined)", "if(false){style({color:'red'})}"]
    );
    assert_eq!(
        plan.native_calls
            .iter()
            .map(|call| text(source, call.span))
            .collect::<Vec<_>>(),
        vec!["style({color:'blue'})", "style({color:'red'})"]
    );
    assert_eq!(plan.roots[0].native_calls, vec![plan.native_calls[0].node]);
    assert_eq!(plan.roots[1].native_calls, vec![plan.native_calls[1].node]);
    assert!(
        plan.reads
            .iter()
            .all(|read| !matches!(read.name.as_str(), "host" | "window"))
    );
    assert_eq!((plan.escapes.len(), plan.checks.len()), (0, 0));
}

#[test]
fn demands_whole_import_when_computed_key_is_a_parameter() -> Result<(), Box<dyn std::error::Error>>
{
    // Given
    let source = "import {style} from '@vanilla-extract/css';\nimport {data} from './data';\nfunction make(key){return style({padding:data[key]})}\nconst box=make('space');";
    let read_start = u32::try_from(source.find("data[key]").ok_or("missing authored read")?)?;
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["make", "box"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.native_calls.len(), 1);
    assert_eq!(
        text(source, plan.native_calls[0].span),
        "style({padding:data[key]})"
    );
    assert_eq!(plan.roots[0].native_calls, vec![plan.native_calls[0].node]);
    let import = plan
        .imports
        .iter()
        .find(|import| import.source == "./data")
        .ok_or("missing data import")?;
    assert_eq!(text(source, import.specifier), "data");
    let mut demands = plan
        .demands
        .iter()
        .filter(|demand| demand.symbol == import.binding.symbol);
    let demand = demands.next().ok_or("missing data demand")?;
    assert_eq!(demands.count(), 0);
    assert_eq!(
        (demand.read.start, text(source, demand.read)),
        (read_start, "data")
    );
    assert!(matches!(demand.member, MemberDemand::Whole));
    Ok(())
}

#[test]
fn distinguishes_declaration_and_reference_escapes_when_outer_helper_is_exported()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {style} from '@vanilla-extract/css';\nexport function outer(){function inner(){return style({color:'blue'})}return inner()}\nconst box=outer();";
    let outer = u32::try_from(source.find("outer(){").ok_or("missing outer declaration")?)?;
    let inner = u32::try_from(source.find("inner(){").ok_or("missing inner declaration")?)?;
    let reference = u32::try_from(source.rfind("inner()").ok_or("missing inner return call")?)?;
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["outer", "box"]);
    assert_eq!(plan.consumed.len(), 0);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.native_calls.len(), 1);
    assert_eq!(
        text(source, plan.native_calls[0].span),
        "style({color:'blue'})"
    );
    assert_eq!(plan.roots[0].native_calls, vec![plan.native_calls[0].node]);
    assert_eq!(
        plan.escapes
            .iter()
            .map(|escape| (escape.span.start, text(source, escape.span), escape.kind))
            .collect::<Vec<_>>(),
        vec![
            (outer, "outer", EscapeKind::RuntimeHelper),
            (reference, "inner", EscapeKind::RuntimeHelper)
        ]
    );
    assert!(plan.escapes.iter().all(|escape| escape.span.start != inner));
    assert_eq!(plan.escapes[0].symbol, plan.units[0].bindings[0].symbol);
    let inner_read = plan
        .reads
        .iter()
        .find(|read| read.span.start == reference)
        .ok_or("missing inner value read")?;
    assert_eq!(Some(plan.escapes[1].symbol), inner_read.symbol);
    Ok(())
}
