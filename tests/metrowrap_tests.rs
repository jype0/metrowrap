use std::path::PathBuf;
use std::sync::Arc;

use encoding_rs::UTF_8;

use metrowrap;
use metrowrap::NamedString;
use metrowrap::SourceType;
use metrowrap::assembler;
use metrowrap::compiler;
use metrowrap::preprocessor;

use object::{self, Object, ObjectSection, SectionKind};

#[test]
fn test_process_c_file() {
    let preprocessor = Arc::new(preprocessor::Preprocessor {
        asm_dir_prefix: Some(PathBuf::from(".")),
    });

    let c_flags: Vec<String> = vec![
        "-Itests/data".to_string(),
        "-c".to_string(),
        "-lang".to_string(),
        "c".to_string(),
        "-sdatathreshold".to_string(),
        "0".to_string(),
        "-char".to_string(),
        "unsigned".to_string(),
        "-fl".to_string(),
        "divbyzerocheck".to_string(),
        "-opt".to_string(),
        "nointrinsics".to_string(),
    ];
    let compiler = compiler::Compiler::new(
        c_flags,
        "target/.private/bin/mwccpsp.exe".into(),
        true,
        "target/.private/bin/wibo".into(),
    );

    let assembler = assembler::Assembler {
        as_path: "mipsel-linux-gnu-as".into(),
        as_march: "allegrex".into(),
        as_mabi: "32".into(),
        as_flags: vec!["-G0".into()],
        macro_inc_path: Some("tests/data/macro.inc".into()),
    };

    let c_path = PathBuf::from("tests/data/assembler.c");
    let c_content = NamedString {
        source: SourceType::Path(c_path.display().to_string()),
        content: std::fs::read_to_string(&c_path).unwrap(),
        encoding: UTF_8,
        src_dir: PathBuf::from("tests/data"),
    };

    let result = metrowrap::process_c_file(
        &c_content,
        &PathBuf::from("target/.private/tests/metrowrap/process_c_file/assembler.o"),
        &preprocessor,
        &compiler,
        &assembler,
    );

    assert!(matches!(result, Ok(())), "this is not ok: {result:?}");

    let _obj_bytes = std::fs::read("target/.private/tests/metrowrap/process_c_file/assembler.o");
}

