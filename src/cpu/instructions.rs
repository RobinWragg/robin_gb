use super::*;

pub mod cb;

pub struct FlagDiff {
    pub z: Option<bool>,
    pub n: Option<bool>,
    pub h: Option<bool>,
    pub c: Option<bool>,
}

// rwtodo: documentation will be important here.
pub struct CpuDiff {
    pub flag_diff: FlagDiff,
    pub pc_delta: i16,
    pub cycles: u8,
}

impl CpuDiff {
    pub fn new(pc_delta: i16, cycles: u8) -> CpuDiff {
        CpuDiff {
            pc_delta,
            cycles,
            flag_diff: FlagDiff {
                z: None,
                n: None,
                h: None,
                c: None,
            },
        }
    }

    pub fn flag_z(mut self, new_z: bool) -> Self {
        self.flag_diff.z = Some(new_z);
        self
    }

    pub fn flag_n(mut self, new_n: bool) -> Self {
        self.flag_diff.n = Some(new_n);
        self
    }

    pub fn flag_h(mut self, new_h: bool) -> Self {
        self.flag_diff.h = Some(new_h);
        self
    }

    pub fn flag_c(mut self, new_c: bool) -> Self {
        self.flag_diff.c = Some(new_c);
        self
    }
}

// rwtodo: Maybe these checks should return a FlagDiff.
pub fn subtraction_produces_full_carry(a: u8, b: u8) -> bool {
    i16::from(a) - i16::from(b) < 0
}

pub fn addition_produces_full_carry(a: u8, b: u8) -> bool {
    i16::from(a) + i16::from(b) > 0xff
}

pub fn subtraction_produces_half_carry(a: u8, b: u8, register_f: u8, include_carry: bool) -> bool {
    let optional_carry: i16 = if include_carry && (register_f & Registers::FLAG_CARRY) != 0 {
        1
    } else {
        0
    };

    i16::from(a & 0x0f) - i16::from(b & 0x0f) - optional_carry < 0
}

pub fn addition_produces_half_carry(a: u8, b: u8, register_f: u8, include_carry: bool) -> bool {
    let optional_carry: i16 = if include_carry && (register_f & Registers::FLAG_CARRY) != 0 {
        1
    } else {
        0
    };

    i16::from(a & 0x0f) + i16::from(b & 0x0f) + optional_carry > 0x0f
}

pub fn print_instruction(pc: u16, memory: &Memory) {
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
        0x28 => println!("JR Z,{}", 2 + i16::from(memory.read(pc + 1) as i8)),
        0x2a => println!("LD A,(HL+)"),
        0x2c => println!("INC L"),
        0x32 => println!("LD (HL-),A"),
        0x36 => println!("LD (HL),{:#04x}", memory.read(pc + 1)),
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
        0xc8 => println!("RET Z"),
        0xcd => println!("CALL {:#06x}", memory.read_u16(pc + 1)),
        0xe0 => println!("LDH {:#06x},A", 0xff00 + u16::from(memory.read(pc + 1))),
        0xe6 => println!("AND {:#04x}", memory.read(pc + 1)),
        0xf0 => println!("LDH A,{:#06x}", 0xff00 + u16::from(memory.read(pc + 1))),
        0xf3 => println!("DI"),
        0xfe => println!("CP {:#04x}", memory.read(pc + 1)),
        _ => println!("op {:#04x} at address {:#06x}", opcode, pc),
    };
}

pub fn inc_u8(value_to_increment: &mut u8, register_f: u8, cycles: u8) -> CpuDiff {
    let half_carry = addition_produces_half_carry(*value_to_increment, 1, register_f, false);
    *value_to_increment = value_to_increment.wrapping_add(1);

    CpuDiff::new(1, cycles)
        .flag_z(*value_to_increment == 0)
        .flag_n(false)
        .flag_h(half_carry)
}

pub fn xor(xor_input: u8, register_a: &mut u8, pc_delta: i16, cycles: u8) -> CpuDiff {
    *register_a ^= xor_input;
    CpuDiff::new(pc_delta, cycles)
        .flag_z(*register_a == 0)
        .flag_n(false)
        .flag_h(false)
        .flag_c(false)
}

pub fn or(or_input: u8, register_a: &mut u8, pc_delta: i16, cycles: u8) -> CpuDiff {
    // rwtodo: I think pc_delta might always be 1, thereby allowing us to remove it as from the param list.
    *register_a |= or_input;
    CpuDiff::new(pc_delta, cycles)
        .flag_z(*register_a == 0)
        .flag_n(false)
        .flag_h(false)
        .flag_c(false)
}

pub fn and(and_input: u8, register_a: &mut u8, pc_delta: i16, cycles: u8) -> CpuDiff {
    *register_a &= and_input;

    CpuDiff::new(pc_delta, cycles)
        .flag_z(*register_a == 0)
        .flag_n(false)
        .flag_h(true)
        .flag_c(false)
}

pub fn ld_reg8_mem8(dst_register: &mut u8, src_memory: u8) -> CpuDiff {
    *dst_register = src_memory;
    CpuDiff::new(2, 8)
}

