//! CLI for sigmaos-shell-config.

use clap::{Parser, Subcommand};
use sigmaos_shell_config::{default_config_dir, default_config_path, ShellConfig};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "sigmaos-shell-config",
    about = "SigmaOS shell configuration manager"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a shell.toml file.
    Validate {
        /// Path to shell.toml (default: user config).
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Print the default SigmaOS shell configuration as TOML.
    Default,
    /// Apply a configuration file (or defaults) to the user config dir.
    Apply {
        /// Path to shell.toml (default: write built-in defaults).
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Print the effective config path.
    Path,
    /// Enable or disable a bar module in the user config.
    Module {
        /// Module name (e.g. clock, battery).
        name: String,
        /// "on" or "off".
        state: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Validate { file } => {
            let path = file
                .or_else(default_config_path)
                .ok_or("cannot determine config path")?;
            let cfg = ShellConfig::load(&path)?;
            println!(
                "valid: {} ({} bar modules, {} pinned apps)",
                path.display(),
                cfg.bar.modules.len(),
                cfg.dock.pinned.len()
            );
        }
        Commands::Default => {
            print!("{}", ShellConfig::sigmaos_default().to_toml()?);
        }
        Commands::Apply { file } => {
            let cfg = match file {
                Some(p) => ShellConfig::load(&p)?,
                None => ShellConfig::sigmaos_default(),
            };
            let dir = default_config_dir().ok_or("cannot determine config dir")?;
            let path = cfg.apply(&dir)?;
            println!("applied: {}", path.display());
        }
        Commands::Path => {
            println!(
                "{}",
                default_config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "(unknown)".into())
            );
        }
        Commands::Module { name, state } => {
            let enabled = match state.as_str() {
                "on" | "enable" | "true" => true,
                "off" | "disable" | "false" => false,
                _ => return Err(format!("state must be on|off, got {state}").into()),
            };
            let path = default_config_path().ok_or("cannot determine config path")?;
            let mut cfg = ShellConfig::load(&path)?;
            cfg.set_module_enabled(&name, enabled)?;
            cfg.apply(path.parent().unwrap())?;
            println!("{name} {}", if enabled { "enabled" } else { "disabled" });
        }
    }
    Ok(())
}
