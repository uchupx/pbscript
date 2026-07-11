# AR/SMG & Shotgun Modes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add AR/SMG (hold-to-spray with optional recoil) and Shotgun (quick-switch without scope) macro modes to pbscript.

**Architecture:** Inline expansion — extend existing pattern. New `ModeType` enum for mode selection; listener dispatches L-Click events to mode-specific logic; AR/SMG uses `AtomicBool`-guarded thread for press-to-spray / release-to-stop.

**Tech Stack:** Same as existing: egui, enigo, evdev/rdev, serde, TOML

## Global Constraints

- Delays clamped to 0-100ms (existing pattern)
- UI labels in Bahasa Indonesia (existing pattern)
- Recoil pixel range 0-50
- Config auto-saves every frame (existing pattern)
- Platform-conditional listener: evdev on Linux, rdev on others

---

### Task 1: Domain — ModeType enum + move_mouse_relative trait

**Files:**
- Modify: `src/domain/entities.rs`
- Modify: `src/domain/ports.rs`

**Interfaces:**
- Consumes: existing `StepAction`, `SwitchMode`, `Key` types
- Produces: `ModeType` enum, `InputEnginePort::move_mouse_relative(&self, dx: i32, dy: i32)`

- [ ] **Step 1: Add `ModeType` enum to `src/domain/entities.rs`**

Add after existing enums:

```rust
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ModeType {
    Sniper,
    ArSmg,
    Shotgun,
}
```

- [ ] **Step 2: Add `move_mouse_relative` to `InputEnginePort` trait in `src/domain/ports.rs`**

Add after `release`:

```rust
    fn move_mouse_relative(&self, dx: i32, dy: i32);
```

- [ ] **Step 3: Commit**

```bash
git add src/domain/entities.rs src/domain/ports.rs && git commit -m "feat: add ModeType enum and move_mouse_relative trait"
```

---

### Task 2: App state — new config fields for AR/SMG and Shotgun

**Files:**
- Modify: `src/app/state.rs`

**Interfaces:**
- Consumes: `ModeType` from Task 1
- Produces: `AppConfig` with new fields, `AppState` unchanged

- [ ] **Step 1: Add `current_mode` and weapon-specific fields to `AppConfig`**

In `src/app/state.rs`, add imports:

```rust
use crate::domain::entities::ModeType;
```

Add fields to `AppConfig` struct:

```rust
    pub current_mode: ModeType,
    pub ar_delay_ms: u32,
    pub ar_recoil_enabled: bool,
    pub ar_recoil_pixels: i32,
    pub shotgun_tembak_delay_ms: u32,
    pub shotgun_ganti_delay_ms: u32,
```

Update `Default` impl:

```rust
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            buka_delay_ms: 70,
            tembak_delay_ms: 30,
            tutup_delay_ms: 50,
            ganti_delay_ms: 50,
            switch_mode: SwitchMode::QQ,
            toggle_key: "F12".to_string(),
            current_mode: ModeType::Sniper,
            ar_delay_ms: 50,
            ar_recoil_enabled: false,
            ar_recoil_pixels: 10,
            shotgun_tembak_delay_ms: 30,
            shotgun_ganti_delay_ms: 50,
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/app/state.rs && git commit -m "feat: add AR/SMG and Shotgun config fields"
```

---

### Task 3: Input engine — implement move_mouse_relative in enigo

**Files:**
- Modify: `src/infra/input_enigo.rs`

**Interfaces:**
- Consumes: `InputEnginePort` trait from Task 1
- Produces: working `move_mouse_relative` impl

- [ ] **Step 1: Add `move_mouse_relative` to `InputEngineEnigo`**

Add import at top:

```rust
use enigo::{..., Coordinate};
```

If `Coordinate` not imported, add to the existing `use enigo::` line.

Add method to `impl InputEnginePort for InputEngineEnigo`:

```rust
    fn move_mouse_relative(&self, dx: i32, dy: i32) {
        let _ = self
            .enigo
            .lock()
            .unwrap()
            .move_mouse(dx, dy, Coordinate::Relative);
    }
```

- [ ] **Step 2: Commit**

```bash
git add src/infra/input_enigo.rs && git commit -m "feat: implement move_mouse_relative for enigo"
```