pub fn ld_reg8_reg8(dst_register: &mut u8, src_register: u8) -> CpuDiff {
    *dst_register = src_register;

    // Same as NOP, as LD B,B is a de facto NOP.
    CpuDiff::new(1, 4)
}

pub fn nop() -> CpuDiff {
    // Same as ld_reg8_reg8
    CpuDiff::new(1, 4)
}

pub fn dec_u8(value_to_dec: &mut u8, register_f: u8, cycles: u8) -> CpuDiff {
    let half_carry = subtraction_produces_half_carry(*value_to_dec, 1, register_f, false);

    *value_to_dec = value_to_dec.wrapping_sub(1);

    CpuDiff::new(1, cycles)
        .flag_h(half_carry)
        .flag_n(true)
        .flag_z(*value_to_dec == 0)
}

// rwtodo: having the params in order src,dst isn't idiomatic.
pub fn add_reg16(src: u16, dst_register: &mut u16) -> CpuDiff {
    // Check for 16-bit full- and half-carry
    let full_carry = i32::from(*dst_register) + i32::from(src) > 0xffff;
    let half_carry = (*dst_register & 0x0fff) + (src & 0x0fff) > 0x0fff;

    *dst_register = dst_register.wrapping_add(src);

    CpuDiff::new(1, 8)
        .flag_n(false)
        .flag_h(half_carry)
        .flag_c(full_carry)
}

pub fn call(condition: bool, registers: &mut Registers, memory: &mut Memory) -> CpuDiff {
    if condition {
        stack_push(registers.pc + 3, &mut registers.sp, memory);
        registers.pc = memory.read_u16(registers.pc + 1);
        CpuDiff::new(0, 24)
    } else {
        CpuDiff::new(3, 12)
    }
}

// rwtodo: Can I reuse this for INC? and SUB for DEC?
pub fn add_u8(add_src: u8, registers: &mut Registers, pc_delta: i16, cycles: u8) -> CpuDiff {
    let half_carry = addition_produces_half_carry(registers.a, add_src, registers.f, false);
    let full_carry = addition_produces_full_carry(registers.a, add_src);

    registers.a = registers.a.wrapping_add(add_src);

    CpuDiff::new(pc_delta, cycles)
        .flag_z(registers.a == 0)
        .flag_n(false)
        .flag_h(half_carry)
        .flag_c(full_carry)
}

pub fn adc(add_src: u8, registers: &mut Registers, pc_delta: i16, cycles: u8) -> CpuDiff {
    let carry_value = if (registers.f & Registers::FLAG_CARRY) != 0 {
        1
    } else {
        0
    };

    let half_carry_flag = addition_produces_half_carry(registers.a, add_src, registers.f, true);
    let full_carry_flag = addition_produces_full_carry(registers.a, add_src + carry_value);

    registers.a += add_src + carry_value;

    CpuDiff::new(pc_delta, cycles)
        .flag_h(half_carry_flag)
        .flag_c(full_carry_flag)
        .flag_z(registers.a == 0)
        .flag_n(false)
}

pub fn sub(sub_src: u8, registers: &mut Registers, pc_delta: i16, cycles: u8) -> CpuDiff {
    let half_carry = subtraction_produces_half_carry(registers.a, sub_src, registers.f, false);
    let full_carry = subtraction_produces_full_carry(registers.a, sub_src);

    registers.a = registers.a.wrapping_sub(sub_src);

    CpuDiff::new(pc_delta, cycles)
        .flag_z(registers.a == 0)
        .flag_n(true)
        .flag_h(half_carry)
        .flag_c(full_carry)
}

pub fn sbc(sub_src: u8, registers: &mut Registers, pc_delta: i16, cycles: u8) -> CpuDiff {
    let carry = if (registers.f & Registers::FLAG_CARRY) != 0 {
        1
    } else {
        0
    };

    let half_carry = subtraction_produces_half_carry(registers.a, sub_src, registers.f, true);
    let full_carry = subtraction_produces_full_carry(registers.a, sub_src + carry);

    registers.a -= sub_src + carry;

    CpuDiff::new(pc_delta, cycles)
        .flag_z(registers.a == 0)
        .flag_n(true)
        .flag_h(half_carry)
        .flag_c(full_carry)
}

// rwtodo: I think this always increments pc by 1, so that param can be removed.
pub fn cp(comparator: u8, registers: &Registers, pc_delta: i16, cycles: u8) -> CpuDiff {
    let half_carry = subtraction_produces_half_carry(registers.a, comparator, registers.f, false);
    let full_carry = subtraction_produces_full_carry(registers.a, comparator);

    let sub_result: u8 = registers.a.wrapping_sub(comparator);

    CpuDiff::new(pc_delta, cycles)
        .flag_z(sub_result == 0)
        .flag_n(true)
        .flag_h(half_carry)
        .flag_c(full_carry)
}

pub fn rst(address_lower_byte: u8, registers: &mut Registers, memory: &mut Memory) -> CpuDiff {
    stack_push(registers.pc + 1, &mut registers.sp, memory);
    registers.pc = address_lower_byte.into();
    CpuDiff::new(0, 16)
}
