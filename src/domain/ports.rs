use crate::domain::entities::Key;

/// Abstraction for simulating mouse/keyboard input.
pub trait InputEnginePort: Send + Sync {
    fn right_click(&self);
    fn left_click(&self);
    fn press(&self, key: Key);
    fn release(&self, key: Key);
    fn move_mouse_relative(&self, dx: i32, dy: i32);
}

/// Persistence for app config.
pub trait ConfigPort: Send {
    fn load(&mut self);
    fn save(&self);
}
