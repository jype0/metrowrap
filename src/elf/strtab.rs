// SPDX-FileCopyrightText: © 2026 TTKB, LLC
use super::Section;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StrTab {
    pub section: Section,
    pub symbols: Vec<String>,
}

impl StrTab {
    pub fn new(section: Section) -> Self {
        let symbols = if section.data.is_empty() {
            vec![]
        } else {
            let end = if Some(&0) == section.data.last() {
                section.data.len() - 1
            } else {
                section.data.len()
            };
            section.data[0..end]
                .split(|&b| b == 0)
                .flat_map(std::str::from_utf8)
                .map(str::to_string)
                .collect()
        };

        Self { section, symbols }
    }

    /// Packs the strings into the internal data buffer with null terminators
    pub fn pack_data(&mut self) -> &[u8] {
        let mut buf = Vec::new();
        for symbol in &self.symbols {
            buf.extend_from_slice(symbol.as_bytes());
            buf.push(0); // Null terminator
        }
        self.section.data = buf;
        &self.section.data
    }

    pub fn get_str(&self, index: usize) -> &str {
        let slice = &self.section.data[index..];
        let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
        std::str::from_utf8(&slice[..end]).unwrap_or("")
    }

    pub fn get_string(&self, index: usize) -> String {
        self.get_str(index).to_string()
    }

    /// Adds a symbol if it doesn't exist, otherwise returns the existing offset
    pub fn add_symbol(&mut self, symbol_name: impl Into<String>) -> u32 {
        let symbol_name = symbol_name.into();
        let encoded_name = symbol_name.as_bytes();

        // Check if this exact string (with null terminator) already exists in data
        // We look for the sequence in the current raw data buffer
        let mut needle = encoded_name.to_vec();
        needle.push(0);

        if let Some(idx) = self
            .section
            .data
            .windows(needle.len())
            .position(|window| window == needle)
        {
            return idx as u32;
        }

        // Not found: Append to the end
        let idx = self.section.data.len();
        self.section.data.extend_from_slice(&needle);
        self.section.sh_size = self.section.data.len() as u32;
        self.symbols.push(symbol_name);

        idx as u32
    }

    pub fn pack(&mut self) -> (Vec<u8>, Vec<u8>) {
        // Ensure data is synced before returning the header/data pair
        let data = self.pack_data().to_vec();
        (self.section.pack_header(), data)
    }
}

#[cfg(test)]
mod test {
    use super::super::super::elf::SHT_STRTAB;
    use super::Section;
    use super::*;

    #[test]
    fn test_add_symbol() {
        let section = Section::new(0, SHT_STRTAB, 0, 0, 0, 0, 0, 0, 0, 0, vec![]);
        let mut strtab = StrTab::new(section);
        assert!(strtab.symbols.is_empty());

        let index = strtab.add_symbol("");
        assert_eq!(0, index);
        assert_eq!(vec!["".to_string()], strtab.symbols);
        assert_eq!(vec![0], strtab.section.data);

        strtab.add_symbol(".rel.text");
        assert_eq!(
            vec!["".to_string(), ".rel.text".to_string()],
            strtab.symbols
        );

        let (_header, data) = strtab.pack();
        assert_eq!("\0.rel.text\0".bytes().collect::<Vec<u8>>(), data);
    }
}
