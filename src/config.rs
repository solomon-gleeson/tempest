use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;
use crate::TempestError;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub auth: AuthConfig,
    pub paths: PathConfig,
    pub wine: WineConfig,
    pub launcher: LauncherConfig,
}

#[derive(Serialize, Deserialize, Default)]
pub struct AuthConfig {
    pub session_token: Option<String>,
    pub username: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PathConfig {
    pub wine_prefix: PathBuf,
    pub vortex_exe: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct WineConfig {
    pub binary: String,
    pub env: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct LauncherConfig {
    pub filter_wine_noise: bool,
    pub auto_update: bool,
}

impl Default for PathConfig {
    fn default() -> Self {
        let data = Config::data_dir();
        Self {
            wine_prefix: data.join("prefix"),
            vortex_exe: data.join("Vortex.exe"),
        }
    }
}

impl Default for WineConfig {
    fn default() -> Self {
        Self {
            binary: "wine".to_string(),
            env: HashMap::new(),
        }
    }
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            filter_wine_noise: true,
            auto_update: true,
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("tempest")
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("tempest")
    }

    pub fn load() -> Self {
        let path = Self::config_dir().join("config.toml");
        if !path.exists() {
            return Self::default();
        }
        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&contents).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), TempestError> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("config.toml");
        let contents = toml::to_string_pretty(self)
            .map_err(|e| TempestError::ConfigError(e.to_string()))?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let cfg = Config::default();
        let serialized = toml::to_string_pretty(&cfg).unwrap();
        let loaded: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(loaded.launcher.filter_wine_noise, cfg.launcher.filter_wine_noise);
        assert_eq!(loaded.launcher.auto_update, cfg.launcher.auto_update);
        assert_eq!(loaded.wine.binary, cfg.wine.binary);
    }
}
