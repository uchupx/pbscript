use std::sync::Mutex;

use enigo::{Direction, Enigo, Key as EnigoKey, Keyboard, Mouse, Settings};

use crate::domain::entities::Key;
use crate::domain::ports::InputEnginePort;

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
        let _ = self
            .enigo
            .lock()
            .unwrap()
            .button(enigo::Button::Right, Direction::Click);
    }

    fn left_click(&self) {
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
}
