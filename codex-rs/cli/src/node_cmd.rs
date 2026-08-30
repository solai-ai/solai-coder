use std::collections::BTreeMap;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use reqwest::RequestBuilder;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[path = "node_admin.rs"]
mod admin;

use admin::ListEventsQuery;
use admin::ListMeteringQuery;
use admin::RuntimeModeArg;
use admin::UpdateRuntimeModeRequest;

const DEFAULT_NODE_URL: &str = "http://127.0.0.1:9876";
const API_KEY_HEADER: &str = "x-solai-node-key";

#[derive(Debug, Parser)]
pub struct NodeCli {
    /// SOLAI Node base URL.
    #[arg(long, env = "SOLAI_NODE_URL", default_value = DEFAULT_NODE_URL, global = true)]
    url: String,

    /// SOLAI Node API key. Prefer SOLAI_NODE_API_KEY over this flag.
    #[arg(
        long,
        env = "SOLAI_NODE_API_KEY",
        hide_env_values = true,
        global = true
    )]
    api_key: Option<String>,

    #[command(subcommand)]
    command: NodeCommand,
}

#[derive(Debug, clap::Subcommand)]
enum NodeCommand {
    /// Check whether SOLAI Node is reachable.
    Health,

    /// Print runtime and API version information.
    Version,

    /// Check whether SOLAI Node finished loading persistent state.
    Ready,

    /// Print provider, job, and metering counters.
    Metrics,

    /// Inspect the runtime mode and active queue counts.
    Runtime,

    /// Change whether the runtime starts queued jobs.
    Mode {
        /// New runtime mode.
        mode: RuntimeModeArg,
    },

    /// Inspect bounded operational events.
    Events {
        /// Event kind in snake_case, for example job_completed.
        #[arg(long)]
        kind: Option<String>,

        /// Only events for this provider.
        #[arg(long)]
        provider: Option<String>,

        /// Only events for this job id.
        #[arg(long)]
        job: Option<String>,

        /// Maximum records to return.
        #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=500))]
        limit: u16,

        /// Records to skip before applying the limit.
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },

    /// Inspect bounded usage and metering records.
    Metering {
        /// Only records for this tenant.
        #[arg(long)]
        tenant: Option<String>,

        /// Only records for this provider.
        #[arg(long)]
        provider: Option<String>,

        /// Only records for this job id.
        #[arg(long)]
        job: Option<String>,

        /// Workload in snake_case, for example chat_complete.
        #[arg(long)]
        workload: Option<String>,

        /// Only records for this model.
        #[arg(long)]
        model: Option<String>,

        /// Maximum records to return.
        #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=500))]
        limit: u16,

        /// Records to skip before applying the limit.
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },

    /// Discover and register a model provider.
    Probe {
        /// Provider address, for example 192.168.1.20:11434.
        address: String,
    },

    /// Submit a chat job and wait for the model response.
    Chat {
        /// User message sent to the model.
        prompt: String,

        /// Model exposed by the selected provider.
        #[arg(long, default_value = "gemma3:4b")]
        model: String,

        /// Provider id returned by `solai node probe`. Node routes automatically when omitted.
        #[arg(long)]
        provider: Option<String>,

        /// Tenant identifier recorded in job and metering data.
        #[arg(long, default_value = "solai-coder-local")]
        tenant: String,

        /// Maximum time to wait for the asynchronous job.
        #[arg(long, default_value_t = 600)]
        timeout_seconds: u64,

        /// Print the complete job detail JSON instead of only assistant text.
        #[arg(long)]
        json: bool,
    },
}

impl NodeCli {
    pub async fn run(self) -> Result<()> {
        let client = NodeClient::new(self.url, self.api_key)?;
        match self.command {
            NodeCommand::Health => client.print_get("/v1/health").await?,
            NodeCommand::Version => client.print_get("/v1/version").await?,
            NodeCommand::Ready => client.print_get("/v1/ready").await?,
            NodeCommand::Metrics => client.print_get("/v1/metrics").await?,
            NodeCommand::Runtime => client.print_get("/v1/runtime").await?,
            NodeCommand::Mode { mode } => {
                let response: Value = client
                    .post("/v1/runtime/mode", &UpdateRuntimeModeRequest { mode })
                    .await?;
                print_json(&response)?;
            }
            NodeCommand::Events {
                kind,
                provider,
                job,
                limit,
                offset,
            } => {
                let response: Value = client
                    .get_with_query(
                        "/v1/events",
                        &ListEventsQuery {
                            kind,
                            provider_id: provider,
                            job_id: job,
                            limit,
                            offset,
                        },
                    )
                    .await?;
                print_json(&response)?;
            }
            NodeCommand::Metering {
                tenant,
                provider,
                job,
                workload,
                model,
                limit,
                offset,
            } => {
                let response: Value = client
                    .get_with_query(
                        "/v1/metering",
                        &ListMeteringQuery {
                            tenant_id: tenant,
                            provider_id: provider,
                            job_id: job,
                            workload,
                            model,
                            limit,
                            offset,
                        },
                    )
                    .await?;
                print_json(&response)?;
            }
            NodeCommand::Probe { address } => {
                let response: Value = client
                    .post("/v1/providers/probe", &ProbeProviderRequest { address })
                    .await?;
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
            NodeCommand::Chat {
                prompt,
                model,
                provider,
                tenant,
                timeout_seconds,
                json,
            } => {
                client
                    .chat(ChatOptions {
                        prompt,
                        model,
                        provider_id: provider,
                        tenant_id: tenant,
                        timeout: Duration::from_secs(timeout_seconds),
                        output: if json {
                            ChatOutput::Json
                        } else {
                            ChatOutput::Text
                        },
                    })
                    .await?;
            }
        }
        Ok(())
    }
}

struct NodeClient {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

struct ChatOptions {
    prompt: String,
    model: String,
    provider_id: Option<String>,
    tenant_id: String,
    timeout: Duration,
    output: ChatOutput,
}

enum ChatOutput {
    Text,
    Json,
}

impl NodeClient {
    fn new(base_url: String, api_key: Option<String>) -> Result<Self> {
        let base_url = base_url.trim().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            bail!("SOLAI Node URL must not be empty");
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .context("failed to create SOLAI Node HTTP client")?;
        Ok(Self {
            http,
            base_url,
            api_key: api_key.filter(|key| !key.trim().is_empty()),
        })
    }

