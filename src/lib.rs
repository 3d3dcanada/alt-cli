pub mod compact_context;
pub mod config;
pub mod engine;
pub mod extensions;
pub mod hardware;
pub mod jobs;
pub mod models;
pub mod practice;
pub mod process;
pub mod project;
pub mod project_services;
pub mod project_worker;
pub mod runtime;
pub mod sandbox;
pub mod schema;
pub mod storage;
pub mod store;
pub mod toolbox;
pub mod tui;
pub mod verification;
pub const BUILD_INFO: &str = include_str!(concat!(env!("OUT_DIR"), "/build-info.json"));
pub mod workspace;

/// Remove terminal control characters from untrusted model/tool text.
pub fn display_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect()
}

pub mod packs;

pub mod syntax;

pub mod benchmark;

pub mod language;

pub mod analysis;
pub mod candidates;
pub mod capability;
pub mod inference;
pub mod instructions;
pub mod native;
pub mod qualification;
pub mod skills;
pub mod workflow;
