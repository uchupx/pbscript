use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use log::{debug, error, info};

use crate::app::state::AppState;
use crate::domain::entities::{Key, SequenceStep, StepAction, SwitchMode};
use crate::domain::ports::InputEnginePort;

pub struct Listener;

impl Listener {
    pub fn spawn(state: Arc<AppState>, engine: Arc<dyn InputEnginePort>) {
        #[cfg(target_os = "linux")]
        Self::spawn_linux(state, engine);

        #[cfg(not(target_os = "linux"))]
        Self::spawn_other(state, engine);
    }

    // ---- Linux: evdev (non-exclusive, works on X11 + Wayland) ----

    #[cfg(target_os = "linux")]
    fn spawn_linux(state: Arc<AppState>, engine: Arc<dyn InputEnginePort>) {
        use evdev::{enumerate, Device, EventSummary, KeyCode};

        let devices: Vec<(_, Device)> = enumerate().collect();
        if devices.is_empty() {
            error!("No evdev devices found. Need 'input' group membership.");
            return;
        }

        let (tx, rx) = mpsc::channel::<(KeyCode, i32)>();

        for (_path, mut device) in devices {
            let tx = tx.clone();
            std::thread::spawn(move || {
                let _ = device.set_nonblocking(true);
                loop {
                    match device.fetch_events() {
                        Ok(events) => {
                            for event in events {
                                if let EventSummary::Key(_, key, value) = event.destructure() {
                                    let _ = tx.send((key, value));
                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(e) => {
                            debug!("evdev read error: {}", e);
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            });
        }

        drop(tx);

        info!("Listener started (evdev, non-exclusive)");
        std::thread::spawn(move || {
            let toggle = Self::toggle_key_evdev;
            for (key, value) in rx {
                if value != 1 {
                    continue;
                }
                if key == KeyCode::BTN_LEFT {
                    if state.active.load(Ordering::Relaxed) {
                        debug!("Left click detected, executing sequence");
                        Self::execute_sequence(&state, &*engine);
                    }
                } else if key == toggle(&state) {
                    let old = state.active.fetch_xor(true, Ordering::Relaxed);
                    info!("Toggle key pressed, active: {} -> {}", old, !old);
                }
            }
        });
    }

    #[cfg(target_os = "linux")]
    fn toggle_key_evdev(state: &AppState) -> evdev::KeyCode {
        use evdev::KeyCode;
        let config = state.config.lock().unwrap();
        let key = config.toggle_key.to_uppercase();
        drop(config);
        match key.as_str() {
            "F1" => KeyCode::KEY_F1,
            "F2" => KeyCode::KEY_F2,
            "F3" => KeyCode::KEY_F3,
            "F4" => KeyCode::KEY_F4,
            "F5" => KeyCode::KEY_F5,
            "F6" => KeyCode::KEY_F6,
            "F7" => KeyCode::KEY_F7,
            "F8" => KeyCode::KEY_F8,
            "F9" => KeyCode::KEY_F9,
            "F10" => KeyCode::KEY_F10,
            "F11" => KeyCode::KEY_F11,
            "F12" => KeyCode::KEY_F12,
            _ => KeyCode::KEY_F12,
        }
    }

    // ---- Non-Linux (Windows / macOS): rdev listen ----

    #[cfg(not(target_os = "linux"))]
    fn spawn_other(state: Arc<AppState>, engine: Arc<dyn InputEnginePort>) {
        use rdev::{listen, Event, EventType};

        info!("Listener thread started (rdev listen)");
        std::thread::spawn(move || {
            let result = listen(move |event: Event| {
                match event.event_type {
                    EventType::ButtonPress(rdev::Button::Left) => {
                        if state.active.load(Ordering::Relaxed) {
                            debug!("Left click detected, executing sequence");
                            Self::execute_sequence(&state, &*engine);
                        }
                    }
                    EventType::KeyPress(key) => {
                        let current_toggle = Self::toggle_key_rdev(&state);
                        if key == current_toggle {
                            let old = state.active.fetch_xor(true, Ordering::Relaxed);
                            info!("Toggle key pressed, active: {} -> {}", old, !old);
                        }
                    }
                    _ => {}
                }
            });
            error!("listen() returned: {:?}", result);
        });
    }

    #[cfg(not(target_os = "linux"))]
    fn toggle_key_rdev(state: &AppState) -> rdev::Key {
        let config = state.config.lock().unwrap();
        let key = config.toggle_key.to_uppercase();
        drop(config);
        match key.as_str() {
            "F1" => rdev::Key::F1,
            "F2" => rdev::Key::F2,
            "F3" => rdev::Key::F3,
            "F4" => rdev::Key::F4,
            "F5" => rdev::Key::F5,
            "F6" => rdev::Key::F6,
            "F7" => rdev::Key::F7,
            "F8" => rdev::Key::F8,
            "F9" => rdev::Key::F9,
            "F10" => rdev::Key::F10,
            "F11" => rdev::Key::F11,
            "F12" => rdev::Key::F12,
            _ => rdev::Key::F12,
        }
    }

    // ---- Shared sequence execution ----

    fn execute_sequence(state: &AppState, engine: &dyn InputEnginePort) {
        let config = state.config.lock().unwrap();
        let mode = config.switch_mode;
        let delays = config.delays();
        drop(config);

        debug!("Sequence: mode={:?}, delays={:?}", mode, delays);

        let steps = vec![
            SequenceStep::new(StepAction::RightClick, delays[0]),
            SequenceStep::new(StepAction::LeftClick, delays[1]),
            SequenceStep::new(StepAction::RightClick, delays[2]),
            SequenceStep::new(StepAction::Switch(mode), delays[3]),
        ];

        for (i, step) in steps.iter().enumerate() {
            debug!("Step {}: {:?} (delay={}ms)", i + 1, step.action, step.delay_ms);
            match step.action {
                StepAction::RightClick => engine.right_click(),
                StepAction::LeftClick => engine.left_click(),
                StepAction::Switch(mode) => Self::do_switch(engine, mode),
            }
            if step.delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(step.delay_ms as u64));
            }
        }
        debug!("Sequence done");
    }

    fn do_switch(engine: &dyn InputEnginePort, mode: SwitchMode) {
        debug!("Switch: {:?}", mode);
        match mode {
            SwitchMode::QQ => {
                engine.press(Key::Q);
                engine.release(Key::Q);
                std::thread::sleep(Duration::from_millis(10));
                engine.press(Key::Q);
                engine.release(Key::Q);
            }
            SwitchMode::Num31 => {
                engine.press(Key::Key3);
                engine.release(Key::Key3);
                std::thread::sleep(Duration::from_millis(10));
                engine.press(Key::Key1);
                engine.release(Key::Key1);
            }
        }
    }
}
