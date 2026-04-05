// SPDX-FileCopyrightText: © 2026 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use unescape::unescape;

use crate::constants::*;

#[derive(Debug, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub size: usize,
    pub local: bool,
}

#[derive(Debug)]
pub struct Preprocessor {
    pub asm_dir_prefix: Option<PathBuf>,
}

static STRING_REGEX: OnceLock<Regex> = OnceLock::new();

fn string_regex() -> &'static Regex {
    STRING_REGEX.get_or_init(|| Regex::new(r#"^\.(asci[iz])\s+"(.*)""#).unwrap())
}

static INCLUDE_REGEX: OnceLock<Regex> = OnceLock::new();
fn include_regex() -> &'static Regex {
    INCLUDE_REGEX
        .get_or_init(|| Regex::new(r#"INCLUDE_(?:ASM|RODATA)\(\s*"(.*)"\s*,\s*(.*)\s*\)"#).unwrap())
}

impl Preprocessor {
    pub fn new(asm_dir_prefix: Option<PathBuf>) -> Self {
        Self { asm_dir_prefix }
    }

    pub fn preprocess_s_file(function_name: &str, content: &str) -> (Vec<String>, Vec<Symbol>) {
        // eprintln!("preprocessing: {function_name}");
        let mut rodata_entries = Vec::new();
        let mut c_lines = Vec::new();
        let mut nops_needed = 0;
        let mut in_rodata = false;
        let mut current_symbol: Option<usize> = None;

        let mut i = 0;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with(".section") {
                if line.ends_with(".text") {
                    in_rodata = false;
                    continue;
                }
                if line.ends_with(".rodata") {
                    in_rodata = true;
                    continue;
                }

                panic!("Unsupported .section found at line {}: {line}", i + 1);
            }

            if in_rodata {
                if line.starts_with(".align") {
                    continue;
                }
                if line.starts_with(".size") {
                    continue;
                }
                if line.starts_with("nonmatching") {
                    continue;
                }
                if line.starts_with("enddlabel") {
                    continue;
                }
                if line.starts_with("glabel") || line.starts_with("dlabel") {
                    let prefix_str = line.replace(LOCAL_SUFFIX, "");
                    let parts: Vec<&str> = prefix_str.split_whitespace().collect();

                    let name = parts[1].to_string();
                    let is_local = line.ends_with(LOCAL_SUFFIX) || name.contains(DOLLAR_SIGN);
                    rodata_entries.push(Symbol {
                        name,
                        size: 0,
                        local: is_local,
                    });
                    current_symbol = Some(rodata_entries.len() - 1);

                    continue;
                }

                if let Some(idx) = current_symbol {
                    if line.contains(".byte ") {
                        rodata_entries[idx].size += 1;
                        continue;
                    } else if line.contains(".short ") {
                        rodata_entries[idx].size += 2;
                        continue;
                    } else if line.contains(".word ")
                        || line.contains(".long ")
                        || line.contains(".float ")
                    {
                        rodata_entries[idx].size += 4;
                        continue;
                    } else if line.contains(".double ") {
                        rodata_entries[idx].size += 8;
                        continue;
                    } else if let Some(caps) = string_regex().captures(line) {
                        // 1. Extract type and literal
                        let directive = &caps[1];
                        let literal = &caps[2];

                        // 2. Unescape the string to get actual bytes
                        // unescape() expects the content without the surrounding quotes
                        if let Some(unescaped) = unescape(literal) {
                            let mut len = unescaped.len();

                            // 3. Match asciz condition (null terminator)
                            if directive == "asciz" {
                                len += 1;
                            }
                            rodata_entries[idx].size += len;
                        }
                        continue;
                    }
                }
                panic!(
                    "Unexpected entry in .rodata section at line {}: {line}",
                    i + 1
                );
            } else {
                if line.starts_with(".set")
                    || line.starts_with(".include")
                    || line.starts_with(".size")
                    || line.starts_with(".align")
                    || line.starts_with(".balign")
                    || line.starts_with("glabel")
                    || line.starts_with("endlabel")
                    || line.starts_with("jlabel")
                    || line.starts_with(".L")
                    || line.ends_with(":")
                    || line.starts_with("/* Generated by spimdisasm")
                    || line.starts_with("nonmatching")
                    || line.starts_with("/* Handwritten function")
                    || line.starts_with("#")
                {
                    continue;
                }
                // Simplified text section scan
                if !line.starts_with('.') && !line.contains(':') {
                    nops_needed += 1;
                }
            }

            i += 1;
        }

        if nops_needed > 0 {
            // prototpye
            c_lines.push(format!("void {}();", function_name));
            c_lines.push(format!("asm void {}() {{", function_name));
            for _ in 0..nops_needed {
                c_lines.push("  nop".to_string());
            }
            c_lines.push("}".to_string());
        }

        for sym in &rodata_entries {
            let prefix = if sym.local { "static " } else { "" };
            let size = sym.size;

            let sym_name = if sym.name.starts_with("\"@") && sym.name.ends_with("\"") {
                SYMBOL_AT.to_owned() + &sym.name[2..(sym.name.len() - 1)]
            } else if sym.name.contains(DOLLAR_SIGN) {
                sym.name.replace(DOLLAR_SIGN, SYMBOL_DOLLAR)
            } else {
                sym.name.to_owned()
            };

            c_lines.push(format!(
                "{}const unsigned char {}[{}] = {{0}};",
                prefix, sym_name, size
            ));
        }

        (c_lines, rodata_entries)
    }

    pub fn find_macros(&self, content: &str) -> (String, Vec<(PathBuf, usize)>) {
        let mut out_lines = String::with_capacity(content.len());
        let mut asm_files = vec![];

        let include_regex = include_regex();

        let mut last_match = 0;
        for caps in include_regex.captures_iter(content) {
            let m = caps.get(0).unwrap();
            out_lines.push_str(&content[last_match..m.start()]);
            last_match = m.end();

            let asm_dir = Path::new(&caps[1]);
            let asm_func = &caps[2];

            let mut asm_path = asm_dir.join(format!("{}.s", asm_func));
            if let Some(prefix) = &self.asm_dir_prefix {
                asm_path = prefix.join(asm_path);
            }

            if let Ok(asm_content) = fs::read_to_string(&asm_path) {
                let (stubs, rodata_entries) =
                    Self::preprocess_s_file(&format!("{FUNCTION_PREFIX}{asm_func}"), &asm_content);
                asm_files.push((asm_path, rodata_entries.len()));
                for stub in stubs {
                    out_lines.push_str(&stub);
                    out_lines.push('\n');
                }
            }
        }
        if last_match == 0 {
            out_lines = String::from(content);
        } else {
            out_lines.push_str(&content[last_match..]);
        }
        (out_lines, asm_files)
    }
}

#[cfg(test)]
mod tests {
    use crate::preprocessor::{Preprocessor, Symbol};

    #[test]
    fn test_empty() {
        let (c_lines, rodata_entries) = Preprocessor::preprocess_s_file("empty.s", "");
        assert_eq!(c_lines.len(), 0);
        assert_eq!(rodata_entries.len(), 0);
    }

    #[test]
    fn test_empty_function() {
        let asm_contents = ".section .text\nglabel empty_func";
        let (c_lines, rodata_entries) =
            Preprocessor::preprocess_s_file("text_only.s", asm_contents);
        assert_eq!(c_lines, Vec::<String>::new());
        assert_eq!(rodata_entries, Vec::<Symbol>::new());
    }

    #[test]
    fn test_simple() {
        let asm_contents = r#"
.set noat
.set noreorder
glabel Bg_Disp_Switch
    addiu      $sp, $sp, -0x10
    sb         $a0, 0x0($sp)
    lbu        $v1, 0x0($sp)
    sb         $v1, %gp_rel(bg_disp_off)($gp)
    addiu      $sp, $sp, 0x10
    jr         $ra
    nop
.size Bg_Disp_Switch, . - Bg_Disp_Switch
    nop
"#;
        let (c_lines, _) = Preprocessor::preprocess_s_file("simple.s", asm_contents);

        // eprintln!("c clines: {c_lines:?}");

        // 8 instructions total (including 2 nops) + 2 wrapper lines (asm void { ... })
        let expected_nops = 8;
        assert_eq!(c_lines.len(), expected_nops + 2);
    }

    #[test]
    fn test_rodata_words() {
        let asm_contents = r#"
.section .rodata
.align 3
dlabel literal_515_00552620
    .word 0x00396B08
    .word 0x00396B08
    .word 0x00396AD0
    .word 0x00396B08
    .word 0x00396AD0
    .word 0x00396A30
    .word 0x00396AE8
    .word 0x00396A58
    .word 0x00396B08
    .word 0x00396A78
    .word 0x00000000
    .word 0x00000000
.size literal_515_00552620, . - literal_515_00552620
"#;
        let (c_lines, rodata_entries) = Preprocessor::preprocess_s_file("rodata.s", asm_contents);

        assert_eq!(c_lines.len(), 1);
        let sym = rodata_entries
            .iter()
            .find(|s| s.name == "literal_515_00552620")
            .unwrap();
        assert_eq!(sym.size, 12 * 4);
    }

    #[test]
    fn test_rodata_asciz() {
        let asm_contents = r#"
.section .rodata
.align 2
dlabel foobar
    .asciz "SHAUN PALMER"
.size foobar, . - foobar
"#;
        let (_, rodata_entries) = Preprocessor::preprocess_s_file("asciz.s", asm_contents);

        let sym = rodata_entries.iter().find(|s| s.name == "foobar").unwrap();
        // 12 chars + 1 null terminator
        assert_eq!(sym.size, 13);
    }

    #[test]
    fn test_local_symbol() {
        let asm_contents = r#"
.section .rodata
glabel D_psp_0914A7B8, local
    .word 0x12345678
"#;
        let (c_lines, rodata_entries) = Preprocessor::preprocess_s_file("local.s", asm_contents);

        // eprintln!("rodata entries: {:?}", rodata_entries);
        // eprintln!("c_lines: {:?}", c_lines);

        let sym = rodata_entries
            .iter()
            .find(|s| s.name == "D_psp_0914A7B8")
            .unwrap();
        assert!(sym.local);
        assert!(c_lines[0].starts_with("static "));
    }

    #[test]
    fn test_dollar_symbol() {
        let asm_contents = r#"
.section .rodata
dlabel foo$bar$baz
    .word 0x1234
"#;
        let (_, rodata_entries) = Preprocessor::preprocess_s_file("dollar.s", asm_contents);
        let sym = rodata_entries
            .iter()
            .find(|s| s.name == "foo$bar$baz")
            .unwrap();
        println!("sym: {sym:?}");
        assert!(sym.local); // Symbols with $ are treated as local in this tool
    }

    #[test]
    #[should_panic]
    fn test_rodata_unknown_directive() {
        let asm_contents = ".section .rodata\ndlabel my_literal\n    .weird 0x1234";
        // Preprocessor should ideally return a Result for errors
        // (Assuming you updated the signature to return Result)
        Preprocessor::preprocess_s_file("unknown.s", asm_contents);
    }
}