#[test]
fn test_process_c_file_no_include_asm() {
    let preprocessor = Arc::new(preprocessor::Preprocessor {
        asm_dir_prefix: Some(PathBuf::from(".")),
    });

    let c_flags: Vec<String> = vec!["-Itests/data".to_string(), "-c".to_string()];
    let compiler = compiler::Compiler::new(
        c_flags,
        "target/.private/bin/mwccpsp.exe".into(),
        true,
        "target/.private/bin/wibo".into(),
    );

    let assembler = assembler::Assembler {
        as_path: "mipsel-linux-gnu-as".into(),
        as_march: "allegrex".into(),
        as_mabi: "32".into(),
        as_flags: vec!["-G0".into()],
        macro_inc_path: Some("tests/data/macro.inc".into()),
    };

    let c_path = PathBuf::from("tests/data/compiler.c");
    let c_content = NamedString {
        source: SourceType::Path(c_path.display().to_string()),
        content: std::fs::read_to_string(&c_path).unwrap(),
        encoding: UTF_8,
        src_dir: PathBuf::from("tests/data"),
    };

    let _result = metrowrap::process_c_file(
        &c_content,
        &PathBuf::from("target/.private/tests/metrowrap/process_c_file/compiler.o"),
        &preprocessor,
        &compiler,
        &assembler,
    )
    .expect("process_c_file");

    let obj_bytes = std::fs::read("target/.private/tests/metrowrap/process_c_file/compiler.o")
        .expect("compiler.o");
    let obj = object::File::parse(&*obj_bytes).expect("no object");
    let mut sections = obj.sections();

    //  [Nr] Name              Type            Addr     Off    Size   ES Flg Lk Inf Al
    //  [ 0]                   NULL            00000000 000000 000000 00      0   0  0
    let Some(null_section) = sections.next() else {
        panic!("no NULL")
    };
    assert!(
        matches!(null_section.kind(), SectionKind::Metadata),
        "NULL Section Kind: {null_section:?}"
    );
    assert_eq!(0, null_section.size());
    assert_eq!(0, null_section.file_range().unwrap().0);

    //  [ 1] .symtab           SYMTAB          00000000 000040 000030 10      2   2  0
    let Some(symtab_section) = sections.next() else {
        panic!("no symtab")
    };
    assert!(
        matches!(symtab_section.kind(), SectionKind::Metadata),
        "SYMTAB Section Kind: {symtab_section:?}"
    );
    assert_eq!(48, symtab_section.size());
    assert_eq!(0x40, symtab_section.file_range().unwrap().0);
    assert_eq!(
        vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 3,
            0, 6, 0, 1, 0, 0, 0, 0, 0, 0, 0, 36, 0, 0, 0, 18, 0, 5, 0
        ],
        symtab_section.data().unwrap().to_vec()
    );

    //  [ 2] .strtab           STRTAB          00000000 000070 000011 00      0   0  0
    let Some(strtab_section) = sections.next() else {
        panic!("no strtab")
    };
    assert!(
        matches!(strtab_section.kind(), SectionKind::Metadata),
        "STRTAB Section Kind: {strtab_section:?}"
    );
    assert_eq!(17, strtab_section.size());
    assert_eq!(0x70, strtab_section.file_range().unwrap().0);
    assert_eq!(
        "\0Add\0.mwcats_Add\0",
        String::from_utf8(strtab_section.data().unwrap().to_vec()).unwrap()
    );

    //  [ 3] .shstrtab         STRTAB          00000000 000090 00003e 00      0   0  0
    let Some(shstrtab_section) = sections.next() else {
        panic!("no shstrtab")
    };
    assert!(
        matches!(shstrtab_section.kind(), SectionKind::Metadata),
        "SHSTRTAB Section Kind: {shstrtab_section:?}"
    );
    assert_eq!(62, shstrtab_section.size());
    assert_eq!(0x90, shstrtab_section.file_range().unwrap().0);
    assert_eq!(
        "\0.symtab\0.strtab\0.shstrtab\0.comment\0.text\0.mwcats\0.rel.mwcats\0",
        String::from_utf8(shstrtab_section.data().unwrap().to_vec()).unwrap()
    );

    //  [ 4] .comment          PROGBITS        00000000 0000e0 00001b 00      0   0  0
    //  [ 5] .text             PROGBITS        00000000 0000f0 000024 00  AX  0   0  4
    //  [ 6] .mwcats           LOUSER+0x4a2a82 00000000 000120 000008 00      5   0  4
    //  [ 7] .rel.mwcats       REL             00000000 000130 000008 08      1   6  0
}

