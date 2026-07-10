# Wayland/GNOME Compatibility Fix Implementation Plan

> **For agentic workers:** Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make pbscript work on Wayland/GNOME Linux while keeping Windows support.

**Architecture:** Replace `rdev::listen()` (X11-only) with `rdev::grab()` (evdev, works on X11+Wayland). Upgrade `enigo` from 0.2 (X11-only) to 0.6 with `wayland` feature.

**Tech Stack:** rdev 0.5 (unstable_grab feature), enigo 0.6 (wayland feature)

## Global Constraints

- Must still compile and work on Windows unchanged
- rdev `grab()` requires `unstable_grab` feature flag
- rdev `grab()` on Linux needs `input` group membership (doc note)
- enigo 0.6 wayland support is experimental (doc note)
- Consumed left-click events when active prevent double-fire

---

### Task 1: Update Cargo.toml dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Add unstable_grab feature to rdev, upgrade enigo to 0.6 with wayland**

```toml
rdev = { version = "0.5", features = ["unstable_grab"] }
enigo = { version = "0.6", features = ["wayland"] }
```

- [ ] **Run cargo check**

Run: `cargo check`
Expected: Dependency resolution succeeds

---

### Task 2: Rewrite listener to use rdev::grab instead of listen

**Files:**
- Modify: `src/infra/listener.rs`

**Key change:** `listen()` callback is `fn(Event)` — fire-and-forget. `grab()` callback is `fn(Event) -> Option<Event>` — return `None` to consume event, `Some(event)` to pass through.

- [ ] **Replace listen with grab and update callback signature**

Old:
```rust
use rdev::{listen, Event, EventType};
...
let result = listen(move |event: Event| { ... });
```

New:
```rust
use rdev::{grab, Event, EventType};
...
let result = grab(move |event: Event| -> Option<Event> { ... });
```

- [ ] **Update callback body to return Option<Event>**

When active + left-click: consume event (return None), execute sequence.
Toggle keys: always pass through (return Some(event)).
Everything else: pass through (return Some(event)).

Full callback body:
```rust
move |event: Event| -> Option<Event> {
    match event.event_type {
        EventType::ButtonPress(rdev::Button::Left) => {
            if state.active.load(Ordering::Relaxed) {
                Self::execute_sequence(&state, &*engine);
                None  // consume, no double-fire
            } else {
                Some(event)
            }
        }
        EventType::KeyPress(key) => {
            let current_toggle = Self::toggle_key(&state);
            if key == current_toggle {
                let old = state.active.fetch_xor(true, Ordering::Relaxed);
                info!("Toggle key pressed, active: {} -> {}", old, !old);
            }
            Some(event)
        }
        _ => Some(event)
    }
}
```

- [ ] **Verify unchanged imports are still correct**

Needed imports still present: `std::sync::atomic::Ordering`, `std::sync::Arc`, `std::time::Duration`, `log::{debug, error, info}`, app state, domain entities, ports.

- [ ] **Run cargo check**

Run: `cargo check`
Expected: Compiles with 0 errors

---

### Task 3: Update .agents files and docs

**Files:**
- Modify: `.agents/TODO`

- [ ] **Check if TODO needs updating**

No functional change to pending items, skip unless added new ones.

- [ ] **Update project memory**

Log the change: listener now uses grab(), enigo upgraded to 0.6 with wayland.
