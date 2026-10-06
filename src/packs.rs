//! Versioned, explicit workflows with raw records and evidence-backed findings.
use crate::{
    models::Cancel,
    project::{Policy, Project},
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pack {
    pub id: String,
    pub version: u32,
    pub description: String,
    pub prerequisite: String,
    pub inputs: Value,
    pub parser: String,
}
pub fn catalog() -> Vec<Pack> {
    [
("build","Run project test runner","cargo, python3 or npm",json!({"language":"rust | python | javascript"}),"test-count-v1"),
("git","Inspect changes; branch, stage or commit only selected paths","git",json!({"action":"status | diff | branch | stage | commit","branch":"new-name","paths":["file"],"message":"commit description"}),"git-v1"),
("http","Check an application's HTTP response","network access",json!({"url":"http://127.0.0.1:3000/","status":200,"contains":"expected text"}),"http-v1"),
("browser","Check visible behavior with optional click and screenshot","node, project-local playwright and installed Chromium",json!({"url":"http://127.0.0.1:3000/","selector":"body","contains":"expected text","click":"optional selector"}),"browser-v1"),
("semgrep","Review Python eval and shell execution, or chosen rules","semgrep",json!({"config":"optional rule file; defaults to bundled Python review rules"}),"semgrep-v1"),
("pip-audit","Audit locked Python dependencies","python3 with pip-audit",json!({"requirements":"requirements.txt"}),"pip-audit-v1"),
("npm-audit","Audit application dependencies","npm with lockfile",json!({}),"npm-audit-v1")
].into_iter().map(|(id,description,prerequisite,inputs,parser)|Pack{id:id.into(),version:1,description:description.into(),prerequisite:prerequisite.into(),inputs,parser:parser.into()}).collect()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub rule: String,
    pub title: String,
    pub severity: String,
    pub confidence: String,
    pub path: Option<String>,
    pub line: Option<u64>,
    pub evidence: String,
    pub reproduction: Value,
    pub proposed_fix: String,
    #[serde(default)]
    pub retests: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub pack: String,
    pub version: u32,
    pub input: Value,
    pub status: String,
    pub error: Option<String>,
    pub evidence: Value,
    pub findings: Vec<Finding>,
}
fn finding(
    rule: &str,
    title: &str,
    severity: &str,
    path: Option<String>,
    line: Option<u64>,
    evidence: &str,
    input: &Value,
) -> Finding {
    Finding{id:uuid::Uuid::new_v4().to_string(),rule:rule.into(),title:title.into(),severity:severity.into(),confidence:"review required".into(),path,line,evidence:evidence.into(),reproduction:input.clone(),proposed_fix:"Review the recorded location and upstream advisory; change the implementation or dependency, then retest.".into(),retests:vec![]}
}
pub fn parse(pack: &str, raw: &str, evidence: &str, input: &Value) -> Result<Vec<Finding>> {
    let v: Value = serde_json::from_str(raw)
        .context("Tool output is not valid JSON; no clean result can be inferred")?;
    let mut findings = Vec::new();
    match pack {
        "semgrep" => {
            ensure!(
                v["paths"]["scanned"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty()),
                "Semgrep scanned zero files; no clean result can be inferred"
            );
            let rows = v["results"].as_array().context("Semgrep omitted results")?;
            ensure!(
                v["errors"].as_array().is_some_and(|v| v.is_empty()),
                "Semgrep reported incomplete scanning"
            );
            for r in rows {
                findings.push(finding(
                    r["check_id"].as_str().context("Rule ID missing")?,
                    r["extra"]["message"]
                        .as_str()
                        .context("Finding message missing")?,
                    match r["extra"]["severity"].as_str().unwrap_or("") {
                        "ERROR" => "high",
                        "WARNING" => "medium",
                        _ => "low",
                    },
                    r["path"].as_str().map(str::to_owned),
                    r["start"]["line"].as_u64(),
                    evidence,
                    input,
                ));
            }
        }
        "pip-audit" => {
            let deps = v["dependencies"]
                .as_array()
                .or_else(|| v.as_array())
                .context("pip-audit omitted dependencies")?;
            ensure!(
                !deps.is_empty(),
                "Zero dependencies were audited; verify the requirements input"
            );
            for d in deps {
                ensure!(
                    d.get("skip_reason").is_none(),
                    "A dependency was skipped; audit incomplete"
                );
                for r in d["vulns"]
                    .as_array()
                    .context("Dependency omitted vulnerabilities")?
                {
                    let mut f = finding(
                        r["id"].as_str().context("Advisory ID missing")?,
                        &format!(
                            "{} {}: {}",
                            d["name"],
                            d["version"],
                            r["description"].as_str().unwrap_or("Dependency advisory")
                        ),
                        "unknown",
                        Some(
                            input["requirements"]
                                .as_str()
                                .unwrap_or("requirements.txt")
                                .into(),
                        ),
                        None,
                        evidence,
                        input,
                    );
                    f.proposed_fix =
                        format!("Review compatible fixed versions: {}", r["fix_versions"]);
                    findings.push(f);
                }
            }
        }
        "npm-audit" => {
            ensure!(v.get("error").is_none(), "npm audit returned an error");
            ensure!(
                v["metadata"]["dependencies"]["total"]
                    .as_u64()
                    .is_some_and(|n| n > 0),
                "npm audit did not report any audited dependencies"
            );
            for (name, r) in v["vulnerabilities"]
                .as_object()
                .context("npm audit omitted vulnerabilities")?
            {
                let mut f = finding(
                    name,
                    &format!("Dependency advisory: {name}"),
                    r["severity"].as_str().unwrap_or("unknown"),
                    Some("package-lock.json".into()),
                    None,
                    evidence,
                    input,
                );
                f.proposed_fix = format!(
                    "Review fix availability and breaking changes: {}",
                    r["fixAvailable"]
                );
                findings.push(f);
            }
        }
        "browser" => {
            ensure!(v["passed"].is_boolean(), "Browser output omitted outcome");
            if v["passed"] != true {
                findings.push(finding(
                    "browser.expected-behavior",
                    v["error"].as_str().unwrap_or("Browser assertion failed"),
                    "medium",
                    None,
                    None,
                    evidence,
                    input,
                ));
            }
        }
        _ => bail!("No structured parser for {pack}"),
    }
    Ok(findings)
}
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
fn text<'a>(input: &'a Value, key: &str) -> Result<&'a str> {
    input[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .with_context(|| format!("Provide {key}"))
}
fn argv(pack: &str, input: &Value, assets: &Path) -> Result<Vec<String>> {
    let args: Vec<String> = match pack {
        "build" => match text(input, "language")? {
            "rust" => vec!["cargo".into(), "test".into(), "--locked".into()],
            "python" => vec![
                "python3".into(),
                "-m".into(),
                "unittest".into(),
                "discover".into(),
                "-v".into(),
            ],
            "javascript" => vec!["npm".into(), "test".into()],
            _ => bail!("Choose rust, python or javascript"),
        },
        "git" => {
            let action = text(input, "action")?;
            let paths: Vec<String> = input["paths"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|v| v.as_str().map(str::to_owned).context("Path must be text"))
                        .collect::<Result<_>>()
                })
                .transpose()?
                .unwrap_or_default();
            for p in &paths {
                ensure!(!p.starts_with(":("), "Literal paths required");
                Project::validate_path(p)?;
            }
            match action {
                "status" => vec![
                    "git".into(),
                    "status".into(),
                    "--short".into(),
                    "--branch".into(),
                ],
                "diff" => [
                    vec!["git".into(), "diff".into(), "HEAD".into(), "--".into()],
                    paths,
                ]
                .concat(),
                "branch" => {
                    let name = text(input, "branch")?;
                    ensure!(!name.starts_with('-'), "Invalid branch name");
                    vec!["git".into(), "switch".into(), "-c".into(), name.into()]
                }
                "stage" => {
                    ensure!(!paths.is_empty(), "Select files to stage");
                    [
                        vec![
                            "git".into(),
                            "--literal-pathspecs".into(),
                            "add".into(),
                            "--".into(),
                        ],
                        paths,
                    ]
                    .concat()
                }
                "commit" => {
                    ensure!(
                        !paths.is_empty(),
                        "Select files to commit; unrelated staged changes are preserved"
                    );
                    [
                        vec![
                            "git".into(),
                            "--literal-pathspecs".into(),
                            "commit".into(),
                            "--only".into(),
                            "-m".into(),
                            text(input, "message")?.into(),
                            "--".into(),
                        ],
                        paths,
                    ]
                    .concat()
                }
                _ => bail!("Unknown Git action"),
            }
        }
        "semgrep" => vec![
            "semgrep".into(),
            "--json".into(),
            "--metrics=off".into(),
            "--no-git-ignore".into(),
            "--config".into(),
            input["config"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| assets.join("semgrep.yml").display().to_string()),
            ".".into(),
        ],
        "pip-audit" => vec![
            "python3".into(),
            "-m".into(),
            "pip_audit".into(),
            "--format".into(),
            "json".into(),
            "--requirement".into(),
            input["requirements"]
                .as_str()
                .unwrap_or("requirements.txt")
                .into(),
        ],
        "npm-audit" => vec!["npm".into(), "audit".into(), "--json".into()],
        "browser" => {
            text(input, "url")?;
            vec![
                "node".into(),
                "-e".into(),
                include_str!("../assets/browser-check.cjs").into(),
                input.to_string(),
            ]
        }
        _ => bail!("Unknown workflow"),
    };
    Ok(args)
}
pub async fn run(
    data: &Path,
    cwd: &Path,
    pack: &str,
    mut input: Value,
    policy: Policy,
    cancel: Cancel,
) -> Result<Run> {
    ensure!(
        policy == Policy::Trusted,
        "Tool packs use host tools and network. Select Full access to run one"
    );
    ensure!(catalog().iter().any(|p| p.id == pack), "Unknown tool pack");
    let id = uuid::Uuid::new_v4().to_string();
    let state = {
        let p = Project::open(data, cwd)?;
        p.start_task(&id, &format!("{pack} workflow"))?;
        p.state.clone()
    };
    let assets = data.join("pack-assets-v1");
    crate::config::private_dir(&assets)?;
    crate::config::atomic_write(
        &assets.join("semgrep.yml"),
        include_bytes!("../assets/semgrep.yml"),
    )?;
    let evidence_dir = state.join("workflows");
    crate::config::private_dir(&evidence_dir)?;
    if pack == "browser" {
        input["screenshot"] = json!(evidence_dir.join(format!("{id}.png")));
    }
    let mut report = Run {
        id: id.clone(),
        pack: pack.into(),
        version: 1,
        input: input.clone(),
        status: "failed".into(),
        error: None,
        evidence: json!({}),
        findings: vec![],
    };
    let result:Result<()>=async {
 if pack=="http" {
  let url=text(&input,"url")?;let expected=input["status"].as_u64().unwrap_or(200);let started=std::time::Instant::now();
  let response=crate::models::client()?.get(url).timeout(std::time::Duration::from_secs(20)).send().await?;let status=response.status().as_u16();
  let mut bytes=Vec::new();use futures_util::StreamExt;let mut stream=response.bytes_stream();while let Some(chunk)=stream.next().await {bytes.extend_from_slice(&chunk?);ensure!(bytes.len()<=1024*1024,"HTTP body exceeds 1 MiB; check incomplete");}
  let body=String::from_utf8_lossy(&bytes).into_owned();report.evidence=json!({"url":url,"status":status,"body":body,"elapsed_ms":started.elapsed().as_millis()});
  if status as u64!=expected||input["contains"].as_str().is_some_and(|s|!body.contains(s)){report.findings.push(finding("http.expected-behavior","HTTP response did not match expected status or text","medium",None,None,&id,&input));}report.status=if report.findings.is_empty(){"passed"}else{"findings"}.into();
 }else{
  let args=argv(pack,&input,&assets)?;let command=args.iter().map(|s|quote(s)).collect::<Vec<_>>().join(" ");
  let r=crate::sandbox::terminal(data,cwd,&id,&command,300,cancel.clone()).await?;
  report.evidence=json!({"command":args,"terminal":r});
  ensure!(!r.cancelled&&!r.timed_out&&!r.output_truncated,"Workflow stopped or output was incomplete; inspect raw evidence");
  let stdout=r.output.strip_prefix("stdout:\n").and_then(|s|s.split_once("\nstderr:\n").map(|x|x.0)).unwrap_or("");
  if matches!(pack,"semgrep"|"pip-audit"|"npm-audit"|"browser"){report.findings=parse(pack,stdout,&id,&input)?;ensure!(r.exit_code==Some(0)||(r.exit_code==Some(1)&&!report.findings.is_empty()),"Tool failed, exit {:?}",r.exit_code);}
  else {ensure!(r.exit_code==Some(0),"Tool failed, exit {:?}",r.exit_code);if pack=="build"{let tests=r.tests_run;report.evidence["tests_run"]=json!(tests);ensure!(tests!=Some(0),"Runner reported zero tests");}}
  report.status=if report.findings.is_empty(){"passed"}else{"findings"}.into();
 }
 Ok(())}.await;
    if let Err(e) = result {
        report.error = Some(format!("{e:#}"));
        report.status = "failed".into();
    }
    crate::config::atomic_write(
        &evidence_dir.join(format!("{id}.json")),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    let p = Project::open(data, cwd)?;
    p.note(
        &id,
        if report.status == "passed" {
            "observation"
        } else {
            "failure"
        },
        &format!(
            "Workflow {pack} v1: {}. Raw evidence: {}",
            report.status,
            evidence_dir.join(format!("{id}.json")).display()
        ),
        &id,
    )?;
    Ok(report)
}
pub fn runs(data: &Path, cwd: &Path) -> Result<Vec<Run>> {
    let p = Project::open(data, cwd)?;
    let dir = p.state.join("workflows");
    let mut rows = Vec::new();
    if dir.exists() {
        for e in std::fs::read_dir(dir)? {
            let path = e?.path();
            if path.extension().is_some_and(|x| x == "json") {
                rows.push(serde_json::from_slice(&std::fs::read(path)?)?);
            }
        }
    }
    Ok(rows)
}
pub fn export(data: &Path, cwd: &Path, format: &str) -> Result<String> {
    let rows = runs(data, cwd)?;
    let findings: Vec<_> = rows.iter().flat_map(|r| r.findings.iter()).collect();
    match format {
        "json" => Ok(serde_json::to_string_pretty(&rows)?),
        "markdown" => Ok(rows
            .iter()
            .map(|r| {
                format!(
                    "## {} v{}: {}\n\nEvidence: `{}`\n\n{}\n{}\n",
                    r.pack,
                    r.version,
                    r.status,
                    r.id,
                    r.error.as_deref().unwrap_or(""),
                    r.findings
                        .iter()
                        .map(|f| format!(
                            "- {} [{}; {}] {}:{}\n  Reproduce: `{}`\n  Fix: {}\n",
                            f.title,
                            f.severity,
                            f.confidence,
                            f.path.as_deref().unwrap_or("remote"),
                            f.line.unwrap_or(0),
                            f.reproduction,
                            f.proposed_fix
                        ))
                        .collect::<String>()
                )
            })
            .collect()),
        "sarif" => Ok(serde_json::to_string_pretty(
            &json!({"version":"2.1.0","$schema":"https://json.schemastore.org/sarif-2.1.0.json","runs":[{"tool":{"driver":{"name":"Alt tool packs","version":"1"}},"results":findings.iter().map(|f| {let mut r=json!({"ruleId":f.rule,"level":match f.severity.as_str(){"high"|"critical"=>"error","low"=>"note",_=>"warning"},"message":{"text":f.title},"properties":{"evidence":f.evidence,"confidence":f.confidence,"reproduction":f.reproduction,"retests":f.retests}});if let Some(path)=&f.path{r["locations"]=json!([{"physicalLocation":{"artifactLocation":{"uri":path.replace(' ',"%20")},"region":{"startLine":f.line.unwrap_or(1).max(1)}}}]);}r}).collect::<Vec<_>>()}]}),
        )?),
        _ => bail!("Choose json, markdown or sarif"),
    }
}
pub fn retest(data: &Path, cwd: &Path, finding_id: &str, evidence: &str) -> Result<()> {
    let rows = runs(data, cwd)?;
    let proof = rows
        .iter()
        .find(|r| r.id == evidence)
        .context("Retest must reference an existing workflow record")?;
    let mut source = rows
        .iter()
        .find(|r| r.findings.iter().any(|f| f.id == finding_id))
        .context("Finding does not exist")?
        .clone();
    let normalized = |v: &Value| {
        let mut v = v.clone();
        if let Some(o) = v.as_object_mut() {
            o.remove("screenshot");
        }
        v
    };
    ensure!(
        proof.pack == source.pack && normalized(&proof.input) == normalized(&source.input),
        "Retest must run the same pack with the same inputs"
    );
    let target = source
        .findings
        .iter_mut()
        .find(|f| f.id == finding_id)
        .unwrap();
    target.retests.push(json!({"evidence":evidence,"status":proof.status,"finding_present":proof.findings.iter().any(|f|f.rule==target.rule&&f.path==target.path)}));
    let p = Project::open(data, cwd)?;
    crate::config::atomic_write(
        &p.state
            .join("workflows")
            .join(format!("{}.json", source.id)),
        &serde_json::to_vec_pretty(&source)?,
    )?;
    Ok(())
}
pub fn evidence_path(data: &Path, cwd: &Path, id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(id)?;
    Ok(Project::open(data, cwd)?
        .state
        .join("workflows")
        .join(format!("{id}.json")))
}
