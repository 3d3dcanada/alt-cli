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
                spec.contract=Contract{kind,report,assertion}; p.set_check(&spec)?;
                Ok(JobResult::Saved("Contract saved. Run the check again, then review Task → Verification status.".into()))
            })?,
            _=>anyhow::bail!("Unknown verification step"),
        }
        Ok(())
    }
}
