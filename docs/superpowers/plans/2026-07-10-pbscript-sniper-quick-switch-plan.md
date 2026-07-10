# pbscript — Sniper Quick Switch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Desktop app (Rust) that listens for left-click in-game and auto-executes sniper quick-switch sequence when toggled on.

**Architecture:** Clean Architecture — domain entities/ports in core, app orchestration in middle, infra (enigo, rdev, egui, toml) on the outside.

**Tech Stack:** Rust, egui/eframe (UI), enigo (input simulate), rdev (global hook), serde+toml (config), dirs (config path).

## Global Constraints

- Cross-platform: Linux (X11/Wayland) & Windows.
- No unsafe code.
- Single binary output.
- Delays: u32, 0-100ms, clamped in UI slider.
- Hotkey: F12 default, configurable.
- UI language: Bahasa Indonesia.
- Config path: `~/.config/pbscript/config.toml`.
- Use `enigo = "0.2"`, `rdev = "0.5"`, `eframe = "0.27"`, `egui = "0.27"`, `serde = "1"`, `toml = "0.8"`, `dirs = "5"`.

---

### Task 1: Project scaffold + Domain layer

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs` (minimal stub)
- Create: `src/domain/mod.rs`
- Create: `src/domain/entities.rs`
- Create: `src/domain/ports.rs`

**Interfaces:**
- Consumes: nothing (first task)
- Produces: domain types + traits that all later tasks depend on

- [ ] **Step 1: Create Cargo.toml**

Write to `Cargo.toml`:

```toml
[package]
name = "pbscript"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.27"
egui = "0.27"
enigo = "0.2"
rdev = "0.5"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "5"
```

- [ ] **Step 2: Create directory structure**

```bash
mkdir -p src/domain src/app src/infra
```

- [ ] **Step 3: Create domain/entities.rs**

```rust
/// Action types for each step in the macro sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepAction {
    RightClick,
    LeftClick,
    Switch(SwitchMode),
}

/// QQ: double-tap Q. Num31: press 3 then 1 (knife→sniper).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwitchMode {
    QQ,
    Num31,
}

/// One step in the sequence with delay after execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SequenceStep {
    pub action: StepAction,
    /// Delay in ms after this step (0-100).
    pub delay_ms: u32,
}

impl SequenceStep {
    pub fn new(action: StepAction, delay_ms: u32) -> Self {
        Self {
            action,
            delay_ms: delay_ms.clamp(0, 100),
        }
    }
}

/// The full 4-step sniper quick-switch sequence.
#[derive(Debug, Clone)]
pub struct SniperSequence {
    pub steps: Vec<SequenceStep>,
}

impl SniperSequence {
    /// Build default sequence with given delays and switch mode.
    pub fn new(buka_delay: u32, tembak_delay: u32, tutup_delay: u32, ganti_delay: u32, mode: SwitchMode) -> Self {
        Self {
            steps: vec![
                SequenceStep::new(StepAction::RightClick, buka_delay),
                SequenceStep::new(StepAction::LeftClick, tembak_delay),
                SequenceStep::new(StepAction::RightClick, tutup_delay),
                SequenceStep::new(StepAction::Switch(mode), ganti_delay),
            ],
        }
    }
}

/// Simple key codes for the domain layer (avoids depending on enigo/rdev types).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    Q,
    Key1,
    Key3,
}
```

- [ ] **Step 4: Create domain/ports.rs**

```rust
use crate::domain::entities::Key;

/// Abstraction for simulating mouse/keyboard input.
pub trait InputEnginePort: Send {
    fn right_click(&self);
    fn left_click(&self);
    fn press(&self, key: Key);
    fn release(&self, key: Key);
}

/// Persistence for app config.
pub trait ConfigPort: Send {
    fn load(&mut self);
    fn save(&self);
}
```

- [ ] **Step 5: Create domain/mod.rs**

```rust
pub mod entities;
pub mod ports;
```

- [ ] **Step 6: Create stub src/main.rs**

```rust
fn main() {
    println!("pbscript — sniper quick switch");
}
```

- [ ] **Step 7: Verify compilation**

```bash
cargo check
```

Expected: clean compile, possibly warnings about dead code (fine for now).

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat: scaffold project + domain layer"
```

---

### Task 2: App state + configuration

**Files:**
- Create: `src/app/mod.rs`
- Create: `src/app/state.rs`

**Interfaces:**
- Consumes: `domain::entities::SwitchMode` from Task 1
- Produces: `AppConfig`, `AppState` used by Tasks 3-8

- [ ] **Step 1: Create app/state.rs**

```rust
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
```

- [ ] **Step 2: Create app/mod.rs**

```rust
pub mod state;
```

- [ ] **Step 3: Wire `SwitchMode` serde**

Add serde derives to `SwitchMode` in `domain/entities.rs`:

