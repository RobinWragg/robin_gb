use super::*;
use crate::cpu::Registers;
use crate::Memory;

pub fn execute_cb_instruction(registers: &mut Registers, memory: &mut Memory) -> CpuDiff {
    let immediate_byte = memory.read(registers.pc + 1);

    let active_byte = registers.register_specified_by_opcode(immediate_byte);

    let flag_diff = match immediate_byte {
        0x30..=0x37 => swap(active_byte),
        0x80..=0x87 => res(0, active_byte),
        0x88..=0x8f => res(1, active_byte),
        0x90..=0x97 => res(2, active_byte),
        0x98..=0x9f => res(3, active_byte),
        0xa0..=0xa7 => res(4, active_byte),
        0xa8..=0xaf => res(5, active_byte),
        0xb0..=0xb7 => res(6, active_byte),
        0xb8..=0xbf => res(7, active_byte),
        0xc0..=0xc7 => set(0, active_byte),
        0xc8..=0xcf => set(1, active_byte),
        0xd0..=0xd7 => set(2, active_byte),
        0xd8..=0xdf => set(3, active_byte),
        0xe0..=0xe7 => set(4, active_byte),
        0xe8..=0xef => set(5, active_byte),
        0xf0..=0xf7 => set(6, active_byte),
        0xf8..=0xff => set(7, active_byte),
        _ => todo!(
            "Unknown 0xcb opcode {:#04x} at address {:#06x}\n",
            immediate_byte,
            registers.pc
        ),
    };

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
