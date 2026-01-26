// SPDX-FileCopyrightText: © 2026 TTKB, LLC
use std::fmt;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Symbol {
    pub st_name: u32,
    pub st_value: u32,
    pub st_size: u32,
    pub st_info: u8,
    pub st_other: u8,
    pub st_shndx: u16,
    pub name: String,
}

impl Symbol {
    // Equivalent to __init__
    pub fn new(
        st_name: u32,
        st_value: u32,
        st_size: u32,
        st_info: u8,
        st_other: u8,
        st_shndx: u16,
    ) -> Self {
        Self {
            st_name,
            st_value,
            st_size,
            st_info,
            st_other,
            st_shndx,
            name: String::new(),
        }
    }

    // Equivalent to from_data
    pub fn from_data(data: &[u8]) -> Self {
        let (st_name, st_value, st_size, st_info, st_other, st_shndx) = Self::unpack(data);
        Self::new(st_name, st_value, st_size, st_info, st_other, st_shndx)
    }

    pub fn unpack_all(data: &[u8]) -> Vec<Self> {
        data.chunks(16).map(Self::from_data).collect()
    }

    // Equivalent to unpack (format: <IIIBBH)
    pub fn unpack(data: &[u8]) -> (u32, u32, u32, u8, u8, u16) {
        let st_name = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let st_value = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let st_size = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let st_info = data[12];
        let st_other = data[13];
        let st_shndx = u16::from_le_bytes(data[14..16].try_into().unwrap());

        (st_name, st_value, st_size, st_info, st_other, st_shndx)
    }

    // Equivalent to pack
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(16);
        buf.extend_from_slice(&self.st_name.to_le_bytes());
        buf.extend_from_slice(&self.st_value.to_le_bytes());
        buf.extend_from_slice(&self.st_size.to_le_bytes());
        buf.push(self.st_info);
        buf.push(self.st_other);
        buf.extend_from_slice(&self.st_shndx.to_le_bytes());
        buf
    }

    pub fn bind(&self) -> u8 {
        self.st_info >> 4
    }
    pub fn type_id(&self) -> u8 {
        self.st_info & 0xf
    }
}

// Equivalent to __str__
impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_name = if self.name.is_empty() {
            "NO_NAME"
        } else {
            &self.name
        };
        write!(
            f,
            "{} st_name: 0x{:X}, st_info: 0x{:X}, st_other: 0x{:X}, st_shndx: 0x{:X}, st_value: 0x{:X}, st_size: 0x{:X}",
            display_name,
            self.st_name,
            self.st_info,
            self.st_other,
            self.st_shndx,
            self.st_value,
            self.st_size
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::elf;
    use crate::elf::STB_GLOBAL;
    use crate::elf::STB_LOCAL;
    use crate::elf::STT_FUNC;
    use crate::elf::STT_NOTYPE;
    use crate::elf::STT_SECTION;

    #[test]
    fn test_unpack_all() {
        let symtab_data: [u8; 48] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 3,
            0, 6, 0, 1, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 18, 0, 5, 0,
        ];

        let symbols = elf::Symbol::unpack_all(&symtab_data);

        assert_eq!(3, symbols.len());

        let sym0 = &symbols[0];
        assert_eq!(
            elf::Symbol {
                st_name: 0,
                st_value: 0,
                st_size: 0,
                st_info: 0,
                st_other: 0,
                st_shndx: 0,
                name: "".to_string(),
            },
            *sym0
        );
        assert_eq!(STT_NOTYPE, sym0.type_id());
        assert_eq!(STB_LOCAL, sym0.bind());

        let sym1 = &symbols[1];
        assert_eq!(
            elf::Symbol {
                st_name: 13,
                st_value: 0,
                st_size: 8,
                st_info: 3,
                st_other: 0,
                st_shndx: 6,
                name: "".to_string(),
            },
            *sym1
        );
        assert_eq!(STT_SECTION, sym1.type_id());
        assert_eq!(STB_LOCAL, sym1.bind());

        let sym2 = &symbols[2];
        assert_eq!(
            elf::Symbol {
                st_name: 1,
                st_value: 0,
                st_size: 16,
                st_info: 18,
                st_other: 0,
                st_shndx: 5,
                name: "".to_string(),
            },
            *sym2
        );
        assert_eq!(STT_FUNC, sym2.type_id());
        assert_eq!(STB_GLOBAL, sym2.bind());

        let packed = symbols.iter().flat_map(Symbol::pack).collect::<Vec<u8>>();
        assert_eq!(symtab_data.to_vec(), packed);
    }
}
