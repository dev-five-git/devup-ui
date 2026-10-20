use super::css_prop_execution::{CAPTURED, css_props, element, execute};
use super::*;

#[rstest]
#[case(
    "<ClassNames>{({css})=><section className={css`${Card}{color:red;}`}><Card/></section>}</ClassNames>"
)]
#[case("<section css={css`${Card}{color:red;}`}><Card/></section>")]
#[case(
    "<ClassNames>{({css})=><section className={css({[Card]:{color:'red'}})}><Card/></section>}</ClassNames>"
)]
#[case(
    "<ClassNames>{({css:outerCss})=><ClassNames>{({theme})=><section className={outerCss`${Card}{color:${theme.brand};}`}><Card/></section>}</ClassNames>}</ClassNames>"
)]
#[serial]
fn selector_edges_match_when_template_entrypoints_read_component_markers(#[case] app: &str) {
    // Given: selected components occur before template classification at either entrypoint.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';const Card=styled.article({{color:'green'}});const App=()=>{app};"
    );

    // When: the emitted parent and child render through the native JSX harness.
    let result = execute(
        "test.jsx",
        &source,
        "const outer=render(App());const child=render(outer.props.children);const entry=Object.entries(rules).find(([,rule])=>rule[0]==='color'&&rule[3]);const [name,rule]=entry;const marker=rule[3].slice(rule[3].lastIndexOf('.')+1);JSON.stringify([outer.target,child.target,outer.props.className.split(/\\s+/).includes(name),child.props.className.split(/\\s+/).filter(name=>name===marker).length,values(child,'color'),rule[1]]);",
    );

    // Then: the descendant declaration selects the rendered marker exactly once.
    let color = if app.contains("theme.brand") {
        "var(--brand)"
    } else {
        "red"
    };
    assert_eq!(
        result,
        serde_json::json!(["section", "article", true, 1, ["green"], color])
    );
}

#[rstest]
#[case(".child", "& .child")]
#[case("#target", "& #target")]
#[case("[data-card]", "&[data-card]")]
#[case("*", "& *")]
#[case("> article", "& > article")]
#[case("+ article", "& + article")]
#[case("~ article", "& ~ article")]
#[case(":hover", "&:hover")]
#[case("&:hover", "&:hover")]
#[serial]
fn selector_edges_restore_alias_when_key_uses_shared_selector_policy(
    #[case] key: &str,
    #[case] selector: &str,
) {
    // Given: every existing selector prefix reads an alias of retained CSS source.
    let source = format!(
        "const leaf=css({{outlineColor:'orange'}});const alias=leaf;const Parent=styled.section({{'{key}':alias}});const App=()=> <Parent/>;"
    );

    // When: the emitted parent renders with its restored nested declaration.
    let result = execute(
        "test.jsx",
        &source,
        "const parent=render(App());const nested=atoms(parent).filter(rule=>rule[0]==='outline-color');JSON.stringify(nested.map(rule=>[rule[1],rule[3]]));",
    );

    // Then: the alias is rules, not a generated class string, under that selector.
    assert_eq!(result, serde_json::json!([["orange", selector]]));
}

#[rstest]
#[case("[Card]")]
#[case("[`&:hover ${Card}`]")]
#[case("[`${Card}`]")]
#[serial]
fn nested_css_source_survives_when_component_selector_reads_alias(#[case] key: &str) {
    // Given: a nested rule reads the retained source of an aliased static CSS result.
    let source = format!(
        "const Card=styled.article({{color:'green'}});const leaf=css({{outlineColor:'orange'}});const alias=leaf;const Parent=styled.section({{{key}:alias}});const App=()=> <Card />;"
    );

    // When: the selector and its component render from the combined extraction.
    let result = execute(
        "test.jsx",
        &source,
        "const rule=Object.values(rules).find(rule=>rule[0]==='outline-color'&&rule[3]);const marker=rule[3].slice(rule[3].lastIndexOf('.')+1);const child=render(App());JSON.stringify([rule[1],child.props.className.split(/\\s+/).includes(marker),values(child,'color')]);",
    );

    // Then: the nested declaration is restored rather than treating a class as a rule.
    assert_eq!(result, serde_json::json!(["orange", true, ["green"]]));
}