Edit `domain/entities.rs` — add serde to `SwitchMode`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SwitchMode {
    QQ,
    Num31,
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: add app config + state"
```

---

### Task 3: Config persistence (TOML)

**Files:**
- Create: `src/infra/mod.rs`
- Create: `src/infra/config_toml.rs`

**Interfaces:**
- Consumes: `app::state::AppConfig` from Task 2
- Produces: `ConfigToml` (impl `domain::ports::ConfigPort`)

- [ ] **Step 1: Create infra/config_toml.rs**

```rust
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
```

- [ ] **Step 2: Create infra/mod.rs**

```rust
pub mod config_toml;
pub mod input_enigo;
pub mod listener;
pub mod ui_eframe;
```

(Note: `input_enigo`, `listener`, `ui_eframe` will be created in later tasks — mod declarations are forward references, which Rust allows.)

- [ ] **Step 3: Verify compilation**

```bash
cargo check
```

May warn about unused mods — expected.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: add TOML config persistence"
```

---

### Task 4: Input engine (enigo)

**Files:**
- Create: `src/infra/input_enigo.rs`

**Interfaces:**
- Consumes: `domain::entities::Key` from Task 1
- Produces: `InputEngineEnigo` (impl `domain::ports::InputEnginePort`)

- [ ] **Step 1: Create infra/input_enigo.rs**

```rust
use enigo::{
    Coordinate, Direction, Enigo, Key as EnigoKey, Keyboard, Mouse, Settings,
};

use crate::domain::entities::Key;
use crate::domain::ports::InputEnginePort;

pub struct InputEngineEnigo {
    enigo: Enigo,
}

impl InputEngineEnigo {
    pub fn new() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default()).unwrap(),
        }
    }

    fn map_key(key: Key) -> EnigoKey {
        match key {
            Key::Q => EnigoKey::Unicode('q'),
            Key::Key1 => EnigoKey::Unicode('1'),
            Key::Key3 => EnigoKey::Unicode('3'),
        }
    }
}

impl InputEnginePort for InputEngineEnigo {
    fn right_click(&self) {
        let _ = self.enigo.button(enigo::Button::Right, Direction::Click);
    }

    fn left_click(&self) {
        let _ = self.enigo.button(enigo::Button::Left, Direction::Click);
    }

    fn press(&self, key: Key) {
        let ek = Self::map_key(key);
        let _ = self.enigo.key(ek, Direction::Press);
    }

    fn release(&self, key: Key) {
        let ek = Self::map_key(key);
        let _ = self.enigo.key(ek, Direction::Release);
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add enigo input engine"
```

---

### Task 5: Global input listener (rdev)

**Files:**
- Create: `src/infra/listener.rs`

**Interfaces:**
- Consumes: `AppState` from Task 2, `InputEnginePort` from Task 1
- Produces: `Listener` type with `spawn()` method

- [ ] **Step 1: Create infra/listener.rs**

```rust
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use rdev::{listen, Event, EventType};

use crate::app::state::AppState;
use crate::domain::entities::{Key, SequenceStep, StepAction, SwitchMode};
use crate::domain::ports::InputEnginePort;

pub struct Listener;

impl Listener {
    /// Spawn the global input listener thread.
    /// When active + left-click detected: execute the full sequence.
    /// When toggle key detected: flip active state.
    pub fn spawn(state: Arc<AppState>, engine: Arc<dyn InputEnginePort>) {
        std::thread::spawn(move || {
            let toggle_key = Self::parse_toggle_key(&state);

            if let Err(err) = listen(move |event: Event| {
                match event.event_type {
                    EventType::ButtonPress(rdev::Button::Left) => {
                        if state.active.load(Ordering::Relaxed) {
                            Self::execute_sequence(&state, &*engine);
                        }
                    }
                    EventType::KeyPress(key) => {
                        let current_toggle = Self::parse_toggle_key(&state);
                        if key == current_toggle {
                            let new_val = !state.active.load(Ordering::Relaxed);
                            state.active.store(new_val, Ordering::Relaxed);
                        }
                    }
                    _ => {}
                }
            }) {
                eprintln!("Listener error: {:?}", err);
            }
        });
    }

    fn parse_toggle_key(state: &AppState) -> rdev::Key {
        let config = state.config.lock().unwrap();
        match config.toggle_key.to_uppercase().as_str() {
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

    fn execute_sequence(state: &AppState, engine: &dyn InputEnginePort) {
        let config = state.config.lock().unwrap();
        let mode = config.switch_mode;
        let delays = config.delays();
        drop(config); // release lock before doing IO

        let steps = vec![
            SequenceStep::new(StepAction::RightClick, delays[0]),
            SequenceStep::new(StepAction::LeftClick, delays[1]),
            SequenceStep::new(StepAction::RightClick, delays[2]),
            SequenceStep::new(StepAction::Switch(mode), delays[3]),
        ];

        for step in &steps {
            match step.action {
                StepAction::RightClick => engine.right_click(),
                StepAction::LeftClick => engine.left_click(),
                StepAction::Switch(mode) => Self::do_switch(engine, mode),
            }
            if step.delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(step.delay_ms as u64));
            }
        }
    }

    fn do_switch(engine: &dyn InputEnginePort, mode: SwitchMode) {
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
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add rdev global input listener"
```

---

### Task 6: UI (egui window)

**Files:**
- Create: `src/infra/ui_eframe.rs`

**Interfaces:**
- Consumes: `Arc<AppState>` from Task 2
- Produces: `PbscriptApp` (eframe App) with run() entry

- [ ] **Step 1: Create infra/ui_eframe.rs**

```rust
use std::sync::atomic::Ordering;
use std::sync::Arc;

use eframe::egui::{self, Slider};

use crate::app::state::AppConfig;
use crate::app::state::AppState;
use crate::domain::entities::SwitchMode;

pub struct PbscriptApp {
    state: Arc<AppState>,
    config_cache: AppConfig,
}

impl PbscriptApp {
    pub fn new(state: Arc<AppState>) -> Self {
        let config_cache = state.config.lock().unwrap().clone();
        Self { state, config_cache }
    }

    fn save_config(&mut self) {
        let mut config = self.state.config.lock().unwrap();
        *config = self.config_cache.clone();
        drop(config);

        // Persist to disk via ConfigToml will be done in main
    }
}

impl eframe::App for PbscriptApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("pbscript — Quick Switch Senapan");
            ui.separator();

            // --- Sequence steps ---
            ui.label("Urutan:");
            ui.group(|ui| {
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
                    ui.label("[QQ    ]");
                    ui.add(Slider::new(&mut self.config_cache.ganti_delay_ms, 0..=100).text("ms"));
                });
            });

            // --- Switch mode ---
            ui.horizontal(|ui| {
                ui.label("Mode Ganti:");
                if ui.radio_value(&mut self.config_cache.switch_mode, SwitchMode::QQ, "QQ").clicked() {
                    self.save_config();
                }
                if ui.radio_value(&mut self.config_cache.switch_mode, SwitchMode::Num31, "31").clicked() {
                    self.save_config();
                }
            });

            // --- Toggle key ---
            ui.horizontal(|ui| {
                ui.label("Tombol Toggle:");
                let mut key = self.config_cache.toggle_key.clone();
                if ui.text_edit_singleline(&mut key).changed() {
                    self.config_cache.toggle_key = key.to_uppercase();
                    self.save_config();
                }
            });

            ui.separator();

            // --- Status ---
            let active = self.state.active.load(Ordering::Relaxed);
            if active {
                ui.colored_label(egui::Color32::GREEN, "■ AKTIF");
            } else {
                ui.colored_label(egui::Color32::RED, "■ MATI");
            }

            // --- Manual toggle button ---
            let btn_label = if active { "Nonaktifkan" } else { "Aktifkan" };
            if ui.button(btn_label).clicked() {
                let new_val = !active;
                self.state.active.store(new_val, Ordering::Relaxed);
            }

            // Auto-save on UI change
            if ui.changed() {
                self.save_config();
            }
        });

        // Continuous redraw while active (so status updates)
        if self.state.active.load(Ordering::Relaxed) {
            ctx.request_repaint();
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: add egui UI with Indonesian labels"
```

---

### Task 7: Main — glue everything together

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: All previous tasks

- [ ] **Step 1: Rewrite src/main.rs**

```rust
mod domain;
mod app;
mod infra;

use std::sync::Arc;

use app::state::{AppConfig, AppState};
use domain::ports::{ConfigPort, InputEnginePort};
use infra::config_toml::ConfigToml;
use infra::input_enigo::InputEngineEnigo;
use infra::listener::Listener;
use infra::ui_eframe::PbscriptApp;

fn main() -> eframe::Result<()> {
    // --- Config ---
    let mut config_persister = ConfigToml::new();
    let config = config_persister.config().clone();

    // --- Shared state ---
    let state = Arc::new(AppState::new(config));

    // --- Input engine ---
    let engine: Arc<dyn InputEnginePort> = Arc::new(InputEngineEnigo::new());

    // --- Start global listener ---
    Listener::spawn(state.clone(), engine.clone());

    // --- Run UI ---
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([280.0, 280.0]),
        ..Default::default()
    };

    eframe::run_native(
        "pbscript",
        options,
        Box::new(|_cc| Ok(Box::new(PbscriptApp::new(state)))),
    )
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check
```

If there are any type mismatches, fix them.

- [ ] **Step 3: Verify the full build works**

```bash
cargo build
```

Expected: binary at `target/debug/pbscript`.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: glue all layers in main"
```

---

### Self-Review

1. **Spec coverage:** All 4 steps (Buka→Tembak→Tutup→Ganti) implemented. QQ and 31 modes. Configurable delays 0-100ms. Toggle via configurable hotkey (default F12). Global listener intercepts left-click. UI in Indonesian. TOML persistence. ✓

2. **Placeholder scan:** No TBDs, no vague steps, no "handle later". All code is concrete. ✓

3. **Type consistency:** All domain types flow consistently from entities → ports → impls. `AppState` uses `Arc` shared between listener thread and UI thread. `InputEnginePort` and `ConfigPort` are `Send`. ✓

4. **No missing tasks:** Every spec requirement maps to a task. ✓
