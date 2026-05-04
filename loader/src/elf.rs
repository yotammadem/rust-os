const ELF_MAGIC: &[u8; 4] = b"\x7FELF";
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LE: u8 = 1;
const ELF_MACHINE_X86_64: u16 = 0x3E;
const ELF_TYPE_EXEC: u16 = 2;
const ELF_TYPE_DYN: u16 = 3;
const PT_LOAD: u32 = 1;

#[derive(Clone, Copy)]
pub struct ElfImage<'a> {
    bytes: &'a [u8],
}

#[derive(Clone, Copy)]
pub struct LoadSegment {
    pub file_offset: u64,
    pub virtual_address: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub flags: u32,
    pub align: u64,
}

#[derive(Clone, Copy)]
pub struct ElfError {
    pub stage: &'static [u8],
}

impl<'a> ElfImage<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ElfError> {
        let ph_table_end = required_header_bytes(bytes)?;
        if ph_table_end > bytes.len() {
            return Err(ElfError {
                stage: b"ph_table_bounds",
            });
        }

        Ok(Self { bytes })
    }

    pub fn entry_point(&self) -> Result<u64, ElfError> {
        read_u64(self.bytes, 24)
    }

    pub fn load_segments(&self) -> LoadSegmentIter<'a> {
        let phoff = read_u64(self.bytes, 32).unwrap_or(0) as usize;
        let phentsize = read_u16(self.bytes, 54).unwrap_or(0) as usize;
        let phnum = read_u16(self.bytes, 56).unwrap_or(0) as usize;

        LoadSegmentIter {
            bytes: self.bytes,
            phoff,
            phentsize,
            index: 0,
            phnum,
        }
    }
}

pub fn required_header_bytes(bytes: &[u8]) -> Result<usize, ElfError> {
    validate_header_prefix(bytes)?;

    let phentsize = read_u16(bytes, 54)? as usize;
    if phentsize < 56 {
        return Err(ElfError {
            stage: b"phentsize",
        });
    }

    let phoff = read_u64(bytes, 32)? as usize;
    let phnum = read_u16(bytes, 56)? as usize;
    Ok(phoff.saturating_add(phentsize.saturating_mul(phnum)))
}

fn validate_header_prefix(bytes: &[u8]) -> Result<(), ElfError> {
    if bytes.len() < 64 {
        return Err(ElfError {
            stage: b"ehdr_size",
        });
    }

    if &bytes[0..4] != ELF_MAGIC {
        return Err(ElfError { stage: b"magic" });
    }

    if bytes[4] != ELF_CLASS_64 {
        return Err(ElfError { stage: b"class" });
    }

    if bytes[5] != ELF_DATA_LE {
        return Err(ElfError {
            stage: b"endianness",
        });
    }

    let elf_type = read_u16(bytes, 16)?;
    if elf_type != ELF_TYPE_EXEC && elf_type != ELF_TYPE_DYN {
        return Err(ElfError { stage: b"type" });
    }

    if read_u16(bytes, 18)? != ELF_MACHINE_X86_64 {
        return Err(ElfError { stage: b"machine" });
    }

    let ehsize = read_u16(bytes, 52)? as usize;
    if ehsize < 64 {
        return Err(ElfError { stage: b"ehsize" });
    }

    Ok(())
}

pub struct LoadSegmentIter<'a> {
    bytes: &'a [u8],
    phoff: usize,
    phentsize: usize,
    index: usize,
    phnum: usize,
}

impl<'a> Iterator for LoadSegmentIter<'a> {
    type Item = Result<LoadSegment, ElfError>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.phnum {
            let offset = self.phoff + self.index * self.phentsize;
            self.index += 1;

            let ph = &self.bytes[offset..offset + self.phentsize];
            let typ = match read_u32(ph, 0) {
                Ok(value) => value,
                Err(error) => return Some(Err(error)),
            };

            if typ != PT_LOAD {
                continue;
            }

            let segment = LoadSegment {
                file_offset: match read_u64(ph, 8) {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                },
                virtual_address: match read_u64(ph, 16) {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                },
                file_size: match read_u64(ph, 32) {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                },
                memory_size: match read_u64(ph, 40) {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                },
                flags: match read_u32(ph, 4) {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                },
                align: match read_u64(ph, 48) {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                },
            };

            return Some(Ok(segment));
        }

        None
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ElfError> {
    let slice = bytes.get(offset..offset + 2).ok_or(ElfError {
        stage: b"u16_bounds",
    })?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ElfError> {
    let slice = bytes.get(offset..offset + 4).ok_or(ElfError {
        stage: b"u32_bounds",
    })?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ElfError> {
    let slice = bytes.get(offset..offset + 8).ok_or(ElfError {
        stage: b"u64_bounds",
    })?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}
