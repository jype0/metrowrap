// tests/add_symbol_get_index_tests.rs
// Tests for the Elf::add_symbol_get_index method

use metrowrap::elf::{Elf, Symbol};
use metrowrap::elf::{STB_GLOBAL, STB_LOCAL, STT_FUNC, STT_NOTYPE};

/*
/// Helper to create a minimal ELF for testing
fn create_test_elf() -> Elf {
    // Create a minimal valid ELF with empty sections
    let elf_bytes = create_minimal_elf_bytes();
    Elf::from_bytes(&elf_bytes)
}

/// Creates minimal ELF bytes for testing
fn create_minimal_elf_bytes() -> Vec<u8> {
    // This is a simplified version - in practice, you'd want a real minimal ELF
    // For now, we'll assume this is implemented elsewhere or use an existing test file
    vec![
        0x7f, 0x45, 0x4c, 0x46, // ELF magic
        0x01, 0x01, 0x01, 0x00, // 32-bit, little-endian, current version
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
        0x01, 0x00, // e_type: relocatable
        0x08,
        0x00, // e_machine: MIPS
              // ... rest would be filled in with proper ELF structure
    ]
}
*/

/// Test 1: Adding a new symbol
#[test]
fn test_add_symbol_get_index_new_symbol() {
    // We need a real ELF file for this test
    // Load a minimal test object file
    let test_obj = std::fs::read("tests/data/assembler.c.phase-1.o");

    let mut elf = Elf::from_bytes(&test_obj.unwrap());
    let initial_symbol_count = elf.get_symbols().len();

    let new_symbol = Symbol {
        st_name: 0,
        st_value: 0,
        st_size: 0,
        st_info: (STB_GLOBAL << 4) | STT_FUNC,
        st_other: 0,
        st_shndx: 1,
        name: "new_test_symbol".to_string(),
    };

    let index = elf.add_symbol_get_index(new_symbol, false);

    assert!(
        index >= initial_symbol_count,
        "New symbol should be added at or after the initial count"
    );

    let symbols = elf.get_symbols();
    assert_eq!(
        symbols[index].name, "new_test_symbol",
        "Symbol at returned index should have the correct name"
    );
}

/// Test 2: Adding duplicate symbol without force
#[test]
fn test_add_symbol_get_index_duplicate_no_force() {
    let test_obj = std::fs::read("tests/data/assembler.c.phase-1.o");

    let mut elf = Elf::from_bytes(&test_obj.unwrap());

    let new_symbol = Symbol {
        st_name: 0,
        st_value: 100,
        st_size: 50,
        st_info: (STB_GLOBAL << 4) | STT_FUNC,
        st_other: 0,
        st_shndx: 1,
        name: "duplicate_test".to_string(),
    };

    // Add the symbol first time
    let index1 = elf.add_symbol_get_index(new_symbol.clone(), false);

    // Try to add it again without force
    let index2 = elf.add_symbol_get_index(new_symbol.clone(), false);

    assert_eq!(
        index1, index2,
        "Adding duplicate symbol without force should return same index"
    );
}

/// Test 3: Adding duplicate symbol with force
#[test]
fn test_add_symbol_get_index_duplicate_with_force() {
    let test_obj = std::fs::read("tests/data/assembler.c.phase-1.o");
    let mut elf = Elf::from_bytes(&test_obj.unwrap());

    let new_symbol = Symbol {
        st_name: 0,
        st_value: 100,
        st_size: 50,
        st_info: (STB_GLOBAL << 4) | STT_FUNC,
        st_other: 0,
        st_shndx: 1,
        name: "forced_duplicate".to_string(),
    };

    // Add the symbol first time
    let index1 = elf.add_symbol_get_index(new_symbol.clone(), false);
    let count_after_first = elf.get_symbols().len();

    // Force add it again
    let index2 = elf.add_symbol_get_index(new_symbol.clone(), true);
    let count_after_second = elf.get_symbols().len();

    assert_ne!(
        index1, index2,
        "Adding duplicate symbol with force should create a new entry"
    );
    assert_eq!(
        count_after_second,
        count_after_first + 1,
        "Force adding should increase symbol count"
    );
}

