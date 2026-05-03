use rust_os::boot::uefi::{PeImageError, PeImageMetadata, apply_pe_relocations, parse_pe_image_metadata};

#[test]
fn parse_pe_image_metadata_reads_relocation_directory() {
    let mut image = vec![0u8; 0x400];

    image[0..2].copy_from_slice(b"MZ");
    image[0x3c..0x40].copy_from_slice(&(0x80u32).to_le_bytes());

    let pe_offset = 0x80usize;
    image[pe_offset..pe_offset + 4].copy_from_slice(b"PE\0\0");

    let optional_header_offset = pe_offset + 4 + 20;
    image[optional_header_offset..optional_header_offset + 2]
        .copy_from_slice(&(0x20bu16).to_le_bytes());
    image[optional_header_offset + 16..optional_header_offset + 20]
        .copy_from_slice(&(0x1234u32).to_le_bytes());
    image[optional_header_offset + 24..optional_header_offset + 32]
        .copy_from_slice(&(0x1400_0000_0u64).to_le_bytes());
    image[optional_header_offset + 56..optional_header_offset + 60]
        .copy_from_slice(&(0x400u32).to_le_bytes());
    image[optional_header_offset + 108..optional_header_offset + 112]
        .copy_from_slice(&(16u32).to_le_bytes());

    let reloc_directory_offset = optional_header_offset + 112 + (5 * 8);
    image[reloc_directory_offset..reloc_directory_offset + 4]
        .copy_from_slice(&(0x3000u32).to_le_bytes());
    image[reloc_directory_offset + 4..reloc_directory_offset + 8]
        .copy_from_slice(&(0x180u32).to_le_bytes());

    let metadata =
        parse_pe_image_metadata(&image, 0x0000_0000_0e01_a000).expect("valid PE metadata");

    assert_eq!(metadata.loaded_base, 0x0000_0000_0e01_a000);
    assert_eq!(metadata.loaded_size, image.len() as u64);
    assert_eq!(metadata.preferred_base, 0x0000_0001_4000_0000);
    assert_eq!(metadata.entry_point_rva, 0x1234);
    assert_eq!(metadata.size_of_image, 0x400);
    assert_eq!(metadata.base_relocations_rva, 0x3000);
    assert_eq!(metadata.base_relocations_size, 0x180);
}

#[test]
fn parse_pe_image_metadata_rejects_missing_relocation_directory() {
    let mut image = vec![0u8; 0x200];

    image[0..2].copy_from_slice(b"MZ");
    image[0x3c..0x40].copy_from_slice(&(0x80u32).to_le_bytes());
    image[0x80..0x84].copy_from_slice(b"PE\0\0");

    let optional_header_offset = 0x80 + 4 + 20;
    image[optional_header_offset..optional_header_offset + 2]
        .copy_from_slice(&(0x20bu16).to_le_bytes());
    image[optional_header_offset + 56..optional_header_offset + 60]
        .copy_from_slice(&(0x200u32).to_le_bytes());
    image[optional_header_offset + 108..optional_header_offset + 112]
        .copy_from_slice(&(5u32).to_le_bytes());

    assert_eq!(
        parse_pe_image_metadata(&image, 0x1000),
        Err(PeImageError::MissingDataDirectory)
    );
}

#[test]
fn apply_pe_relocations_updates_dir64_entries_for_new_base() {
    let mut image = vec![0u8; 0x5000];
    let loaded_base = 0x0000_0000_0e01_a000u64;
    let new_base = 0xffff_8000_0000_0000u64;

    let target_rva = 0x1238usize;
    let initial_value = loaded_base + 0x4567;
    image[target_rva..target_rva + 8].copy_from_slice(&initial_value.to_le_bytes());

    let block_rva = 0x3000usize;
    image[block_rva..block_rva + 4].copy_from_slice(&(0x1000u32).to_le_bytes());
    image[block_rva + 4..block_rva + 8].copy_from_slice(&(10u32).to_le_bytes());
    let entry = ((10u16) << 12) | 0x238u16;
    image[block_rva + 8..block_rva + 10].copy_from_slice(&entry.to_le_bytes());

    let metadata = PeImageMetadata {
        loaded_base,
        loaded_size: image.len() as u64,
        preferred_base: 0x0000_0001_4000_0000,
        entry_point_rva: 0x1234,
        size_of_image: image.len() as u32,
        base_relocations_rva: block_rva as u32,
        base_relocations_size: 10,
    };

    apply_pe_relocations(&mut image, &metadata, new_base).expect("relocation should succeed");

    let relocated = u64::from_le_bytes(
        image[target_rva..target_rva + 8]
            .try_into()
            .expect("8-byte relocation field"),
    );
    assert_eq!(relocated, new_base + 0x4567);
}
