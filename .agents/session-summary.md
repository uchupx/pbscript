# Session Context

## Date
2026-07-12

## Changes Made
1. **Fullscreen support** → asked about it, opted for normal window (removed overlay/transparent/decorations)
2. **Debug logging** → default log level from `warn` → `info` in `main.rs`
3. **GitHub Actions** → `runs-on` toggled between self-hosted/ubuntu, staying on `ubuntu-latest`
4. **Listener rewrite (Windows)** — first attempt:
    - Removed `rdev::listen` entirely (was failing silently on Windows)
    - Replaced with `SetWindowsHookEx(WH_MOUSE_LL)` for left-click detection
    - Replaced `RegisterHotKey` (kept failing even as admin) with `SetWindowsHookEx(WH_KEYBOARD_LL)` for F12 toggle
    - Both hooks run on same thread with `GetMessage` loop
    - Events sent via `mpsc` channel to processor thread
5. **Overlay mode** added then removed — user chose normal window
6. **RegisterHotKey failure** → even with admin rights, kept failing. Switched to keyboard hook.
7. **Listener rewrite v2 (Windows)** — hooks blocked by game anti-cheat:
    - Removed `SetWindowsHookEx(WH_MOUSE_LL)` + `SetWindowsHookEx(WH_KEYBOARD_LL)`
    - Removed: `GetMessageW` message pump, `UnhookWindowsHookEx`, `CallNextHookEx`, `GetModuleHandleW`
    - Replaced with `GetAsyncKeyState` polling loop (1ms interval) for both mouse + toggle key
    - Kept same `mpsc` channel → processor thread architecture
    - Kept `SIMULATING_CLICK` flag for feedback loop prevention
    - No new dependencies required (just raw FFI to `user32.dll`)

## Current State (Working)
- ✅ `GetAsyncKeyState` polling — detects left click + F12 toggle at 1ms interval
- ✅ Enigo input simulation — sends clicks & key presses correctly
- ✅ No hooks = no anti-cheat blocking

## Architecture (Windows)
```
[Polling Thread]
  loop (1ms sleep):
    GetAsyncKeyState(VK_LBUTTON)   → edge detect → mpsc::Sender<HookMsg>
    GetAsyncKeyState(toggle_key)   → edge detect → mpsc::Sender<HookMsg>
      ↓
[Processor Thread]
  HookMsg::Press → handle_lclick_press (mode dispatch)
  HookMsg::Release → handle_lclick_release (stop spray)
  HookMsg::Toggle → stop_spray + toggle state.active

[UI Thread (egui)]
  PbscriptApp::update — reads/writes state.config, shows status
```

## Raw Input API (Alternative if polling fails)
If `GetAsyncKeyState` polling also gets blocked or has issues:
- Use `RegisterRawInputDevices` with `RIDEV_INPUTSINK` flag
- Requires hidden message-only window (`HWND_MESSAGE`)
- Requires message pump + `WM_INPUT` handler
- Parses `RAWINPUT → RAWMOUSE → usButtonFlags` for button events
- More complex but even more reliable for anti-cheat games
- Reference: https://learn.microsoft.com/en-us/windows/win32/inputdev/using-raw-input

## TODO (next session)
- [ ] Test `GetAsyncKeyState` polling — push to master, download artifact, run `debug.bat`
- [ ] If polling fails → implement Raw Input API fallback (details above)
- [ ] Remove `rdev` from deps (still needed for macOS — or switch macOS to polling too)
- [ ] Add tray icon or minimize-to-tray
