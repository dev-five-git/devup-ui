use rstest::rstest;
use serial_test::serial;

use super::consumer_scheduling::PRODUCER;
use super::consumer_support::located_failure;
use super::demand_support::{TestResult, reset, run};

#[rstest]
#[case("if(window.flag){css({color:read()})}", "window.flag")]
#[case(
    "function view(enabled){if(enabled){css({color:read()})}}",
    "enabled){"
)]
#[case("if(count()){css({color:read()})}", "count()")]
#[serial]
fn consumer_failure_when_guard_is_runtime_only_reports_original_guard(
    #[case] body: &str,
    #[case] marker: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "const 한글='😀';\r\nimport {{css}} from '@devup-ui/react';\r\nimport {{token,read,count}} from './producer';\r\ncss({{margin:token}});\r\n{body}"
    );
    let offset = source.rfind(marker).ok_or("fixture guard missing")?;
    let place = crate::locate("/consumer-guard-failure.ts", &source, offset);
    // When
    let result = run(
        "/consumer-guard-failure.ts",
        &source,
        &[("./producer", "/counter-owner.ts", PRODUCER)],
    );
    // Then
    located_failure(result, &place, "runtime execution guard");
    Ok(())
}

#[rstest]
#[case("for(let n=0;n<1;n++){css({color:read()})}")]
#[case(concat!("for(const key in {", "a:1}){css({color:read()})}"))]
#[case("for(const value of [1]){css({color:read()})}")]
#[case("while(false){css({color:read()})}")]
#[case("do{css({color:read()})}while(false);")]
#[serial]
fn consumer_failure_when_application_loop_schedules_helper_reports_original_loop(
    #[case] body: &str,
) {
    // Given
    reset();
    let prefix = "import {css} from '@devup-ui/react';import {token,read} from './producer';css({margin:token});\n";
    let source = format!("{prefix}{body}");
    let place = crate::locate("/consumer-loop-failure.ts", &source, prefix.len());
    // When
    let result = run(
        "/consumer-loop-failure.ts",
        &source,
        &[("./producer", "/counter-owner.ts", PRODUCER)],
    );
    // Then
    located_failure(result, &place, "application iteration schedule");
}
