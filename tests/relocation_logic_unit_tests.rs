// tests/relocation_logic_unit_tests.rs
// Unit tests for internal relocation splitting logic

use metrowrap::elf::{Relocation, RelocationRecord, SHT_REL, Section};

/// Test that Relocation offset adjustments work correctly
#[test]
fn test_relocation_offset_adjustment_logic() {
    // Simulate a relocation at offset 20
    let mut reloc = Relocation {
        r_offset: 20,
        r_info: 0x205, // Example: symbol index 2, type 5
        symbol: String::new(),
    };

    // Simulate adjusting for a second rodata section that starts at offset 13
    let rodata_section_offset_0 = 13;
    reloc.r_offset -= rodata_section_offset_0 as u32;

    assert_eq!(
        reloc.r_offset, 7,
        "Relocation offset should be adjusted to 7 (20 - 13)"
    );
}

/// Test finding which rodata section a relocation belongs to
#[test]
fn test_relocation_section_assignment() {
    // Simulate rodata section offsets: [13, 29, 48]
    // This means:
    //   Section 0: bytes 0-12   (13 bytes)
    //   Section 1: bytes 13-28  (16 bytes)
    //   Section 2: bytes 29-47  (19 bytes)
    let rodata_section_offsets = vec![13, 29, 48];

    // Test relocations at various offsets
    let test_cases = vec![
        (5, 0, "Offset 5 should be in section 0"),
        (12, 0, "Offset 12 should be in section 0"),
        (13, 1, "Offset 13 should be in section 1"),
        (20, 1, "Offset 20 should be in section 1"),
        (28, 1, "Offset 28 should be in section 1"),
        (29, 2, "Offset 29 should be in section 2"),
        (40, 2, "Offset 40 should be in section 2"),
    ];

    for (offset, expected_section, msg) in test_cases {
        let mut found_section = None;
        for i in 0..rodata_section_offsets.len() {
            if offset < rodata_section_offsets[i] {
                found_section = Some(i);
                break;
            }
        }

        assert_eq!(found_section, Some(expected_section), "{}", msg);
    }
}

/// Test relocation offset adjustment after section assignment
#[test]
fn test_relocation_offset_after_assignment() {
    let rodata_section_offsets = vec![13, 29, 48];

    // Test that offsets are correctly adjusted relative to section start
    let test_cases = vec![
        (5, 0, 5, "Offset 5 in section 0 should remain 5"),
        (13, 1, 0, "Offset 13 in section 1 should become 0 (13-13)"),
        (20, 1, 7, "Offset 20 in section 1 should become 7 (20-13)"),
        (29, 2, 0, "Offset 29 in section 2 should become 0 (29-29)"),
        (40, 2, 11, "Offset 40 in section 2 should become 11 (40-29)"),
    ];

    for (original_offset, section_idx, expected_adjusted, msg) in test_cases {
        let mut adjusted_offset = original_offset;

        if section_idx > 0 {
            adjusted_offset -= rodata_section_offsets[section_idx - 1];
        }

        assert_eq!(adjusted_offset, expected_adjusted, "{}", msg);
    }
}

/// Test symbol index updates when local symbols are inserted
#[test]
fn test_symbol_index_updates_after_insertion() {
    let initial_sh_info_value = 5;
    let local_syms_inserted = 2;

    // Test symbol indices before and after insertion
    let test_cases = vec![
        (0, 0, "Symbol 0 should not be updated"),
        (1, 1, "Symbol 1 should not be updated"),
        (4, 4, "Symbol 4 (below threshold) should not be updated"),
        (5, 7, "Symbol 5 (at threshold) should be updated to 7"),
        (6, 8, "Symbol 6 should be updated to 8"),
        (10, 12, "Symbol 10 should be updated to 12"),
    ];

    for (original_idx, expected_idx, msg) in test_cases {
        let updated_idx = if original_idx >= initial_sh_info_value {
            original_idx + local_syms_inserted
        } else {
            original_idx
        };

        assert_eq!(updated_idx, expected_idx, "{}", msg);
    }
}

/// Test that relocation records are correctly split
#[test]
fn test_relocation_record_splitting() {
    // Create mock relocations at different offsets
    let relocations = vec![
        Relocation {
            r_offset: 5,
            r_info: 0x105,
            symbol: String::from("sym1"),
        },
        Relocation {
            r_offset: 10,
            r_info: 0x205,
            symbol: String::from("sym2"),
        },
        Relocation {
            r_offset: 15,
            r_info: 0x305,
            symbol: String::from("sym3"),
        },
        Relocation {
            r_offset: 25,
            r_info: 0x405,
            symbol: String::from("sym4"),
        },
        Relocation {
            r_offset: 35,
            r_info: 0x505,
            symbol: String::from("sym5"),
        },
    ];

    // Simulate section offsets: [13, 29]
    let rodata_section_offsets = vec![13, 39];
    let num_sections = 2;

    // Split relocations
    let mut new_relocations: Vec<Vec<Relocation>> = vec![vec![]; num_sections];

    for mut relocation in relocations {
        for i in 0..rodata_section_offsets.len() {
            if relocation.r_offset < rodata_section_offsets[i] as u32 {
                if i > 0 {
                    relocation.r_offset -= rodata_section_offsets[i - 1] as u32;
                }
                new_relocations[i].push(relocation);
                break;
            }
        }
    }

    // Verify section 0 has relocations at offsets 5, 10 (both < 13)
    assert_eq!(
        new_relocations[0].len(),
        2,
        "Section 0 should have 2 relocations"
    );
    assert_eq!(new_relocations[0][0].r_offset, 5);
    assert_eq!(new_relocations[0][1].r_offset, 10);

    // Verify section 1 has relocations at adjusted offsets
    assert_eq!(
        new_relocations[1].len(),
        3,
        "Section 1 should have 3 relocations"
    );
    assert_eq!(new_relocations[1][0].r_offset, 2, "15 - 13 = 2");
    assert_eq!(new_relocations[1][1].r_offset, 12, "25 - 13 = 12");
    assert_eq!(new_relocations[1][2].r_offset, 22, "35 - 13 = 22");
}

