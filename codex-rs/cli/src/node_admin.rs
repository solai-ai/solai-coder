use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;
use serde_json::Value;

pub(super) const DEFAULT_NODE_URL: &str = "http://127.0.0.1:9876";
pub(super) const API_KEY_HEADER: &str = "x-solai-node-key";

pub(super) fn print_json(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub(super) fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[derive(Debug, Clone, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RuntimeModeArg {
    Running,
    Paused,
    Draining,
}

#[derive(Serialize)]
pub(super) struct UpdateRuntimeModeRequest {
    pub(super) mode: RuntimeModeArg,
}

#[derive(Serialize)]
pub(super) struct ListEventsQuery {
    pub(super) kind: Option<String>,
    pub(super) provider_id: Option<String>,
    pub(super) job_id: Option<String>,
    pub(super) limit: u16,
    pub(super) offset: usize,
}

#[derive(Serialize)]
pub(super) struct ListMeteringQuery {
    pub(super) tenant_id: Option<String>,
    pub(super) provider_id: Option<String>,
    pub(super) job_id: Option<String>,
    pub(super) workload: Option<String>,
    pub(super) model: Option<String>,
    pub(super) limit: u16,
    pub(super) offset: usize,
}
