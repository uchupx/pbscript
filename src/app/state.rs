use crate::domain::entities::SwitchMode;

/// Persistent app configuration (serde-serialized to TOML).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub buka_delay_ms: u32,
    pub tembak_delay_ms: u32,
    pub tutup_delay_ms: u32,
    pub ganti_delay_ms: u32,
    pub switch_mode: SwitchMode,
    pub toggle_key: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            buka_delay_ms: 70,
            tembak_delay_ms: 30,
            tutup_delay_ms: 50,
            ganti_delay_ms: 50,
            switch_mode: SwitchMode::QQ,
            toggle_key: "F12".to_string(),
        }
    }
}

impl AppConfig {
    pub fn delays(&self) -> [u32; 4] {
        [
            self.buka_delay_ms,
            self.tembak_delay_ms,
            self.tutup_delay_ms,
            self.ganti_delay_ms,
        ]
    }
}

/// Runtime shared state between UI thread and listener thread.
pub struct AppState {
    /// Is macro mode active?
    pub active: std::sync::atomic::AtomicBool,
    /// Current config (shared via mutex).
    pub config: std::sync::Mutex<AppConfig>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            active: std::sync::atomic::AtomicBool::new(false),
            config: std::sync::Mutex::new(config),
        }
    }
}
