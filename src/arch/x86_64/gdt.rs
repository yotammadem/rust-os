#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentSelectors {
    pub code: u16,
    pub data: u16,
    pub tss: u16,
}

impl SegmentSelectors {
    pub const fn kernel_flat() -> Self {
        Self {
            code: 0x08,
            data: 0x10,
            tss: 0x18,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DescriptorTablePointer {
    pub base: u64,
    pub limit: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskStateSegmentLayout {
    pub rsp0: u64,
    pub ist1: u64,
}

impl TaskStateSegmentLayout {
    pub const fn empty() -> Self {
        Self { rsp0: 0, ist1: 0 }
    }
}
