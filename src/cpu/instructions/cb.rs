use super::Finish2;
use super::FlagChanges;
use crate::cpu::Registers;
use crate::Memory;

pub fn execute_cb_instruction(registers: &mut Registers, memory: &mut Memory) -> Finish2 {
    let immediate_byte = memory.read(registers.pc + 1);

    let active_byte = registers.register_specified_by_opcode(immediate_byte);

    let flag_changes = match immediate_byte {
        0x30..=0x37 => swap(active_byte),
        _ => todo!(
            "Unknown 0xcb opcode {:#04x} at address {:#06x}\n",
            immediate_byte,
            registers.pc
        ),
    };

    let cycle_nybble = immediate_byte & 0x0f;
    let elapsed_cycles = if cycle_nybble == 0x06 || cycle_nybble == 0x0e {
        16
    } else {
        8
    };
    Finish2 {
        flag_changes,
        pc_increment: 2,
        elapsed_cycles,
    }
}

fn swap(byte_to_swap: &mut u8) -> FlagChanges {
    let upper_4_bits = (*byte_to_swap) & 0xf0;
    let lower_4_bits = (*byte_to_swap) & 0x0f;
    *byte_to_swap = upper_4_bits >> 4;
    *byte_to_swap |= lower_4_bits << 4;

    FlagChanges {
        z: Some(*byte_to_swap == 0),
        n: Some(false),
        h: Some(false),
        c: Some(false),
    }
}
