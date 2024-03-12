use super::*;
use crate::cpu::Registers;
use crate::Memory;

pub fn execute_cb_instruction(registers: &mut Registers, memory: &mut Memory) -> CpuDiff {
    let immediate_byte = memory.read(registers.pc + 1);

    let active_byte = registers.register_specified_by_opcode(immediate_byte);

    let flag_diff = match immediate_byte {
        0x30..=0x37 => swap(active_byte),
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
