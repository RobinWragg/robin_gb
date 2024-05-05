use super::*;
use crate::cpu::Registers;
use crate::Memory;

fn get_bit_index_from_immediate_byte(immediate_byte: u8) -> u8 {
    (immediate_byte / 8) % 8
}

pub fn execute_cb_instruction(registers: &mut Registers, memory: &mut Memory) -> CpuDiff {
    let immediate_byte = memory.read(registers.pc + 1);

    let bit_index = get_bit_index_from_immediate_byte(immediate_byte);

    // Find the operand using the lower 3 bits of the immediate byte.
    let operand_id = immediate_byte & 0b00000111;
    let mut operand: u8 = registers.read_operand_8bit(operand_id, memory);

    let flag_diff = match immediate_byte {
        0x30..=0x37 => swap(&mut operand),
        0x40..=0x7f => bit(bit_index, operand),
        0x80..=0xbf => res(bit_index, &mut operand),
        0xc0..=0xff => set(bit_index, &mut operand),
        _ => todo!(
            "Unknown 0xcb opcode {:#04x} at address {:#06x}\n",
            immediate_byte,
            registers.pc
        ),
    };

    registers.write_operand_8bit(operand, operand_id, memory);

    let cycle_nybble = immediate_byte & 0x0f;
    let cycles = if cycle_nybble == 0x06 || cycle_nybble == 0x0e {
        16
    } else {
        8
    };
    CpuDiff {
        flag_diff,
        pc_delta: 2,
        cycles,
    }
}

fn swap(byte_to_swap: &mut u8) -> FlagDiff {
    let upper_4_bits = (*byte_to_swap) & 0xf0;
    let lower_4_bits = (*byte_to_swap) & 0x0f;
    *byte_to_swap = upper_4_bits >> 4;
    *byte_to_swap |= lower_4_bits << 4;

    FlagDiff {
        z: Some(*byte_to_swap == 0),
        n: Some(false),
        h: Some(false),
        c: Some(false),
    }
}

fn bit(bit_index: u8, byte_to_check: u8) -> FlagDiff {
    FlagDiff {
        z: Some(byte_to_check & (0x01 << bit_index) == 0),
        n: Some(false),
        h: Some(true),
        c: None,
    }
}

fn res(bit_index: u8, byte_to_reset: &mut u8) -> FlagDiff {
    *byte_to_reset &= !(0x01 << bit_index);
    FlagDiff {
        z: None,
        n: None,
        h: None,
        c: None,
    }
}

