// SPDX-FileCopyrightText: © 2026 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE
pub mod assembler;
pub mod compiler;
pub mod constants;
pub mod elf;
pub mod error;
pub mod makerule;
pub mod preprocessor;
pub mod split;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::LazyLock;

use encoding_rs::Encoding;
use rayon::prelude::*;
use tempfile::Builder;

use crate::assembler::Assembler;
use crate::compiler::Compiler;
use crate::constants::*;
use crate::elf::Elf;
use crate::elf::STT_SECTION;
use crate::elf::SHT_REL;
use crate::elf::Section;
use crate::elf::section::{Relocation, RelocationRecord};
use crate::makerule::MakeRule;
use crate::preprocessor::Preprocessor;

#[macro_export]
macro_rules! strings {
    ($($str:expr),*) => ({
        vec![$(String::from($str),)*] as Vec<String>
    });
}

pub fn escape_symbol(name: &str) -> String {
    name.replace(".", SYMBOL_AT).replace("$", SYMBOL_DOLLAR)
}

pub fn unescape_symbol(name: &str) -> String {
    name.replace(SYMBOL_AT, ".").replace(SYMBOL_DOLLAR, "$")
}

pub enum SourceType {
    StdIn,
    Path(String),
}

static STDIN_NAME: LazyLock<String> = LazyLock::new(|| String::from("<stdin>"));

impl SourceType {
    fn name(&self) -> &String {
        match self {
            Self::StdIn => &STDIN_NAME,
            Self::Path(s) => s,
        }
    }
}

pub struct NamedString {
    pub source: SourceType,
    pub content: String,
    pub encoding: &'static Encoding,
    pub src_dir: PathBuf,
}

impl NamedString {
    fn with_tmp<F, T>(&self, f: F) -> Result<T, Box<dyn Error>>
    where
        F: FnOnce(&Path) -> Result<T, Box<dyn Error>>,
    {
        // TODO this should really check and do this only
        //      for stdin. relatively expensive

        let (output, _, failure) = self.encoding.encode(&self.content);

        if failure {
            panic!(
                "Could not encode {} as {}",
                self.source.name(),
                self.encoding.name()
            );
        }

        let c_file = Builder::new().suffix(".c").tempfile_in(&self.src_dir)?;
        std::fs::write(c_file.path(), &output)?;
        let r = f(c_file.path());
        if let Err(e) = r {
            eprintln!(
                "Error occurred, temporary file available at {}",
                c_file.path().display()
            );
            // TODO: make saving intermediate temp files an option
            // std::mem::forget(c_file);
            return Err(e);
        }
        r
    }
}