    async fn chat(&self, options: ChatOptions) -> Result<()> {
        let request = CreateJobRequest {
            workload: "chat_complete",
            model: options.model,
            provider_id: options.provider_id,
            tenant_id: options.tenant_id,
            input: ChatInput {
                messages: vec![ChatMessage {
                    role: "user",
                    content: options.prompt,
                }],
                keep_alive: "30m",
            },
            metadata: JobMetadata {
                source: "solai-coder",
            },
        };
        let created: JobResponse = self.post("/v1/jobs", &request).await?;
        eprintln!("SOLAI Node job {} queued", created.job_id);

        let started = Instant::now();
        loop {
            let detail: JobDetailResponse = self
                .get(&format!("/v1/jobs/{}/detail", created.job_id))
                .await?;
            match detail.job.status.as_str() {
                "completed" => {
                    match options.output {
                        ChatOutput::Text => println!("{}", assistant_content(&detail)?),
                        ChatOutput::Json => {
                            println!("{}", serde_json::to_string_pretty(&detail)?)
                        }
                    }
                    return Ok(());
                }
                "failed" | "cancelled" => {
                    let error = detail.error.unwrap_or(JobError {
                        code: detail.job.status,
                        message: "job ended without an error description".to_string(),
                        extra: BTreeMap::new(),
                    });
                    bail!("SOLAI Node job failed ({}): {}", error.code, error.message);
                }
                "queued" | "running" => {}
                status => bail!("SOLAI Node returned unknown job status `{status}`"),
            }
            if started.elapsed() >= options.timeout {
                bail!(
                    "timed out after {} seconds waiting for SOLAI Node job {}",
                    options.timeout.as_secs(),
                    created.job_id
                );
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.send(self.authorize(self.http.get(self.endpoint(path))))
            .await
    }

    async fn get_with_query<T, Q>(&self, path: &str, query: &Q) -> Result<T>
    where
        T: DeserializeOwned,
        Q: Serialize + ?Sized,
    {
        self.send(
            self.authorize(self.http.get(self.endpoint(path)))
                .query(query),
        )
        .await
    }

    async fn print_get(&self, path: &str) -> Result<()> {
        let response: Value = self.get(path).await?;
        print_json(&response)
    }

    async fn post<T, B>(&self, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.send(
            self.authorize(self.http.post(self.endpoint(path)))
                .json(body),
        )
        .await
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn authorize(&self, request: RequestBuilder) -> RequestBuilder {
        match self.api_key.as_ref() {
            Some(api_key) => request.header(API_KEY_HEADER, api_key),
            None => request,
        }
    }

    async fn send<T>(&self, request: RequestBuilder) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = request
            .send()
            .await
            .context("could not connect to SOLAI Node")?;
        let status = response.status();
        let body = response
            .text()
            .await
            .context("failed to read SOLAI Node response")?;
        if !status.is_success() {
            bail!("SOLAI Node returned {status}: {}", truncate(&body, 512));
        }
        serde_json::from_str(&body).context("SOLAI Node returned invalid JSON")
    }
}

fn assistant_content(detail: &JobDetailResponse) -> Result<&str> {
    detail
        .result
        .as_ref()
        .and_then(|result| result.output.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .context("completed SOLAI Node job did not contain assistant text")
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn print_json(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[derive(Serialize)]
struct ProbeProviderRequest {
    address: String,
}

#[derive(Serialize)]
struct CreateJobRequest {
    workload: &'static str,
    model: String,
    provider_id: Option<String>,
    tenant_id: String,
    input: ChatInput,
    metadata: JobMetadata,
}

#[derive(Serialize)]
struct ChatInput {
    messages: Vec<ChatMessage>,
    keep_alive: &'static str,
}

#[derive(Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct JobMetadata {
    source: &'static str,
}

#[derive(Debug, Deserialize, Serialize)]
struct JobResponse {
    job_id: String,
    status: String,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct JobDetailResponse {
    job: JobResponse,
    result: Option<JobResult>,
    error: Option<JobError>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct JobResult {
    output: Value,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct JobError {
    code: String,
    message: String,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[cfg(test)]
#[path = "node_cmd_tests.rs"]
mod tests;
