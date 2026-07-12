mod domain;
mod app;
mod infra;

use std::sync::Arc;

use log::info;

use app::state::AppState;
use domain::ports::InputEnginePort;
use infra::config_toml::ConfigToml;
use infra::input_enigo::InputEngineEnigo;
use infra::listener::Listener;
use infra::ui_eframe::PbscriptApp;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    info!("Starting pbscript...");

    // --- Config ---
    let config_persister = ConfigToml::new();
    let config = config_persister.config().clone();
    info!("Config loaded: {:?}", config);

    // --- Shared state ---
    let state = Arc::new(AppState::new(config));

    // --- Input engine ---
    info!("Initializing enigo input engine...");
    let engine: Arc<dyn InputEnginePort> = Arc::new(InputEngineEnigo::new());
    info!("Input engine ready");

    // --- Start global listener (spawns thread) ---
    info!("Spawning global listener...");
    Listener::spawn(state.clone(), engine.clone());

    // --- Run UI ---
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([300.0, 420.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "pbscript",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(PbscriptApp::new(state))
        }),
    )
}
