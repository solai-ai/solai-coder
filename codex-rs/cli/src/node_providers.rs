use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use serde::Serialize;
use serde_json::Value;

use super::NodeClient;
use super::admin::print_json;

#[derive(Debug, Parser)]
pub(super) struct NodeProvidersCli {
    #[command(subcommand)]
    command: ProviderCommand,
}

#[derive(Debug, clap::Subcommand)]
enum ProviderCommand {
    /// List bounded providers, optionally filtered by status or model.
    List {
        /// Provider status in snake_case, for example available.
        #[arg(long)]
        status: Option<String>,

        /// Only providers exposing this model.
        #[arg(long)]
        model: Option<String>,

        /// Maximum records to return.
        #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=500))]
        limit: u16,

        /// Records to skip before applying the limit.
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },

    /// Print one provider descriptor.
    Get {
        /// Provider id.
        provider_id: String,
    },

    /// Rediscover the model inventory for one provider.
    Refresh {
        /// Provider id.
        provider_id: String,
    },

    /// Mark a provider available for new jobs.
    Enable {
        /// Provider id.
        provider_id: String,
    },

    /// Disable a provider for new jobs.
    Disable {
        /// Provider id.
        provider_id: String,
    },

    /// Mark a provider unavailable without removing its record.
    Unavailable {
        /// Provider id.
        provider_id: String,
    },

    /// Delete an inactive provider record.
    Delete {
        /// Provider id.
        provider_id: String,

        /// Confirm that this provider record should be deleted.
        #[arg(long)]
        yes: bool,
    },
}

impl NodeProvidersCli {
    pub(super) async fn run(self, client: &NodeClient) -> Result<()> {
        match self.command {
            ProviderCommand::List {
                status,
                model,
                limit,
                offset,
            } => {
                let response: Value = client
                    .get_with_query(
                        "/v1/providers",
                        &ListProvidersQuery {
                            status,
                            model,
                            limit,
                            offset,
                        },
                    )
                    .await?;
                print_json(&response)
            }
            ProviderCommand::Get { provider_id } => {
                client
                    .print_get(&format!("/v1/providers/{provider_id}"))
                    .await
            }
            ProviderCommand::Refresh { provider_id } => {
                post_empty(client, &format!("/v1/providers/{provider_id}/refresh")).await
            }
            ProviderCommand::Enable { provider_id } => {
                post_empty(client, &format!("/v1/providers/{provider_id}/enable")).await
            }
            ProviderCommand::Disable { provider_id } => {
                post_empty(client, &format!("/v1/providers/{provider_id}/disable")).await
            }
            ProviderCommand::Unavailable { provider_id } => {
                post_empty(client, &format!("/v1/providers/{provider_id}/unavailable")).await
            }
            ProviderCommand::Delete { provider_id, yes } => {
                if !yes {
                    bail!("refusing to delete provider without --yes");
                }
                let response: Value = client
                    .send(
                        client.authorize(
                            client
                                .http
                                .delete(client.endpoint(&format!("/v1/providers/{provider_id}"))),
                        ),
                    )
                    .await?;
                print_json(&response)
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
pub(super) struct ListProvidersQuery {
    pub(super) status: Option<String>,
    pub(super) model: Option<String>,
    pub(super) limit: u16,
    pub(super) offset: usize,
}
