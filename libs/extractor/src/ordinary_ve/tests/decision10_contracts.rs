use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rstest::rstest;
use serial_test::serial;

use super::consumer_support::located_failure;
use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, extract, has_static};

const CHILD_ROLE: &str = "DEVUP_W35_DECISION10_CYCLIC_CONTRACT_CHILD";
const COMPLETED: &str = "W35_DECISION10_CYCLIC_DIAGNOSTIC_ASSERTED";
const CALLS: &[&str] = &[
    "createThemeContract(tokens)",
    "createGlobalThemeContract(tokens,(_,path)=>path.join('-'))",
    "createTheme(tokens)",
    "createGlobalTheme(':root',tokens)",
    "assignVars(tokens,tokens)",
    "createTheme(tokens,tokens)",
    "createGlobalTheme(':root',tokens,tokens)",
];

struct OwnedChild {
    process: Child,
    output: Option<JoinHandle<std::io::Result<String>>>,
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        match self.process.try_wait() {
            Ok(Some(_)) => {}
            status => {
                if let Err(error) = status {
                    eprintln!("owned cycle child status failed: {error}");
                }
                if let Err(error) = self.process.kill() {
                    eprintln!("owned cycle child termination failed: {error}");
                }
                if let Err(error) = self.process.wait() {
                    eprintln!("owned cycle child reap failed: {error}");
                }
            }
        }
        if let Some(output) = self.output.take() {
            match output.join() {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => eprintln!("owned cycle child output failed: {error}"),
                Err(error) => eprintln!("owned cycle child output reader panicked: {error:?}"),
            }
        }
    }
}

#[test]
#[serial]
fn decision10_cyclic_contract_rejection_is_bounded_and_located()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    if let Ok(role) = std::env::var(CHILD_ROLE) {
        let (extension, call) = role.split_once('|').ok_or("invalid cycle child role")?;
        assert!(["tsx", "css.ts"].contains(&extension));
        assert!(CALLS.contains(&call));
        let source = format!(
            "import {{createThemeContract,createGlobalThemeContract,createTheme,createGlobalTheme,assignVars}} from '@vanilla-extract/css';\nexport const result=(()=>{{const tokens={{leaf:null}};tokens.self=tokens;\nreturn {call};}})();"
        );
        // When
        let result = extract(extension, &source);
        // Then
        let error = match &result {
            Ok(output) => panic!("cyclic contract accepted: {}", output.code),
            Err(error) => error.to_string(),
        };
        let cause = error
            .split("Fix:")
            .next()
            .ok_or("cycle cause missing")?
            .to_lowercase();
        assert!(
            cause.contains("cyclic") || cause.contains("cycle"),
            "{error}"
        );
        assert!(error.contains("self"), "{error}");
        let repair = error
            .split_once("Fix:")
            .ok_or("cycle repair missing")?
            .1
            .to_lowercase();
        assert!(
            repair.contains("acyclic") || repair.contains("finite"),
            "{error}"
        );
        located_failure(result, &format!("/mixed.{extension}:3:"), "Fix:");
        println!("{COMPLETED}");
        return Ok(());
    }
    let filter = format!(
        "{}::decision10_cyclic_contract_rejection_is_bounded_and_located",
        module_path!()
            .split_once("::")
            .ok_or("test module path missing")?
            .1
    );
    for extension in ["tsx", "css.ts"] {
        for call in CALLS {
            let mut child = OwnedChild {
                process: Command::new(std::env::current_exe()?)
                    .args(["--exact", &filter, "--nocapture", "--test-threads=1"])
                    .env(CHILD_ROLE, format!("{extension}|{call}"))
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .spawn()?,
                output: None,
            };
            let mut stdout = child
                .process
                .stdout
                .take()
                .ok_or("cycle child stdout missing")?;
            child.output = Some(std::thread::spawn(move || {
                let mut output = String::new();
                stdout.read_to_string(&mut output)?;
                Ok(output)
            }));
            let deadline = Instant::now() + Duration::from_secs(5);
            // When / Then: only the owned subprocess can enter recursive native traversal.
            loop {
                if let Some(status) = child.process.try_wait()? {
                    assert!(
                        status.success(),
                        "{extension}: {call}: child failed with {status}"
                    );
                    let output = child
                        .output
                        .take()
                        .ok_or("cycle child reader missing")?
                        .join()
                        .map_err(|_| "cycle child output reader panicked")??;
                    assert!(
                        output.contains(COMPLETED),
                        "exact cycle child filter did not assert its diagnostic: {output}"
                    );
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "{extension}: {call}: no located cyclic-contract rejection within five seconds; owned child will be killed and reaped"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    Ok(())
}

#[rstest]
#[serial]
fn decision10_shared_contract_dag_preserves_distinct_path_variables(
    #[values("tsx", "css.ts")] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {createThemeContract,createGlobalThemeContract,createTheme,createGlobalTheme,assignVars,style} from '@vanilla-extract/css';\nconst shared={space:null};const vars=createThemeContract({left:shared,right:shared});\nconst value={space:'8px'};const values={left:value,right:value};\nexport const local=createTheme(vars,values);\ncreateGlobalTheme(':root',vars,values);\nconst assigned=assignVars(vars,values);\nexport const globalVars=createGlobalThemeContract({left:shared,right:shared},(_,path)=>'app-'+path.join('-'));\nexport const box=style({vars:assigned,padding:vars.left.space,margin:vars.right.space});\nexport const paths=[vars.left.space,vars.right.space];";
    // When
    let output = extract(extension, source)?;
    // Then
    assert_consumed(&output);
    assert_preserved(
        &output.code,
        "export const paths=['var(--left-space-0-0)','var(--right-space-0-1)'];",
    );
    assert_preserved(
        &output.code,
        "export const globalVars={'left':{'space':'var(--app-left-space)'},'right':{'space':'var(--app-right-space)'}};",
    );
    for (property, value) in [
        ("padding", "var(--left-space-0-0)"),
        ("margin", "var(--right-space-0-1)"),
        ("--left-space-0-0", "8px"),
        ("--right-space-0-1", "8px"),
    ] {
        assert!(
            has_static(&output, property, value),
            "{property}:{value}: {:?}",
            output.styles
        );
    }
    Ok(())
}
