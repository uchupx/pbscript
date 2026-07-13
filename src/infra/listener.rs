use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use log::{error, info};
use log::debug;

use crate::app::state::AppState;
use crate::domain::entities::{Key, ModeType, SequenceStep, StepAction, SwitchMode};
use crate::domain::ports::InputEnginePort;

// ── Windows: Raw Input API (reads from HID, bypasses GetAsyncKeyState hooks) ──
#[cfg(windows)]
mod win32 {
    use std::sync::atomic::Ordering;
    use std::sync::mpsc::{self, Sender};
    use std::sync::{Arc, OnceLock};
    use std::thread;
    use log::{debug, info};
    use crate::app::state::AppState;
    use crate::domain::ports::InputEnginePort;
    use crate::infra::listener::Listener;

    // ── Win32 type aliases ──

    type HWND = *mut std::ffi::c_void;
    type HRAWINPUT = *mut std::ffi::c_void;
    type HINSTANCE = *mut std::ffi::c_void;
    type WPARAM = usize;
    type LPARAM = isize;
    type LRESULT = isize;
    type BOOL = i32;
    type UINT = u32;
    type WORD = u16;
    type DWORD = u32;
    type LONG = i32;
    type ATOM = u16;

    const TRUE: BOOL = 1;
    const FALSE: BOOL = 0;

    // ── Raw Input constants ──

    const RIDEV_INPUTSINK: DWORD = 0x00000100;
    const WM_INPUT: UINT = 0x00FF;
    const WM_DESTROY: UINT = 0x0002;
    const RID_INPUT: UINT = 0x10000003;
    const RIM_TYPEMOUSE: DWORD = 0;
    const RIM_TYPEKEYBOARD: DWORD = 1;
    const RI_MOUSE_LEFT_BUTTON_DOWN: WORD = 0x0001;
    const RI_MOUSE_LEFT_BUTTON_UP: WORD = 0x0002;
    const HID_USAGE_PAGE_GENERIC: WORD = 0x01;
    const HID_USAGE_MOUSE: WORD = 0x02;
    const HID_USAGE_KEYBOARD: WORD = 0x06;
    const RI_KEY_MAKE: WORD = 0;

    // ── Virtual key codes for toggle key ──

    const VK_F1: WORD = 0x70;  const VK_F2: WORD = 0x71;
    const VK_F3: WORD = 0x72;  const VK_F4: WORD = 0x73;
    const VK_F5: WORD = 0x74;  const VK_F6: WORD = 0x75;
    const VK_F7: WORD = 0x76;  const VK_F8: WORD = 0x77;
    const VK_F9: WORD = 0x78;  const VK_F10: WORD = 0x79;
    const VK_F11: WORD = 0x7A; const VK_F12: WORD = 0x7B;

    fn vk_from_string(s: &str) -> WORD {
        match s.to_uppercase().as_str() {
            "F1"=>VK_F1,"F2"=>VK_F2,"F3"=>VK_F3,"F4"=>VK_F4,
            "F5"=>VK_F5,"F6"=>VK_F6,"F7"=>VK_F7,"F8"=>VK_F8,
            "F9"=>VK_F9,"F10"=>VK_F10,"F11"=>VK_F11,"F12"=>VK_F12,
            _ => VK_F12,
        }
    }

    // ── Win32 structs ──

    #[repr(C)]
    struct RAWINPUTDEVICE {
        usUsagePage: WORD,
        usUsage: WORD,
        dwFlags: DWORD,
        hwndTarget: HWND,
    }

    #[repr(C)]
    struct RAWINPUTHEADER {
        dwType: DWORD,
        dwSize: DWORD,
        hDevice: HRAWINPUT,
        wParam: WPARAM,
    }

    #[repr(C)]
    struct RAWMOUSE {
        usFlags: WORD,
        usButtonFlags: WORD,
        usButtonData: WORD,
        ulRawButtons: DWORD,
        lLastX: LONG,
        lLastY: LONG,
        ulExtraInformation: DWORD,
    }

    #[repr(C)]
    struct RAWKEYBOARD {
        MakeCode: WORD,
        Flags: WORD,
        Reserved: WORD,
        VKey: WORD,
        Message: UINT,
        ExtraInformation: DWORD,
    }