pub fn process_c_file(
    c_content: &NamedString,
    o_file: &Path,
    preprocessor: &Arc<Preprocessor>,
    compiler: &Compiler,
    assembler: &Assembler,
    split_sections: bool,
    split_plain_names: bool,
    elf_flags: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Preprocess to find INCLUDE_ASM macros and produce stub source.
    let (new_lines, asm_files) = preprocessor.find_macros(&c_content.content);

    if asm_files.is_empty() {
        // No INCLUDE_ASM macros: compile the original file directly.
        let (obj_bytes, make_rule) = c_content
            .with_tmp(|c_file| Ok(compiler.compile_file(c_file, c_content.source.name())?))?;
        write_dependency_file(compiler, make_rule, c_content, o_file)?;
        if split_sections {
            let mut elf = Elf::from_bytes(&obj_bytes);
            if let Some(flags) = elf_flags {
                elf.header.e_flags = flags;
            }
            split::split_monolithic_sections(&mut elf, split_plain_names)?;
            return write_obj(o_file, &elf.pack());
        }
        if let Some(flags) = elf_flags {
            let mut patched = obj_bytes;
            patched[36..40].copy_from_slice(&flags.to_le_bytes());
            return write_obj(o_file, &patched);
        }
        return write_obj(o_file, &obj_bytes);
    }


    if split_sections {
        // Compile stubs into monolithic .o
        let temp_c = Builder::new()
            .suffix(".c")
            .tempfile_in(&c_content.src_dir)?;
        std::fs::write(temp_c.path(), &new_lines)?;
        let (obj_bytes, make_rule) =
            compiler.compile_file(temp_c.path(), c_content.source.name())?;
        write_dependency_file(compiler, make_rule, c_content, o_file)?;
        let mut elf = Elf::from_bytes(&obj_bytes);
        if let Some(flags) = elf_flags {
            elf.header.e_flags = flags;
        }
        elf.symbol_cleanup();

        // INCLUDE_ASM merge: move DEFINED ___mw___ stub into its UND slot so
        // the symbol keeps the earlier position mwcc would have emitted natively.
        {
            let old_len = elf.symtab.symbols.len();
            let mut merges: Vec<(usize, usize)> = Vec::new();
            for (i, sym) in elf.symtab.symbols.iter().enumerate() {
                if sym.st_shndx != 0 || sym.name.is_empty() {
                    continue;
                }
                if let Some(def_idx) = elf.symtab.symbols.iter()
                    .position(|s| s.name == sym.name && s.st_shndx != 0)
                {
                    merges.push((i, def_idx));
                }
            }

            if !merges.is_empty() {
                for &(und_idx, def_idx) in &merges {
                    let def_sym = elf.symtab.symbols[def_idx].clone();
                    let und_st_name = elf.symtab.symbols[und_idx].st_name;
                    elf.symtab.symbols[und_idx] = def_sym;
                    elf.symtab.symbols[und_idx].st_name = und_st_name;
                }

                // Build old→new index map; removed DEFINED entries collapse to
                // the UND slot they were merged into.
                let remove_set: HashSet<usize> =
                    merges.iter().map(|(_, d)| *d).collect();
                let mut final_map: Vec<usize> = vec![0; old_len];
                let mut new_idx = 0;
                for old_idx in 0..old_len {
                    if !remove_set.contains(&old_idx) {
                        final_map[old_idx] = new_idx;
                        new_idx += 1;
                    }
                }
                for &(und_idx, def_idx) in &merges {
                    final_map[def_idx] = final_map[und_idx];
                }
                // Remap all relocations
                for section in &mut elf.sections {
                    if section.sh_type == SHT_REL {
                        for chunk in section.data.chunks_mut(8) {
                            let r_info = u32::from_le_bytes(chunk[4..8].try_into().unwrap());
                            let sym_idx = (r_info >> 8) as usize;
                            let type_id = r_info & 0xff;
                            if sym_idx < old_len {
                                let new_info = ((final_map[sym_idx] as u32) << 8) | type_id;
                                chunk[4..8].copy_from_slice(&new_info.to_le_bytes());
                            }
                        }
                    }
                }
                // Remove symbols in reverse order
                let mut to_remove: Vec<usize> = remove_set.into_iter().collect();
                to_remove.sort();
                for idx in to_remove.into_iter().rev() {
                    elf.symtab.symbols.remove(idx);
                }
                // Update sh_info (first non-local index)
                elf.symtab.section.sh_info = elf.symtab.symbols.iter()
                    .position(|s| s.bind() != 0)
                    .unwrap_or(elf.symtab.symbols.len()) as u32;
            }
        }
        // Assemble all .s files in parallel
        let asm_objects: Vec<(&PathBuf, usize, Vec<u8>)> = asm_files
            .par_iter()
            .map(|(asm_file, num_rodata_symbols)| {
                let assembled_bytes = assembler
                    .assemble_file(asm_file)
                    .expect("assembled bytes");
                (asm_file, *num_rodata_symbols, assembled_bytes)
            })
            .collect();

        // Splice each into the monolithic sections
        for (asm_file, _num_rodata, assembled_bytes) in &asm_objects {
            splice_asm_into_monolithic(&mut elf, asm_file, assembled_bytes)?;
        }

        // Sync tables and split
        elf.symtab.pack_data();
        elf.sections[elf.symtab_idx] = elf.symtab.section.clone();
        elf.strtab.pack_data();
        elf.sections[elf.strtab_idx] = elf.strtab.section.clone();
        split::split_monolithic_sections(&mut elf, split_plain_names)?;
        return write_obj(o_file, &elf.pack());
    }

    // --- Original non-split INCLUDE_ASM path (unchanged) ---

    // 3. Create temp C file with stubs
    let temp_c = Builder::new()
        .suffix(".c")
        .tempfile_in(&c_content.src_dir)?;
    std::fs::write(temp_c.path(), new_lines)?;

    let (recompiled_bytes, make_rule) =
        compiler.compile_file(temp_c.path(), c_content.source.name())?;
    let mut compiled_elf = Elf::from_bytes(&recompiled_bytes);

    let rel_text_sh_name = compiled_elf.add_sh_symbol(".rel.text".to_string());

    let stub_functions: HashSet<String> = compiled_elf
        .function_names()
        .iter()
        .filter_map(|name| name.strip_prefix(FUNCTION_PREFIX))
        .map(str::to_string)
        .collect();

    compiled_elf.symbol_cleanup();

    let symbol_to_section_idx: HashMap<String, u16> = compiled_elf
        .symtab()
        .symbols
        .iter()
        .map(|sym| (sym.name.clone(), sym.st_shndx))
        .collect();

    let asm_objects: Vec<(&PathBuf, usize, Vec<u8>)> = asm_files
        .par_iter()
        .map(|(asm_file, num_rodata_symbols)| {
            let assembled_bytes = assembler.assemble_file(asm_file).expect("assembled bytes");
            (asm_file, *num_rodata_symbols, assembled_bytes)
        })
        .collect();

    for (asm_file, num_rodata_symbols, assembled_bytes) in asm_objects {
        let function = asm_file.file_stem().unwrap().to_str().unwrap();

        let mut assembled_elf = Elf::from_bytes(&assembled_bytes);

        let asm_functions = assembled_elf.get_functions();
        assert_eq!(1, asm_functions.len());

        let asm_text = &asm_functions[0].section.data;

        // if this is a function and that function is not an INCLUDE_ASM, ignore
        let asm_main_symbol = asm_file.file_stem().unwrap().display().to_string();
        if !asm_text.is_empty() && !stub_functions.contains(&asm_main_symbol) {
            continue;
        }

        let mut rodata_section_indices: Vec<usize> = vec![];
        let mut text_section_index: usize = 0xFFFFFFFF;

        if !asm_text.is_empty() {
            text_section_index = compiled_elf.text_section_by_name(function);

            // assumption is that .rodata will immediately follow the .text section
            if num_rodata_symbols > 0 {
                let mut i = text_section_index + 1;
                for section in &compiled_elf.sections[i..] {
                    if section.name == ".rodata" {
                        rodata_section_indices.push(i);
                        if rodata_section_indices.len() == num_rodata_symbols {
                            break;
                        }
                    }
                    i += 1;
                }
            }

            assert_eq!(
                num_rodata_symbols,
                rodata_section_indices.len(),
                ".rodata section count mismatch"
            );

            // transplant .text section data from assembled object
            let text_section = &mut compiled_elf.sections[text_section_index];
            assert!(
                asm_text.len() >= text_section.data.len(),
                "Not enough assembly to fill {function} in {}",
                c_content.source.name()
            );
            text_section.data = asm_text[..text_section.data.len()].to_vec();
            if text_section.data.len() < text_section.sh_size as usize {
                let needed_bytes: usize = text_section.sh_size as usize - text_section.data.len();
                text_section.data.extend(vec![0u8; needed_bytes]);
            }
        } else {
            // this file only contains rodata
            assert_eq!(1, num_rodata_symbols);
            let idx = symbol_to_section_idx[function];
            rodata_section_indices.push(idx.into());
        }

        let mut rodata_section_offsets: Vec<usize> = vec![];

        let rel_rodata_sh_name = if num_rodata_symbols > 0 {
            let rodata_sections = assembled_elf.rodata_sections();
            assert_eq!(
                1,
                rodata_sections.len(),
                "Expected ASM to contain 1 .rodata section, found {}",
                rodata_sections.len()
            );

            let asm_rodata = rodata_sections[0];
            let mut offset: usize = 0;
            for idx in &rodata_section_indices {
                // copy slices of rodata from ASM object into each .rodata section
                let data_len = compiled_elf.sections[*idx].data.len();
                compiled_elf.sections[*idx].data =
                    asm_rodata.data[offset..(offset + data_len)].to_vec();
                offset += data_len;
                rodata_section_offsets.push(offset);

                // force 4-byte alignment for .rodata sections (defaults to 16-byte)
                compiled_elf.sections[*idx].sh_addralign = 2;
            }

            compiled_elf.add_sh_symbol(".rel.rodata")
        } else {
            0xFFFFFFFFu32
        };

        let relocation_records = assembled_elf.reloc_sections();
        assert!(
            relocation_records.len() < 3,
            "{} has too many relocation records",
            asm_file.display()
        );
        let mut reloc_symbols: HashSet<String> = HashSet::new();

        let initial_sh_info_value = compiled_elf.symtab().section.sh_info;
        let mut local_syms_inserted: usize = 0;

        // assumes .text relocations precede .rodata relocations
        for (i, (_, relocation_record)) in relocation_records.into_iter().enumerate() {
            let mut relocation_record = relocation_record.clone();
            relocation_record.sh_link = compiled_elf.symtab_idx as u32;
            if !asm_text.is_empty() && i == 0 {
                relocation_record.sh_name = rel_text_sh_name;
                relocation_record.sh_info = text_section_index as u32;
            } else {
                relocation_record.sh_name = rel_rodata_sh_name;
                relocation_record.sh_info = rodata_section_indices[0] as u32;
            }

            let mut assembled_symtab = assembled_elf.symtab().clone();
            let mut rr = RelocationRecord::new(relocation_record);

            for relocation in &mut rr.relocations {
                let symbol = &mut assembled_symtab.symbols[relocation.symbol_index()];
                if symbol.bind() == 0 {
                    local_syms_inserted += 1;
                }

                let force = asm_text.is_empty() || i != 0;
                if !asm_text.is_empty() && i == 1 {
                    // repoint .rodata reloc to .text section
                    symbol.st_shndx = text_section_index as u16;
                }

                let index = compiled_elf.add_symbol_get_index(symbol.clone(), force) as u32;
                relocation.set_symbol_index(index);
                reloc_symbols.insert(symbol.name.clone());
            }
            rr.pack();
            assembled_elf.set_symtab(&assembled_symtab);
            compiled_elf.add_section(rr.section);
        }

        let mut new_rodata_relocs: Vec<Section> = vec![];
        if local_syms_inserted > 0 {
            // update relocations
            let relocation_sections = compiled_elf.reloc_sections();
            for (idx, relocation_section) in relocation_sections {
                let mut relocation_record = RelocationRecord::new(relocation_section.clone());

                // Check if this is a rodata relocation that needs splitting
                if relocation_record.section.sh_info == rodata_section_indices[0] as u32 {
                    if num_rodata_symbols == 1 {
                        continue; // nothing to do
                    }

                    // Split relocations across multiple .rodata sections
                    let mut new_relocations: Vec<Vec<Relocation>> =
                        vec![vec![]; rodata_section_indices.len()];

                    for mut relocation in relocation_record.relocations.clone() {
                        // Find which rodata section this relocation belongs to
                        for i in 0..rodata_section_offsets.len() {
                            if relocation.r_offset < rodata_section_offsets[i] as u32 {
                                if i > 0 {
                                    // Adjust offset relative to this section's start
                                    relocation.r_offset -= rodata_section_offsets[i - 1] as u32;
                                }
                                new_relocations[i].push(relocation);
                                break;
                            }
                        }
                    }

                    // Create new relocation records for each rodata section
                    for (i, relocations) in new_relocations.iter().enumerate() {
                        let mut new_rodata_reloc = if i == 0 {
                            relocation_record.section.clone()
                        } else {
                            relocation_section.clone()
                        };

                        new_rodata_reloc.sh_info = rodata_section_indices[i] as u32;

                        let mut new_reloc_record = RelocationRecord::new(new_rodata_reloc);
                        new_reloc_record.relocations = relocations.clone();
                        new_reloc_record.pack();
                        new_rodata_relocs.push(new_reloc_record.section.clone());

                        if i == 0 {
                            // the original relocation section needs to be updated here
                            compiled_elf.sections[idx] = new_reloc_record.section;
                        }
                    }

                    continue;
                }

                // Update symbol indices for other relocations
                for relocation in &mut relocation_record.relocations {
                    if relocation.symbol_index() >= initial_sh_info_value as usize {
                        relocation.set_symbol_index(
                            (relocation.symbol_index() + local_syms_inserted) as u32,
                        );
                    }
                }

                // Update the section in the ELF
                relocation_record.pack();
                let section_idx = compiled_elf
                    .sections
                    .iter()
                    .position(|s| {
                        s.sh_type == relocation_section.sh_type
                            && s.sh_info == relocation_section.sh_info
                            && s.sh_name == relocation_section.sh_name
                    })
                    .expect("relocation section not found");
                compiled_elf.sections[section_idx] = relocation_record.section;
            }

            // Add the new rodata relocation sections (skip first as it was amended in place)
            for new_rodata_reloc in new_rodata_relocs.into_iter().skip(1) {
                compiled_elf.add_section(new_rodata_reloc);
            }
        }

        for symbol in assembled_elf.get_symbols() {
            if symbol.st_name == 0 {
                continue; // Skip null symbol
            }

            if symbol.bind() == 0 {
                continue; // Ignore local symbols
            }

            // TODO: is the symbol text alread here?
            if !asm_text.is_empty() && !reloc_symbols.contains(&symbol.name) {
                let mut sym = symbol.clone();
                sym.st_shndx = text_section_index as u16;
                compiled_elf.add_symbol(sym);
            }
        }
    }

    write_dependency_file(compiler, make_rule, c_content, o_file)?;
    if let Some(flags) = elf_flags {
        compiled_elf.header.e_flags = flags;
    }
    write_obj(o_file, &compiled_elf.pack())?;

    Ok(())
}

