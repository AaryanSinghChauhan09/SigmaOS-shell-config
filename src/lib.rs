//! SigmaOS shell-config — declarative shell configuration manager.
//!
//! Manages bar, dock, launcher, and theming configuration for the SigmaOS
//! desktop shell (Quickshell-based). All configuration is declarative TOML;
//! this crate validates, merges, and applies it idempotently.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Errors produced by this component.
#[derive(Debug, thiserror::Error)]
pub enum ShellConfigError {
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// TOML parse/serialize error.
    #[error("toml error: {0}")]
    Toml(#[from] toml::ser::Error),
    #[error("toml parse error: {0}")]
    TomlDe(#[from] toml::de::Error),
    /// Validation failure.
    #[error("validation error: {0}")]
    Validation(String),
    /// Config file not found.
    #[error("config not found: {0}")]
    NotFound(PathBuf),
}

/// Panel/bar configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarConfig {
    pub position: BarPosition,
    pub height: u32,
    pub modules: Vec<BarModule>,
    pub autohide: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BarPosition {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarModule {
    pub name: String,
    pub enabled: bool,
    #[serde(default = "default_module_config")]
    pub config: toml::Value,
}

fn default_module_config() -> toml::Value {
    toml::Value::Table(Default::default())
}

/// Strict `#RRGGBB` check — length, `#` prefix, and hex digits only.
fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// Dock configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockConfig {
    pub position: BarPosition,
    pub icon_size: u32,
    pub autohide: bool,
    pub pinned: Vec<String>,
}

/// Launcher configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub provider: String,
    pub max_results: usize,
    pub fuzzy: bool,
}

/// Theme configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub accent: String,
    pub background: String,
    pub foreground: String,
    pub font: String,
}

/// Root shell configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    pub bar: BarConfig,
    pub dock: DockConfig,
    pub launcher: Option<LauncherConfig>,
    pub theme: ThemeConfig,
}

impl ShellConfig {
    /// Load and validate shell configuration from a TOML file.
    pub fn load(path: &Path) -> Result<Self, ShellConfigError> {
        let text = std::fs::read_to_string(path)
            .map_err(|_| ShellConfigError::NotFound(path.to_path_buf()))?;
        let cfg: ShellConfig = toml::from_str(&text)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate configuration invariants.
    pub fn validate(&self) -> Result<(), ShellConfigError> {
        if self.bar.height == 0 || self.bar.height > 200 {
            return Err(ShellConfigError::Validation(format!(
                "bar height {} out of range 1-200",
                self.bar.height
            )));
        }
        if self.dock.icon_size == 0 || self.dock.icon_size > 256 {
            return Err(ShellConfigError::Validation(format!(
                "dock icon size {} out of range 1-256",
                self.dock.icon_size
            )));
        }
        for (field, color) in [
            ("accent", &self.theme.accent),
            ("background", &self.theme.background),
            ("foreground", &self.theme.foreground),
        ] {
            if !is_hex_color(color) {
                return Err(ShellConfigError::Validation(format!(
                    "{field} must be #RRGGBB, got {color}"
                )));
            }
        }
        if let Some(l) = &self.launcher {
            if l.max_results == 0 || l.max_results > 500 {
                return Err(ShellConfigError::Validation(format!(
                    "launcher max_results {} out of range 1-500",
                    l.max_results
                )));
            }
        }
        Ok(())
    }

    /// Serialize to TOML.
    pub fn to_toml(&self) -> Result<String, ShellConfigError> {
        Ok(toml::to_string(self)?)
    }

    /// Default SigmaOS shell configuration.
    pub fn sigmaos_default() -> Self {
        Self {
            bar: BarConfig {
                position: BarPosition::Top,
                height: 32,
                modules: vec![
                    BarModule {
                        name: "workspaces".into(),
                        enabled: true,
                        config: toml::Value::Table(Default::default()),
                    },
                    BarModule {
                        name: "clock".into(),
                        enabled: true,
                        config: toml::Value::Table(Default::default()),
                    },
                    BarModule {
                        name: "system-tray".into(),
                        enabled: true,
                        config: toml::Value::Table(Default::default()),
                    },
                    BarModule {
                        name: "battery".into(),
                        enabled: true,
                        config: toml::Value::Table(Default::default()),
                    },
                    BarModule {
                        name: "network".into(),
                        enabled: true,
                        config: toml::Value::Table(Default::default()),
                    },
                ],
                autohide: false,
            },
            dock: DockConfig {
                position: BarPosition::Bottom,
                icon_size: 48,
                autohide: false,
                pinned: vec!["firefox".into(), "terminal".into(), "files".into()],
            },
            launcher: Some(LauncherConfig {
                provider: "fuzzel".into(),
                max_results: 50,
                fuzzy: true,
            }),
            theme: ThemeConfig {
                name: "sigmaos-dark".into(),
                accent: "#7aa2f7".into(),
                background: "#1a1b26".into(),
                foreground: "#c0caf5".into(),
                font: "monospace 11".into(),
            },
        }
    }

    /// Enable or disable a bar module by name. Errors when absent.
    pub fn set_module_enabled(
        &mut self,
        name: &str,
        enabled: bool,
    ) -> Result<(), ShellConfigError> {
        let module = self
            .bar
            .modules
            .iter_mut()
            .find(|m| m.name == name)
            .ok_or_else(|| ShellConfigError::Validation(format!("unknown module: {name}")))?;
        module.enabled = enabled;
        Ok(())
    }

    /// Apply configuration: write to the user's config dir atomically.
    pub fn apply(&self, config_dir: &Path) -> Result<PathBuf, ShellConfigError> {
        self.validate()?;
        std::fs::create_dir_all(config_dir)?;
        let path = config_dir.join("shell.toml");
        let tmp = config_dir.join("shell.toml.tmp");
        std::fs::write(&tmp, self.to_toml()?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(path)
    }
}

/// Locate the standard SigmaOS shell config path.
pub fn default_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "sigmaos", "sigmaos")
        .map(|p| p.config_dir().join("shell.toml"))
}

/// Locate the shell config directory.
pub fn default_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "sigmaos", "sigmaos")
        .map(|p| p.config_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors_must_be_valid() {
        let mut cfg = ShellConfig::sigmaos_default();
        cfg.validate().unwrap();
        cfg.theme.accent = "#gggggg".into();
        assert!(cfg.validate().is_err());
        cfg.theme.accent = "#7aa2f7".into();
        cfg.theme.background = "7aa2f7".into(); // missing '#'
        assert!(cfg.validate().is_err());
        cfg.theme.background = "#1a1b26ff".into(); // too long
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn set_module_enabled_toggles() {
        let mut cfg = ShellConfig::sigmaos_default();
        cfg.set_module_enabled("clock", false).unwrap();
        assert!(
            !cfg.bar
                .modules
                .iter()
                .find(|m| m.name == "clock")
                .unwrap()
                .enabled
        );
        cfg.set_module_enabled("clock", true).unwrap();
        assert!(cfg.set_module_enabled("ghost", true).is_err());
    }
}
