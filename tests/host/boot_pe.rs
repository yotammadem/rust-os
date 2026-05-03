use rust_os::boot::uefi::{PeImageError, parse_pe_image_metadata};

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