---

### Task 4: Listener — mode dispatch, AR/SMG spray loop, Shotgun sequence

**Files:**
- Modify: `src/infra/listener.rs`

**Interfaces:**
- Consumes: `AppState` (with new config), `InputEnginePort`
- Produces: working mode-specific execution

- [ ] **Step 1: Add `ModeType` import**

```rust
use crate::domain::entities::{Key, SequenceStep, StepAction, SwitchMode, ModeType};
```

- [ ] **Step 2: Implement `execute_sequence_sniper` (refactor existing inline code)**

Rename existing `execute_sequence` to `execute_sequence_sniper`. Keep body identical (4-step sniper sequence).

- [ ] **Step 3: Implement `execute_sequence_shotgun`**

```rust
    fn execute_sequence_shotgun(state: &AppState, engine: &dyn InputEnginePort) {
        let config = state.config.lock().unwrap();
        let mode = config.switch_mode;
        let tembak_delay = config.shotgun_tembak_delay_ms;
        let ganti_delay = config.shotgun_ganti_delay_ms;
        drop(config);

        engine.left_click();
        if tembak_delay > 0 {
            std::thread::sleep(Duration::from_millis(tembak_delay as u64));
        }
        Self::do_switch(engine, mode);
        if ganti_delay > 0 {
            std::thread::sleep(Duration::from_millis(ganti_delay as u64));
        }
    }
```

- [ ] **Step 4: Implement AR/SMG spray start/stop**

Add static `AtomicBool` for spray state at module level, and `Arc<dyn InputEnginePort>` param so the thread can own a clone:

```rust
use std::sync::atomic::AtomicBool;
static SPRAYING: AtomicBool = AtomicBool::new(false);
```

```rust
    fn start_spray(state: &Arc<AppState>, engine: Arc<dyn InputEnginePort>) {
        let config = state.config.lock().unwrap();
        let delay = config.ar_delay_ms;
        let recoil_enabled = config.ar_recoil_enabled;
        let recoil_pixels = config.ar_recoil_pixels;
        drop(config);

        let state_clone = state.clone();
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
        });
    }

    fn stop_spray() {
        SPRAYING.store(false, Ordering::Relaxed);
    }
```

- [ ] **Step 5: Modify `execute_sequence` to dispatch by mode**

Rename `execute_sequence` to `handle_lclick_press` and add dispatch:

```rust
    fn handle_lclick_press(state: &Arc<AppState>, engine: &Arc<dyn InputEnginePort>) {
        let config = state.config.lock().unwrap();
        let mode = config.current_mode;
        drop(config);

        match mode {
            ModeType::Sniper => Self::execute_sequence_sniper(state, &**engine),
            ModeType::Shotgun => Self::execute_sequence_shotgun(state, &**engine),
            ModeType::ArSmg => Self::start_spray(state, engine.clone()),
        }
    }
```

- [ ] **Step 6: Add release handler for AR/SMG**

```rust
    fn handle_lclick_release(state: &AppState) {
        let config = state.config.lock().unwrap();
        let mode = config.current_mode;
        drop(config);
        if mode == ModeType::ArSmg {
            Self::stop_spray();
        }
    }
```

- [ ] **Step 7: Wire into evdev listener (Linux)**

In `spawn_linux`, modify the event loop:

```rust
for (key, value) in rx {
    if key == KeyCode::BTN_LEFT {
        if value == 1 {
            if state.active.load(Ordering::Relaxed) {
                debug!("Left click press, dispatching");
                Self::handle_lclick_press(&state, &engine);
            }
        } else if value == 0 {
            Self::handle_lclick_release(&state);
        }
    } else if key == toggle(&state) {
        if value == 1 {
            // toggle off while spraying? stop spray first
            Self::stop_spray();
            let old = state.active.fetch_xor(true, Ordering::Relaxed);
            info!("Toggle key pressed, active: {} -> {}", old, !old);
        }
    }
}
```

- [ ] **Step 8: Wire into rdev listener (non-Linux)**

In `spawn_other`, modify `listen` callback:

```rust
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
        // ... existing toggle logic ...
        // Also stop spray on toggle:
        Self::stop_spray();
        // ... rest of toggle ...
    }
    _ => {}
}
```

