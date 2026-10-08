use alt_cli::{
    extensions::{self, Connection, Transport},
    project::Policy,
};
use std::{path::Path, time::Duration};
fn connection(command: &Path, marker: &Path) -> Connection {
    Connection {
        name: "owned".into(),
        transport: Transport::Stdio {
            command: "python3".into(),
            args: vec![
                command.to_string_lossy().into(),
                marker.to_string_lossy().into(),
            ],
            env_names: vec![],
        },
        enabled: false,
        selected_tools: vec![],
    }
}
#[tokio::test]
async fn guided_and_review_probes_do_not_launch_stdio_or_contact_http() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("marker");
    let script = root.path().join("marker.py");
    std::fs::write(
        &script,
        "import pathlib,sys;pathlib.Path(sys.argv[1]).write_text('executed')\n",
    )
    .unwrap();
    extensions::save(root.path(), &connection(&script, &marker)).unwrap();
    for policy in [Policy::Guided, Policy::ReviewOnly] {
        assert!(
            extensions::probe_with_policy(root.path(), "owned", policy)
                .await
                .unwrap_err()
                .to_string()
                .contains("Full access")
        );
    }
    assert!(!marker.exists());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut c = connection(&script, &marker);
    c.transport = Transport::Http {
        url: format!("http://{}/mcp", listener.local_addr().unwrap()),
        auth_env: None,
    };
    extensions::save(root.path(), &c).unwrap();
    for policy in [Policy::Guided, Policy::ReviewOnly] {
        assert!(
            extensions::probe_with_policy(root.path(), "owned", policy)
                .await
                .is_err()
        );
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn mcp_close_and_cancel_kill_term_ignoring_helpers() {
    for stalled in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let marker = root.path().join("helper.pid");
        let script = root.path().join("server.py");
        std::fs::write(&script,format!("import json,subprocess,sys,time\np=subprocess.Popen([sys.executable,'-c','import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);time.sleep(60)'],stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)\nopen(sys.argv[1],'w').write(str(p.pid))\ntime.sleep(.2)\n{}\nfor line in sys.stdin:\n q=json.loads(line)\n if 'id' not in q: continue\n result={{'protocolVersion':'2025-03-26','serverInfo':{{'name':'fixture','version':'1'}},'capabilities':{{'tools':{{}}}}}} if q['method']=='initialize' else {{'tools':[]}}\n print(json.dumps({{'jsonrpc':'2.0','id':q['id'],'result':result}}),flush=True)\n",if stalled{"time.sleep(60)"}else{"pass"})).unwrap();
        extensions::save(root.path(), &connection(&script, &marker)).unwrap();
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            extensions::probe_with_policy(root.path(), "owned", Policy::Trusted),
        )
        .await;
        assert_eq!(result.is_err(), stalled);
        if let Ok(result) = result {
            result.unwrap();
        }
        let pid: u32 = std::fs::read_to_string(&marker).unwrap().parse().unwrap();
        for _ in 0..20 {
            if alt_cli::jobs::Identity::read(pid).is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(
            alt_cli::jobs::Identity::read(pid).is_none(),
            "MCP helper survived close/cancel"
        );
    }
}
