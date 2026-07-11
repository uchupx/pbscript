use std::sync::atomic::Ordering;
use std::sync::Arc;

use eframe::egui::{self, Slider};

use crate::app::state::AppConfig;
use crate::app::state::AppState;
use crate::domain::entities::{ModeType, SwitchMode};

pub struct PbscriptApp {
    state: Arc<AppState>,
    config_cache: AppConfig,
}

impl PbscriptApp {
    pub fn new(state: Arc<AppState>) -> Self {
        let config_cache = state.config.lock().unwrap().clone();
        Self {
            state,
            config_cache,
        }
    }

    fn save_config(&self) {
        let mut config = self.state.config.lock().unwrap();
        *config = self.config_cache.clone();
    }
}

impl eframe::App for PbscriptApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("pbscript");
            ui.separator();

            // --- Mode tabs ---
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.config_cache.current_mode, ModeType::Sniper, "Senapan");
                ui.selectable_value(&mut self.config_cache.current_mode, ModeType::ArSmg, "AR / SMG");
                ui.selectable_value(&mut self.config_cache.current_mode, ModeType::Shotgun, "Shotgun");
            });
            ui.separator();

            match self.config_cache.current_mode {
                ModeType::Sniper => {
                    // --- Sequence steps ---
                    ui.label("Urutan:");
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("1. Buka");
                            ui.label("[R-Click]");
                            ui.add(Slider::new(&mut self.config_cache.buka_delay_ms, 0..=100).text("ms"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("2. Tembak");
                            ui.label("[L-Click]");
                            ui.add(Slider::new(&mut self.config_cache.tembak_delay_ms, 0..=100).text("ms"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("3. Tutup");
                            ui.label("[R-Click]");
                            ui.add(Slider::new(&mut self.config_cache.tutup_delay_ms, 0..=100).text("ms"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("4. Ganti");
                            ui.label(match self.config_cache.switch_mode {
                                SwitchMode::QQ => "[QQ]",
                                SwitchMode::Num31 => "[31]",
                            });
                            ui.add(Slider::new(&mut self.config_cache.ganti_delay_ms, 0..=100).text("ms"));
                        });
                    });

                    // --- Switch mode ---
                    ui.horizontal(|ui| {
                        ui.label("Mode Ganti:");
                        ui.radio_value(&mut self.config_cache.switch_mode, SwitchMode::QQ, "QQ");
                        ui.radio_value(&mut self.config_cache.switch_mode, SwitchMode::Num31, "31");
                    });

                    // --- Toggle key ---
                    ui.horizontal(|ui| {
                        ui.label("Tombol Toggle:");
                        let mut key = self.config_cache.toggle_key.clone();
                        if ui.text_edit_singleline(&mut key).changed() {
                            self.config_cache.toggle_key = key.to_uppercase();
                        }
                    });
                }

                ModeType::ArSmg => {
                    ui.label("Spray Control:");
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Delay Tembak");
                            ui.add(Slider::new(&mut self.config_cache.ar_delay_ms, 0..=100).text("ms"));
                        });
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.config_cache.ar_recoil_enabled, "Recoil");
                            if self.config_cache.ar_recoil_enabled {
                                ui.add(egui::Slider::new(&mut self.config_cache.ar_recoil_pixels, 0..=50).text("px"));
                            }
                        });
                    });
                    ui.label("Tekan & tahan L-Click untuk spray");
                }

                ModeType::Shotgun => {
                    ui.label("Urutan:");
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("1. Tembak");
                            ui.label("[L-Click]");
                            ui.add(Slider::new(&mut self.config_cache.shotgun_tembak_delay_ms, 0..=100).text("ms"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("2. Ganti");
                            ui.label(match self.config_cache.switch_mode {
                                SwitchMode::QQ => "[QQ]",
                                SwitchMode::Num31 => "[31]",
                            });
                            ui.add(Slider::new(&mut self.config_cache.shotgun_ganti_delay_ms, 0..=100).text("ms"));
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Mode Ganti:");
                        ui.radio_value(&mut self.config_cache.switch_mode, SwitchMode::QQ, "QQ");
                        ui.radio_value(&mut self.config_cache.switch_mode, SwitchMode::Num31, "31");
                    });
                }
            }

            ui.separator();

            // --- Status ---
            let active = self.state.active.load(Ordering::Relaxed);
            if active {
                ui.colored_label(egui::Color32::GREEN, "\u{25a0} AKTIF");
            } else {
                ui.colored_label(egui::Color32::RED, "\u{25a0} MATI");
            }

            // --- Manual toggle ---
            let btn_label = if active { "Nonaktifkan" } else { "Aktifkan" };
            if ui.button(btn_label).clicked() {
                let new_val = !active;
                self.state.active.store(new_val, Ordering::Relaxed);
            }

            // Auto-save config_cache to shared state every frame
            self.save_config();
        });

        // Keep repainting while active so status indicator updates
        if self.state.active.load(Ordering::Relaxed) {
            ctx.request_repaint();
        }
    }
}
