use std::{
    fs, io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;

use crate::{application::config_service::ConfigRepository, domain::config::AppConfig};

#[derive(Clone, Debug)]
pub struct TomlConfigRepository {
    path: PathBuf,
}

impl TomlConfigRepository {
    pub fn new() -> Self {
        let path = Self::config_path();
        Self { path }
    }

    fn config_path() -> PathBuf {
        let project = ProjectDirs::from("dev", "trabajo-paga", "cfdi-zip-processor")
            .map(|dirs| dirs.config_local_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        project.join("config.toml")
    }

    fn ensure_parent_dir(path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

impl Default for TomlConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigRepository for TomlConfigRepository {
    fn cargar(&self) -> io::Result<AppConfig> {
        if !self.path.exists() {
            return Ok(AppConfig::default());
        }

        let bytes = fs::read(&self.path)?;
        let text = String::from_utf8_lossy(&bytes);
        match toml::from_str::<AppConfig>(&text) {
            Ok(mut config) => {
                config.version = config.version.max(1);
                config.reordenar_version();
                Ok(config)
            }
            Err(_) => {
                let backup = self.path.with_extension("toml.bak");
                let _ = fs::copy(&self.path, &backup);
                let default = AppConfig::default();
                let _ = self.guardar(&default);
                Ok(default)
            }
        }
    }

    fn guardar(&self, config: &AppConfig) -> io::Result<()> {
        Self::ensure_parent_dir(&self.path)?;
        let tmp = self.path.with_extension("toml.tmp");
        let text = toml::to_string_pretty(config).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("config serializable: {error}"),
            )
        })?;
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}
