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
    /// Manage tool-identification rules
    Tool {
        #[command(subcommand)]
        action: ToolCmd,
    },
    /// Manage settings stored in the DB (domain, port, ...)
    Settings {
        #[command(subcommand)]
        action: SettingsCmd,
    },
    /// Show the client endpoint URL + the sudo commands to make it port-less
    Endpoint,
    /// Manage the /etc/hosts friendly-domain entry
    Hosts {
        #[command(subcommand)]
        action: HostsCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum SettingsCmd {
    /// Set a setting (e.g. `settings set domain token9.test`, `settings set port 9527`)
    Set { key: String, value: String },
    /// Get a setting
    Get { key: String },
    /// List all settings
    List,
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
pub enum ToolCmd {
    /// Add a tool-identification rule (header contains pattern -> label)
    Add {
        #[arg(long)]
        label: String,
        /// Header to inspect (default: user-agent)
        #[arg(long, default_value = "user-agent")]
        header: String,
        /// Case-insensitive substring to match
        #[arg(long)]
        pattern: String,
        /// Lower = checked first
        #[arg(long, default_value_t = 100)]
        priority: i64,
    },
    /// List tool rules
    List,
    /// Remove a tool rule by id
    Rm {
        #[arg(long)]
        id: i64,
    },
    /// Show distinct real tool identifiers seen in traffic (discover unmapped)
    Observed,
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

pub async fn run_tool(store: &SqliteStore, cmd: ToolCmd) -> anyhow::Result<()> {
    match cmd {
        ToolCmd::Add {
            label,
            header,
            pattern,
            priority,
        } => {
            let id = store.add_tool_rule(&label, &header, &pattern, priority).await?;
            println!("tool rule #{id}: [{header} ~ \"{pattern}\"] -> {label}");
        }
        ToolCmd::List => {
            let rs = store.list_tool_rules().await?;
            if rs.is_empty() {
                println!("(no tool rules)");
            }
            for r in rs {
                println!(
                    "#{:<4} p{:<4} {:<16} {} ~ \"{}\"",
                    r.id, r.priority, r.label, r.header, r.pattern
                );
            }
        }
        ToolCmd::Rm { id } => {
            let n = store.remove_tool_rule(id).await?;
            println!("removed {n} tool rule(s)");
        }
        ToolCmd::Observed => {
            let obs = store.observed_tools().await?;
            if obs.is_empty() {
                println!("(no traffic yet)");
            }
            for o in obs {
                println!("{:<8} {:<6} {}", o.tool, o.requests, o.tool_raw);
            }
        }
    }
    Ok(())
}

pub async fn run_settings(store: &SqliteStore, cmd: SettingsCmd) -> anyhow::Result<()> {
    match cmd {
        SettingsCmd::Set { key, value } => {
            store.set_setting(&key, &value).await?;
            println!("{key} = {value} (restart `token9 serve` to apply)");
        }
        SettingsCmd::Get { key } => {
            match store.get_setting(&key).await? {
                Some(v) => println!("{v}"),
                None => println!("(unset)"),
            }
        }
        SettingsCmd::List => {
            let s = store.list_settings().await?;
            if s.is_empty() {
                println!("(no settings; using bootstrap defaults)");
            }
            for (k, v) in s {
                println!("{k:<12} {v}");
            }
        }
    }
    Ok(())
}

pub async fn run_endpoint(store: &SqliteStore, config: &crate::config::Config) -> anyhow::Result<()> {
    let domain = store.get_setting("domain").await?.unwrap_or(config.domain.clone());
    let port = store
        .get_setting("port")
        .await?
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(config.port);
    hosts::print_endpoint(&domain, port);
    Ok(())
}

pub fn run_hosts(domain: &str, cmd: HostsCmd) -> anyhow::Result<()> {
    match cmd {
        HostsCmd::Install => hosts::install(domain),
        HostsCmd::Status => hosts::status(domain),
    }
}