    #[repr(C)]
    struct MSG {
        hwnd: HWND,
        message: UINT,
        wParam: WPARAM,
        lParam: LPARAM,
        time: DWORD,
        pt: i64,
    }

    #[repr(C)]
    struct WNDCLASSW {
        style: UINT,
        lpfnWndProc: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>,
        cbClsExtra: i32,
        cbWndExtra: i32,
        hInstance: HINSTANCE,
        hIcon: *mut std::ffi::c_void,
        hCursor: *mut std::ffi::c_void,
        hbrBackground: *mut std::ffi::c_void,
        lpszMenuName: *const u16,
        lpszClassName: *const u16,
    }

    // ── FFI functions ──

    #[link(name = "user32")]
    extern "system" {
        fn RegisterRawInputDevices(
            pRawInputDevices: *const RAWINPUTDEVICE,
            uiNumDevices: UINT,
            cbSize: DWORD,
        ) -> BOOL;
        fn GetRawInputData(
            hRawInput: HRAWINPUT,
            uiCommand: UINT,
            pData: *mut std::ffi::c_void,
            pcbSize: *mut UINT,
            cbSizeHeader: UINT,
        ) -> UINT;
        fn CreateWindowExW(
            dwExStyle: DWORD,
            lpClassName: *const u16,
            lpWindowName: *const u16,
            dwStyle: DWORD,
            x: i32, y: i32, nWidth: i32, nHeight: i32,
            hWndParent: HWND,
            hMenu: *mut std::ffi::c_void,
            hInstance: HINSTANCE,
            lpParam: *mut std::ffi::c_void,
        ) -> HWND;
        fn DefWindowProcW(
            hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM,
        ) -> LRESULT;
        fn GetMessageW(
            lpMsg: *mut MSG, hWnd: HWND,
            wMsgFilterMin: UINT, wMsgFilterMax: UINT,
        ) -> BOOL;
        fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
        fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> ATOM;
    }

    // ── Channel (same as before) ──

    #[derive(Clone, Copy)]
    enum HookMsg { Press, Release, Toggle }
    static HOOK_TX: OnceLock<Sender<HookMsg>> = OnceLock::new();

    // ── State for window proc ──

    static RAW_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

