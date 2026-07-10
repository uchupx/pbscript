use crate::domain::entities::Key;

/// Abstraction for simulating mouse/keyboard input.
pub trait InputEnginePort: Send + Sync {
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
