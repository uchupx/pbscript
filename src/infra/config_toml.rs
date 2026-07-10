use std::fs;
use std::path::PathBuf;

use crate::app::state::AppConfig;
use crate::domain::ports::ConfigPort;

pub struct ConfigToml {
    path: PathBuf,
    config: AppConfig,
}

impl ConfigToml {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pbscript");
        let path = config_dir.join("config.toml");
        let mut this = Self {
            path,
            config: AppConfig::default(),
        };
        this.load();
        this
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }
}

impl ConfigPort for ConfigToml {
    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.path) {
            if let Ok(parsed) = toml::from_str::<AppConfig>(&content) {
                self.config = parsed;
                return;
            }
        }
        // If load fails, create default and save it
        let _ = fs::create_dir_all(self.path.parent().unwrap());
        self.save();
    }

    fn save(&self) {
        let content = toml::to_string_pretty(&self.config).unwrap_or_default();
        let _ = fs::create_dir_all(self.path.parent().unwrap());
        let _ = fs::write(&self.path, content);
    }
}