#[test]
fn test_rewritten_c_file() {
    let preprocessor = preprocessor::Preprocessor {
        asm_dir_prefix: Some(PathBuf::from(".")),
    };

    let c_flags: Vec<String> = vec!["-Itests/data".to_string(), "-c".to_string()];
    let compiler = compiler::Compiler::new(
        c_flags,
        "target/.private/bin/mwccpsp.exe".into(),
        true,
        "target/.private/bin/wibo".into(),
    );

    // 2. Preprocess to find INCLUDE_ASM macros
    let content = std::fs::read_to_string("tests/data/assembler.c").expect("");
    let (new_lines, _asm_files) = preprocessor.find_macros(&content);

    let temp_c = tempfile::NamedTempFile::with_suffix(".c").expect("temp_string");
    std::fs::write(temp_c.path(), new_lines).expect("temp_c file");

    // Re-compile with stubs
    let (recompiled_bytes, _) = compiler
        .compile_file(temp_c.path(), "tests/data/assembler.c")
        .expect("recompile");
    std::fs::write("/tmp/recompiled.o", &recompiled_bytes).expect("debug file");

    let Ok(obj) = object::File::parse(&*recompiled_bytes) else {
        panic!("no object")
    };
    let mut sections = obj.sections();

    //  [Nr] Name              Type            Addr     Off    Size   ES Flg Lk Inf Al
    //  [ 0]                   NULL            00000000 000000 000000 00      0   0  0
    let Some(null_section) = sections.next() else {
        panic!("no NULL")
    };
    assert!(
        matches!(null_section.kind(), SectionKind::Metadata),
        "NULL Section Kind: {null_section:?}"
    );
    assert_eq!(0, null_section.size());
    assert_eq!(0, null_section.file_range().unwrap().0);

    //  [ 1] .symtab           SYMTAB          00000000 000040 000030 10      2   2  0
    let Some(symtab_section) = sections.next() else {
        panic!("no symtab")
    };
    assert!(
        matches!(symtab_section.kind(), SectionKind::Metadata),
        "SYMTAB Section Kind: {symtab_section:?}"
    );
    assert_eq!(48, symtab_section.size());
    assert_eq!(0x40, symtab_section.file_range().unwrap().0);
    assert_eq!(
        vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 3,
            0, 6, 0, 1, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 18, 0, 5, 0
        ],
        symtab_section.data().unwrap().to_vec()
    );

    //  [ 2] .strtab           STRTAB          00000000 000070 000021 00      0   0  0
    let Some(strtab_section) = sections.next() else {
        panic!("no strtab")
    };
    assert!(
        matches!(strtab_section.kind(), SectionKind::Metadata),
        "STRTAB Section Kind: {strtab_section:?}"
    );
    assert_eq!(33, strtab_section.size());
    assert_eq!(0x70, strtab_section.file_range().unwrap().0);
    assert_eq!(
        "\0___mw___Add\0.mwcats____mw___Add\0",
        String::from_utf8(strtab_section.data().unwrap().to_vec()).unwrap()
    );

    //  [ 3] .shstrtab         STRTAB          00000000 0000a0 00003e 00      0   0  0
    let Some(shstrtab_section) = sections.next() else {
        panic!("no shstrtab")
    };
    assert!(
        matches!(shstrtab_section.kind(), SectionKind::Metadata),
        "SHSTRTAB Section Kind: {shstrtab_section:?}"
    );
    assert_eq!(62, shstrtab_section.size());
    assert_eq!(0xa0, shstrtab_section.file_range().unwrap().0);
    assert_eq!(
        "\0.symtab\0.strtab\0.shstrtab\0.comment\0.text\0.mwcats\0.rel.mwcats\0",
        String::from_utf8(shstrtab_section.data().unwrap().to_vec()).unwrap()
    );

    //  [ 4] .comment          PROGBITS        00000000 0000e0 00001b 00      0   0  0
    let Some(comment_section) = sections.next() else {
        panic!("no comment")
    };
    assert!(
        matches!(comment_section.kind(), SectionKind::Other),
        "PROGBITS Section Kind: {comment_section:?}"
    );
    assert_eq!(27, comment_section.size());
    assert_eq!(0xe0, comment_section.file_range().unwrap().0);
    assert_eq!(
        "MW MIPS C Compiler (3.0.0)\0",
        String::from_utf8(comment_section.data().unwrap().to_vec()).unwrap()
    );

    //  [ 5] .text             PROGBITS        00000000 000100 000010 00  AX  0   0  4
    let text_section = sections.next().expect("PROGBITS section");
    assert!(
        matches!(text_section.kind(), SectionKind::Text),
        "Text Section Kind: {text_section:?}"
    );
    assert_eq!(16, text_section.size());
    assert_eq!(0x100, text_section.file_range().unwrap().0);

    //  [ 6] .mwcats           LOUSER+0x4a2a82 00000000 000110 000008 00      5   0  4
    let _mwcats_section = sections.next().expect("LOUSER section");

    //  [ 7] .rel.mwcats       REL             00000000 000120 000008 08      1   6  0
    let _rel_mwcats_section = sections.next().expect("REL section");

    let no_section = sections.next();
    assert!(
        matches!(no_section, None),
        "expected none, got: {no_section:?}"
    );
}
