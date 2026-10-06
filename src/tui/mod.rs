mod app;
mod input;
mod keys;
mod task;
mod view;
mod workbench;

use crate::engine::Event;
use serde_json::{Value, json};

pub use app::run;

pub fn record(event: &Event) -> Value {
    match event {
        Event::Update(value) => json!({"type":"update","data":value}),
        Event::Permission { id, params } => {
            json!({"type":"permission_request","id":id,"data":params})
        }
        Event::Log(text) => json!({"type":"engine_log","text":text}),
        Event::Disconnected(reason) => json!({"type":"disconnected","text":reason}),
    }
}

mod management;

mod verification;