/// Splice one assembled .s file's code/data into the monolithic ELF sections.
/// Replaces nop stubs with real instructions and appends relocations.
fn splice_asm_into_monolithic(
    elf: &mut Elf,
    asm_file: &Path,
    assembled_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let function_name = asm_file.file_stem().unwrap().to_str().unwrap();
    let asm_elf = Elf::from_bytes(assembled_bytes);

    let asm_functions = asm_elf.get_functions();
    let has_text = !asm_functions.is_empty() && !asm_functions[0].section.data.is_empty();

    // Find monolithic section indices
    let text_idx = elf.sections.iter().position(|s| s.name == ".text");
    let rodata_idx = elf.sections.iter().position(|s| s.name == ".rodata");

    // Find the function symbol in the compiled ELF (after symbol_cleanup)
    let func_sym = if has_text {
        elf.find_symbol(function_name)
    } else {
        None
    };

    // 1. Splice .text bytes
    if let (Some(fs), Some(ti)) = (&func_sym, text_idx) {
        let asm_text = &asm_functions[0].section.data;
        let offset = fs.st_value as usize;
        let size = fs.st_size as usize;
        assert!(
            asm_text.len() >= size,
            "Assembly for {} is {} bytes but stub is {}",
            function_name, asm_text.len(), size
        );
        elf.sections[ti].data[offset..offset + size]
            .copy_from_slice(&asm_text[..size]);
    }

    // 2. Splice .rodata bytes
    let asm_rodata_sections = asm_elf.rodata_sections();
    if let (Some(asm_rd), Some(ri)) = (asm_rodata_sections.first(), rodata_idx) {
        let asm_rodata_idx = asm_elf
            .sections
            .iter()
            .position(|s| s.name == ".rodata")
            .unwrap_or(0);

        let asm_rodata_syms: Vec<&crate::elf::Symbol> = asm_elf
            .symtab
            .symbols
            .iter()
            .filter(|s| {
                s.st_shndx as usize == asm_rodata_idx
                    && s.st_size > 0
                    && !s.name.contains("NON_MATCHING")
            })
            .collect();

        let mut asm_offset = 0usize;
        for asm_sym in &asm_rodata_syms {
            if let Some(compiled_sym) = elf.find_symbol(&asm_sym.name) {
                let rdo = compiled_sym.st_value as usize;
                let rdsz = compiled_sym.st_size as usize;
                elf.sections[ri].data[rdo..rdo + rdsz]
                    .copy_from_slice(&asm_rd.data[asm_offset..asm_offset + rdsz]);
                asm_offset += rdsz;
            }
        }
    }

    // 3. Append relocations
    let asm_reloc_sections = asm_elf.reloc_sections();
    for (_, rel_section) in &asm_reloc_sections {
        // Only process .rel.text and .rel.rodata; skip .rel.pdr etc.
        let is_text_rel = rel_section.name.contains(".text");
        let is_rodata_rel = rel_section.name.contains(".rodata");
        if !is_text_rel && !is_rodata_rel {
            continue;
        }
        let relocations = crate::elf::section::Relocation::unpack_all(&rel_section.data);

        let base_offset = if is_text_rel {
            func_sym.as_ref().map(|fs| fs.st_value).unwrap_or(0)
        } else {
            let asm_rodata_idx = asm_elf
                .sections
                .iter()
                .position(|s| s.name == ".rodata")
                .unwrap_or(0);
            let first_asm_rd = asm_elf
                .symtab
                .symbols
                .iter()
                .find(|s| {
                    s.st_shndx as usize == asm_rodata_idx
                        && s.st_size > 0
                        && !s.name.contains("NON_MATCHING")
                });
            if let Some(asm_sym) = first_asm_rd {
                elf.find_symbol(&asm_sym.name)
                    .map(|s| s.st_value)
                    .unwrap_or(0)
            } else {
                0
            }
        };

        let rel_name = if is_text_rel { ".rel.text" } else { ".rel.rodata" };
        let compiled_rel_idx = match elf.sections.iter().position(|s| s.name == rel_name) {
            Some(idx) => idx,
            None => {
                let sh_name = elf.shstrtab.add_symbol(rel_name);
                let target_idx = if is_text_rel {
                    match text_idx {
                        Some(ti) => ti,
                        None => continue,
                    }
                } else {
                    match rodata_idx {
                        Some(ri) => ri,
                        None => continue,
                    }
                };
                let mut sec = Section::new(
                    sh_name, SHT_REL, 0, 0, 0, 0,
                    elf.symtab_idx as u32,
                    target_idx as u32,
                    4, 8, vec![],
                );
                sec.name = rel_name.to_string();
                elf.sections.push(sec);
                elf.sections.len() - 1
            }
        };

        for reloc in &relocations {
            let mut new_reloc = reloc.clone();
            new_reloc.r_offset += base_offset;

            let asm_sym = &asm_elf.symtab().symbols[reloc.symbol_index()];

            if asm_sym.type_id() == STT_SECTION {
                let asm_sec = &asm_elf.sections[asm_sym.st_shndx as usize];
                if asm_sec.name.contains("text") {
                    if !is_text_rel {
                        // Rodata jtable entry referencing .text section symbol.
                        // Read the addend from rodata data and find the matching
                        // .L label so objdiff sees the correct symbol reference.
                        let mut found_label = false;
                        if let Some(ri) = rodata_idx {
                            let offset = new_reloc.r_offset as usize;
                            if offset + 4 <= elf.sections[ri].data.len() {
                                let addend = u32::from_le_bytes(
                                    elf.sections[ri].data[offset..offset + 4]
                                        .try_into()
                                        .unwrap(),
                                );
                                let asm_text_idx = asm_elf
                                    .sections
                                    .iter()
                                    .position(|s| s.name == ".text")
                                    .unwrap_or(0);
                                let label_sym = asm_elf.symtab.symbols.iter().find(|s| {
                                    s.st_shndx as usize == asm_text_idx
                                        && s.st_value == addend
                                        && !s.name.is_empty()
                                        && s.type_id() != STT_SECTION
                                });
                                if let Some(label) = label_sym {
                                    let mut sym = label.clone();
                                    if let Some(ti) = text_idx {
                                        sym.st_shndx = ti as u16;
                                        if let Some(ref fs) = func_sym {
                                            sym.st_value += fs.st_value;
                                        }
                                    }
                                    let idx = elf.add_symbol_get_index(sym.clone(), false);
                                    let existing = &mut elf.symtab.symbols[idx];
                                    if existing.st_shndx == 0 && sym.st_shndx != 0 {
                                        existing.st_shndx = sym.st_shndx;
                                        existing.st_value = sym.st_value;
                                        existing.st_size = sym.st_size;
                                        existing.st_info = sym.st_info;
                                    }
                                    new_reloc.set_symbol_index(idx as u32);
                                    // Zero the data — symbol value now carries the address
                                    elf.sections[ri].data[offset..offset + 4]
                                        .copy_from_slice(&[0, 0, 0, 0]);
                                    found_label = true;
                                }
                            }
                        }
                        if !found_label {
                            // Fallback: use function symbol
                            if let Some((fi, _)) =
                                elf.symtab.get_symbol_by_name(function_name.to_string())
                            {
                                new_reloc.set_symbol_index(fi as u32);
                            }
                        }
                    } else {
                        // .text relocation — use function symbol
                        if let Some((fi, _)) =
                            elf.symtab.get_symbol_by_name(function_name.to_string())
                        {
                            new_reloc.set_symbol_index(fi as u32);
                        }
                    }
                } else if asm_sec.name.contains("rodata") {
                    let asm_ri = asm_elf
                        .sections
                        .iter()
                        .position(|s| s.name == ".rodata")
                        .unwrap_or(0);
                    let first = asm_elf
                        .symtab
                        .symbols
                        .iter()
                        .find(|s| s.st_shndx as usize == asm_ri && s.st_size > 0);
                    if let Some(fs) = first {
                        if let Some((ri, _)) =
                            elf.symtab.get_symbol_by_name(fs.name.clone())
                        {
                            new_reloc.set_symbol_index(ri as u32);
                        }
                    }
                }
            } else {
                let mut sym_to_add = asm_sym.clone();

                // Local .text symbols (.L labels, jump targets): convert relocation
                // to reference the function symbol instead of adding locals to symtab
                // (adding locals shifts global indices and breaks existing relocations)
                if sym_to_add.bind() == 0 && sym_to_add.st_shndx != 0 {
                    let asm_sec = &asm_elf.sections[sym_to_add.st_shndx as usize];
                    if asm_sec.name.contains("text") {
                        if let Some((fi, _)) =
                            elf.symtab.get_symbol_by_name(function_name.to_string())
                        {
                            new_reloc.set_symbol_index(fi as u32);
                            elf.sections[compiled_rel_idx]
                                .data
                                .extend_from_slice(&new_reloc.pack());
                            elf.sections[compiled_rel_idx].sh_size =
                                elf.sections[compiled_rel_idx].data.len() as u32;
                            continue;
                        }
                    }
                }

                // Normal path: remap section index to monolithic
                if sym_to_add.st_shndx != 0 {
                    let asm_sec = &asm_elf.sections[sym_to_add.st_shndx as usize];
                    if asm_sec.name.contains("text") {
                        if let Some(ti) = text_idx {
                            sym_to_add.st_shndx = ti as u16;
                            if let Some(ref fs) = func_sym {
                                sym_to_add.st_value += fs.st_value;
                            }
                        }
                    } else if asm_sec.name.contains("rodata") {
                        if let Some(ri) = rodata_idx {
                            sym_to_add.st_shndx = ri as u16;
                        }
                    }
                }
                let idx = elf.add_symbol_get_index(sym_to_add.clone(), false);
                // Upgrade UND → defined if we now have a definition
                if sym_to_add.st_shndx != 0 {
                    let existing = &mut elf.symtab.symbols[idx];
                    if existing.st_shndx == 0 {
                        existing.st_shndx = sym_to_add.st_shndx;
                        existing.st_value = sym_to_add.st_value;
                        existing.st_size = sym_to_add.st_size;
                        existing.st_info = sym_to_add.st_info;
                    }
                }
                new_reloc.set_symbol_index(idx as u32);
            }

            elf.sections[compiled_rel_idx]
                .data
                .extend_from_slice(&new_reloc.pack());
            elf.sections[compiled_rel_idx].sh_size =
                elf.sections[compiled_rel_idx].data.len() as u32;
        }
    }
    // Add all defined .text symbols from assembled .o (e.g., .L jump labels
    // that the jtable references but .text doesn't branch to directly)
    for symbol in asm_elf.get_symbols() {
        if symbol.st_name == 0 || symbol.st_shndx == 0 || symbol.st_shndx >= 0xFF00 {
            continue;
        }
        let asm_sec = &asm_elf.sections[symbol.st_shndx as usize];
        if asm_sec.name.contains("text") {
            let mut sym = symbol.clone();
            if let Some(ti) = text_idx {
                sym.st_shndx = ti as u16;
                if let Some(ref fs) = func_sym {
                    sym.st_value += fs.st_value;
                }
            }
            let idx = elf.add_symbol_get_index(sym.clone(), false);
            let existing = &mut elf.symtab.symbols[idx];
            if existing.st_shndx == 0 && sym.st_shndx != 0 {
                existing.st_shndx = sym.st_shndx;
                existing.st_value = sym.st_value;
                existing.st_size = sym.st_size;
                existing.st_info = sym.st_info;
            }
        }
    }

    Ok(())
}