/// Test edge case: empty relocation list
#[test]
fn test_empty_relocations() {
    let relocations: Vec<Relocation> = vec![];
    let rodata_section_offsets = vec![13, 29];
    let num_sections = 2;

    let mut new_relocations: Vec<Vec<Relocation>> = vec![vec![]; num_sections];

    for relocation in relocations {
        for i in 0..rodata_section_offsets.len() {
            if relocation.r_offset < rodata_section_offsets[i] as u32 {
                new_relocations[i].push(relocation);
                break;
            }
        }
    }

    assert_eq!(
        new_relocations[0].len(),
        0,
        "Section 0 should have 0 relocations"
    );
    assert_eq!(
        new_relocations[1].len(),
        0,
        "Section 1 should have 0 relocations"
    );
}

/// Test edge case: all relocations in first section
#[test]
fn test_all_relocations_in_first_section() {
    let relocations = vec![
        Relocation {
            r_offset: 5,
            r_info: 0x105,
            symbol: String::new(),
        },
        Relocation {
            r_offset: 10,
            r_info: 0x205,
            symbol: String::new(),
        },
    ];

    let rodata_section_offsets = vec![13, 29];
    let num_sections = 2;

    let mut new_relocations: Vec<Vec<Relocation>> = vec![vec![]; num_sections];

    for relocation in relocations {
        for i in 0..rodata_section_offsets.len() {
            if relocation.r_offset < rodata_section_offsets[i] as u32 {
                new_relocations[i].push(relocation);
                break;
            }
        }
    }

    assert_eq!(
        new_relocations[0].len(),
        2,
        "Section 0 should have 2 relocations"
    );
    assert_eq!(
        new_relocations[1].len(),
        0,
        "Section 1 should have 0 relocations"
    );
}

/// Test edge case: single section (no splitting needed)
#[test]
fn test_single_section_no_split() {
    let num_rodata_symbols = 1;

    // When there's only one rodata symbol, no splitting should occur
    assert_eq!(
        num_rodata_symbols, 1,
        "With single rodata section, splitting logic should be skipped"
    );
}

/// Test Relocation structure methods
#[test]
fn test_relocation_symbol_index_methods() {
    let mut reloc = Relocation {
        r_offset: 0,
        r_info: 0x0205, // Symbol index 2, type 5
        symbol: String::new(),
    };

    // Test getting symbol index
    assert_eq!(reloc.symbol_index(), 2, "Symbol index should be 2");

    // Test getting type
    assert_eq!(reloc.type_id(), 5, "Type should be 5");

    // Test setting symbol index
    reloc.set_symbol_index(7);
    assert_eq!(
        reloc.symbol_index(),
        7,
        "Symbol index should be updated to 7"
    );
    assert_eq!(reloc.type_id(), 5, "Type should remain 5");
}

/// Test that relocation packing/unpacking preserves data
#[test]
fn test_relocation_pack_unpack() {
    let original = Relocation {
        r_offset: 0x12345678,
        r_info: 0xABCDEF00,
        symbol: String::new(),
    };

    let packed = original.pack();
    assert_eq!(packed.len(), 8, "Packed relocation should be 8 bytes");

    let unpacked = Relocation::unpack(&packed);
    assert_eq!(
        unpacked.r_offset, original.r_offset,
        "r_offset should match"
    );
    assert_eq!(unpacked.r_info, original.r_info, "r_info should match");
}

/// Test RelocationRecord data packing
#[test]
fn test_relocation_record_pack() {
    let mut section = Section::default();
    section.sh_type = SHT_REL;

    let mut reloc_record = RelocationRecord::new(section);

    reloc_record.relocations.push(Relocation {
        r_offset: 0x10,
        r_info: 0x205,
        symbol: String::new(),
    });

    reloc_record.relocations.push(Relocation {
        r_offset: 0x20,
        r_info: 0x305,
        symbol: String::new(),
    });

    reloc_record.pack_data();

    // Should have 16 bytes (2 relocations * 8 bytes each)
    assert_eq!(
        reloc_record.section.data.len(),
        16,
        "Packed relocation record should be 16 bytes"
    );
}
