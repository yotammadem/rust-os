pub const PIC1_COMMAND_PORT: u16 = 0x20;
pub const PIC1_DATA_PORT: u16 = 0x21;
pub const PIT_COMMAND_PORT: u16 = 0x43;
pub const PIT_CHANNEL0_PORT: u16 = 0x40;
pub const PIT_BASE_FREQUENCY_HZ: u32 = 1_193_182;
pub const PIT_DEFAULT_DIVISOR: u16 = 11_932;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerControl {
    pub irq_vector: u8,
    pub divisor: u16,
}

impl TimerControl {
    pub const fn legacy_default() -> Self {
        Self {
            irq_vector: 32,
            divisor: PIT_DEFAULT_DIVISOR,
        }
    }
}
