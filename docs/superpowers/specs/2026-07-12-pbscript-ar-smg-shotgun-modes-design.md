# AR/SMG & Shotgun Modes — Design Spec

**Date:** 2026-07-12
**Project:** pbscript
**Status:** Approved Design

## Overview

Add two new weapon macro modes (AR/SMG & Shotgun) to complement the existing Sniper mode. The user can switch between modes via tabs in the UI, and each mode has its own configurable parameters and trigger behavior.

## Architecture

**Approach:** Inline expansion — extend existing code with minimal new files. Same architecture as current sniper mode (no new traits or executors).

## Components

### 1. Domain — `src/domain/entities.rs`

Add enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModeType {
    Sniper,
    ArSmg,
    Shotgun,
}
```

### 2. Domain — `src/domain/ports.rs`

Add method to `InputEnginePort`:

```rust
fn move_mouse_relative(&self, dx: i32, dy: i32);
```

### 3. App Config — `src/app/state.rs`

New fields in `AppConfig`:

| Field | Type | Default | Range | Description |
|-------|------|---------|-------|-------------|
| `current_mode` | `ModeType` | `Sniper` | — | Selected active mode |
| `ar_delay_ms` | `u32` | `50` | 0-100 | Delay between shots in AR/SMG spray |
| `ar_recoil_enabled` | `bool` | `false` | — | Toggle recoil pull on/off |
| `ar_recoil_pixels` | `i32` | `10` | 0-50 | Pixels to move mouse down per shot |
| `shotgun_tembak_delay_ms` | `u32` | `30` | 0-100 | Delay after shotgun fire |
| `shotgun_ganti_delay_ms` | `u32` | `50` | 0-100 | Delay after shotgun switch |

`delays()` method stays for sniper compatibility.

### 4. Listener — `src/infra/listener.rs`

**Trigger behavior per mode:**

- **Sniper** (unchanged): L-Click press → execute 4-step sequence
- **Shotgun** (new): L-Click press → execute 2-step sequence: `LeftClick → Switch(mode)`
- **AR/SMG** (new):
  - L-Click press → start spray loop in new thread
  - L-Click release → stop spray loop via `AtomicBool`
  - Spray loop: `LeftClick → [if recoil: move_mouse(0, -pixels)] → sleep(delay) → repeat`
  - L-Click tap (quick press+release) → execute 1 shot (no loop if release comes before next shot fires)

**Linux (evdev):** Already distinguishes press (value=1) and release (value=0). Add check for value=0 → stop spray if in AR/SMG mode.

**Non-Linux (rdev):** `ButtonPress` / `ButtonRelease` events already separate. Add `ButtonRelease` handler.

### 5. Input Engine — `src/infra/input_enigo.rs`

Implement `move_mouse_relative()`:

```rust
fn move_mouse_relative(&self, dx: i32, dy: i32) {
    let _ = self.enigo.lock().unwrap().move_mouse(dx, dy, Coordinate::Relative);
}
```

### 6. UI — `src/infra/ui_eframe.rs`

**Tabs:** Enable all 3 tabs (`Sniper`, `AR/SMG`, `Shotgun`) as selectable values bound to `current_mode`.

**Sniper tab:** Unchanged (4 sliders + QQ/31 radio + toggle key).

**AR/SMG tab:**
- "Delay Tembak" slider (0-100ms)
- "Recoil:" checkbox "Aktif" + pixel slider (0-50)
- Info label: "Tekan & tahan L-Click untuk spray"

**Shotgun tab:**
- "1. Tembak [L-Click]" + delay slider
- "2. Ganti [QQ/31]" + delay slider
- Mode ganti radio (QQ/31) — shared with sniper config

## Data Flow

```
User clicks tab → config_cache.current_mode changes → auto-save to AppState
User adjusts sliders → config_cache updated → auto-save to AppState
User presses toggle key → AppState.active flips
L-Click event → listener checks:
  1. Is active? → no: ignore
  2. Match current_mode → execute sequence / start spray / stop spray
```

## Configuration Persistence

Config is auto-saved to `~/.config/pbscript/config.toml` every frame. No changes needed — new `AppConfig` fields are serde-derived and serialize to TOML automatically.

## Files Changed

1. `src/domain/entities.rs` — add `ModeType` enum
2. `src/domain/ports.rs` — add `move_mouse_relative` to trait
3. `src/app/state.rs` — add new config fields
4. `src/infra/listener.rs` — dispatch per mode, AR/SMG spray loop
5. `src/infra/input_enigo.rs` — implement `move_mouse_relative`
6. `src/infra/ui_eframe.rs` — enable shotgun/arsmg tabs + sliders

## Edge Cases

- **AR/SMG spray + mode switch:** If user switches mode while spraying, stop spray loop via AtomicBool.
- **Toggle off while spraying:** Same — stop spray loop.
- **Recoil = 0 pixels or disabled:** Just skip the mouse move.
- **Delay = 0:** No sleep between shots (maximum speed).
