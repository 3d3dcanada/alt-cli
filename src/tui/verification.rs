use super::app::*;
use crate::{
    project::CheckSpec,
    verification::{Contract, Format, Kind, ReportSpec},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
impl App {
    pub(super) fn manage_verification(&mut self, action: &str, mut state: Value) -> Result<()> {
        match action {
            "check-inputs" => self.launch_project("Reading declared check inputs",|p,_| {
                let items=p.checks()?.into_iter().map(|spec|(spec.name.clone(),format!("{} external inputs · {} generated paths",spec.contract.inputs.files.len(),spec.contract.inputs.generated.len()),MenuAction::Manager {action:"check-inputs-select".into(),state:json!({"spec":spec})})).collect();
                Ok(JobResult::Dialog(Dialog::Menu{title:"Choose a check to configure".into(),description:"Declare files that affect the command and NEW paths it may generate. Existing source inputs can never be exempted.".into(),items,selected:0}))
            })?,
            "check-inputs-select" => self.manager_menu("Check inputs and generated files","Standard build/cache exclusions already apply. Declaring generated output never exempts an existing source file from change detection.",vec![("External input files".into(),"One project-relative or absolute regular file per line".into(),"check-inputs-edit".into(),json!({"spec":state["spec"],"field":"files"})),("Generated output paths".into(),"One NEW project-relative file or directory prefix per line".into(),"check-inputs-edit".into(),json!({"spec":state["spec"],"field":"generated"}))]),
            "check-inputs-edit" => {
                let field=state["field"].as_str().context("Input field")?;
                let text=state["spec"]["contract"]["inputs"][field].as_array().into_iter().flatten().filter_map(|v|v.as_str()).collect::<Vec<_>>().join("\n");
                self.dialog=Some(Dialog::Input{title:if field=="files"{"External input files"}else{"Generated output paths"}.into(),hint:"One path per line, up to 64. Ctrl+S reviews. Empty clears this list; other check settings are preserved.".into(),editor:super::input::Editor::new(text),action:InputAction::Manager{action:"check-inputs-review".into(),state},multiline:true});
            },
            "check-inputs-review" => {
                let paths:Vec<_>=state["value"].as_str().context("Paths")?.lines().map(str::trim).filter(|p|!p.is_empty()).map(str::to_owned).collect();
                ensure!(paths.len()<=64,"Declare at most 64 paths");let field=state["field"].as_str().context("Input field")?.to_owned();state["spec"]["contract"]["inputs"][&field]=json!(paths);
                self.manager_confirm("Save these check inputs?",format!("Check: {}\n{}\n\n{}\n\nExisting source files remain protected from mutation claims. Previous check evidence must be rerun after this configuration changes.",state["spec"]["name"].as_str().unwrap_or(""),if field=="files"{"External input files"}else{"New generated paths"},paths.join("\n")),"check-inputs-save",state);
            },
            "check-inputs-save" => self.launch_project("Saving declared check inputs",move|p,_|{let spec:CheckSpec=serde_json::from_value(state["spec"].clone())?;p.set_check(&spec)?;Ok(JobResult::Saved("Check input declaration saved. Run the check again to establish fresh evidence.".into()))})?,
            "check-logs" => {
                let check=self.task_view.latest.as_ref().context("Run or select a project check first")?;
                self.manager_menu("Retained check output","Separate raw stdout and stderr retain a bounded final window; older bytes outside that window are unavailable. Integrity is checked before viewing.",vec![("Latest stdout".into(),"Start at the end so late failures remain visible".into(),"check-log-read".into(),json!({"id":check.id,"stream":"stdout","offset":null})),("Latest stderr".into(),"Compiler and command errors may be here".into(),"check-log-read".into(),json!({"id":check.id,"stream":"stderr","offset":null}))]);
            },
            "check-log-read" => {
                let id=state["id"].as_str().context("Execution")?.to_owned();let stream=state["stream"].as_str().context("Stream")?.to_owned();let offset=state["offset"].as_u64();
                self.launch_project("Reading retained execution output",move|p,_|Ok(JobResult::Manager("check-log-result".into(),crate::verification::read_execution_log(p,&id,&stream,offset,16384)?)))?;
            },
            "check-log-result" => {
                let id=state["id"].as_str().context("Execution")?;let stream=state["stream"].as_str().context("Stream")?;let offset=state["offset"].as_u64().unwrap_or(0);let next=state["next_offset"].as_u64().unwrap_or(offset);let first=state["retained_from"].as_u64().unwrap_or(0);let total=state["total_bytes"].as_u64().unwrap_or(0);
                let mut items=Vec::new();
                if offset>first{items.push(("Previous segment".into(),"Read earlier retained output".into(),"check-log-read".into(),json!({"id":id,"stream":stream,"offset":offset.saturating_sub(16384).max(first)})));}
                if next<total{items.push(("Next segment".into(),"Read later output".into(),"check-log-read".into(),json!({"id":id,"stream":stream,"offset":next})));}
                items.push(("Switch stdout / stderr".into(),"Read the other stream's final segment".into(),"check-log-read".into(),json!({"id":id,"stream":if stream=="stdout"{"stderr"}else{"stdout"},"offset":null})));
                self.manager_menu("Execution output navigation","Choose another segment or Esc to return to Task.",items);
                self.recovery_dialog=self.dialog.take().map(Box::new);
                self.dialog=Some(Dialog::Notice{title:format!("Retained {stream} · {offset}–{next} of {total} bytes"),body:format!("{}\nRetained bytes begin at {first}.\n\n{}\n\nEnter returns to segment navigation.",state["scope"].as_str().unwrap_or("Bounded retained log"),state["text"].as_str().unwrap_or("")),scroll:0});
            },
            "check-contract" => self.launch_project("Reading check contracts", |p,_| Ok(JobResult::Manager("check-select".into(),json!({"checks":p.checks()?}))))?,
            "check-select" => {
                let checks:Vec<CheckSpec> = serde_json::from_value(state["checks"].take())?;
                ensure!(!checks.is_empty(),"Register a check command first, then set its purpose here.");
                self.manager_menu("Which check needs evidence?","Select a saved command.",checks.into_iter().map(|c|(c.name.clone(),format!("{:?}: {}",c.contract.kind,c.argv.join(" ")),"check-kind".into(),json!({"spec":c}))).collect());
            }
            "check-kind" => self.manager_menu("What does this check establish?", "Only tests with a complete structured report count as executed tests. Build, lint and health results keep their stated scope.",
                [("Tests","tests"),("Build","build"),("Lint","lint"),("Service health","health"),("Custom command","custom")].into_iter().map(|(label,kind)| { let mut next=state.clone(); next["kind"]=json!(kind); (label.into(),String::new(),"check-format".into(),next) }).collect()),
            "check-format" => {
                if state["kind"] != "tests" { return self.manage_verification("check-preview",state); }
                self.manager_menu("Test report format", "The command must write a fresh report to ALT_CHECK_REPORT or its configured relative path. Printing a test count is insufficient.",
                    [("Alt JSON","json"),("JUnit XML","junit"),("TAP 13","tap")].into_iter().map(|(label,format)| { let mut next=state.clone(); next["format"]=json!(format); (label.into(),String::new(),"check-path".into(),next) }).collect());
            }
            "check-path" => self.manager_input("Where will the report be written?","Relative path in the disposable project copy. Previous reports are removed before execution.","check-assertion",state,".alt-check-results.json"),
            "check-assertion" => {
                state["report"]=state["value"].take();
                self.manager_menu("Who owns the behavioral assertion?","An external assertion is pinned by hash outside the project and copied separately. Full access still has host permissions.",vec![
                    ("Use the project's test runner".into(),"Reported test execution; no independent behavioral acceptance".into(),"check-preview".into(),state.clone()),
                    ("Pin an independent assertion file".into(),"Select your external Python, Node or shell assertion; replaces this command with that runner".into(),"check-pin".into(),state)]);
            }
            "check-pin" => self.manager_input("Independent assertion file","Absolute path outside the project, .py, .js or .sh. It must test the disposable current directory and write ALT_CHECK_REPORT.","check-pinned",state,""),
            "check-pinned" => {
                state["assertion"]=state["value"].take();
                self.manage_verification("check-preview",state)?;
            }
            "check-preview" => self.manager_confirm("Save this verification contract?",format!("Check: {}\nPurpose: {}\nReport: {} at {}\nAssertion: {}\n\nA contract change invalidates previous results. Run the check again and inspect its coverage. An independent assertion replaces the command with its script runner.",state["spec"]["name"].as_str().unwrap_or("Check"),state["kind"].as_str().unwrap_or("custom"),state["format"].as_str().unwrap_or("none"),state["report"].as_str().unwrap_or("not required"),state["assertion"].as_str().unwrap_or("Project runner; behavioral coverage is not independent")),"check-save",state),
            "check-save" => self.launch_project("Saving verification contract", move |p,_| {
                let mut spec:CheckSpec=serde_json::from_value(state["spec"].take())?;
                let kind:Kind=serde_json::from_value(state["kind"].take())?;
                let report=if kind==Kind::Tests { Some(ReportSpec{format:serde_json::from_value::<Format>(state["format"].take())?,path:state["report"].as_str().context("Report path")?.into()}) } else {None};
                let assertion=if let Some(path)=state["assertion"].as_str() {
                    let path=std::path::Path::new(path);
                    let runner=match path.extension().and_then(|x| x.to_str()) { Some("py")=>"python3",Some("js")=>"node",Some("sh")=>"bash",_=>anyhow::bail!("Choose a .py, .js or .sh assertion; CLI also supports direct executable assertions") };
                    spec.argv=vec![runner.into(),"{assertion}".into()];
                    Some(Contract::pin(path)?)
                }else{None};
                spec.contract=Contract{kind,report,assertion,inputs:spec.contract.inputs.clone()}; p.set_check(&spec)?;
                Ok(JobResult::Saved("Contract saved. Run the check again, then review Task → Verification status.".into()))
            })?,
            _=>anyhow::bail!("Unknown verification step"),
        }
        Ok(())
    }
}
