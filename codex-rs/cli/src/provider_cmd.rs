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
        }

        run_provider_node_cli(args).await
    }
}

async fn run_provider_node_cli(args: Vec<String>) -> Result<()> {
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
