// SPDX-FileCopyrightText: © 2026 ThirstyWraith
// SPDX-License-Identifier: BSD-3-CLAUSE

//! Splits monolithic ELF sections into individual per-symbol sections
//! for use with GNU ld --gc-sections.

use std::collections::{HashMap, HashSet};

use crate::elf::{Elf, Section, SHT_NOBITS, SHT_REL, SHT_SYMTAB};
use crate::elf::section::Relocation;
use crate::error::MWError;

/// Alignment rule per original game linker:
///   size 1 → align 1, size 2 → align 2, size ≥3 or @-prefixed → align 4
fn alignment_for(name: &str, size: u32) -> u32 {
    if name.starts_with('@') || name.starts_with("__at__") {
        return 4;
    }
    match size {
        1 => 1,
        2 => 2,
        _ => 4,
    }
}

struct SplitEntry {
    sym_idx: usize,
    original_value: u32,
    size: u32,
    name: String,
}

struct SectionSplit {
    section_idx: usize,
    rel_section_idx: Option<usize>,
    entries: Vec<SplitEntry>,
}

fn collect_split_info(elf: &Elf, section_name: &str, symbol_order: &HashMap<String, usize>) -> Option<SectionSplit> {
    let section_idx = elf.sections.iter().position(|s| s.name == section_name)?;
    let section = &elf.sections[section_idx];

    let mut entries: Vec<SplitEntry> = elf
        .symtab
        .symbols
        .iter()
        .enumerate()
        .filter(|(_, sym)| {
            sym.st_shndx as usize == section_idx
                && sym.st_size > 0
                && !sym.name.contains("NON_MATCHING")
        })
        .map(|(idx, sym)| SplitEntry {
            sym_idx: idx,
            original_value: sym.st_value,
            size: sym.st_size,
            name: sym.name.clone(),
        })
        .collect();

    if entries.is_empty() {
        return None;
    }

    entries.sort_by_key(|e| e.original_value);

    // Validate contiguous coverage
    let mut expected = 0u32;
    for entry in &entries {
        // Account for alignment padding between symbols
        let align = alignment_for(&entry.name, entry.size);
        let aligned = (expected + align - 1) & !(align - 1);
        if entry.original_value != expected && entry.original_value != aligned {
            eprintln!(
                "split: gap in {} at 0x{:x} (expected 0x{:x} or 0x{:x}, symbol {})",
                section_name, entry.original_value, expected, aligned, entry.name
            );
            return None;
        }
        expected = entry.original_value + entry.size;
    }

    let actual_size = if section.sh_type != SHT_NOBITS {
        section.data.len() as u32
    } else {
        section.sh_size
    };

    if expected != actual_size {
        eprintln!(
            "split: {} symbol coverage 0x{:x} != section size 0x{:x}",
            section_name, expected, actual_size
        );
        return None;
    }
    
    if section.sh_type == SHT_NOBITS {
        entries.sort_by_key(|e| {
        *symbol_order.get(&e.name).unwrap_or(&e.sym_idx)
    });
    } else {
        entries.sort_by_key(|e| {
            let bind = elf.symtab.symbols[e.sym_idx].bind();
            (bind, e.original_value)
        });
    }
    
    let rel_name = format!(".rel{}", section_name);
    let rel_section_idx = elf
        .sections
        .iter()
        .position(|s| s.name == rel_name && s.sh_type == SHT_REL);

    Some(SectionSplit {
        section_idx,
        rel_section_idx,
        entries,
    })
}