fn set(bit_index: u8, byte_to_set: &mut u8) -> FlagDiff {
    *byte_to_set |= 0x01 << bit_index;
    FlagDiff {
        z: None,
        n: None,
        h: None,
        c: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bit_index_from_immediate_byte() {
        assert_eq!(get_bit_index_from_immediate_byte(0x40), 0);
        assert_eq!(get_bit_index_from_immediate_byte(0x47), 0);
        assert_eq!(get_bit_index_from_immediate_byte(0x48), 1);
        assert_eq!(get_bit_index_from_immediate_byte(0x4f), 1);
        assert_eq!(get_bit_index_from_immediate_byte(0x50), 2);
        assert_eq!(get_bit_index_from_immediate_byte(0x57), 2);
        assert_eq!(get_bit_index_from_immediate_byte(0x58), 3);
        assert_eq!(get_bit_index_from_immediate_byte(0x5f), 3);
        assert_eq!(get_bit_index_from_immediate_byte(0x60), 4);
        assert_eq!(get_bit_index_from_immediate_byte(0x67), 4);
        assert_eq!(get_bit_index_from_immediate_byte(0x68), 5);
        assert_eq!(get_bit_index_from_immediate_byte(0x6f), 5);
        assert_eq!(get_bit_index_from_immediate_byte(0x70), 6);
        assert_eq!(get_bit_index_from_immediate_byte(0x77), 6);
        assert_eq!(get_bit_index_from_immediate_byte(0x78), 7);
        assert_eq!(get_bit_index_from_immediate_byte(0x7f), 7);
        assert_eq!(get_bit_index_from_immediate_byte(0x80), 0);
        assert_eq!(get_bit_index_from_immediate_byte(0x87), 0);
        assert_eq!(get_bit_index_from_immediate_byte(0x88), 1);
        assert_eq!(get_bit_index_from_immediate_byte(0x8f), 1);
        assert_eq!(get_bit_index_from_immediate_byte(0x90), 2);
        assert_eq!(get_bit_index_from_immediate_byte(0x97), 2);
        assert_eq!(get_bit_index_from_immediate_byte(0x98), 3);
        assert_eq!(get_bit_index_from_immediate_byte(0x9f), 3);
        assert_eq!(get_bit_index_from_immediate_byte(0xa0), 4);
        assert_eq!(get_bit_index_from_immediate_byte(0xa7), 4);
        assert_eq!(get_bit_index_from_immediate_byte(0xa8), 5);
        assert_eq!(get_bit_index_from_immediate_byte(0xaf), 5);
        assert_eq!(get_bit_index_from_immediate_byte(0xb0), 6);
        assert_eq!(get_bit_index_from_immediate_byte(0xb7), 6);
        assert_eq!(get_bit_index_from_immediate_byte(0xb8), 7);
        assert_eq!(get_bit_index_from_immediate_byte(0xbf), 7);
        assert_eq!(get_bit_index_from_immediate_byte(0xc0), 0);
        assert_eq!(get_bit_index_from_immediate_byte(0xc7), 0);
        assert_eq!(get_bit_index_from_immediate_byte(0xc8), 1);
        assert_eq!(get_bit_index_from_immediate_byte(0xcf), 1);
        assert_eq!(get_bit_index_from_immediate_byte(0xd0), 2);
        assert_eq!(get_bit_index_from_immediate_byte(0xd7), 2);
        assert_eq!(get_bit_index_from_immediate_byte(0xd8), 3);
        assert_eq!(get_bit_index_from_immediate_byte(0xdf), 3);
        assert_eq!(get_bit_index_from_immediate_byte(0xe0), 4);
        assert_eq!(get_bit_index_from_immediate_byte(0xe7), 4);
        assert_eq!(get_bit_index_from_immediate_byte(0xe8), 5);
        assert_eq!(get_bit_index_from_immediate_byte(0xef), 5);
        assert_eq!(get_bit_index_from_immediate_byte(0xf0), 6);
        assert_eq!(get_bit_index_from_immediate_byte(0xf7), 6);
        assert_eq!(get_bit_index_from_immediate_byte(0xf8), 7);
        assert_eq!(get_bit_index_from_immediate_byte(0xff), 7);
    }

    #[test]
    fn test_bit_instruction() {
        // Test n, h, and c flags.
        // These flags should be the same for all calls to bit(...).
        let bit_outer = |bit_index, byte_to_check| {
            let flag_diff = bit(bit_index, byte_to_check);

            assert_eq!(flag_diff.n, Some(false));
            assert_eq!(flag_diff.h, Some(true));
            assert_eq!(flag_diff.c, None);
            flag_diff
        };

        // Test z flag.
        assert_eq!(bit_outer(0, 0b11111111).z, Some(false));
        assert_eq!(bit_outer(0, 0b11111110).z, Some(true));
        assert_eq!(bit_outer(0, 0b00000000).z, Some(true));

        assert_eq!(bit_outer(1, 0b11111111).z, Some(false));
        assert_eq!(bit_outer(1, 0b11111101).z, Some(true));
        assert_eq!(bit_outer(1, 0b00000000).z, Some(true));

        assert_eq!(bit_outer(2, 0b11111111).z, Some(false));
        assert_eq!(bit_outer(2, 0b11111011).z, Some(true));
        assert_eq!(bit_outer(2, 0b00000000).z, Some(true));

        assert_eq!(bit_outer(3, 0b11111111).z, Some(false));
        assert_eq!(bit_outer(3, 0b11110111).z, Some(true));
        assert_eq!(bit_outer(3, 0b00000000).z, Some(true));

        assert_eq!(bit_outer(4, 0b11111111).z, Some(false));
        assert_eq!(bit_outer(4, 0b11101111).z, Some(true));
        assert_eq!(bit_outer(4, 0b00000000).z, Some(true));

        assert_eq!(bit_outer(5, 0b11111111).z, Some(false));
        assert_eq!(bit_outer(5, 0b11011111).z, Some(true));
        assert_eq!(bit_outer(5, 0b00000000).z, Some(true));

        assert_eq!(bit_outer(6, 0b11111111).z, Some(false));
        assert_eq!(bit_outer(6, 0b10111111).z, Some(true));
        assert_eq!(bit_outer(6, 0b00000000).z, Some(true));

        assert_eq!(bit_outer(7, 0b11111111).z, Some(false));
        assert_eq!(bit_outer(7, 0b01111111).z, Some(true));
        assert_eq!(bit_outer(7, 0b00000000).z, Some(true));
    }
}
