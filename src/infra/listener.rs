use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use log::{debug, error, info};

use crate::app::state::AppState;
use crate::domain::entities::{Key, ModeType, SequenceStep, StepAction, SwitchMode};
use crate::domain::ports::InputEnginePort;

static SPRAYING: AtomicBool = AtomicBool::new(false);

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
                if key == KeyCode::BTN_LEFT {
                    if value == 1 && state.active.load(Ordering::Relaxed) {
                        debug!("Left click press, dispatching");
                        Self::handle_lclick_press(&state, &engine);
                    } else if value == 0 {
                        Self::handle_lclick_release(&state);
                    }
                } else if key == toggle(&state) {
                    if value == 1 {
                        Self::stop_spray();
                        let old = state.active.fetch_xor(true, Ordering::Relaxed);
                        info!("Toggle key pressed, active: {} -> {}", old, !old);
                    }
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

    // ---- Non-Linux (Windows / macOS): rdev listen (channel-based, avoids blocking inside hook) ----

    #[cfg(not(target_os = "linux"))]
    fn spawn_other(state: Arc<AppState>, engine: Arc<dyn InputEnginePort>) {
        use rdev::{listen, Event, EventType};

        let (tx, rx) = mpsc::channel::<Event>();

        // Thread 1: rdev listen — must NOT block inside the callback (runs in hook context on Windows)
        std::thread::spawn(move || {
            let result = listen(move |event: Event| {
                let _ = tx.send(event);
            });
            error!("listen() returned: {:?}", result);
        });

        // Thread 2: event processing — can block freely
        info!("Listener thread started (rdev, channel-based)");
        std::thread::spawn(move || {
            for event in rx {
                match event.event_type {
                    EventType::ButtonPress(rdev::Button::Left) => {
                        if state.active.load(Ordering::Relaxed) {
                            debug!("Left click press, dispatching");
                            Self::handle_lclick_press(&state, &engine);
                        }
                    }
                    EventType::ButtonRelease(rdev::Button::Left) => {
                        Self::handle_lclick_release(&state);
                    }
                    EventType::KeyPress(key) => {
                        let current_toggle = Self::toggle_key_rdev(&state);
                        if key == current_toggle {
                            Self::stop_spray();
                            let old = state.active.fetch_xor(true, Ordering::Relaxed);
                            info!("Toggle key pressed, active: {} -> {}", old, !old);
                        }
                    }
                    _ => {}
                }
            }
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

    // ---- Shared mode dispatch ----

    fn handle_lclick_press(state: &Arc<AppState>, engine: &Arc<dyn InputEnginePort>) {
        let config = state.config.lock().unwrap();
        let mode = config.current_mode;
        drop(config);

        match mode {
            ModeType::Sniper => Self::execute_sequence_sniper(state, &**engine),
            ModeType::Shotgun => Self::execute_sequence_shotgun(state, &**engine),
            ModeType::ArSmg => Self::start_spray(state.clone(), engine.clone()),
        }
    }

    fn handle_lclick_release(state: &AppState) {
        let config = state.config.lock().unwrap();
        let mode = config.current_mode;
        drop(config);
        if mode == ModeType::ArSmg {
            Self::stop_spray();
        }
    }

    // ---- Sniper: 4-step scope -> shoot -> unscope -> switch ----

    fn execute_sequence_sniper(state: &AppState, engine: &dyn InputEnginePort) {
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

    // ---- Shotgun: 2-step shoot -> switch ----

    fn execute_sequence_shotgun(state: &AppState, engine: &dyn InputEnginePort) {
        let config = state.config.lock().unwrap();
        let mode = config.switch_mode;
        let tembak_delay = config.shotgun_tembak_delay_ms;
        let ganti_delay = config.shotgun_ganti_delay_ms;
        drop(config);

        debug!("Shotgun: mode={:?}", mode);
        engine.left_click();
        if tembak_delay > 0 {
            std::thread::sleep(Duration::from_millis(tembak_delay as u64));
        }
        Self::do_switch(engine, mode);
        if ganti_delay > 0 {
            std::thread::sleep(Duration::from_millis(ganti_delay as u64));
        }
    }

    // ---- AR/SMG: hold-to-spray with optional recoil pull ----

    fn start_spray(state: Arc<AppState>, engine: Arc<dyn InputEnginePort>) {
        let config = state.config.lock().unwrap();
        let delay = config.ar_delay_ms;
        let recoil_enabled = config.ar_recoil_enabled;
        let recoil_pixels = config.ar_recoil_pixels;
        drop(config);

        debug!("Spray start: delay={}ms, recoil={} pixels={}", delay, recoil_enabled, recoil_pixels);
        SPRAYING.store(true, Ordering::Relaxed);
        std::thread::spawn(move || {
            while SPRAYING.load(Ordering::Relaxed) {
                engine.left_click();
                if recoil_enabled {
                    engine.move_mouse_relative(0, recoil_pixels);
                }
                if delay > 0 {
                    std::thread::sleep(Duration::from_millis(delay as u64));
                }
            }
            debug!("Spray stopped");
        });
    }

    fn stop_spray() {
        SPRAYING.store(false, Ordering::Relaxed);
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
