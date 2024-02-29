use crate::cpu::*;

pub struct Finish {
    pub pc_increment: i16,
    pub elapsed_cycles: u8,
}

pub fn subtraction_produces_u8_full_carry(a: u8, b: u8) -> bool {
    i16::from(a) - i16::from(b) < 0
}

pub fn addition_produces_u8_full_carry(a: u8, b: u8) -> bool {
    i16::from(a) + i16::from(b) > 0xff
}

pub fn subtraction_produces_u8_half_carry(
    a: u8,
    b: u8,
    register_f: u8,
    include_carry: bool,
) -> bool {
    let optional_carry: i16 = if include_carry && (register_f & Registers::FLAG_CARRY) != 0 {
        1
    } else {
        0
    };

    i16::from(a & 0x0f) - i16::from(b & 0x0f) - optional_carry < 0
}

pub fn addition_produces_u8_half_carry(a: u8, b: u8, register_f: u8, include_carry: bool) -> bool {
    let optional_carry: i16 = if include_carry && (register_f & Registers::FLAG_CARRY) != 0 {
        1
    } else {
        0
    };

    i16::from(a & 0x0f) + i16::from(b & 0x0f) + optional_carry > 0x0f
}

pub fn inc_u8(value_to_increment: &mut u8, register_f: &mut u8, elapsed_cycles: u8) -> Finish {
    if addition_produces_u8_half_carry(*value_to_increment, 1, *register_f, false) {
        *register_f |= Registers::FLAG_HALFCARRY;
    } else {
        *register_f &= !Registers::FLAG_HALFCARRY;
    }

    *value_to_increment += 1;

    if *value_to_increment != 0 {
        *register_f &= !Registers::FLAG_ZERO;
    } else {
        *register_f |= Registers::FLAG_ZERO;
    }

    *register_f &= !Registers::FLAG_SUBTRACTION;

    Finish {
        pc_increment: 1,
        elapsed_cycles,
    }
}

pub fn xor(xor_input: u8, registers: &mut Registers, elapsed_cycles: u8) -> Finish {
    registers.a ^= xor_input;

    if registers.a == 0 {
        registers.f |= Registers::FLAG_ZERO;
    } else {
        registers.f &= !Registers::FLAG_ZERO;
    }

    registers.f &= !Registers::FLAG_SUBTRACTION;
    registers.f &= !Registers::FLAG_HALFCARRY;
    registers.f &= !Registers::FLAG_CARRY;

    Finish {
        pc_increment: 1,
        elapsed_cycles,
    }
}

pub fn or(
    or_input: u8,
    registers: &mut Registers,
    pc_increment: i16,
    elapsed_cycles: u8,
) -> Finish {
    // rwtodo: I think pc_increment might always be 1, thereby allowing us to remove it as from the param list.

    registers.a |= or_input;

    if registers.a == 0 {
        registers.f |= Registers::FLAG_ZERO;
    } else {
        registers.f &= !Registers::FLAG_ZERO;
    }

    registers.f &= !Registers::FLAG_SUBTRACTION;
    registers.f &= !Registers::FLAG_HALFCARRY;
    registers.f &= !Registers::FLAG_CARRY;

    Finish {
        pc_increment,
        elapsed_cycles,
    }
}

pub fn and(
    and_input: u8,
    registers: &mut Registers,
    pc_increment: i16,
    elapsed_cycles: u8,
) -> Finish {
    registers.a &= and_input;

    if registers.a == 0 {
        registers.f |= Registers::FLAG_ZERO;
    } else {
        registers.f &= !Registers::FLAG_ZERO;
    }

    registers.f &= !Registers::FLAG_SUBTRACTION;
    registers.f |= Registers::FLAG_HALFCARRY;
    registers.f &= !Registers::FLAG_CARRY;

    Finish {
        pc_increment,
        elapsed_cycles,
    }
}

fn set(bit_to_set: u8, byte_to_set: &mut u8, elapsed_cycles: u8) -> Finish {
    *byte_to_set |= 0x01 << bit_to_set;
    Finish {
        pc_increment: 1,
        elapsed_cycles,
    }
}

fn res(bit_to_reset: u8, byte_to_reset: &mut u8, elapsed_cycles: u8) -> Finish {
    *byte_to_reset &= !(0x01 << bit_to_reset);
    Finish {
        pc_increment: 1,
        elapsed_cycles,
    }
}

