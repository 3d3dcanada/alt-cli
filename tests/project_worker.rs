use alt_cli::{models::Cancel, project::Project, project_worker};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
fn token() -> Cancel {
    Arc::new(AtomicBool::new(false))
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn busy_project_wait_is_cancellable_without_blocking_async_input() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let held = Project::open(data.path(), cwd.path()).unwrap();
    let cancel = token();
    let worker = tokio::spawn(project_worker::run(
        data.path().into(),
        cwd.path().into(),
        cancel.clone(),
        |_| Ok(()),
    ));
    tokio::time::timeout(
        std::time::Duration::from_millis(100),
        tokio::time::sleep(std::time::Duration::from_millis(5)),
    )
    .await
    .unwrap();
    cancel.store(true, Ordering::Relaxed);
    let result = tokio::time::timeout(std::time::Duration::from_millis(250), worker)
        .await
        .unwrap()
        .unwrap();
    assert!(result.unwrap_err().to_string().contains("cancelled"));
    drop(held);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_during_walk_preserves_existing_index_and_worker_retries_lock() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("a.py"), "old").unwrap();
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    p.index().unwrap();
    let cancel = token();
    let worker = tokio::spawn(project_worker::run(
        data.path().into(),
        cwd.path().into(),
        cancel.clone(),
        |p| p.browse(),
    ));
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    drop(p);
    assert_eq!(worker.await.unwrap().unwrap(), vec!["a.py"]);
    let cancel2 = token();
    let c = cancel2.clone();
    let result = project_worker::run(data.path().into(), cwd.path().into(), cancel2, move |p| {
        c.store(true, Ordering::Relaxed);
        p.index()
    })
    .await;
    assert!(result.unwrap_err().to_string().contains("cancelled"));
    assert_eq!(
        Project::open(data.path(), cwd.path())
            .unwrap()
            .browse()
            .unwrap(),
        vec!["a.py"]
    );
}
