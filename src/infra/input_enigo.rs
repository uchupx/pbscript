use std::sync::Mutex;

use enigo::{Coordinate, Direction, Enigo, Key as EnigoKey, Keyboard, Mouse, Settings};

use crate::domain::entities::Key;
use crate::domain::ports::InputEnginePort;

// ── Raw Windows SendInput (more reliable for games/anti-cheat) ──
#[cfg(windows)]
mod win_input {
    use std::mem;

    const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
    const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
    const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
    const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;

    #[repr(C)]
    struct MOUSEINPUT {
        dx: i32,
        dy: i32,
        mouse_data: u32,
        dw_flags: u32,
        time: u32,
        dw_extra_info: usize,
    }

    #[repr(C)]
    struct INPUT {
        type_: u32, // 0 = mouse
        mi: MOUSEINPUT,
    }

    #[link(name = "user32")]
    extern "system" {
        fn SendInput(c_inputs: u32, p_inputs: *mut INPUT, cb_size: i32) -> u32;
    }

    fn send_mouse(dw_flags: u32) {
        let mut input = INPUT {
            type_: 0,
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouse_data: 0,
                dw_flags,
                time: 0,
                dw_extra_info: 0,
            },
        };
        unsafe {
            SendInput(1, &mut input, mem::size_of::<INPUT>() as i32);
        }
    }

    pub fn left_click() {
        send_mouse(MOUSEEVENTF_LEFTDOWN);
        send_mouse(MOUSEEVENTF_LEFTUP);
    }

    pub fn right_click() {
        send_mouse(MOUSEEVENTF_RIGHTDOWN);
        send_mouse(MOUSEEVENTF_RIGHTUP);
    }
}

pub struct InputEngineEnigo {
    enigo: Mutex<Enigo>,
}

impl InputEngineEnigo {
    pub fn new() -> Self {
        Self {
            enigo: Mutex::new(Enigo::new(&Settings::default()).unwrap()),
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

// Enigo needs &mut self, so we wrap in Mutex.
// Mutex<T> is Sync when T: Send, and Enigo is Send.
unsafe impl Sync for InputEngineEnigo {}

impl InputEnginePort for InputEngineEnigo {
    fn right_click(&self) {
        #[cfg(windows)]
        win_input::right_click();
        #[cfg(not(windows))]
        let _ = self
            .enigo
            .lock()
            .unwrap()
            .button(enigo::Button::Right, Direction::Click);
    }

    fn left_click(&self) {
        #[cfg(windows)]
        win_input::left_click();
        #[cfg(not(windows))]
        let _ = self
            .enigo
            .lock()
            .unwrap()
            .button(enigo::Button::Left, Direction::Click);
    }

    fn press(&self, key: Key) {
        let ek = Self::map_key(key);
        let _ = self.enigo.lock().unwrap().key(ek, Direction::Press);
    }

    fn release(&self, key: Key) {
        let ek = Self::map_key(key);
        let _ = self.enigo.lock().unwrap().key(ek, Direction::Release);
    }

    fn move_mouse_relative(&self, dx: i32, dy: i32) {
        let _ = self
            .enigo
            .lock()
            .unwrap()
            .move_mouse(dx, dy, Coordinate::Rel);
    }
}
