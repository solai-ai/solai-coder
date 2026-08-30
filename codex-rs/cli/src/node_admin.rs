use clap::ValueEnum;
use serde::Serialize;

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
