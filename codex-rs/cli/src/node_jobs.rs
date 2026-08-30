use anyhow::Result;
use clap::Parser;
use serde::Serialize;
use serde_json::Value;

use super::NodeClient;
use super::admin::print_json;

#[derive(Debug, Parser)]
pub(super) struct NodeJobsCli {
    #[command(subcommand)]
    command: JobCommand,
}

#[derive(Debug, clap::Subcommand)]
enum JobCommand {
    /// List bounded jobs, optionally filtered by lifecycle or routing fields.
    List {
        /// Job status in snake_case, for example completed.
        #[arg(long)]
        status: Option<String>,

        /// Workload in snake_case, for example chat_complete.
        #[arg(long)]
        workload: Option<String>,

        /// Only jobs assigned to this provider.
        #[arg(long)]
        provider: Option<String>,

        /// Only jobs for this tenant.
        #[arg(long)]
        tenant: Option<String>,

        /// Only jobs for this model.
        #[arg(long)]
        model: Option<String>,

        /// Maximum records to return.
        #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=500))]
        limit: u16,

        /// Records to skip before applying the limit.
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },

    /// Print the persisted detail for one job.
    Detail {
        /// Node job id.
        job_id: String,
    },

    /// Explain the provider selected for one job.
    Route {
        /// Node job id.
        job_id: String,
    },

    /// Assign a queued job to an available provider.
    Assign {
        /// Node job id.
        job_id: String,

        /// Available provider id.
        #[arg(long)]
        provider: String,
    },

    /// Cancel a queued or running job.
    Cancel {
        /// Node job id.
        job_id: String,
    },

    /// Create a new attempt from a failed or cancelled job.
    Retry {
        /// Node job id.
        job_id: String,
    },
}

impl NodeJobsCli {
    pub(super) async fn run(self, client: &NodeClient) -> Result<()> {
        match self.command {
            JobCommand::List {
                status,
                workload,
                provider,
                tenant,
                model,
                limit,
                offset,
            } => {
                let response: Value = client
                    .get_with_query(
                        "/v1/jobs",
                        &ListJobsQuery {
                            status,
                            workload,
                            provider_id: provider,
                            tenant_id: tenant,
                            model,
                            limit,
                            offset,
                        },
                    )
                    .await?;
                print_json(&response)
            }
            JobCommand::Detail { job_id } => {
                client.print_get(&format!("/v1/jobs/{job_id}/detail")).await
            }
            JobCommand::Route { job_id } => {
                client.print_get(&format!("/v1/jobs/{job_id}/route")).await
            }
            JobCommand::Assign { job_id, provider } => {
                let response: Value = client
                    .post(
                        &format!("/v1/jobs/{job_id}/assign"),
                        &AssignJobProviderRequest {
                            provider_id: provider,
                        },
                    )
                    .await?;
                print_json(&response)
            }
            JobCommand::Cancel { job_id } => {
                post_empty(client, &format!("/v1/jobs/{job_id}/cancel")).await
            }
            JobCommand::Retry { job_id } => {
                post_empty(client, &format!("/v1/jobs/{job_id}/retry")).await
            }
        }
    }
}

async fn post_empty(client: &NodeClient, path: &str) -> Result<()> {
    let response: Value = client.post(path, &EmptyRequest {}).await?;
    print_json(&response)
}

#[derive(Serialize)]
struct EmptyRequest {}

#[derive(Serialize)]
pub(super) struct ListJobsQuery {
    pub(super) status: Option<String>,
    pub(super) workload: Option<String>,
    pub(super) provider_id: Option<String>,
    pub(super) tenant_id: Option<String>,
    pub(super) model: Option<String>,
    pub(super) limit: u16,
    pub(super) offset: usize,
}

#[derive(Serialize)]
struct AssignJobProviderRequest {
    provider_id: String,
}
