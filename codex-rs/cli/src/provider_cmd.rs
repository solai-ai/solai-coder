use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use std::env;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Parser)]
#[command(bin_name = "solai provider")]
pub struct ProviderCli {
    #[command(subcommand)]
    subcommand: ProviderSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum ProviderSubcommand {
    /// Enable provider mode and start the local daemon.
    Enable,

    /// Disable provider mode and stop the local daemon.
    Disable,

    /// Show provider configuration, daemon status, detection and heartbeat.
    Status,

    /// Set a SOLAI/hour price for a model.
    Price(ProviderPriceArgs),

    /// Set the provider availability window.
    Schedule(ProviderScheduleArgs),

    /// Register the local provider in the local marketplace registry.
    Register(ProviderRegisterArgs),

    /// Probe a remote provider endpoint and add it to the marketplace registry.
    Probe(ProviderProbeArgs),

    /// List marketplace providers from the local registry.
    List(ProviderListArgs),

    /// Refresh registered providers by fetching signed heartbeats.
    Refresh(ProviderRefreshArgs),

    /// Select the best provider for a model and print a quote.
    Quote(ProviderQuoteArgs),

    /// Run a prompt on a selected marketplace provider.
    Run(ProviderRunArgs),

    /// Remove a provider from the marketplace registry.
    Remove(ProviderRemoveArgs),

    /// Run the provider daemon in the foreground.
    Daemon,
}

#[derive(Debug, Parser)]
struct ProviderPriceArgs {
    model: String,
    solai_per_hour: f64,
}

#[derive(Debug, Parser)]
struct ProviderScheduleArgs {
    #[arg(long)]
    from: String,

    #[arg(long)]
    to: String,
}

#[derive(Debug, Parser)]
struct ProviderRegisterArgs {
    #[arg(long)]
    endpoint: Option<String>,

    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Parser)]
struct ProviderProbeArgs {
    endpoint: String,

    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Parser)]
struct ProviderListArgs {
    #[arg(long)]
    model: Option<String>,

    #[arg(long)]
    max_price: Option<f64>,

    #[arg(long)]
    available: bool,

    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct ProviderRefreshArgs {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct ProviderQuoteArgs {
    #[arg(long)]
    model: String,

    #[arg(long)]
    max_price: Option<f64>,

    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct ProviderRunArgs {
    #[arg(long)]
    model: String,

    #[arg(long)]
    prompt: Option<String>,

    #[arg(long)]
    prompt_file: Option<String>,

    #[arg(long)]
    provider: Option<String>,

    #[arg(long)]
    max_price: Option<f64>,
}

#[derive(Debug, Parser)]
struct ProviderRemoveArgs {
    provider_public_key: String,
}

impl ProviderCli {
    pub async fn run(self) -> Result<()> {
        let mut args = Vec::new();
        match self.subcommand {
            ProviderSubcommand::Enable => args.push("enable".to_string()),
            ProviderSubcommand::Disable => args.push("disable".to_string()),
            ProviderSubcommand::Status => args.push("status".to_string()),
            ProviderSubcommand::Daemon => args.push("daemon".to_string()),
            ProviderSubcommand::Price(ProviderPriceArgs {
                model,
                solai_per_hour,
            }) => {
                args.push("price".to_string());
                args.push(model);
                args.push(solai_per_hour.to_string());
            }
            ProviderSubcommand::Schedule(ProviderScheduleArgs { from, to }) => {
                args.push("schedule".to_string());
                args.push("--from".to_string());
                args.push(from);
                args.push("--to".to_string());
                args.push(to);
            }
            ProviderSubcommand::Register(ProviderRegisterArgs { endpoint, name }) => {
                args.push("register".to_string());
                if let Some(endpoint) = endpoint {
                    args.push("--endpoint".to_string());
                    args.push(endpoint);
                }
                if let Some(name) = name {
                    args.push("--name".to_string());
                    args.push(name);
                }
            }
            ProviderSubcommand::Probe(ProviderProbeArgs { endpoint, name }) => {
                args.push("probe".to_string());
                args.push(endpoint);
                if let Some(name) = name {
                    args.push("--name".to_string());
                    args.push(name);
                }
            }
            ProviderSubcommand::List(ProviderListArgs {
                model,
                max_price,
                available,
                json,
            }) => {
                args.push("list".to_string());
                if let Some(model) = model {
                    args.push("--model".to_string());
                    args.push(model);
                }
                if let Some(max_price) = max_price {
                    args.push("--max-price".to_string());
                    args.push(max_price.to_string());
                }
                if available {
                    args.push("--available".to_string());
                }
                if json {
                    args.push("--json".to_string());
                }
            }
            ProviderSubcommand::Refresh(ProviderRefreshArgs { json }) => {
                args.push("refresh".to_string());
                if json {
                    args.push("--json".to_string());
                }
            }
            ProviderSubcommand::Quote(ProviderQuoteArgs {
                model,
                max_price,
                json,
            }) => {
                args.push("quote".to_string());
                args.push("--model".to_string());
                args.push(model);
                if let Some(max_price) = max_price {
                    args.push("--max-price".to_string());
                    args.push(max_price.to_string());
                }
                if json {
                    args.push("--json".to_string());
                }
            }
            ProviderSubcommand::Run(ProviderRunArgs {
                model,
                prompt,
                prompt_file,
                provider,
                max_price,
            }) => {
                args.push("run".to_string());
                args.push("--model".to_string());
                args.push(model);
                if let Some(prompt) = prompt {
                    args.push("--prompt".to_string());
                    args.push(prompt);
                }
                if let Some(prompt_file) = prompt_file {
                    args.push("--prompt-file".to_string());
                    args.push(prompt_file);
                }
                if let Some(provider) = provider {
                    args.push("--provider".to_string());
                    args.push(provider);
                }
                if let Some(max_price) = max_price {
                    args.push("--max-price".to_string());
                    args.push(max_price.to_string());
                }
            }
            ProviderSubcommand::Remove(ProviderRemoveArgs {
                provider_public_key,
            }) => {
                args.push("remove".to_string());
                args.push(provider_public_key);
            }
        }

        run_provider_node_cli(args).await
    }
}

pub(crate) async fn run_provider_node_cli(args: Vec<String>) -> Result<()> {
    let script = resolve_provider_cli()?;
    let status = Command::new("node")
        .arg(&script)
        .args(args)
        .status()
        .await
        .with_context(|| format!("failed to run Node.js provider CLI at {}", script.display()))?;

    if !status.success() {
        bail!("provider command exited with status {status}");
    }

    Ok(())
}

fn resolve_provider_cli() -> Result<PathBuf> {
    if let Ok(path) = env::var("SOLAI_PROVIDER_CLI") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        bail!(
            "SOLAI_PROVIDER_CLI does not point to a file: {}",
            path.display()
        );
    }

    let mut roots = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        roots.extend(cwd.ancestors().map(PathBuf::from));
    }
    roots.extend(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .map(PathBuf::from),
    );

    for root in roots {
        let candidate = root.join("provider").join("src").join("cli.js");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    bail!(
        "could not find provider/src/cli.js; run from the SOLAI workspace or set SOLAI_PROVIDER_CLI"
    )
}
