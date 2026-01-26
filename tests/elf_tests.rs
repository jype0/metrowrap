// SPDX-FileCopyrightText: © 2026 TTKB, LLC
// SPDX-License-Identifier: BSD-3-CLAUSE
use metrowrap::elf;
use metrowrap::elf::STB_GLOBAL;
use metrowrap::elf::STB_LOCAL;
use metrowrap::elf::STT_FUNC;
use metrowrap::elf::STT_NOTYPE;
use metrowrap::elf::STT_SECTION;
use object::{self, File, Object, ObjectSection, SectionKind};

#[test]
fn test_symbols() {
    let obj_bytes =
        std::fs::read("tests/data/assembler.c.phase-3.o").expect("assembler.c.phase-3.o");
    let obj = File::parse(&*obj_bytes).expect("no object");

    let symtab = obj
        .sections()
        .find(|s| matches!(s.kind(), SectionKind::Metadata) && matches!(s.name(), Ok(".symtab")))
        .expect("symtab");
    let symtab_data = symtab.data().expect("symtab data");

    let symbols = elf::Symbol::unpack_all(&symtab_data);

    assert_eq!(3, symbols.len());

    let sym0 = symbols.get(0).expect("symbols[0]");
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

    let sym1 = symbols.get(1).expect("symbols[1]");
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

    let sym2 = symbols.get(2).expect("symbols[2]");
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
}
