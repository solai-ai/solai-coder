use anyhow::Result;
use clap::Parser;

use crate::provider_cmd::run_provider_node_cli;

#[derive(Debug, Parser)]
#[command(bin_name = "solai marketplace")]
pub struct SolaiMarketplaceCli {
    #[command(subcommand)]
    subcommand: SolaiMarketplaceSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum SolaiMarketplaceSubcommand {
    /// Register the local provider in the marketplace registry.
    Register(RegisterArgs),

    /// Probe a remote provider endpoint and add it to the marketplace registry.
    Probe(ProbeArgs),

    /// List marketplace providers from the local registry.
    List(ListArgs),

    /// Refresh registered providers by fetching signed heartbeats.
    Refresh(RefreshArgs),

    /// Select the best provider for a model and print a quote.
    Quote(QuoteArgs),

    /// Run a prompt on a selected marketplace provider.
    Run(RunArgs),

    /// Remove a provider from the marketplace registry.
    Remove(RemoveArgs),
}

#[derive(Debug, Parser)]
struct RegisterArgs {
    #[arg(long)]
    endpoint: Option<String>,

    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Parser)]
struct ProbeArgs {
    endpoint: String,

    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Parser)]
struct ListArgs {
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
struct RefreshArgs {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct QuoteArgs {
    #[arg(long)]
    model: String,

    #[arg(long)]
    max_price: Option<f64>,

    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct RunArgs {
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
struct RemoveArgs {
    provider_public_key: String,
}

impl SolaiMarketplaceCli {
    pub async fn run(self) -> Result<()> {
        let mut args = Vec::new();
        match self.subcommand {
            SolaiMarketplaceSubcommand::Register(RegisterArgs { endpoint, name }) => {
                args.push("register".to_string());
                push_optional_flag(&mut args, "--endpoint", endpoint);
                push_optional_flag(&mut args, "--name", name);
            }
            SolaiMarketplaceSubcommand::Probe(ProbeArgs { endpoint, name }) => {
                args.push("probe".to_string());
                args.push(endpoint);
                push_optional_flag(&mut args, "--name", name);
            }
            SolaiMarketplaceSubcommand::List(ListArgs {
                model,
                max_price,
                available,
                json,
            }) => {
                args.push("list".to_string());
                push_optional_flag(&mut args, "--model", model);
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
            SolaiMarketplaceSubcommand::Refresh(RefreshArgs { json }) => {
                args.push("refresh".to_string());
                if json {
                    args.push("--json".to_string());
                }
            }
            SolaiMarketplaceSubcommand::Quote(QuoteArgs {
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
            SolaiMarketplaceSubcommand::Run(RunArgs {
                model,
                prompt,
                prompt_file,
                provider,
                max_price,
            }) => {
                args.push("run".to_string());
                args.push("--model".to_string());
                args.push(model);
                push_optional_flag(&mut args, "--prompt", prompt);
                push_optional_flag(&mut args, "--prompt-file", prompt_file);
                push_optional_flag(&mut args, "--provider", provider);
                if let Some(max_price) = max_price {
                    args.push("--max-price".to_string());
                    args.push(max_price.to_string());
                }
            }
            SolaiMarketplaceSubcommand::Remove(RemoveArgs {
                provider_public_key,
            }) => {
                args.push("remove".to_string());
                args.push(provider_public_key);
            }
        }

        run_provider_node_cli(args).await
    }
}

fn push_optional_flag(args: &mut Vec<String>, flag: &str, value: Option<String>) {
    if let Some(value) = value {
        args.push(flag.to_string());
        args.push(value);
    }
}