#[rstest]
#[case("styled.section({[Base]:{outlineColor:'orange'},[`&:hover ${Card}`]:{borderColor:'cyan'}})")]
#[case("styled.section`${Base}{outline-color:orange;} &:hover ${Card}{border-color:cyan;}`")]
#[serial]
fn component_selectors_survive_when_class_names_supplies_scoping(
    #[case] parent: &str,
    #[values(false, true)] captured: bool,
) {
    // Given: ClassNames shares semantic IDs with selected, retargeted styled components.
    let base = if captured {
        CAPTURED.replace("Card", "Base")
    } else {
        "const trace=[];const Base=styled.article({color:'green'});".into()
    };
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';{base}const Retarget=Base.withComponent('aside');const Card=styled(Retarget)({{padding:'2px'}});const Parent={parent};const Names=()=> <ClassNames>{{({{css:outerCss}})=><ClassNames>{{({{css,theme}})=>({{title:((css)=>css())(()=>{{trace.push('shadow');return 'kept';}}),className:outerCss({{color:theme.brand}})}})}}</ClassNames>}}</ClassNames>;const App=()=> <Card css={{{{color:'blue'}}}} ref={{7}} children='child' />;"
    );

    // When: emitted callbacks render the component and the nested binding crossing.
    let result = execute(
        "test.jsx",
        &source,
        "const markers=['outline-color','border-color'].map(property=>Object.values(rules).find(rule=>rule[0]===property)[3]).map(selector=>selector.slice(selector.lastIndexOf('.')+1));const final=render(App());const names=Names();JSON.stringify([final.target,final.props.ref,final.props.children,markers.map(marker=>final.props.className.split(/\\s+/).filter(name=>name===marker).length),values(final,'color'),values(final,'padding'),slots(final),names.title,trace]);",
    );

    // Then: selector identities, later-wins styles, refs, captures and ordinary calls survive.
    let trace = if captured {
        vec!["controller", "leaf", "shadow"]
    } else {
        vec!["shadow"]
    };
    assert_eq!(
        result,
        serde_json::json!([
            "aside",
            7,
            "child",
            [1, 1],
            ["blue"],
            ["2px"],
            [],
            "kept",
            trace
        ])
    );
}

#[rstest]
#[case("test.jsx")]
#[case("test.ts")]
#[serial]
fn identities_survive_when_native_chain_composes_and_retargets(
    #[case] filename: &str,
    #[values(false, true)] captured: bool,
    #[values(false, true)] retarget: bool,
) {
    // Given: four selected identities across tagged/object extensions and native retargeting.
    let base = if captured {
        CAPTURED.replace("Card", "Base")
    } else {
        "const trace=[];let selected=true;const Base=styled.article({color:'green'});".into()
    };
    let retarget = if retarget {
        "Middle.withComponent('aside')"
    } else {
        "styled(Middle)({})"
    };
    let extra = if filename.ends_with('x') {
        "ref={ref} children='child' className='external'"
    } else {
        "ref,children:'child',className:'external'"
    };
    let plain = element(filename, extra);
    let app = element(
        filename,
        &css_props(filename, "{color:'blue',padding:'4px'}", extra),
    );
    let covered_extra = if filename.ends_with('x') {
        "ref={ref} children='child' className={override+' external'}"
    } else {
        "ref,children:'child',className:override+' external'"
    };
    let covered = element(
        filename,
        &css_props(filename, "{color:'blue',padding:'4px'}", covered_extra),
    );
    let source = format!(
        "{base}const Middle=styled(Base)`padding:2px;`;const Retarget={retarget};const Card=styled(Retarget)({{color:'green'}});const Parent=styled.section({{[Base]:{{outlineColor:'orange'}},[Middle]:{{borderColor:'cyan'}},[Retarget]:{{caretColor:'pink'}},[Card]:{{accentColor:'yellow'}}}});const override=css({{color:'purple',padding:'6px'}});const Plain=(trace,selected,ref)=>{plain};const App=(trace,selected,ref)=>{app};const Covered=(trace,selected,ref)=>{covered};"
    );

    // When: alternating no-CSS, replacement, covered-empty and no-CSS calls execute emitted code.
    let result = execute(
        filename,
        &source,
        "const identities=['outline-color','border-color','caret-color','accent-color'].map(property=>Object.values(rules).find(rule=>rule[0]===property)[3]).map(selector=>selector.slice(selector.lastIndexOf('.')+1));const before=trace.slice();const first=render(Plain([],false,7));const raw=App([],false,8);const second=render(raw);const covered=Covered([],false,9);const keys=slots(covered);const third=render(covered);selected=false;const fourth=render(Plain([],true,10));JSON.stringify([before,raw.target===Card,keys.length,keys.length===1&&covered.props[keys[0]]==='',identities.length,[first,second,third,fourth].map(e=>[e.target,e.props.ref,e.props.children,slots(e),identities.map(marker=>(e.props.className||'').split(/\\s+/).filter(name=>name===marker).length),e.props.className.includes('external'),values(e,'color'),values(e,'padding')]),trace]);",
    );

    // Then: all identities survive replacement exactly once, while later atoms and captures win.
    let tag = if retarget.contains("withComponent") {
        "aside"
    } else {
        "article"
    };
    let trace = if captured {
        vec![
            "controller",
            "leaf",
            "controller",
            "leaf",
            "controller",
            "leaf",
            "controller",
        ]
    } else {
        vec![]
    };
    assert_eq!(
        result,
        serde_json::json!([
            [],
            captured,
            usize::from(captured),
            captured,
            4,
            [
                [tag, 7, "child", [], [1, 1, 1, 1], true, ["green"], ["2px"]],
                [tag, 8, "child", [], [1, 1, 1, 1], true, ["blue"], ["4px"]],
                [tag, 9, "child", [], [1, 1, 1, 1], true, ["purple"], ["6px"]],
                [tag, 10, "child", [], [1, 1, 1, 1], true, ["green"], ["2px"]]
            ],
            trace
        ])
    );
}