/// Test 4: Local vs Global symbols
#[test]
fn test_add_symbol_get_index_local_vs_global() {
    let test_obj = std::fs::read("tests/data/assembler.c.phase-1.o");

    let mut elf = Elf::from_bytes(&test_obj.unwrap());

    let local_symbol = Symbol {
        st_name: 0,
        st_value: 0,
        st_size: 0,
        st_info: (STB_LOCAL << 4) | STT_FUNC,
        st_other: 0,
        st_shndx: 1,
        name: "local_sym".to_string(),
    };

    let global_symbol = Symbol {
        st_name: 0,
        st_value: 0,
        st_size: 0,
        st_info: (STB_GLOBAL << 4) | STT_FUNC,
        st_other: 0,
        st_shndx: 1,
        name: "global_sym".to_string(),
    };

    let local_index = elf.add_symbol_get_index(local_symbol, false);
    let global_index = elf.add_symbol_get_index(global_symbol, false);

    assert!(
        local_index < global_index,
        "Local symbols should come before global symbols"
    );
}

/// Test 5: Empty name symbol
#[test]
fn test_add_symbol_get_index_empty_name() {
    let test_obj = std::fs::read("tests/data/assembler.c.phase-1.o");

    let mut elf = Elf::from_bytes(&test_obj.unwrap());

    let empty_name_symbol = Symbol {
        st_name: 0,
        st_value: 0,
        st_size: 0,
        st_info: (STB_LOCAL << 4) | STT_NOTYPE,
        st_other: 0,
        st_shndx: 0,
        name: "".to_string(),
    };

    // Should not panic with empty name
    let index = elf.add_symbol_get_index(empty_name_symbol, false);

    assert_eq!(0, index, "Should return a valid index even for empty name");
}

/// Test 6: Verify force parameter behavior
#[test]
fn test_add_symbol_get_index_force_parameter() {
    let test_obj = std::fs::read("tests/data/assembler.c.phase-1.o");

    let mut elf = Elf::from_bytes(&test_obj.unwrap());

    let symbol = Symbol {
        st_name: 0,
        st_value: 123,
        st_size: 456,
        st_info: (STB_GLOBAL << 4) | STT_FUNC,
        st_other: 0,
        st_shndx: 1,
        name: "force_test_symbol".to_string(),
    };

    // Add with force=false
    let index1 = elf.add_symbol_get_index(symbol.clone(), false);
    let symbols_after_first = elf.get_symbols().clone();

    // Modify the symbol
    let mut modified_symbol = symbol.clone();
    modified_symbol.st_value = 999;

    // Add again with force=false (should return existing)
    let index2 = elf.add_symbol_get_index(modified_symbol.clone(), false);
    assert_eq!(
        index1, index2,
        "Without force, should return existing symbol"
    );

    // Add again with force=true (should create new entry)
    let index3 = elf.add_symbol_get_index(modified_symbol.clone(), true);
    assert_ne!(index1, index3, "With force, should create new symbol");

    let symbols_after_force = elf.get_symbols();
    assert_eq!(
        symbols_after_force.len(),
        symbols_after_first.len() + 1,
        "Force should add a new symbol"
    );
}

/// Test 7: Return value is usable as relocation symbol index
#[test]
fn test_add_symbol_get_index_usable_for_relocation() {
    let test_obj = std::fs::read("tests/data/assembler.c.phase-1.o");

    let mut elf = Elf::from_bytes(&test_obj.unwrap());

    let symbol = Symbol {
        st_name: 0,
        st_value: 0,
        st_size: 16,
        st_info: (STB_GLOBAL << 4) | STT_FUNC,
        st_other: 0,
        st_shndx: 1,
        name: "reloc_test_func".to_string(),
    };

    let index = elf.add_symbol_get_index(symbol, false);

    // Verify the index is within bounds
    let symbols = elf.get_symbols();
    assert!(
        index < symbols.len(),
        "Returned index {} should be within symbol table size {}",
        index,
        symbols.len()
    );

    // Verify we can access the symbol at that index
    let retrieved_symbol = &symbols[index];
    assert_eq!(
        retrieved_symbol.name, "reloc_test_func",
        "Symbol at returned index should be accessible"
    );
}