fn bit(bit_to_check: u8, byte_to_check: u8, register_f: &mut u8, elapsed_cycles: u8) -> Finish {
    if (byte_to_check & (0x01 << bit_to_check)) != 0 {
        *register_f &= !Registers::FLAG_ZERO;
    } else {
        *register_f |= Registers::FLAG_ZERO;
    }

    *register_f &= !Registers::FLAG_SUBTRACTION;
    *register_f |= Registers::FLAG_HALFCARRY;

    Finish {
        pc_increment: 1,
        elapsed_cycles,
    }
}

fn swap(byte_to_swap: &mut u8, register_f: &mut u8, elapsed_cycles: u8) -> Finish {
    let upper_4_bits = *byte_to_swap & 0xf0;
    let lower_4_bits = *byte_to_swap & 0x0f;
    *byte_to_swap = upper_4_bits >> 4;
    *byte_to_swap |= lower_4_bits << 4;

    if *byte_to_swap == 0 {
        *register_f |= Registers::FLAG_ZERO;
    } else {
        *register_f &= !Registers::FLAG_ZERO;
    }

    *register_f &= !Registers::FLAG_SUBTRACTION;
    *register_f &= !Registers::FLAG_HALFCARRY;
    *register_f &= !Registers::FLAG_CARRY;

    Finish {
        pc_increment: 1,
        elapsed_cycles,
    }
}

pub fn ld_reg8_mem8(dst_register: &mut u8, src_memory: u8) -> Finish {
    *dst_register = src_memory;
    Finish {
        pc_increment: 2,
        elapsed_cycles: 8,
    }
}

pub fn ld_reg8_reg8(dst_register: &mut u8, src_register: u8) -> Finish {
    *dst_register = src_register;
    Finish {
        pc_increment: 1,   // Same as NOP
        elapsed_cycles: 4, // Same as NOP
    }
}

pub fn nop() -> Finish {
    Finish {
        pc_increment: 1,   // Same as ld_reg8_reg8
        elapsed_cycles: 4, // Same as ld_reg8_reg8
    }
}

pub fn dec_reg8(register_to_dec: &mut u8, register_f: &mut u8) -> Finish {
    if subtraction_produces_u8_half_carry(*register_to_dec, 1, *register_f, false) {
        *register_f |= Registers::FLAG_HALFCARRY;
    } else {
        *register_f &= !Registers::FLAG_HALFCARRY;
    }

    *register_to_dec = register_to_dec.wrapping_sub(1);

    if *register_to_dec != 0 {
        *register_f &= !Registers::FLAG_ZERO;
    } else {
        *register_f |= Registers::FLAG_ZERO;
    }

    *register_f |= Registers::FLAG_SUBTRACTION;

    Finish {
        pc_increment: 1,
        elapsed_cycles: 4,
    }
}

pub fn add_reg16(src: u16, dst_register: &mut u16, register_f: &mut u8) -> Finish {
    // Check for 16-bit full carry
    if i32::from(*dst_register) + i32::from(src) > 0xffff {
        *register_f |= Registers::FLAG_CARRY;
    } else {
        *register_f &= !Registers::FLAG_CARRY;
    }

    // Check for 16-bit half carry
    if (*dst_register & 0x0fff) + (src & 0x0fff) > 0x0fff {
        *register_f |= Registers::FLAG_HALFCARRY;
    } else {
        *register_f &= !Registers::FLAG_HALFCARRY;
    }

    *register_f &= !Registers::FLAG_SUBTRACTION;

    *dst_register += src;

    Finish {
        pc_increment: 1,
        elapsed_cycles: 8,
    }
}

pub fn call_condition(condition: bool, pc: &mut u16, sp: &mut u16, memory: &mut Memory) -> Finish {
    if condition {
        stack_push(*pc + 3, sp, memory);

        *pc = memory.read_u16(*pc + 1);
        Finish {
            pc_increment: 0,
            elapsed_cycles: 24,
        }
    } else {
        Finish {
            pc_increment: 3,
            elapsed_cycles: 12,
        }
    }
}

pub fn add_u8(
    add_src: u8,
    registers: &mut Registers,
    pc_increment: i16,
    elapsed_cycles: u8,
) -> Finish {
    if addition_produces_u8_half_carry(registers.a, add_src, registers.f, false) {
        registers.f |= Registers::FLAG_HALFCARRY;
    } else {
        registers.f &= !Registers::FLAG_HALFCARRY;
    }

    if addition_produces_u8_full_carry(registers.a, add_src) {
        registers.f |= Registers::FLAG_CARRY;
    } else {
        registers.f &= !Registers::FLAG_CARRY;
    }

    registers.a += add_src;

    if registers.a == 0 {
        registers.f |= Registers::FLAG_ZERO;
    } else {
        registers.f &= !Registers::FLAG_ZERO;
    }

    registers.f &= !Registers::FLAG_SUBTRACTION;

    Finish {
        pc_increment,
        elapsed_cycles,
    }
}