    // ── Window procedure ──

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_INPUT {
            handle_raw_input(lparam);
            return 0;
        }
        if msg == WM_DESTROY {
            // ponytail: no PostQuitMessage needed — GetMessageW returns 0
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    unsafe fn handle_raw_input(lparam: LPARAM) {
        // Get required buffer size
        let mut size: UINT = 0;
        GetRawInputData(
            lparam as HRAWINPUT,
            RID_INPUT,
            std::ptr::null_mut(),
            &mut size,
            std::mem::size_of::<RAWINPUTHEADER>() as UINT,
        );
        if size == 0 {
            return;
        }

        // Allocate and get actual data
        let mut buf = vec![0u8; size as usize];
        let written = GetRawInputData(
            lparam as HRAWINPUT,
            RID_INPUT,
            buf.as_mut_ptr() as *mut std::ffi::c_void,
            &mut size,
            std::mem::size_of::<RAWINPUTHEADER>() as UINT,
        );
        if written == 0 || written == u32::MAX {
            return;
        }

        let header = &*(buf.as_ptr() as *const RAWINPUTHEADER);

        if header.dwType == RIM_TYPEMOUSE
            && size >= std::mem::size_of::<RAWINPUTHEADER>() as UINT + std::mem::size_of::<RAWMOUSE>() as UINT
        {
            let mouse = &*(buf.as_ptr().add(std::mem::size_of::<RAWINPUTHEADER>()) as *const RAWMOUSE);
            if mouse.usButtonFlags & RI_MOUSE_LEFT_BUTTON_DOWN != 0 {
                if let Some(tx) = HOOK_TX.get() {
                    let _ = tx.send(HookMsg::Press);
                }
            }
            if mouse.usButtonFlags & RI_MOUSE_LEFT_BUTTON_UP != 0 {
                if let Some(tx) = HOOK_TX.get() {
                    let _ = tx.send(HookMsg::Release);
                }
            }
        } else if header.dwType == RIM_TYPEKEYBOARD
            && size >= std::mem::size_of::<RAWINPUTHEADER>() as UINT + std::mem::size_of::<RAWKEYBOARD>() as UINT
        {
            let kbd = &*(buf.as_ptr().add(std::mem::size_of::<RAWINPUTHEADER>()) as *const RAWKEYBOARD);
            // Only key down events for toggle
            if kbd.Flags == RI_KEY_MAKE {
                if let Some(state) = RAW_STATE.get() {
                    let toggle_vk = {
                        let config = state.config.lock().unwrap();
                        vk_from_string(&config.toggle_key)
                    };
                    if kbd.VKey == toggle_vk as WORD {
                        if let Some(tx) = HOOK_TX.get() {
                            let _ = tx.send(HookMsg::Toggle);
                        }
                    }
                }
            }
        }
    }

    // ── Entry point ──

    pub fn spawn(state: Arc<AppState>, engine: Arc<dyn InputEnginePort>) {
        let (tx, rx) = mpsc::channel::<HookMsg>();
        let _ = HOOK_TX.set(tx);
        let _ = RAW_STATE.set(state.clone());

        let state_proc = state.clone();
        let engine_proc = engine.clone();

        // Thread 1: Raw Input message pump (no capture needed — uses static RAW_STATE)
        thread::spawn(|| {
            info!("RawInput: starting message pump");

            unsafe {
                // Register window class
                let class_name = "RawInputWndClass\0".encode_utf16().collect::<Vec<_>>();
                let wc = WNDCLASSW {
                    style: 0,
                    lpfnWndProc: Some(wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: std::ptr::null_mut(),
                    hIcon: std::ptr::null_mut(),
                    hCursor: std::ptr::null_mut(),
                    hbrBackground: std::ptr::null_mut(),
                    lpszMenuName: std::ptr::null(),
                    lpszClassName: class_name.as_ptr(),
                };
                RegisterClassW(&wc);

                // Create hidden message-only window
                let hwnd = CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    std::ptr::null(),
                    0,
                    0, 0, 0, 0,
                    -3isize as HWND, // HWND_MESSAGE
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if hwnd.is_null() {
                    info!("RawInput: CreateWindowExW failed");
                    return;
                }

                // Register raw input devices (mouse + keyboard)
                let devices = [
                    RAWINPUTDEVICE {
                        usUsagePage: HID_USAGE_PAGE_GENERIC,
                        usUsage: HID_USAGE_MOUSE,
                        dwFlags: RIDEV_INPUTSINK,
                        hwndTarget: hwnd,
                    },
                    RAWINPUTDEVICE {
                        usUsagePage: HID_USAGE_PAGE_GENERIC,
                        usUsage: HID_USAGE_KEYBOARD,
                        dwFlags: RIDEV_INPUTSINK,
                        hwndTarget: hwnd,
                    },
                ];
                let ok = RegisterRawInputDevices(
                    devices.as_ptr(),
                    2,
                    std::mem::size_of::<RAWINPUTDEVICE>() as DWORD,
                );
                if ok == FALSE {
                    info!("RawInput: RegisterRawInputDevices failed");
                    return;
                }

                info!("RawInput: pump started (msg-only window)");

                // Message pump
                let mut msg = std::mem::zeroed::<MSG>();
                loop {
                    let ret = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
                    if ret == FALSE || ret == -1 {
                        // WM_QUIT or error
                        break;
                    }
                    DispatchMessageW(&msg);
                }
            }

            info!("RawInput: pump exited");
        });

        // Thread 2: event processor (identical to before)
        info!("RawInput: event processor started");
        thread::spawn(move || {
            for msg in rx {
                match msg {
                    HookMsg::Press => {
                        if super::SIMULATING_CLICK.load(Ordering::Relaxed) {
                            super::SIMULATING_CLICK.store(false, Ordering::Relaxed);
                            super::RUNNING.store(false, Ordering::Release);
                            continue;
                        }
                        if state_proc.active.load(Ordering::Relaxed) {
                            debug!("Left click press (rawinput), dispatching");
                            Listener::handle_lclick_press(&state_proc, &engine_proc);
                        }
                    }
                    HookMsg::Release => {
                        if super::SIMULATING_CLICK.load(Ordering::Relaxed) {
                            continue;
                        }
                        Listener::handle_lclick_release(&state_proc);
                    }
                    HookMsg::Toggle => {
                        Listener::stop_spray();
                        let old = state_proc.active.fetch_xor(true, Ordering::Relaxed);
                        info!("[RawInput] Toggle, active: {} -> {}", old, !old);
                    }
                }
            }
        });
    }
}

static SPRAYING: AtomicBool = AtomicBool::new(false);
static RUNNING: AtomicBool = AtomicBool::new(false);
static SIMULATING_CLICK: AtomicBool = AtomicBool::new(false);

pub struct Listener;

impl Listener {
    pub fn spawn(state: Arc<AppState>, engine: Arc<dyn InputEnginePort>) {
        #[cfg(target_os = "linux")]
        Self::spawn_linux(state, engine);

        #[cfg(target_os = "windows")]
        win32::spawn(state, engine);

        #[cfg(target_os = "macos")]
        Self::spawn_macos(state, engine);
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
                    if value == 1 {
                        // Skip simulated events (prevents feedback loop)
                        if SIMULATING_CLICK.load(Ordering::Relaxed) {
                            SIMULATING_CLICK.store(false, Ordering::Relaxed);
                            RUNNING.store(false, Ordering::Release);
                            continue;
                        }
                        if state.active.load(Ordering::Relaxed) {
                            debug!("Left click press, dispatching");
                            Self::handle_lclick_press(&state, &engine);
                        }
                    } else if value == 0 {
                        if SIMULATING_CLICK.load(Ordering::Relaxed) {
                            continue;
                        }
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

    // ---- macOS: rdev listen (channel-based, avoids blocking inside hook) ----

    #[cfg(target_os = "macos")]
    fn spawn_macos(state: Arc<AppState>, engine: Arc<dyn InputEnginePort>) {
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
                        // Skip simulated events (prevents feedback loop)
                        if SIMULATING_CLICK.load(Ordering::Relaxed) {
                            SIMULATING_CLICK.store(false, Ordering::Relaxed);
                            RUNNING.store(false, Ordering::Release);
                            continue;
                        }
                        if state.active.load(Ordering::Relaxed) {
                            debug!("Left click press, dispatching");
                            Self::handle_lclick_press(&state, &engine);
                        }
                    }
                    EventType::ButtonRelease(rdev::Button::Left) => {
                        if SIMULATING_CLICK.load(Ordering::Relaxed) {
                            continue;
                        }
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

    #[cfg(target_os = "macos")]
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
        if RUNNING.swap(true, Ordering::Acquire) {
            return;
        }
        let config = state.config.lock().unwrap();
        let mode = config.current_mode;
        drop(config);

        match mode {
            ModeType::Sniper => {
                // Flag: simulated events from this sequence should be ignored
                SIMULATING_CLICK.store(true, Ordering::Relaxed);
                Self::execute_sequence_sniper(state, &**engine);
                // RUNNING + SIMULATING_CLICK stay true — event loop clears both
                // when it encounters the first simulated left-click event.
            }
            ModeType::Shotgun => {
                SIMULATING_CLICK.store(true, Ordering::Relaxed);
                Self::execute_sequence_shotgun(state, &**engine);
                // Same as Sniper — event loop drains the simulated event.
            }
            ModeType::ArSmg => {
                RUNNING.store(false, Ordering::Release);
                Self::start_spray(state.clone(), engine.clone());
            }
        }
    }

    fn handle_lclick_release(state: &AppState) {
        if SIMULATING_CLICK.load(Ordering::Relaxed) {
            return;
        }
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
        if SPRAYING.swap(true, Ordering::Acquire) {
            return;
        }
        let config = state.config.lock().unwrap();
        let delay = config.ar_delay_ms;
        let recoil_enabled = config.ar_recoil_enabled;
        let recoil_pixels = config.ar_recoil_pixels;
        drop(config);

        debug!("Spray start: delay={}ms, recoil={} pixels={}", delay, recoil_enabled, recoil_pixels);
        std::thread::spawn(move || {
            while SPRAYING.load(Ordering::Relaxed) {
                SIMULATING_CLICK.store(true, Ordering::Relaxed);
                engine.left_click();
                SIMULATING_CLICK.store(false, Ordering::Relaxed);
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

    pub fn stop_spray() {
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
