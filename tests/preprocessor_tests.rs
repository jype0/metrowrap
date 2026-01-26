use std::path::PathBuf;

use metrowrap::preprocessor;

#[test]
fn test_no_include_asm() {
    let preprocessor = preprocessor::Preprocessor {
        asm_dir_prefix: None,
    };

    let content = std::fs::read_to_string("tests/data/compiler.c").unwrap();
    let (new_lines, asm_files) = preprocessor.find_macros(&content);

    assert_eq!(content, new_lines);
    assert!(
        asm_files.is_empty(),
        "Expected no INCLUDE_ASM: {asm_files:?}"
    );
}

#[test]
fn test_include_one_asm() {
    let preprocessor = preprocessor::Preprocessor {
        asm_dir_prefix: None,
    };

    let content = std::fs::read_to_string("tests/data/assembler.c").unwrap();
    let (new_lines, asm_files) = preprocessor.find_macros(&content);

    assert_eq!(1, asm_files.len());
    assert_eq!(
        &(PathBuf::from("tests/data/Add.s"), 0),
        asm_files.first().unwrap()
    );

    // ensure whitespace is ignored where expected
    let content = r#"#include "common.h"

INCLUDE_ASM(
    "tests/data"   ,
    Add
    );
"#;
    let (new_lines_2, asm_files_2) = preprocessor.find_macros(&content);
    assert_eq!(new_lines, new_lines_2);
    assert_eq!(asm_files, asm_files_2);
}