pub fn adc(
    add_src: u8,
    registers: &mut Registers,
    pc_increment: i16,
    elapsed_cycles: u8,
) -> Finish {
    let carry = if (registers.f & Registers::FLAG_CARRY) != 0 {
        1
    } else {
        0
    };

    if addition_produces_u8_half_carry(registers.a, add_src, registers.f, true) {
        registers.f |= Registers::FLAG_HALFCARRY;
    } else {
        registers.f &= !Registers::FLAG_HALFCARRY;
    }

    // rwtodo: this might have an issue with it similar to the trouble I had with adding the carry for the half-carry calculation above.
    if addition_produces_u8_full_carry(registers.a, add_src + carry) {
        registers.f |= Registers::FLAG_CARRY;
    } else {
        registers.f &= !Registers::FLAG_CARRY;
    }

    registers.a += add_src + carry;

    if registers.a == 0 {
        registers.f |= Registers::FLAG_ZERO;
    } else {
        registers.f &= !Registers::FLAG_ZERO;
    }

    registers.f &= !Registers::FLAG_SUBTRACTION;

    Finish {
        pc_increment,
        elapsed_cycles,
    }
}

pub fn sub(
    sub_src: u8,
    registers: &mut Registers,
    pc_increment: i16,
    elapsed_cycles: u8,
) -> Finish {
    if subtraction_produces_u8_half_carry(registers.a, sub_src, registers.f, false) {
        registers.f |= Registers::FLAG_HALFCARRY;
    } else {
        registers.f &= !Registers::FLAG_HALFCARRY;
    }

    if subtraction_produces_u8_full_carry(registers.a, sub_src) {
        registers.f |= Registers::FLAG_CARRY;
    } else {
        registers.f &= !Registers::FLAG_CARRY;
    }

    registers.a -= sub_src;

    if registers.a == 0 {
        registers.f |= Registers::FLAG_ZERO;
    } else {
        registers.f &= !Registers::FLAG_ZERO;
    }

    registers.f |= Registers::FLAG_SUBTRACTION;

    Finish {
        pc_increment,
        elapsed_cycles,
    }
}

pub fn sbc(
    sub_src: u8,
    registers: &mut Registers,
    pc_increment: i16,
    elapsed_cycles: u8,
) -> Finish {
    let carry = if (registers.f & Registers::FLAG_CARRY) != 0 {
        1
    } else {
        0
    };

    if subtraction_produces_u8_half_carry(registers.a, sub_src, registers.f, true) {
        registers.f |= Registers::FLAG_HALFCARRY;
    } else {
        registers.f &= !Registers::FLAG_HALFCARRY;
    }

    if subtraction_produces_u8_full_carry(registers.a, sub_src + carry) {
        registers.f |= Registers::FLAG_CARRY;
    } else {
        registers.f &= !Registers::FLAG_CARRY;
    }

    registers.a -= sub_src + carry;

    if registers.a == 0 {
        registers.f |= Registers::FLAG_ZERO;
    } else {
        registers.f &= !Registers::FLAG_ZERO;
    }

    registers.f |= Registers::FLAG_SUBTRACTION;

    Finish {
        pc_increment,
        elapsed_cycles,
    }
}

pub fn cp(
    comparator: u8,
    registers: &mut Registers,
    pc_increment: i16,
    elapsed_cycles: u8,
) -> Finish {
    if subtraction_produces_u8_half_carry(registers.a, comparator, registers.f, false) {
        registers.f |= Registers::FLAG_HALFCARRY;
    } else {
        registers.f &= !Registers::FLAG_HALFCARRY;
    }

    if subtraction_produces_u8_full_carry(registers.a, comparator) {
        registers.f |= Registers::FLAG_CARRY;
    } else {
        registers.f &= !Registers::FLAG_CARRY;
    }

    registers.f |= Registers::FLAG_SUBTRACTION; // Set the add/sub flag high, indicating subtraction.

    let sub_result: u8 = registers.a - comparator;

    if sub_result == 0 {
        registers.f |= Registers::FLAG_ZERO;
    } else {
        registers.f &= !Registers::FLAG_ZERO;
    }

    Finish {
        pc_increment,
        elapsed_cycles,
    }
}
