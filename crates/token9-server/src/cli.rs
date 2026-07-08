use std::io::{self, BufRead, Write};
use std::str::FromStr;

use clap::{Parser, Subcommand};

use crate::config::Dialect;
use crate::hosts;
use crate::store::sqlite::SqliteStore;

#[derive(Parser, Debug)]
#[command(name = "token9", about = "Local LLM API router & token meter")]
pub struct Cli {
    /// Path to bootstrap config.toml
    #[arg(long, default_value = "~/.Oraculo/config/token9/config.toml", global = true)]
    pub config: String,
    /// Override the listen port
    #[arg(long, global = true)]
    pub port: Option<u16>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run the gateway (default)
    Serve,
    /// Manage providers
    Provider {
        #[command(subcommand)]
        action: ProviderCmd,
    },
    /// Manage logical models
    Model {
        #[command(subcommand)]
        action: ModelCmd,
    },
    /// Manage the /etc/hosts friendly-domain entry
    Hosts {
        #[command(subcommand)]
        action: HostsCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProviderCmd {
    /// Add or update a provider
    Add {
        #[arg(long)]
        name: String,
        #[arg(long)]
        base_url: String,
        #[arg(long)]
        dialect: String,
        /// API key; omit or pass `-` to read from stdin (kept out of shell history)
        #[arg(long)]
        api_key: Option<String>,
    },
    /// List providers (API key masked)
    List,
    /// Remove a provider (cascades to its models)
    Rm {
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ModelCmd {
    /// Add or update a logical model
    Add {
        #[arg(long)]
        model_id: String,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        real_model: String,
        #[arg(long, default_value_t = false)]
        inject_usage: bool,
    },
    /// List logical models
    List,
    /// Remove a logical model
    Rm {
        #[arg(long)]
        model_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum HostsCmd {
    /// Add the 127.0.0.1 -> domain entry to /etc/hosts
    Install,
    /// Show whether the entry is present
    Status,
}

fn read_secret_from_stdin(prompt: &str) -> anyhow::Result<Option<String>> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim();
    Ok(if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    })
}

pub async fn run_provider(store: &SqliteStore, cmd: ProviderCmd) -> anyhow::Result<()> {
    match cmd {
        ProviderCmd::Add {
            name,
            base_url,
            dialect,
            api_key,
        } => {
            let dialect = Dialect::from_str(&dialect).map_err(|e| anyhow::anyhow!(e))?;
            let key = match api_key.as_deref() {
                Some("-") | None => read_secret_from_stdin("API key (empty for none): ")?,
                Some(k) => Some(k.to_string()),
            };
            store
                .add_provider(&name, &base_url, dialect, key.as_deref())
                .await?;
            println!("provider `{name}` saved");
        }
        ProviderCmd::List => {
            let ps = store.list_providers().await?;
            if ps.is_empty() {
                println!("(no providers)");
            }
            for p in ps {
                let masked = match &p.api_key {
                    Some(k) if k.len() > 4 => format!("****{}", &k[k.len() - 4..]),
                    Some(_) => "****".to_string(),
                    None => "-".to_string(),
                };
                println!("{:<16} {:<10} {:<40} key={}", p.name, p.dialect, p.base_url, masked);
            }
        }
        ProviderCmd::Rm { name } => {
            let n = store.remove_provider(&name).await?;
            println!("removed {n} provider(s)");
        }
    }
    Ok(())
}

pub async fn run_model(store: &SqliteStore, cmd: ModelCmd) -> anyhow::Result<()> {
    match cmd {
        ModelCmd::Add {
            model_id,
            provider,
            real_model,
            inject_usage,
        } => {
            store
                .add_model(&model_id, &provider, &real_model, inject_usage)
                .await?;
            println!("model `{model_id}` -> {provider}/{real_model} saved");
        }
        ModelCmd::List => {
            let ms = store.list_models().await?;
            if ms.is_empty() {
                println!("(no models)");
            }
            for m in ms {
                println!(
                    "{:<24} -> {:<12} {:<24} inject_usage={}",
                    m.model_id, m.provider, m.real_model, m.inject_usage
                );
            }
        }
        ModelCmd::Rm { model_id } => {
            let n = store.remove_model(&model_id).await?;
            println!("removed {n} model(s)");
        }
    }
    Ok(())
}

pub fn run_hosts(domain: &str, cmd: HostsCmd) -> anyhow::Result<()> {
    match cmd {
        HostsCmd::Install => hosts::install(domain),
        HostsCmd::Status => hosts::status(domain),
    }
}
