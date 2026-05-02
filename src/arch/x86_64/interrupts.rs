#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptProofEventKind {
    Breakpoint,
    HardwareTimer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterruptProofEvent {
    pub kind: InterruptProofEventKind,
    pub vector: u8,
    pub handler_marker: &'static str,
    pub acknowledgement_required: bool,
}

impl InterruptProofEvent {
    pub const fn breakpoint() -> Self {
        Self {
            kind: InterruptProofEventKind::Breakpoint,
            vector: 3,
            handler_marker: "breakpoint-handler",
            acknowledgement_required: false,
        }
    }

    pub const fn timer() -> Self {
        Self {
            kind: InterruptProofEventKind::HardwareTimer,
            vector: 32,
            handler_marker: "timer-handler",
            acknowledgement_required: true,
        }
    }
}
