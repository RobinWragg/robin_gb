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
        0x00..=0x07 => rlc(&mut operand),
        0x08..=0x0f => rrc(&mut operand),
        0x10..=0x17 => rl(&mut operand, registers.f & Registers::FLAG_CARRY != 0),
        0x18..=0x1f => rr(&mut operand, registers.f & Registers::FLAG_CARRY != 0),
        0x20..=0x27 => sla(&mut operand),
        0x28..=0x2f => sra(&mut operand),
        0x30..=0x37 => swap(&mut operand),
        0x38..=0x3f => srl(&mut operand),
        0x40..=0x7f => bit(operand, bit_index),
        0x80..=0xbf => res(&mut operand, bit_index),
        0xc0..=0xff => set(&mut operand, bit_index),
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

fn rlc(byte_to_rotate: &mut u8) -> FlagDiff {
    let carry = if *byte_to_rotate & make_bit(7) != 0 {
        *byte_to_rotate <<= 1;
        *byte_to_rotate |= make_bit(0);
        true
    } else {
        *byte_to_rotate <<= 1;
        *byte_to_rotate &= !make_bit(0);
        false
    };

    FlagDiff {
        z: Some(*byte_to_rotate == 0),
        n: Some(false),
        h: Some(false),
        c: Some(carry),
    }
}

fn rrc(byte_to_rotate: &mut u8) -> FlagDiff {
    let carry = if *byte_to_rotate & make_bit(0) != 0 {
        *byte_to_rotate >>= 1;
        *byte_to_rotate |= make_bit(7);
        true
    } else {
        *byte_to_rotate >>= 1;
        *byte_to_rotate &= !make_bit(7);
        false
    };

    FlagDiff {
        z: Some(*byte_to_rotate == 0),
        n: Some(false),
        h: Some(false),
        c: Some(carry),
    }
}

fn rl(byte_to_rotate: &mut u8, previous_carry_flag: bool) -> FlagDiff {
    let new_carry_flag = *byte_to_rotate & make_bit(7) != 0;

    *byte_to_rotate <<= 1;
    debug_assert_eq!(*byte_to_rotate & make_bit(0), 0);

    // The carry flag dictates the value of the new bit.
    if previous_carry_flag {
        *byte_to_rotate |= make_bit(0);
    }

    FlagDiff {
        z: Some(*byte_to_rotate == 0),
        n: Some(false),
        h: Some(false),
        c: Some(new_carry_flag),
    }
}

fn rr(byte_to_rotate: &mut u8, previous_carry_flag: bool) -> FlagDiff {
    let new_carry_flag = *byte_to_rotate & make_bit(0) != 0;

    *byte_to_rotate >>= 1;
    debug_assert_eq!(*byte_to_rotate & make_bit(7), 0);

    // The carry flag dictates the value of the new bit.
    if previous_carry_flag {
        *byte_to_rotate |= make_bit(7);
    }

    FlagDiff {
        z: Some(*byte_to_rotate == 0),
        n: Some(false),
        h: Some(false),
        c: Some(new_carry_flag),
    }
}

fn sla(byte_to_shift: &mut u8) -> FlagDiff {
    let carry = *byte_to_shift & make_bit(7) != 0;

    *byte_to_shift <<= 1;
    *byte_to_shift &= !make_bit(0); // bit 0 should become 0.

    FlagDiff {
        z: Some(*byte_to_shift == 0),
        n: Some(false),
        h: Some(false),
        c: Some(carry),
    }
}

fn sra(byte_to_shift: &mut u8) -> FlagDiff {
    let carry = *byte_to_shift & make_bit(0) != 0;

    *byte_to_shift >>= 1;
    *byte_to_shift |= (*byte_to_shift & make_bit(6)) << 1; // Bit 7 should stay the same.

    FlagDiff {
        z: Some(*byte_to_shift == 0),
        n: Some(false),
        h: Some(false),
        c: Some(carry),
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

fn srl(byte_to_shift: &mut u8) -> FlagDiff {
    let carry_flag = (*byte_to_shift) & make_bit(0) != 0;

    *byte_to_shift >>= 1; // Bit 7 becomes 0.
    debug_assert!(((*byte_to_shift) & make_bit(7)) == 0); // rwtodo can move this to tests.

    FlagDiff {
        z: Some((*byte_to_shift) == 0),
        n: Some(false),
        h: Some(false),
        c: Some(carry_flag),
    }
}

fn bit(byte_to_check: u8, bit_index: u8) -> FlagDiff {
    FlagDiff {
        z: Some(byte_to_check & (0x01 << bit_index) == 0),
        n: Some(false),
        h: Some(true),
        c: None,
    }
}

fn res(byte_to_reset: &mut u8, bit_index: u8) -> FlagDiff {
    *byte_to_reset &= !(0x01 << bit_index);
    FlagDiff {
        z: None,
        n: None,
        h: None,
        c: None,
    }
}

fn set(byte_to_set: &mut u8, bit_index: u8) -> FlagDiff {
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
    fn test_rl_instruction() {
        let mut byte_to_rotate = 0;
        let flag_diff = rl(&mut byte_to_rotate, false);
        assert_eq!(byte_to_rotate, 0);
        assert_eq!(flag_diff.z, Some(true));
        assert_eq!(flag_diff.n, Some(false));
        assert_eq!(flag_diff.h, Some(false));
        assert_eq!(flag_diff.c, Some(false));

        let mut byte_to_rotate = 0b01000001;
        let flag_diff = rl(&mut byte_to_rotate, true);
        assert_eq!(byte_to_rotate, 0b10000011);
        assert_eq!(flag_diff.z, Some(false));
        assert_eq!(flag_diff.n, Some(false));
        assert_eq!(flag_diff.h, Some(false));
        assert_eq!(flag_diff.c, Some(false));

        let mut byte_to_rotate = 0b10000001;
        let flag_diff = rl(&mut byte_to_rotate, false);
        assert_eq!(byte_to_rotate, 0b00000010);
        assert_eq!(flag_diff.z, Some(false));
        assert_eq!(flag_diff.n, Some(false));
        assert_eq!(flag_diff.h, Some(false));
        assert_eq!(flag_diff.c, Some(true));

        let mut byte_to_rotate = 0b10000001;
        let flag_diff = rl(&mut byte_to_rotate, true);
        assert_eq!(byte_to_rotate, 0b00000011);
        assert_eq!(flag_diff.z, Some(false));
        assert_eq!(flag_diff.n, Some(false));
        assert_eq!(flag_diff.h, Some(false));
        assert_eq!(flag_diff.c, Some(true));
    }

    #[test]
    fn test_srl_instruction() {
        for immediate_byte in 0..=0xff {
            let mut mutated_immediate_byte = immediate_byte;
            let flag_diff = srl(&mut mutated_immediate_byte);
            assert_eq!(mutated_immediate_byte, immediate_byte >> 1);
            assert_eq!(flag_diff.z, Some(immediate_byte & 0b11111110 == 0));
            assert_eq!(flag_diff.z, Some(mutated_immediate_byte == 0));
            assert_eq!(flag_diff.h, Some(false));
            assert_eq!(flag_diff.n, Some(false));
            assert_eq!(flag_diff.c, Some(immediate_byte & 0b00000001 != 0));
        }
    }

    #[test]
    fn test_bit_instruction() {
        // Test n, h, and c flags.
        // These flags should be the same for all calls to bit(...).
        let bit_outer = |bit_index, byte_to_check| {
            let flag_diff = bit(byte_to_check, bit_index);

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