- [ ] **Step 9: Update function signatures**

`execute_sequence` calls inside the file need to be updated to `handle_lclick_press`.

Also remove old `execute_sequence` call from toggle handler — currently sniper uses `execute_sequence` directly. Now it dispatches through `handle_lclick_press`.

- [ ] **Step 10: Build check**

```bash
cargo check 2>&1
```

Fix any compilation errors.

- [ ] **Step 11: Commit**

```bash
git add src/infra/listener.rs && git commit -m "feat: add mode dispatch, AR/SMG spray loop, Shotgun sequence"
```

---

### Task 5: UI — enable tabs + sliders for AR/SMG and Shotgun

**Files:**
- Modify: `src/infra/ui_eframe.rs`

**Interfaces:**
- Consumes: `AppConfig` new fields, `ModeType`
- Produces: Working UI for all 3 modes

- [ ] **Step 1: Import `ModeType`**

```rust
use crate::domain::entities::{SwitchMode, ModeType};
```

Remove the local `ModeTab` enum — replace with `ModeType`.

- [ ] **Step 2: Replace `ModeTab` with `ModeType`**

Replace:
```rust
enum ModeTab { Sniper, ArSmg, Shotgun }
```
With usage of `ModeType` from domain.

Change `selected_tab: ModeTab` to `selected_tab: ModeType` in `PbscriptApp`.

Change `ModeTab::Sniper` → `ModeType::Sniper` etc.

- [ ] **Step 3: Enable all 3 tabs (remove `add_enabled(false)`)**

Replace:
```rust
ui.selectable_value(&mut self.selected_tab, ModeTab::Sniper, "Senapan");
ui.add_enabled(false, egui::SelectableLabel::new(...));
ui.add_enabled(false, egui::SelectableLabel::new(...));
```

With:
```rust
ui.selectable_value(&mut self.selected_tab, ModeType::Sniper, "Senapan");
ui.selectable_value(&mut self.selected_tab, ModeType::ArSmg, "AR / SMG");
ui.selectable_value(&mut self.selected_tab, ModeType::Shotgun, "Shotgun");
```

- [ ] **Step 4: Add AR/SMG panel**

Replace `ModeTab::ArSmg | ModeTab::Shotgun => { coming_soon }` with:

```rust
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
```

- [ ] **Step 5: Add Shotgun panel**

```rust
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
```

- [ ] **Step 6: Wire `selected_tab` to `current_mode`**

In `save_config`, also sync `selected_tab` to `config_cache.current_mode`:

```rust
fn save_config(&self) {
    let mut config = self.state.config.lock().unwrap();
    // Sync tab to config for listener to read
    // Actually, the issue is config_cache.selected_tab != config_cache.current_mode
    // Best: just save config_cache.current_mode from selected_tab
    self.config_cache.current_mode = self.selected_tab;
    *config = self.config_cache.clone();
}
```

Or simplify: don't have separate `selected_tab` — just use `config_cache.current_mode` directly as the tab value.

Even simpler: remove `selected_tab` field, use `self.config_cache.current_mode` as the tab selector:

```rust
ui.selectable_value(&mut self.config_cache.current_mode, ModeType::Sniper, "Senapan");
```

And remove `selected_tab` from struct.

Then the `match` statement uses `self.config_cache.current_mode`.

- [ ] **Step 7: Build check**

```bash
cargo check 2>&1
```

Fix any compilation errors.

- [ ] **Step 8: Commit**

```bash
git add src/infra/ui_eframe.rs && git commit -m "feat: add AR/SMG and Shotgun mode UI tabs"
```

---

### Task 6: Integration — toggle-off stops spray

**Files:**
- Modify: `src/infra/listener.rs`

- [ ] **Step 1: Ensure toggle key stops spray in both Linux and non-Linux paths**

In the evdev toggle handler (where `toggle(&state)` is matched), add `Self::stop_spray()` before toggling `active`.

In the rdev toggle handler, add `Self::stop_spray()` before toggling.

These should already be in place from Task 4, but verify.

- [ ] **Step 2: Final build check**

```bash
cargo build 2>&1
```

- [ ] **Step 3: Commit**

```bash
git add src/infra/listener.rs && git commit -m "fix: stop spray on toggle key press"
```
