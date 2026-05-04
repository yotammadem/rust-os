use rust_os::boot::handoff::BootInfo;
use rust_os::boot::multiboot::EFI_CONVENTIONAL_MEMORY;

const TWO_MIB: u64 = 2 * 1024 * 1024;
const FOUR_KIB: u64 = 4096;
const MIN_EARLY_REGION_START: u64 = TWO_MIB;
const DEFAULT_PAGE_TABLE_BYTES: u64 = 16 * FOUR_KIB;
const DEFAULT_BOOT_INFO_BYTES: u64 = 16 * FOUR_KIB;

#[derive(Clone, Copy)]
pub struct PhysicalRange {
    pub start: u64,
    pub end: u64,
}

impl PhysicalRange {
    pub const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    pub fn size_bytes(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

pub struct EarlyLayout {
    pub region: PhysicalRange,
    pub kernel_usable_region: PhysicalRange,
    pub boot_info_region: PhysicalRange,
    pub page_table_region: PhysicalRange,
}

impl EarlyLayout {
    pub fn from_boot_info(boot_info: &BootInfo) -> Self {
        let region = select_early_region(boot_info);
        let page_table_region = carve_from_end(region, DEFAULT_PAGE_TABLE_BYTES, FOUR_KIB)
            .unwrap_or(PhysicalRange::empty());
        let boot_info_source = shrink_end(region, page_table_region.size_bytes());
        let boot_info_region = carve_from_end(boot_info_source, DEFAULT_BOOT_INFO_BYTES, FOUR_KIB)
            .unwrap_or(PhysicalRange::empty());
        let kernel_source = shrink_end(
            region,
            page_table_region
                .size_bytes()
                .saturating_add(boot_info_region.size_bytes()),
        );
        let kernel_usable_region = aligned_prefix(kernel_source, TWO_MIB);

        Self {
            region,
            kernel_usable_region,
            boot_info_region,
            page_table_region,
        }
    }
}

fn select_early_region(boot_info: &BootInfo) -> PhysicalRange {
    let mut best = PhysicalRange::empty();

    for descriptor in boot_info.memory_map.descriptors() {
        if descriptor.typ != EFI_CONVENTIONAL_MEMORY {
            continue;
        }

        let start = descriptor.physical_start.max(MIN_EARLY_REGION_START);
        let end = descriptor
            .physical_start
            .saturating_add(descriptor.number_of_pages.saturating_mul(FOUR_KIB));
        let candidate = PhysicalRange { start, end };
        let aligned = aligned_region(candidate, TWO_MIB);

        if aligned.size_bytes() > best.size_bytes() {
            best = aligned;
        }
    }

    best
}

fn aligned_region(range: PhysicalRange, align: u64) -> PhysicalRange {
    let start = align_up(range.start, align);
    let end = align_down(range.end, align);
    if start < end {
        PhysicalRange { start, end }
    } else {
        PhysicalRange::empty()
    }
}

fn aligned_prefix(range: PhysicalRange, align: u64) -> PhysicalRange {
    let aligned = aligned_region(range, align);
    if aligned.is_empty() {
        return aligned;
    }

    PhysicalRange {
        start: aligned.start,
        end: aligned.end,
    }
}

fn carve_from_end(range: PhysicalRange, size: u64, align: u64) -> Option<PhysicalRange> {
    if range.is_empty() {
        return None;
    }

    let end = align_down(range.end, align);
    let start = align_down(end.saturating_sub(size), align);
    if start >= range.start && start < end {
        Some(PhysicalRange { start, end })
    } else {
        None
    }
}

fn shrink_end(range: PhysicalRange, bytes: u64) -> PhysicalRange {
    if range.size_bytes() <= bytes {
        return PhysicalRange::empty();
    }

    PhysicalRange {
        start: range.start,
        end: range.end - bytes,
    }
}

fn align_up(value: u64, align: u64) -> u64 {
    let mask = align - 1;
    value.saturating_add(mask) & !mask
}

fn align_down(value: u64, align: u64) -> u64 {
    value & !(align - 1)
}
