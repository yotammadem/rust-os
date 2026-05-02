use crate::memory::{PAGE_SIZE, PhysAddr};

pub const PAGE_TABLE_ENTRIES: usize = 512;
pub const KERNEL_VIRT_BASE: u64 = 0xffff_8000_0000_0000;
pub const KERNEL_ALLOC_BASE: u64 = KERNEL_VIRT_BASE + 0x0200_0000;
pub const KERNEL_ALLOC_LIMIT: u64 = KERNEL_VIRT_BASE + 0x4000_0000;
pub const PROCESS_PRIVATE_LIMIT: u64 = 0x0000_8000_0000_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageTableLevel {
    Pml4,
    Pdpt,
    Pd,
    Pt,
}

impl PageTableLevel {
    pub const fn child(self) -> Option<Self> {
        match self {
            Self::Pml4 => Some(Self::Pdpt),
            Self::Pdpt => Some(Self::Pd),
            Self::Pd => Some(Self::Pt),
            Self::Pt => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryFlags(u64);

impl EntryFlags {
    pub const PRESENT: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);
    pub const USER: Self = Self(1 << 2);
    pub const WRITE_THROUGH: Self = Self(1 << 3);
    pub const NO_CACHE: Self = Self(1 << 4);
    pub const GLOBAL: Self = Self(1 << 8);
    pub const NO_EXECUTE: Self = Self(1 << 63);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for EntryFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for EntryFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MappingRequest {
    pub start_virt_addr: u64,
    pub target_phys_start: PhysAddr,
    pub page_count: usize,
    pub flags: EntryFlags,
    pub allow_overwrite: bool,
}

impl MappingRequest {
    pub fn end_virt_addr_exclusive(&self) -> u64 {
        self.start_virt_addr + (self.page_count * PAGE_SIZE) as u64
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MappedPage {
    pub virt_addr: u64,
    pub phys_addr: PhysAddr,
    pub flags: EntryFlags,
}

pub struct VirtualAddressLayout;

impl VirtualAddressLayout {
    pub const fn pml4_index(addr: u64) -> usize {
        ((addr >> 39) & 0x1ff) as usize
    }

    pub const fn pdpt_index(addr: u64) -> usize {
        ((addr >> 30) & 0x1ff) as usize
    }

    pub const fn pd_index(addr: u64) -> usize {
        ((addr >> 21) & 0x1ff) as usize
    }

    pub const fn pt_index(addr: u64) -> usize {
        ((addr >> 12) & 0x1ff) as usize
    }

    pub const fn page_offset(addr: u64) -> usize {
        (addr as usize) & (PAGE_SIZE - 1)
    }

    pub const fn indexes(addr: u64) -> [usize; 4] {
        [
            Self::pml4_index(addr),
            Self::pdpt_index(addr),
            Self::pd_index(addr),
            Self::pt_index(addr),
        ]
    }

    pub const fn is_page_aligned(addr: u64) -> bool {
        Self::page_offset(addr) == 0
    }

    pub const fn is_kernel_address(addr: u64) -> bool {
        addr >= KERNEL_VIRT_BASE
    }

    pub const fn is_process_private_address(addr: u64) -> bool {
        addr < PROCESS_PRIVATE_LIMIT
    }
}
