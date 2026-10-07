//! The UI talks to this worker so connection, inference, and shutdown never
//! block keyboard input or rendering. SQLite remains the source of saved work.
use crate::{
    config::{Preferences, Profile},
    engine::{Engine, Event, Response, response_value},
    models::{Cancel, Progress},
    runtime::{self, LocalRuntime},
    store::{Session, Store},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::mpsc;

pub enum Command {
    Prompt {
        text: String,
        brief: String,
    },
    Permission {
        id: Value,
        params: Value,
        allow: bool,
    },
    Cancel,
    AddAllowance {
        requests: u32,
        tokens: u32,
    },
    Shutdown,
}

#[derive(Debug)]
pub enum Update {
    Ready(Box<Session>),
    Event(Event),
    Done(Value),
    Error(String),
    Progress(Progress),
    Inference(crate::inference::Status),
    Closed,
}

pub struct Workspace {
    pub commands: mpsc::Sender<Command>,
    pub updates: mpsc::Receiver<Update>,
    pub cancel: Cancel,
    task: tokio::task::JoinHandle<()>,
}

pub struct Options {
    pub root: PathBuf,
    pub requested_engine: PathBuf,
    pub preferences: Preferences,
    pub profile_name: String,
    pub profile: Profile,
    pub resume: Option<Session>,
}

impl Workspace {
    pub fn start(options: Options) -> Self {
        let (commands, rx) = mpsc::channel(16);
        let (tx, updates) = mpsc::channel(128);
        let cancel = Arc::new(AtomicBool::new(false));
        let cancellation = cancel.clone();
        let task = tokio::spawn(async move {
            if let Err(error) = worker(options, rx, &tx, cancellation).await {
                let _ = tx.send(Update::Error(format!("{error:#}"))).await;
            }
            let _ = tx.send(Update::Closed).await;
        });
        Self {
            commands,
            updates,
            cancel,
            task,
        }
    }
    pub fn send(&self, command: Command) -> Result<()> {
        self.commands
            .try_send(command)
            .context("The workspace is busy or disconnected; wait a moment or reconnect")
    }
    pub async fn stop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.commands.try_send(Command::Shutdown);
        // Drain output while joining so a full event queue cannot deadlock exit.
        let deadline = tokio::time::sleep(Duration::from_secs(6));
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                _=&mut self.task=>break,
                _=&mut deadline=>{self.task.abort();let _=(&mut self.task).await;break;},
                _=self.updates.recv()=>{},
            }
        }
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.task.abort();
    }
}