pub fn split_monolithic_sections(elf: &mut Elf, plain_names: bool, symbol_order: &HashMap<String, usize>) -> Result<(), MWError> {
    let sections_to_split = [".text", ".rodata", ".data", ".sdata", ".sbss", ".bss"];

    let mut splits: Vec<SectionSplit> = Vec::new();
    let mut removed: HashSet<usize> = HashSet::new();

    for name in &sections_to_split {
        if let Some(split) = collect_split_info(elf, name, symbol_order) {
            removed.insert(split.section_idx);
            if let Some(ri) = split.rel_section_idx {
                removed.insert(ri);
            }
            splits.push(split);
        }
    }

    if splits.is_empty() {
        return Ok(());
    }

    let mut new_sections: Vec<Section> = Vec::new();
    let mut old_to_new: HashMap<usize, usize> = HashMap::new();
    let mut sym_to_new_shndx: HashMap<usize, usize> = HashMap::new();
    let mut fresh_sections: HashSet<usize> = HashSet::new();
    // For zero-size symbols inside split sections: (start, end, new_section_idx)
    let mut section_ranges: HashMap<usize, Vec<(u32, u32, usize)>> = HashMap::new();

    for (old_idx, section) in elf.sections.iter().enumerate() {
        if removed.contains(&old_idx) {
            // Is this a data section being split (not a .rel)?
            if let Some(split) = splits.iter().find(|s| s.section_idx == old_idx) {
                let original = &elf.sections[split.section_idx];

                let relocations = split.rel_section_idx.map_or_else(Vec::new, |ri| {
                    Relocation::unpack_all(&elf.sections[ri].data)
                });

                let mut ranges: Vec<(u32, u32, usize)> = Vec::new();

                for entry in &split.entries {
                    let data_idx = new_sections.len();
                    sym_to_new_shndx.insert(entry.sym_idx, data_idx);
                    fresh_sections.insert(data_idx);
                    ranges.push((entry.original_value, entry.original_value + entry.size, data_idx));

                    // Create data section
                    let data = if original.sh_type != SHT_NOBITS {
                        let s = entry.original_value as usize;
                        let e = s + entry.size as usize;
                        original.data[s..e].to_vec()
                    } else {
                        vec![]
                    };

                    let sec_name = if plain_names {
                        original.name.clone()
                    } else {
                        format!("{}.{}", original.name, entry.name)
                    };
                    let sh_name = elf.shstrtab.add_symbol(&sec_name);

                    let mut new_sec = Section::new(
                        sh_name,
                        original.sh_type,
                        original.sh_flags,
                        0, 0,
                        entry.size,
                        0, 0,
                        if ranges.is_empty() {
                            4.max(alignment_for(&entry.name, entry.size))
                        } else {
                            alignment_for(&entry.name, entry.size)
                        },
                        0,
                        data,
                    );
                    new_sec.name = sec_name;
                    new_sections.push(new_sec);

                    // Create relocation section for this symbol
                    let entry_relocs: Vec<Relocation> = relocations
                        .iter()
                        .filter(|r| {
                            r.r_offset >= entry.original_value
                                && r.r_offset < entry.original_value + entry.size
                        })
                        .map(|r| {
                            let mut nr = r.clone();
                            nr.r_offset -= entry.original_value;
                            nr
                        })
                        .collect();

                    if !entry_relocs.is_empty() {
                        let rel_idx = new_sections.len();
                        fresh_sections.insert(rel_idx);

                        let rel_name = if plain_names {
                            format!(".rel{}", original.name)
                        } else {
                            format!(".rel{}.{}", original.name, entry.name)
                        };
                        let rel_sh_name = elf.shstrtab.add_symbol(&rel_name);

                        let mut rel_data = Vec::with_capacity(entry_relocs.len() * 8);
                        for r in &entry_relocs {
                            rel_data.extend_from_slice(&r.pack());
                        }

                        let mut rel_sec = Section::new(
                            rel_sh_name,
                            SHT_REL,
                            0, 0, 0,
                            rel_data.len() as u32,
                            0,                   // sh_link — fixed below
                            data_idx as u32,     // sh_info — target section
                            4, 8,
                            rel_data,
                        );
                        rel_sec.name = rel_name;
                        new_sections.push(rel_sec);
                    }
                }

                section_ranges.insert(split.section_idx, ranges);
            }
            continue;
        }

        old_to_new.insert(old_idx, new_sections.len());
        new_sections.push(section.clone());
    }

    let new_symtab_idx = *old_to_new.get(&elf.symtab_idx).expect("symtab gone");
    let new_strtab_idx = *old_to_new.get(&elf.strtab_idx).expect("strtab gone");
    let new_shstrtab_idx = *old_to_new.get(&elf.shstrtab_idx).expect("shstrtab gone");

    for (idx, section) in new_sections.iter_mut().enumerate() {
        if section.sh_type == SHT_REL {
            if fresh_sections.contains(&idx) {
                section.sh_link = new_symtab_idx as u32;
            } else {
                if let Some(&ni) = old_to_new.get(&(section.sh_link as usize)) {
                    section.sh_link = ni as u32;
                }
                if let Some(&ni) = old_to_new.get(&(section.sh_info as usize)) {
                    section.sh_info = ni as u32;
                }
            }
        }
        if section.sh_type == SHT_SYMTAB {
            if let Some(&ni) = old_to_new.get(&(section.sh_link as usize)) {
                section.sh_link = ni as u32;
            }
        }
    }

    // Fix symbols
    for (sym_idx, sym) in elf.symtab.symbols.iter_mut().enumerate() {
        if sym.st_shndx == 0 {
            continue;
        }
        if let Some(&new_shndx) = sym_to_new_shndx.get(&sym_idx) {
            sym.st_shndx = new_shndx as u16;
            sym.st_value = 0;
        } else if let Some(ranges) = section_ranges.get(&(sym.st_shndx as usize)) {
            // Zero-size symbol inside a split section
            let val = sym.st_value;
            let target = ranges
                .iter()
                .find(|&&(s, e, _)| val >= s && val < e)
                .or_else(|| ranges.last());
            if let Some(&(start, _, new_shndx)) = target {
                sym.st_shndx = new_shndx as u16;
                sym.st_value -= start;
            }
        } else if let Some(&new_idx) = old_to_new.get(&(sym.st_shndx as usize)) {
            sym.st_shndx = new_idx as u16;
        }
    }

    elf.sections = new_sections;
    elf.symtab_idx = new_symtab_idx;
    elf.strtab_idx = new_strtab_idx;
    elf.shstrtab_idx = new_shstrtab_idx;
    elf.header.e_shstrndx = new_shstrtab_idx as u16;

    elf.symtab.pack_data();
    elf.sections[new_symtab_idx] = elf.symtab.section.clone();
    elf.shstrtab.pack_data();
    elf.sections[new_shstrtab_idx] = elf.shstrtab.section.clone();
    elf.strtab.pack_data();
    elf.sections[new_strtab_idx] = elf.strtab.section.clone();

    Ok(())
}