fn ensure_parent_dir<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.is_dir() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

pub fn write_obj<P: AsRef<Path>>(path: P, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    ensure_parent_dir(&path)?;
    std::fs::write(path.as_ref(), bytes)?;

    Ok(())
}

fn write_dependency_file<P: AsRef<Path>>(
    compiler: &Compiler,
    make_rule: Option<MakeRule>,
    c_content: &NamedString,
    o_file: P,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(mut make_rule) = make_rule else {
        return Ok(());
    };

    make_rule.target = o_file.as_ref().to_string_lossy().into_owned();

    // For stdin there is no meaningful source path to record.
    // For a real file, include it only when running interactively; piped
    // build systems (make, ninja) already know the source from their own rules.
    make_rule.source = if matches!(c_content.source, SourceType::StdIn) {
        None
    } else if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        Some(c_content.source.name().clone())
    } else {
        None
    };

    let d_file = if compiler.gcc_deps {
        // gcc mode: write the .d file alongside the output object as <name>.o.d
        let mut p = o_file.as_ref().to_path_buf();
        let new_ext = format!(
            "{}.d",
            o_file
                .as_ref()
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
        );
        p.set_extension(new_ext);
        p
    } else {
        // mw mode: write the .d file alongside the source .c file as <name>.d
        PathBuf::from(&c_content.source.name()).with_extension("d")
    };

    ensure_parent_dir(&d_file)?;
    std::fs::write(&d_file, make_rule.as_str().as_bytes())?;

    Ok(())
}