async fn worker(
    options: Options,
    mut commands: mpsc::Receiver<Command>,
    updates: &mpsc::Sender<Update>,
    cancel: Cancel,
) -> Result<()> {
    let Options {
        root,
        requested_engine,
        preferences,
        profile_name,
        profile,
        resume,
    } = options;
    let mut profile = profile;
    let cwd = resume
        .as_ref()
        .map(|s| PathBuf::from(&s.cwd))
        .unwrap_or_else(|| preferences.project.clone());
    ensure!(
        cwd.is_dir(),
        "The project folder no longer exists. Choose another folder from Home"
    );
    let binary=runtime::find_engine(&root,&requested_engine,&preferences)
        .context("The agent engine is not installed. Choose Install agent engine on Home, or set its path in Settings")?;
    let mut local_runtime = None;
    if profile.local_model.is_some() {
        let tx = updates.clone();
        let runtime =
            LocalRuntime::start(&root, &preferences, &profile, cancel.clone(), move |p| {
                let _ = tx.try_send(Update::Progress(p));
            })
            .await?;
        profile.endpoint = runtime.endpoint.clone();
        local_runtime = Some(runtime);
    }
    let _ = updates
        .send(Update::Progress(Progress {
            stage: "Connecting your model and tools".into(),
            ..Progress::default()
        }))
        .await;
    let mut engine = Engine::goose(&binary, &root, &cwd, &profile).await?;
    let info = engine.initialize().await?;
    ensure!(!cancel.load(Ordering::Relaxed), "Connection cancelled");
    let session = if let Some(saved) = resume {
        ensure!(
            info["agentCapabilities"]["loadSession"] == true,
            "This engine cannot resume sessions"
        );
        engine
            .restore_session(std::path::Path::new(&saved.cwd))
            .await?;
        saved
    } else {
        let result = engine.new_session(&cwd).await?;
        Session {
            id: uuid::Uuid::new_v4().to_string(),
            engine_id: result["sessionId"]
                .as_str()
                .context("Engine returned no session")?
                .into(),
            cwd: cwd.to_string_lossy().into(),
            profile_name,
            profile,
        }
    };
    engine.set_task(&session.id);
    let store = Store::open(&root)?;
    if store.get(&session.id).is_err() {
        store.create(&session)?;
    }
    updates
        .send(Update::Ready(Box::new(session.clone())))
        .await?;
    let mut response: Option<Response> = None;
    let mut deadline = tokio::time::Instant::now() + Duration::from_secs(1200);
    let mut cancelling = false;
    let mut heartbeat = tokio::time::interval(Duration::from_millis(200));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_inference = None;
    loop {
        tokio::select! {
            _=heartbeat.tick()=>{
                if let Some(status)=engine.inference_status() && Some(&status)!=last_inference.as_ref() {
                    store.append(&session.id,&json!({"type":"inference_status","data":status}))?;
                    updates.send(Update::Inference(status.clone())).await?;
                    last_inference=Some(status);
                }
            },
            result=async {response.as_mut().expect("guarded response").await},if response.is_some()=>{
                // Flush notifications queued before the response.
                while let Some(event)=engine.try_event() {
                    store.append(&session.id,&crate::tui::record(&event))?;
                    updates.send(Update::Event(event)).await?;
                }
                match response_value(result) {
                    Ok(value)=>{store.append(&session.id,&json!({"type":"turn_end","data":value}))?;updates.send(Update::Done(value)).await?;},
                    Err(error)=>{store.append(&session.id,&json!({"type":"error","text":error.to_string()}))?;updates.send(Update::Error(error.to_string())).await?;},
                }
                response=None;cancelling=false;
            }
            event=engine.next_event()=>{
                let event=event.context("The engine connection closed. Your session was saved; reconnect to continue")?;
                store.append(&session.id,&crate::tui::record(&event))?;
                if let Event::Permission{id,params}=&event && (cancelling || params["sessionId"]!=session.engine_id || params.get("unsupportedMethod").is_some()) {
                    engine.permission(id.clone(),params,false).await?;
                    store.append(&session.id,&json!({"type":"permission_decision","id":id,"allow":false}))?;
                    continue;
                }
                let disconnected=matches!(event,Event::Disconnected(_));
                updates.send(Update::Event(event)).await?;
                if disconnected {break;}
            }
            command=commands.recv()=> match command {
                Some(Command::Prompt{text,brief}) if response.is_none()=>{
                    store.append(&session.id,&json!({"type":"user","text":text}))?;
                    if !brief.trim().is_empty() {store.append(&session.id,&json!({"type":"project_brief","text":brief}))?;}
                    response=Some(engine.prompt_with_brief(&session.engine_id,&text,&brief).await?);
                    deadline=tokio::time::Instant::now()+Duration::from_secs(1200);cancelling=false;
                },
                Some(Command::Permission{id,params,allow})=>{
                    let allow=allow && !cancelling && params["sessionId"]==session.engine_id;
                    engine.permission(id.clone(),&params,allow).await?;
                    store.append(&session.id,&json!({"type":"permission_decision","id":id,"allow":allow}))?;
                },
                Some(Command::Cancel) if response.is_some()=>{
                    engine.cancel(&session.engine_id).await?;cancelling=true;
                    deadline=tokio::time::Instant::now()+Duration::from_secs(5);
                },
                Some(Command::AddAllowance{requests,tokens}) if response.is_none()=>{
                    match engine.add_allowance(requests,tokens) {
                        Ok(status)=>{
                            store.append(&session.id,&json!({"type":"allowance_added","requests":requests,"generated_tokens":tokens,"data":status}))?;
                            updates.send(Update::Inference(status.clone())).await?;
                            last_inference=Some(status);
                            updates.send(Update::Progress(Progress{stage:"Allowance added. Send a message to continue the saved task.".into(),..Progress::default()})).await?;
                        },
                        Err(error)=>{updates.send(Update::Error(error.to_string())).await?;},
                    }
                },
                Some(Command::Shutdown)|None=>break,
                _=>{},
            },
            _=tokio::time::sleep_until(deadline),if response.is_some()=>{
                if cancelling {
                    store.append(&session.id,&json!({"type":"error","text":"The model did not respond to cancellation; its engine was stopped. Reconnect to continue."}))?;
                    updates.send(Update::Error("The model did not stop in 5 seconds. Alt stopped its engine. Your conversation is saved; reconnect to continue.".into())).await?;
                    break;
                }
                engine.cancel(&session.engine_id).await?;cancelling=true;deadline=tokio::time::Instant::now()+Duration::from_secs(5);
            }
        }
    }
    engine.shutdown().await;
    if let Some(runtime) = &mut local_runtime {
        runtime.stop().await;
    }
    Ok(())
}
