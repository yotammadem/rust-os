#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptVector {
    Breakpoint,
    TimerIrq,
}

impl InterruptVector {
    pub const fn number(self) -> u8 {
        match self {
            Self::Breakpoint => 3,
            Self::TimerIrq => 32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterruptHandlerRegistration {
    pub vector: InterruptVector,
    pub handler_addr: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterruptDescriptorTableLayout {
    pub breakpoint_vector: u8,
    pub timer_vector: u8,
}

impl InterruptDescriptorTableLayout {
    pub const fn minimal() -> Self {
        Self {
            breakpoint_vector: InterruptVector::Breakpoint.number(),
            timer_vector: InterruptVector::TimerIrq.number(),
        }
    }
}