#[rstest]
#[case("test.jsx")]
#[case("test.ts")]
#[serial]
fn opaque_classes_remain_when_nonoverlapping_css_prop_is_applied(#[case] filename: &str) {
    // Given: a selected base carries an authored styling class, not a compiler identity.
    let app = element(filename, &css_props(filename, "{padding:'4px'}", ""));
    let source = format!(
        "const Base=styled.article({{color:'green'}},'opaque');const Card=styled(Base)({{}});const Parent=styled.section({{[Base]:{{outlineColor:'orange'}}}});const App=()=>{app};"
    );

    // When: nonoverlapping CSS props render the opaque-class component without bypassing it.
    let result = execute(
        filename,
        &source,
        "const raw=App();const final=render(raw);JSON.stringify([raw.target===Card,slots(raw),final.props.className.split(/\\s+/).includes('opaque'),values(final,'color'),values(final,'padding')]);",
    );

    // Then: the original renderer and opaque class remain authoritative.
    assert_eq!(
        result,
        serde_json::json!([true, [], true, ["green"], ["4px"]])
    );
}

#[rstest]
#[case("test.jsx")]
#[case("test.ts")]
#[serial]
fn opaque_classes_still_block_when_overlapping_css_prop_is_applied(#[case] filename: &str) {
    // Given: known selector identity must not turn an opaque styling class into fixed styles.
    let app = element(filename, &css_props(filename, "{color:'blue'}", ""));
    let source = format!(
        "import {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';const Base=styled.article({{color:'green'}},'opaque');const Card=styled(Base)({{}});const Parent=styled.section({{[Base]:{{outlineColor:'orange'}}}});const App=()=>{app};"
    );
    reset_class_map();
    reset_file_map();

    // When: an overlapping CSS prop would require ordering opaque styling classes.
    let error = extract(
        filename,
        &source,
        ExtractOption {
            single_css: true,
            import_aliases: std::collections::HashMap::from([(
                "@emotion/react".to_string(),
                crate::ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
    )
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();

    // Then: the existing located overlap error remains, rather than unchecked lowering.
    assert!(error.starts_with(&format!("{filename}:1:")), "{error}");
    assert!(
        error.contains("`css` on `<Card>` overrides styles `Card` sets"),
        "{error}"
    );
}
