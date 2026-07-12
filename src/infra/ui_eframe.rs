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

/// Helper: draw a labeled slider with label on top, slider full width below.
fn step_slider(ui: &mut egui::Ui, label: &str, value: &mut u32, range: std::ops::RangeInclusive<u32>) {
    ui.label(label);
    ui.add_sized([ui.available_width(), 0.0], Slider::new(value, range).text("ms"));
    ui.add_space(2.0);
}

impl eframe::App for PbscriptApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(ctx, |ui| {
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
                    ui.strong("Urutan:");
                    ui.add_space(2.0);
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        step_slider(ui, "1. Buka — [Klik Kanan]", &mut self.config_cache.buka_delay_ms, 0..=100);
                        step_slider(ui, "2. Tembak — [Klik Kiri]", &mut self.config_cache.tembak_delay_ms, 0..=100);
                        step_slider(ui, "3. Tutup — [Klik Kanan]", &mut self.config_cache.tutup_delay_ms, 0..=100);

                        let ganti_label = match self.config_cache.switch_mode {
                            SwitchMode::QQ => "4. Ganti — [QQ]",
                            SwitchMode::Num31 => "4. Ganti — [31]",
                        };
                        step_slider(ui, ganti_label, &mut self.config_cache.ganti_delay_ms, 0..=100);
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Mode Ganti:");
                        ui.radio_value(&mut self.config_cache.switch_mode, SwitchMode::QQ, "QQ");
                        ui.radio_value(&mut self.config_cache.switch_mode, SwitchMode::Num31, "31");
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tombol Toggle:");
                        let mut key = self.config_cache.toggle_key.clone();
                        if ui.text_edit_singleline(&mut key).changed() {
                            self.config_cache.toggle_key = key.to_uppercase();
                        }
                    });
                }

                ModeType::ArSmg => {
                    ui.strong("Spray Control:");
                    ui.add_space(2.0);
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        step_slider(ui, "Delay Tembak", &mut self.config_cache.ar_delay_ms, 0..=100);

                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.config_cache.ar_recoil_enabled, "Recoil");
                            if self.config_cache.ar_recoil_enabled {
                                ui.add(egui::Slider::new(&mut self.config_cache.ar_recoil_pixels, 0..=50).text("px"));
                            }
                        });
                    });

                    ui.add_space(4.0);
                    ui.label("Tekan & tahan Klik Kiri untuk spray");
                }

                ModeType::Shotgun => {
                    ui.strong("Urutan:");
                    ui.add_space(2.0);
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        step_slider(ui, "1. Tembak — [Klik Kiri]", &mut self.config_cache.shotgun_tembak_delay_ms, 0..=100);

                        let ganti_label = match self.config_cache.switch_mode {
                            SwitchMode::QQ => "2. Ganti — [QQ]",
                            SwitchMode::Num31 => "2. Ganti — [31]",
                        };
                        step_slider(ui, ganti_label, &mut self.config_cache.shotgun_ganti_delay_ms, 0..=100);
                    });

                    ui.add_space(4.0);
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
            let (color, icon) = if active {
                (egui::Color32::GREEN, "\u{25cf} AKTIF")
            } else {
                (egui::Color32::RED, "\u{25cf} MATI")
            };
            ui.colored_label(color, icon);

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
