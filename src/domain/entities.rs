/// Action types for each step in the macro sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepAction {
    RightClick,
    LeftClick,
    Switch(SwitchMode),
}

/// QQ: double-tap Q. Num31: press 3 then 1 (knife→sniper).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SwitchMode {
    QQ,
    Num31,
}

/// One step in the sequence with delay after execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SequenceStep {
    pub action: StepAction,
    /// Delay in ms after this step (0-100).
    pub delay_ms: u32,
}

impl SequenceStep {
    pub fn new(action: StepAction, delay_ms: u32) -> Self {
        Self {
            action,
            delay_ms: delay_ms.clamp(0, 100),
        }
    }
}

/// The full 4-step sniper quick-switch sequence.
#[derive(Debug, Clone)]
pub struct SniperSequence {
    pub steps: Vec<SequenceStep>,
}

impl SniperSequence {
    /// Build default sequence with given delays and switch mode.
    pub fn new(
        buka_delay: u32,
        tembak_delay: u32,
        tutup_delay: u32,
        ganti_delay: u32,
        mode: SwitchMode,
    ) -> Self {
        Self {
            steps: vec![
                SequenceStep::new(StepAction::RightClick, buka_delay),
                SequenceStep::new(StepAction::LeftClick, tembak_delay),
                SequenceStep::new(StepAction::RightClick, tutup_delay),
                SequenceStep::new(StepAction::Switch(mode), ganti_delay),
            ],
        }
    }
}

/// Weapon mode selector.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ModeType {
    Sniper,
    ArSmg,
    Shotgun,
}

/// Simple key codes for the domain layer (avoids depending on enigo/rdev types).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    Q,
    Key1,
    Key3,
}
