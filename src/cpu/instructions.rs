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

fn print_instruction(pc: u16, memory: &mut Memory) {
    let opcode = memory.read(pc);
    match opcode {
        0x00 => println!("NOP"),
        0x05 => println!("DEC B"),
        0x06 => println!("LD B,{:#04x}", memory.read(pc + 1)),
        0x0d => println!("DEC C"),
        0x0e => println!("LD C,{:#04x}", memory.read(pc + 1)),
        0x11 => println!("LD DE,{:#06x}", memory.read_u16(pc + 1)),
        0x20 => println!("JR NZ,{}", 2 + i16::from(memory.read(pc + 1) as i8)),
        0x21 => println!("LD HL,{:#06x}", memory.read_u16(pc + 1)),
        0x2c => println!("INC L"),
        0x32 => println!("LD (HL-),A"),
        0x3e => println!("LD A,{:#04x}", memory.read(pc + 1)),
        0x40 => println!("LD B,B"),
        0x41 => println!("LD B,C"),
        0x42 => println!("LD B,D"),
        0x43 => println!("LD B,E"),
        0x44 => println!("LD B,H"),
        0x45 => println!("LD B,L"),
        0x47 => println!("LD B,A"),
        0x48 => println!("LD C,B"),
        0x49 => println!("LD C,C"),
        0x4a => println!("LD C,D"),
        0x4b => println!("LD C,E"),
        0x4c => println!("LD C,H"),
        0x4d => println!("LD C,L"),
        0x4f => println!("LD C,A"),
        0x50 => println!("LD D,B"),
        0x51 => println!("LD D,C"),
        0x52 => println!("LD D,D"),
        0x53 => println!("LD D,E"),
        0x54 => println!("LD D,H"),
        0x55 => println!("LD D,L"),
        0x57 => println!("LD D,A"),
        0x58 => println!("LD E,B"),
        0x59 => println!("LD E,C"),
        0x5a => println!("LD E,D"),
        0x5b => println!("LD E,E"),
        0x5c => println!("LD E,H"),
        0x5d => println!("LD E,L"),
        0x5f => println!("LD E,A"),
        0x60 => println!("LD H,B"),
        0x61 => println!("LD H,C"),
        0x62 => println!("LD H,D"),
        0x63 => println!("LD H,E"),
        0x64 => println!("LD H,H"),
        0x65 => println!("LD H,L"),
        0x67 => println!("LD H,A"),
        0x68 => println!("LD L,B"),
        0x69 => println!("LD L,C"),
        0x6a => println!("LD L,D"),
        0x6b => println!("LD L,E"),
        0x6c => println!("LD L,H"),
        0x6d => println!("LD L,L"),
        0x6f => println!("LD L,A"),
        0xaf => println!("XOR A"),
        0xc3 => println!("JP {:#06x}", memory.read_u16(pc + 1)),
        0xe0 => println!("LDH {:#06x},A", 0xff00 + u16::from(memory.read(pc + 1))),
        0xf0 => println!("LDH A,{:#06x}", 0xff00 + u16::from(memory.read(pc + 1))),
        0xf3 => println!("DI"),
        0xfe => println!("CP {:#04x}", memory.read(pc + 1)),
        _ => todo!("op {:#04x} at address {:#06x}", opcode, pc),
    };
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

    *dst_register = dst_register.wrapping_add(src);

    Finish {
        pc_increment: 1,
        elapsed_cycles: 8,
    }
}

pub fn call_condition(condition: bool, registers: &mut Registers, memory: &mut Memory) -> Finish {
    if condition {
        stack_push(registers.pc + 3, &mut registers.sp, memory);

        registers.pc = memory.read_u16(registers.pc + 1);
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

    registers.a = registers.a.wrapping_add(add_src);

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

pub fn rst(address_lower_byte: u8, registers: &mut Registers, memory: &mut Memory) -> Finish {
    stack_push(registers.pc + 1, &mut registers.sp, memory);
    registers.pc = address_lower_byte.into();
    Finish {
        pc_increment: 0,
        elapsed_cycles: 16,
    }
}
