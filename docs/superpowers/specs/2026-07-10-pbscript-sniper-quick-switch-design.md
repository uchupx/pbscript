# pbscript — Sniper Quick Switch Macro

## Overview
Desktop app (Rust) for FPS game PB. Records mouse input and automates sniper quick-switch sequence when activated. Toggle on/off via configurable global hotkey.

## Architecture — Clean Architecture

```
┌────────────────────────────────────────────────────────────┐
│                       DOMAIN LAYER                         │
│  no external deps                                          │
│                                                            │
│  entities:     SniperSequence, StepAction, SwitchMode       │
│  use cases:    ExecuteSequenceUseCase                       │
│  ports:        InputEnginePort (trait)                      │
│                ConfigPort (trait)                           │
│                SequenceListenerPort (trait)                 │
│                UiNotificationPort (trait)                   │
└──────────────────────┬─────────────────────────────────────┘
                       │ trait impls
┌──────────────────────┴─────────────────────────────────────┐
│                    APPLICATION LAYER                        │
│  orchestrator:     MacroOrchestrator                       │
│  state:            AppState (aktif/mati, config)            │
└──────────────────────┬─────────────────────────────────────┘
                       │ depends on
┌──────────────────────┴─────────────────────────────────────┐
│                  INFRASTRUCTURE LAYER                       │
│                                                            │
│  InputEngineEnigo  (enigo)   — mouse/keyboard simulation    │
│  ConfigToml        (toml)    — config persistence           │
│  InputListener     (rdev)    — global hotkey + mouse hook   │
│  UiEframe          (egui)    — desktop UI                   │
└─────────────────────────────────────────────────────────────┘
```

## Domain Layer (`src/domain/`)

### Entities
```rust
enum StepAction {
    RightClick,  // buka / tutup scope
    LeftClick,   // tembak
    Switch(SwitchMode),
}

// QQ: double-tap Q key
// Num31: press 3 (knife) then press 1 (sniper) — cancels bolt animation
enum SwitchMode {
    QQ,
    Num31,
}

struct SequenceStep {
    action: StepAction,
    delay_ms: u32,  // 0..100
}

struct SniperSequence {
    steps: Vec<SequenceStep>,
    // always 4 steps: Buka -> Tembak -> Tutup -> Ganti
}
```

### Ports (traits)
```rust
trait InputEnginePort {
    fn right_click(&self);
    fn left_click(&self);
    fn press_key(&self, key: Key);
    fn release_key(&self, key: Key);
    fn press(&self, key: Key); // press+release
}

trait ConfigPort {
    fn load() -> AppConfig;
    fn save(&self, config: &AppConfig);
}

trait SequenceListenerPort {
    fn on_left_click(&mut self, callback: Box<dyn Fn()>);
    fn on_toggle(&mut self, callback: Box<dyn Fn()>);
}

trait UiNotificationPort {
    fn notify_active(&self);
    fn notify_inactive(&self);
}
```

## Application Layer (`src/app/`)

### MacroOrchestrator
```rust
struct AppState {
    active: bool,
    config: AppConfig,
}

struct MacroOrchestrator {
    state: AppState,
    engine: Box<dyn InputEnginePort>,
    config: Box<dyn ConfigPort>,
    ui: Box<dyn UiNotificationPort>,
}

impl MacroOrchestrator {
    fn toggle(&mut self);       // aktif <-> mati
    fn execute_sequence(&self); // jalankan 4 steps
    fn left_click_detected(&self); // trigger
}
```

### How it works
1. `InputListener` (rdev) hooks global mouse left-click + keyboard hotkey.
2. When hotkey (default F12) pressed → `MacroOrchestrator.toggle()`.
3. When `active=true` and left-click detected → `MacroOrchestrator.execute_sequence()`:
    - Buka scope (R-Click) → delay → Tembak (L-Click) → delay → Tutup (R-Click) → delay → Ganti (QQ/31) → delay.
4. The "Ganti" step behavior based on mode:
   - **QQ**: press Q → release Q → (delay_ms/2) → press Q → release Q
   - **31**: press 3 → release 3 → (delay_ms/2) → press 1 → release 1
5. The original left-click is consumed (blocked) and re-issued as Tembak step.
5. When `active=false`, left-click passes through untouched.

## Infrastructure Layer (`src/infra/`)

### InputEngineEnigo
- Uses `enigo` crate for mouse/keyboard simulation.
- `right_click()`, `left_click()`, `press_key()`, `release_key()`.

### ConfigToml
- File: `~/.config/pbscript/config.toml`.
- Fields: delays (4 u32), switch_mode (string), toggle_key (string).
- Auto-save on every UI change.

### InputListener
- Uses `rdev` for global input hook.
- Listens for left-click (block + callback) and hotkey (toggle callback).

### UiEframe
- egui window, ~280x260px, dark theme.
- Bahasa Indonesia.
- 4 slider rows (0-100ms).
- QQ / 31 radio.
- Toggle key input.
- Status indicator (AKTIF/MATI).
- Config keybinding input.

## Files Structure

```
pbscript/
├── Cargo.toml
├── src/
│   ├── main.rs              — entry, glue layers
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── entities.rs      — SniperSequence, Step, SwitchMode
│   │   └── ports.rs         — all traits
│   ├── app/
│   │   ├── mod.rs
│   │   ├── state.rs         — AppState
│   │   └── orchestrator.rs  — MacroOrchestrator
│   └── infra/
│       ├── mod.rs
│       ├── input_enigo.rs   — InputEnginePort impl (enigo)
│       ├── config_toml.rs   — ConfigPort  impl (toml)
│       ├── listener.rs      — SequenceListenerPort impl (rdev)
│       └── ui_eframe.rs     — egui window
├── docs/
│   └── superpowers/specs/
│       └── 2026-07-10-pbscript-sniper-quick-switch-design.md
```

## Constraints
- Cross-platform: Linux (X11/Wayland) & Windows.
- No unsafe code unless enigo/rdev demands it internally.
- Single binary output.
- Delays: u32, 0-100ms, clamped in UI.
- Hotkey: F12 default, configurable.

## Future Swap (anti-cheat)
- To swap input engine: implement another `InputEnginePort` (e.g., serial HID).
- Change `main.rs` glue — that's it.
