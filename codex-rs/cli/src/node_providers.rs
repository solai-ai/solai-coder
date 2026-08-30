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
    /// List bounded providers, optionally filtered by status, model, or capacity.
    List {
        /// Provider status in snake_case, for example available.
        #[arg(long)]
        status: Option<String>,

        /// Only providers exposing this model.
        #[arg(long)]
        model: Option<String>,

        /// Only providers that have (true) or lack (false) a capacity report.
        #[arg(long)]
        has_capacity: Option<bool>,

        /// Only providers at (true) or not at (false) their reported capacity.
        #[arg(long)]
        at_capacity: Option<bool>,

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

    /// Set the SOLAI hourly price for one model.
    Price {
        /// Provider id.
        provider_id: String,

        /// Provider model id.
        #[arg(long)]
        model: String,

        /// Positive SOLAI amount per hour.
        #[arg(long, value_parser = parse_positive_decimal)]
        solai_per_hour: String,
    },

    /// Set the daily availability window for a provider.
    Schedule {
        /// Provider id.
        provider_id: String,

        /// Start time in HH:MM.
        #[arg(long, value_parser = parse_hh_mm)]
        from: String,

        /// End time in HH:MM.
        #[arg(long, value_parser = parse_hh_mm)]
        to: String,

        /// Optional IANA timezone, for example America/Sao_Paulo.
        #[arg(long)]
        timezone: Option<String>,
    },

    /// Send provider liveness and capacity information to Node.
    Heartbeat {
        /// Provider id.
        provider_id: String,

        /// Optional provider status. Omit to preserve the current Node status.
        #[arg(long)]
        status: Option<ProviderStatusArg>,

        /// Optional total concurrent-job capacity.
        #[arg(long)]
        capacity: Option<u32>,

        /// Optional count of active jobs.
        #[arg(long)]
        active_jobs: Option<u32>,
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
                has_capacity,
                at_capacity,
                limit,
                offset,
            } => {
                let response: Value = client
                    .get_with_query(
                        "/v1/providers",
                        &ListProvidersQuery {
                            status,
                            model,
                            has_capacity,
                            at_capacity,
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
            ProviderCommand::Price {
                provider_id,
                model,
                solai_per_hour,
            } => {
                let response: Value = client
                    .post(
                        &format!("/v1/providers/{provider_id}/price"),
                        &UpdateProviderPriceRequest {
                            model,
                            solai_per_hour,
                        },
                    )
                    .await?;
                print_json(&response)
            }
            ProviderCommand::Schedule {
                provider_id,
                from,
                to,
                timezone,
            } => {
                let response: Value = client
                    .post(
                        &format!("/v1/providers/{provider_id}/schedule"),
                        &UpdateProviderScheduleRequest {
                            available_from: from,
                            available_to: to,
                            timezone,
                        },
                    )
                    .await?;
                print_json(&response)
            }
            ProviderCommand::Heartbeat {
                provider_id,
                status,
                capacity,
                active_jobs,
            } => {
                validate_heartbeat_capacity(capacity, active_jobs)?;
                let response: Value = client
                    .post(
                        &format!("/v1/providers/{provider_id}/heartbeat"),
                        &ProviderHeartbeatRequest {
                            status,
                            capacity,
                            active_jobs,
                            models: None,
                        },
                    )
                    .await?;
                print_json(&response)
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
struct UpdateProviderPriceRequest {
    model: String,
    solai_per_hour: String,
}

#[derive(Serialize)]
struct UpdateProviderScheduleRequest {
    available_from: String,
    available_to: String,
    timezone: Option<String>,
}

#[derive(Debug, Clone, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ProviderStatusArg {
    Available,
    Unavailable,
    Disabled,
    Unknown,
}

#[derive(Serialize)]
pub(super) struct ProviderHeartbeatRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) status: Option<ProviderStatusArg>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) capacity: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) active_jobs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) models: Option<Vec<serde_json::Value>>,
}

pub(super) fn parse_positive_decimal(value: &str) -> Result<String, String> {
    let amount = value.trim();
    if amount.is_empty()
        || amount.starts_with('-')
        || amount.contains(['e', 'E'])
        || amount
            .parse::<f64>()
            .ok()
            .is_none_or(|parsed| parsed <= 0.0)
    {
        return Err("must be a positive decimal string".to_string());
    }
    Ok(amount.to_string())
}

pub(super) fn parse_hh_mm(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let valid = bytes.len() == 5
        && bytes[2] == b':'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 2 || byte.is_ascii_digit())
        && value[0..2].parse::<u8>().is_ok_and(|hour| hour < 24)
        && value[3..5].parse::<u8>().is_ok_and(|minute| minute < 60);
    if valid {
        Ok(value.to_string())
    } else {
        Err("must use HH:MM in 24-hour time".to_string())
    }
}

pub(super) fn validate_heartbeat_capacity(
    capacity: Option<u32>,
    active_jobs: Option<u32>,
) -> Result<(), anyhow::Error> {
    if capacity
        .zip(active_jobs)
        .is_some_and(|(capacity, active_jobs)| active_jobs > capacity)
    {
        bail!("active jobs cannot exceed provider capacity");
    }
    Ok(())
}

#[derive(Serialize)]
pub(super) struct ListProvidersQuery {
    pub(super) status: Option<String>,
    pub(super) model: Option<String>,
    pub(super) has_capacity: Option<bool>,
    pub(super) at_capacity: Option<bool>,
    pub(super) limit: u16,
    pub(super) offset: usize,
}
