mod instructions;

use crate::address;
use crate::interrupt;
use crate::Memory;
use crate::{make_bit, make_u16};
use instructions::FlagDiff;

// rwtodo: dang, I've got to check every - and + to ensure wraparounds.

// rwtodo: Apparently STOP is like HALT except the LCD is inoperational as well, and the "stopped" state is only exited when a button is pressed. Look for better documentation on it.

fn stack_push(value_to_push: u16, sp: &mut u16, memory: &mut Memory) {
    *sp -= 2;
    memory.write_u16(*sp, value_to_push);
}

fn stack_pop(sp: &mut u16, memory: &Memory) -> u16 {
    let popped_value = memory.read_u16(*sp);
    *sp += 2;
    popped_value
}

type CycleCount = u8;

#[derive(Default)]
pub struct Registers {
    // General purpose registers
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8, // Flag register
    h: u8,
    l: u8,

    pc: u16,   // Program counter
    sp: u16,   // Stack pointer
    ime: bool, // rwtodo
}

impl Registers {
    const FLAG_ZERO: u8 = 0x80; // "Z", Zero Flag
    const FLAG_SUBTRACTION: u8 = 0x40; // "N", Add/Sub-Flag (BCD)
    const FLAG_HALFCARRY: u8 = 0x20; // "H", Half Carry Flag (BCD)
    const FLAG_CARRY: u8 = 0x10; // "C", Carry Flag

    fn new() -> Self {
        let mut ret = Self {
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xd8,
            f: 0xb0,
            h: 0x01,
            l: 0x4d,
            pc: 0x0100,
            sp: 0xfffe,
            ime: true,
        };

        assert_eq!(ret.de(), 0x00d8);
        ret.set_de(0x1234);
        assert_eq!(ret.de(), 0x1234);
        ret.set_de(0x00d8);
        assert_eq!(ret.de(), 0x00d8);
        assert_eq!(ret.d, 0x00);
        assert_eq!(ret.e, 0xd8);

        ret
    }

    fn af(&self) -> u16 {
        make_u16(self.f, self.a)
    }

    fn set_af(&mut self, new_af: u16) {
        let bytes = new_af.to_le_bytes();
        self.f = bytes[0];
        self.a = bytes[1];
    }

    fn bc(&self) -> u16 {
        make_u16(self.c, self.b)
    }

    fn set_bc(&mut self, new_bc: u16) {
        let bytes = new_bc.to_le_bytes();
        self.c = bytes[0];
        self.b = bytes[1];
    }

    fn de(&self) -> u16 {
        make_u16(self.e, self.d)
    }

    fn set_de(&mut self, new_de: u16) {
        let bytes = new_de.to_le_bytes();
        self.e = bytes[0];
        self.d = bytes[1];
    }

    fn hl(&self) -> u16 {
        make_u16(self.l, self.h)
    }

    fn set_hl(&mut self, new_hl: u16) {
        let bytes = new_hl.to_le_bytes();
        self.l = bytes[0];
        self.h = bytes[1];
    }

    fn read_operand_8bit(&mut self, operand_code_3bit: u8, memory: &Memory) -> u8 {
        debug_assert!(operand_code_3bit & 0b11111000 == 0);
        match operand_code_3bit {
            0x00 => self.b,
            0x01 => self.c,
            0x02 => self.d,
            0x03 => self.e,
            0x04 => self.h,
            0x05 => self.l,
            0x06 => memory.read(self.hl()),
            0x07 => self.a,
            _ => todo!(),
        }
    }

    fn write_operand_8bit(
        &mut self,
        operand_value: u8,
        operand_code_3bit: u8,
        memory: &mut Memory,
    ) {
        debug_assert!(operand_code_3bit & 0b11111000 == 0);
        match operand_code_3bit & 0x0f {
            0x00 => self.b = operand_value,
            0x01 => self.c = operand_value,
            0x02 => self.d = operand_value,
            0x03 => self.e = operand_value,
            0x04 => self.h = operand_value,
            0x05 => self.l = operand_value,
            0x06 => memory.write(self.hl(), operand_value),
            0x07 => self.a = operand_value,
            _ => todo!(),
        };
    }

    fn update_flags(&mut self, flag_changes: FlagDiff) {
        if let Some(z) = flag_changes.z {
            if z {
                self.f |= Self::FLAG_ZERO;
            } else {
                self.f &= !Self::FLAG_ZERO;
            }
        }

        if let Some(n) = flag_changes.n {
            if n {
                self.f |= Self::FLAG_SUBTRACTION;
            } else {
                self.f &= !Self::FLAG_SUBTRACTION;
            }
        }

        if let Some(h) = flag_changes.h {
            if h {
                self.f |= Self::FLAG_HALFCARRY;
            } else {
                self.f &= !Self::FLAG_HALFCARRY;
            }
        }

        if let Some(c) = flag_changes.c {
            if c {
                self.f |= Self::FLAG_CARRY;
            } else {
                self.f &= !Self::FLAG_CARRY;
            }
        }
    }
}

pub struct Cpu {
    registers: Registers, // rwtodo maybe just put the registers in the cpu without wrapping them in a struct
    is_halted: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            registers: Registers::new(),
            is_halted: false,
        }
    }

    fn handle_interrupt_requests(&mut self, memory: &mut Memory) {
        let mut requested_interrupts = *memory.direct_access(address::INTERRUPT_FLAGS);
        let enabled_interrupts = *memory.direct_access(address::INTERRUPT_ENABLE);
        let interrupts_to_handle = requested_interrupts & enabled_interrupts;

        if interrupts_to_handle != 0x00 {
            if self.is_halted {
                self.is_halted = false;
            }

            if self.registers.ime {
                self.registers.ime = false;
                stack_push(self.registers.pc, &mut self.registers.sp, memory);

                // rwtodo can i do this with a match?
                if interrupts_to_handle & interrupt::FLAG_VBLANK != 0 {
                    requested_interrupts &= !interrupt::FLAG_VBLANK;
                    self.registers.pc = 0x0040;
                } else if interrupts_to_handle & interrupt::FLAG_LCD_STAT != 0 {
                    requested_interrupts &= !interrupt::FLAG_LCD_STAT;
                    self.registers.pc = 0x0048;
                } else if interrupts_to_handle & interrupt::FLAG_TIMER != 0 {
                    requested_interrupts &= !interrupt::FLAG_TIMER;
                    self.registers.pc = 0x0050;
                } else if interrupts_to_handle & interrupt::FLAG_SERIAL != 0 {
                    requested_interrupts &= !interrupt::FLAG_SERIAL;
                    self.registers.pc = 0x0058;
                } else if interrupts_to_handle & interrupt::FLAG_JOYPAD != 0 {
                    requested_interrupts &= !interrupt::FLAG_JOYPAD;
                    self.registers.pc = 0x0060;
                } else {
                    unreachable!("Unexpected interrupts_to_handle value");
                }

                *memory.direct_access(address::INTERRUPT_FLAGS) = requested_interrupts;
            }
        }
    }

    #[must_use] // Returns the number of cycles the instruction took.
    pub fn execute_next_instruction(&mut self, memory: &mut Memory) -> u8 {
        let cycles = self.execute_next_instruction_inner(memory);
        self.handle_interrupt_requests(memory);
        cycles
    }

    #[must_use] // Returns the number of cycles the instruction took.
    fn execute_next_instruction_inner(&mut self, memory: &mut Memory) -> u8 {
        if self.is_halted {
            return 4;
        }

        // Check for unexpected addresses for instructions.
        debug_assert!(
            self.registers.pc < 0x8000
                || (self.registers.pc >= 0xff80 && self.registers.pc < 0xffff)
                || (self.registers.pc >= 0xa000 && self.registers.pc < 0xfe00)
        );

        let opcode = memory.read(self.registers.pc);

        // Getters for the data immediately following the program counter (pc).
        let immediate_u8 = || memory.read(self.registers.pc + 1);
        let immediate_u16 = || memory.read_u16(self.registers.pc + 1);

        // print!("{:#06x} {:#04x}: ", self.registers.pc, opcode);
        // print_instruction(self.registers.pc, memory);
        // println!();

        use instructions::*;

        let diff: CpuDiff = match opcode {
            0x00 => nop(), // NOP
            0x01 => {
                self.registers.set_bc(immediate_u16());
                CpuDiff::new(3, 12)
            } // LD BC,xx
            0x02 => {
                memory.write(self.registers.bc(), self.registers.a);
                CpuDiff::new(1, 8)
            } // LD (BC),A
            0x03 => {
                self.registers.set_bc(self.registers.bc().wrapping_add(1));
                CpuDiff::new(1, 8)
            } // INC BC
            0x04 => inc_u8(&mut self.registers.b, self.registers.f, 4), // INC B
            0x05 => dec_u8(&mut self.registers.b, self.registers.f, 4), // DEC B
            0x06 => ld_reg8_mem8(&mut self.registers.b, immediate_u8()), // LD B,x
            0x07 => {
                let bit_7 = self.registers.a & make_bit(7) != 0;

                if bit_7 {
                    self.registers.a <<= 1;
                    self.registers.a |= make_bit(0);
                } else {
                    self.registers.a <<= 1;
                    self.registers.a &= !make_bit(0);
                }

                CpuDiff::new(1, 4)
                    .flag_z(false)
                    .flag_n(false)
                    .flag_h(false)
                    .flag_c(bit_7)
            } // RLCA
            0x08 => {
                memory.write_u16(immediate_u16(), self.registers.sp);
                CpuDiff::new(3, 20)
            } // LD (xx),SP
            0x09 => {
                let mut hl = self.registers.hl();
                let diff = add_reg16(self.registers.bc(), &mut hl);
                self.registers.set_hl(hl);
                diff
            } // ADD HL,BC
            0x0a => {
                self.registers.a = memory.read(self.registers.bc());
                CpuDiff::new(1, 8)
            } // LD A,(BC)
            0x0b => {
                let bc = self.registers.bc().wrapping_sub(1);
                self.registers.set_bc(bc);
                CpuDiff::new(1, 8)
            } // DEC BC
            0x0c => inc_u8(&mut self.registers.c, self.registers.f, 4), // INC C
            0x0d => dec_u8(&mut self.registers.c, self.registers.f, 4), // DEC C
            0x0e => ld_reg8_mem8(&mut self.registers.c, immediate_u8()), // LD C,x
            0x0f => {
                // Note, different flag manipulation to RRC.
                let flag_c;
                if self.registers.a & make_bit(0) != 0 {
                    flag_c = true;
                    self.registers.a >>= 1;
                    self.registers.a |= make_bit(7);
                } else {
                    flag_c = false;
                    self.registers.a >>= 1;
                    self.registers.a &= !make_bit(7);
                }

                CpuDiff::new(1, 4)
                    .flag_z(false)
                    .flag_n(false)
                    .flag_h(false)
                    .flag_c(flag_c)
            } // RRCA
            0x10 => todo!(), // STOP 0
            0x11 => {
                self.registers.set_de(immediate_u16());
                CpuDiff::new(3, 12)
            } // LD DE,xx
            0x12 => {
                memory.write(self.registers.de(), self.registers.a);
                CpuDiff::new(1, 8)
            } // LD (DE),A
            0x13 => {
                self.registers.set_de(self.registers.de().wrapping_add(1));
                CpuDiff::new(1, 8)
            } // INC DE
            0x14 => inc_u8(&mut self.registers.d, self.registers.f, 4), // INC D
            0x15 => dec_u8(&mut self.registers.d, self.registers.f, 4), // DEC D
            0x16 => ld_reg8_mem8(&mut self.registers.d, immediate_u8()), // LD D,x
            0x17 => {
                let previous_carry = self.registers.f & Registers::FLAG_CARRY != 0;
                let bit_7 = self.registers.a & make_bit(7) != 0;

                self.registers.a = self.registers.a << 1;

                if previous_carry {
                    self.registers.a |= make_bit(0);
                }

                CpuDiff::new(1, 4)
                    .flag_z(false)
                    .flag_n(false)
                    .flag_h(false)
                    .flag_c(bit_7)
            } // RLA
            0x18 => CpuDiff::new(2 + i16::from(immediate_u8() as i8), 12), // JR s
            0x19 => {
                let mut hl = self.registers.hl();
                let diff = add_reg16(self.registers.de(), &mut hl);
                self.registers.set_hl(hl);
                diff
            } // ADD HL,DE
            0x1a => {
                self.registers.a = memory.read(self.registers.de());
                CpuDiff::new(1, 8)
            } // LD A,(DE)
            0x1b => {
                self.registers.set_de(self.registers.de().wrapping_sub(1));
                CpuDiff::new(1, 8)
            } // DEC DE
            0x1c => inc_u8(&mut self.registers.e, self.registers.f, 4), // INC E
            0x1d => dec_u8(&mut self.registers.e, self.registers.f, 4), // DEC E
            0x1e => ld_reg8_mem8(&mut self.registers.e, immediate_u8()), // LD E,x
            0x1f => {
                let previous_carry = self.registers.f & Registers::FLAG_CARRY != 0;
                let new_carry = self.registers.a & make_bit(0) != 0;
                self.registers.a = self.registers.a >> 1;

                if previous_carry {
                    self.registers.a |= make_bit(7);
                }

                CpuDiff::new(1, 4)
                    .flag_z(false)
                    .flag_n(false)
                    .flag_h(false)
                    .flag_c(new_carry)
            } // RRA
            0x20 => {
                if self.registers.f & Registers::FLAG_ZERO == 0 {
                    CpuDiff::new(2 + i16::from(immediate_u8() as i8), 12)
                } else {
                    CpuDiff::new(2, 8)
                }
            } // JR NZ,s
            0x21 => {
                self.registers.set_hl(immediate_u16());
                CpuDiff::new(3, 12)
            } // LD HL,xx
            0x22 => {
                memory.write(self.registers.hl(), self.registers.a);
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                CpuDiff::new(1, 8)
            } // LD (HL+),A
            0x23 => {
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                CpuDiff::new(1, 8)
            } // INC HL
            0x24 => inc_u8(&mut self.registers.h, self.registers.f, 4), // INC H
            0x25 => dec_u8(&mut self.registers.h, self.registers.f, 4), // DEC H
            0x26 => ld_reg8_mem8(&mut self.registers.h, immediate_u8()), // LD H,x
            0x27 => {
                let mut new_a: u16 = self.registers.a.into();

                if self.registers.f & Registers::FLAG_SUBTRACTION == 0 {
                    if (self.registers.f & Registers::FLAG_HALFCARRY != 0) || (new_a & 0x0f) > 0x09
                    {
                        new_a += 0x06;
                    }
                    if (self.registers.f & Registers::FLAG_CARRY != 0) || new_a > 0x9f {
                        new_a += 0x60;
                    }
                } else {
                    if self.registers.f & Registers::FLAG_HALFCARRY != 0 {
                        new_a = new_a.wrapping_sub(0x06);
                        if self.registers.f & Registers::FLAG_CARRY == 0 {
                            new_a &= 0xff;
                        }
                    }

                    if self.registers.f & Registers::FLAG_CARRY != 0 {
                        new_a = new_a.wrapping_sub(0x60);
                    }
                }

                self.registers.a = new_a as u8;

                CpuDiff::new(1, 4)
                    .flag_z(self.registers.a == 0)
                    .flag_h(false)
                    .flag_c(new_a & 0x100 != 0)
            } // DAA
            0x28 => {
                if self.registers.f & Registers::FLAG_ZERO != 0 {
                    let imm = immediate_u8() as i8;
                    CpuDiff::new((2 + imm).into(), 12) // rwtodo handle wraparound here.
                } else {
                    CpuDiff::new(2, 8)
                }
            } // JR Z,s
            0x29 => {
                let mut hl = self.registers.hl();
                let diff = add_reg16(self.registers.hl(), &mut hl);
                self.registers.set_hl(hl);
                diff
            } // ADD HL,HL
            0x2a => {
                self.registers.a = memory.read(self.registers.hl());
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                CpuDiff::new(1, 8)
            } // LD A,(HL+)
            0x2b => {
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                CpuDiff::new(1, 8)
            } // DEC HL
            0x2c => inc_u8(&mut self.registers.l, self.registers.f, 4), // INC L
            0x2d => dec_u8(&mut self.registers.l, self.registers.f, 4), // DEC L
            0x2e => ld_reg8_mem8(&mut self.registers.l, immediate_u8()), // LD L,x
            0x2f => {
                self.registers.a ^= 0xff;
                CpuDiff::new(1, 4).flag_n(true).flag_h(true)
            } // CPL
            0x30 => {
                if self.registers.f & Registers::FLAG_CARRY == 0 {
                    CpuDiff::new(2 + i16::from(immediate_u8() as i8), 12)
                } else {
                    CpuDiff::new(2, 8)
                }
            } // JR NC,s
            0x31 => {
                self.registers.sp = immediate_u16();
                CpuDiff::new(3, 12)
            } // LD SP,xx
            0x32 => {
                memory.write(self.registers.hl(), self.registers.a);
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                CpuDiff::new(1, 8)
            } // LD (HL-),A
            0x33 => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
                CpuDiff::new(1, 8)
            } // INC SP
            0x34 => {
                let mut value_at_hl = memory.read(self.registers.hl());
                let diff = inc_u8(&mut value_at_hl, self.registers.f, 12);
                memory.write(self.registers.hl(), value_at_hl);
                diff
            } // INC (HL)
            0x35 => {
                let mut value_at_hl = memory.read(self.registers.hl());
                let diff = dec_u8(&mut value_at_hl, self.registers.f, 12);
                memory.write(self.registers.hl(), value_at_hl);
                diff
            } // DEC (HL)
            0x36 => {
                memory.write(self.registers.hl(), immediate_u8());
                CpuDiff::new(2, 12)
            } // LD (HL),x
            0x37 => CpuDiff::new(1, 4).flag_n(false).flag_h(false).flag_c(true), // SCF
            0x38 => {
                if self.registers.f & Registers::FLAG_CARRY != 0 {
                    CpuDiff::new(2 + i16::from(immediate_u8() as i8), 12)
                } else {
                    CpuDiff::new(2, 8)
                }
            } // JR C,s
            0x39 => {
                let mut hl = self.registers.hl();
                let diff = add_reg16(self.registers.sp, &mut hl);
                self.registers.set_hl(hl);
                diff
            } // ADD HL,SP
            0x3a => {
                self.registers.a = memory.read(self.registers.hl());
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                CpuDiff::new(1, 8)
            } // LD A,(HL-)
            0x3b => {
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                CpuDiff::new(1, 8)
            } // DEC SP
            0x3c => inc_u8(&mut self.registers.a, self.registers.f, 4), // INC A
            0x3d => dec_u8(&mut self.registers.a, self.registers.f, 4), // DEC A
            0x3e => ld_reg8_mem8(&mut self.registers.a, immediate_u8()), // LD A,x
            0x3f => CpuDiff::new(1, 4)
                .flag_n(false)
                .flag_h(false)
                .flag_c(self.registers.f & Registers::FLAG_CARRY == 0), // CCF
            0x40 => nop(), // LD B,B
            0x41 => ld_reg8_reg8(&mut self.registers.b, self.registers.c), // LD B,C
            0x42 => ld_reg8_reg8(&mut self.registers.b, self.registers.d), // LD B,D
            0x43 => ld_reg8_reg8(&mut self.registers.b, self.registers.e), // LD B,E
            0x44 => ld_reg8_reg8(&mut self.registers.b, self.registers.h), // LD B,H
            0x45 => ld_reg8_reg8(&mut self.registers.b, self.registers.l), // LD B,L
            0x46 => {
                self.registers.b = memory.read(self.registers.hl());
                CpuDiff::new(1, 8)
            } // LD B,(HL)
            0x47 => ld_reg8_reg8(&mut self.registers.b, self.registers.a), // LD B,A
            0x48 => ld_reg8_reg8(&mut self.registers.c, self.registers.b), // LD C,B
            0x49 => nop(), // LD C,C
            0x4a => ld_reg8_reg8(&mut self.registers.c, self.registers.d), // LD C,D
            0x4b => ld_reg8_reg8(&mut self.registers.c, self.registers.e), // LD C,E
            0x4c => ld_reg8_reg8(&mut self.registers.c, self.registers.h), // LD C,H
            0x4d => ld_reg8_reg8(&mut self.registers.c, self.registers.l), // LD C,L
            0x4e => {
                self.registers.c = memory.read(self.registers.hl());
                CpuDiff::new(1, 8)
            } // LD C,(HL)
            0x4f => ld_reg8_reg8(&mut self.registers.c, self.registers.a), // LD C,A
            0x50 => ld_reg8_reg8(&mut self.registers.d, self.registers.b), // LD D,B
            0x51 => ld_reg8_reg8(&mut self.registers.d, self.registers.c), // LD D,C
            0x52 => nop(), // LD D,D
            0x53 => ld_reg8_reg8(&mut self.registers.d, self.registers.e), // LD D,E
            0x54 => ld_reg8_reg8(&mut self.registers.d, self.registers.h), // LD D,H
            0x55 => ld_reg8_reg8(&mut self.registers.d, self.registers.l), // LD D,L
            0x56 => {
                self.registers.d = memory.read(self.registers.hl());
                CpuDiff::new(1, 8)
            } // LD D,(HL)
            0x57 => ld_reg8_reg8(&mut self.registers.d, self.registers.a), // LD D,A
            0x58 => ld_reg8_reg8(&mut self.registers.e, self.registers.b), // LD E,B
            0x59 => ld_reg8_reg8(&mut self.registers.e, self.registers.c), // LD E,C
            0x5a => ld_reg8_reg8(&mut self.registers.e, self.registers.d), // LD E,D
            0x5b => nop(), // LD E,E
            0x5c => ld_reg8_reg8(&mut self.registers.e, self.registers.h), // LD E,H
            0x5d => ld_reg8_reg8(&mut self.registers.e, self.registers.l), // LD E,L
            0x5e => {
                self.registers.e = memory.read(self.registers.hl());
                CpuDiff::new(1, 8)
            } // LD E,(HL)
            0x5f => ld_reg8_reg8(&mut self.registers.e, self.registers.a), // LD E,A
            0x60 => ld_reg8_reg8(&mut self.registers.h, self.registers.b), // LD H,B
            0x61 => ld_reg8_reg8(&mut self.registers.h, self.registers.c), // LD H,C
            0x62 => ld_reg8_reg8(&mut self.registers.h, self.registers.d), // LD H,D
            0x63 => ld_reg8_reg8(&mut self.registers.h, self.registers.e), // LD H,E
            0x64 => nop(), // LD H,H
            0x65 => ld_reg8_reg8(&mut self.registers.h, self.registers.l), // LD H,L
            0x66 => {
                self.registers.h = memory.read(self.registers.hl());
                CpuDiff::new(1, 8)
            } // LD H,(HL)
            0x67 => ld_reg8_reg8(&mut self.registers.h, self.registers.a), // LD H,A
            0x68 => ld_reg8_reg8(&mut self.registers.l, self.registers.b), // LD L,B
            0x69 => ld_reg8_reg8(&mut self.registers.l, self.registers.c), // LD L,C
            0x6a => ld_reg8_reg8(&mut self.registers.l, self.registers.d), // LD L,D
            0x6b => ld_reg8_reg8(&mut self.registers.l, self.registers.e), // LD L,E
            0x6c => ld_reg8_reg8(&mut self.registers.l, self.registers.h), // LD L,H
            0x6d => nop(), // LD L,L
            0x6e => {
                self.registers.l = memory.read(self.registers.hl());
                CpuDiff::new(1, 8)
            } // LD L,(HL)
            0x6f => ld_reg8_reg8(&mut self.registers.l, self.registers.a), // LD L,A
            0x70 => {
                memory.write(self.registers.hl(), self.registers.b);
                CpuDiff::new(1, 8)
            } // LD (HL),B
            0x71 => {
                memory.write(self.registers.hl(), self.registers.c);
                CpuDiff::new(1, 8)
            } // LD (HL),C
            0x72 => {
                memory.write(self.registers.hl(), self.registers.d);
                CpuDiff::new(1, 8)
            } // LD (HL),D
            0x73 => {
                memory.write(self.registers.hl(), self.registers.e);
                CpuDiff::new(1, 8)
            } // LD (HL),E
            0x74 => {
                memory.write(self.registers.hl(), self.registers.h);
                CpuDiff::new(1, 8)
            } // LD (HL),H
            0x75 => {
                memory.write(self.registers.hl(), self.registers.l);
                CpuDiff::new(1, 8)
            } // LD (HL),L
            0x76 => {
                assert!(!self.is_halted); // Instructions shouldn't be getting executed while halted.

                if self.registers.ime {
                    self.is_halted = true;
                }

                CpuDiff::new(1, 4)
            } // HALT
            0x77 => {
                memory.write(self.registers.hl(), self.registers.a);
                CpuDiff::new(1, 8)
            } // LD (HL),A
            0x78 => {
                self.registers.a = self.registers.b;
                CpuDiff::new(1, 4)
            } // LD A,B
            0x79 => {
                self.registers.a = self.registers.c;
                CpuDiff::new(1, 4)
            } // LD A,C
            0x7a => {
                self.registers.a = self.registers.d;
                CpuDiff::new(1, 4)
            } // LD A,D
            0x7b => {
                self.registers.a = self.registers.e;
                CpuDiff::new(1, 4)
            } // LD A,E
            0x7c => {
                self.registers.a = self.registers.h;
                CpuDiff::new(1, 4)
            } // LD A,H
            0x7d => {
                self.registers.a = self.registers.l;
                CpuDiff::new(1, 4)
            } // LD A,L
            0x7e => {
                self.registers.a = memory.read(self.registers.hl());
                CpuDiff::new(1, 8)
            } // LD A,(HL)
            0x7f => {
                self.registers.a = self.registers.a;
                CpuDiff::new(1, 4)
            } // LD A,A
            0x80 => add_u8(
                self.registers.b,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADD A,B
            0x81 => add_u8(
                self.registers.c,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADD A,C
            0x82 => add_u8(
                self.registers.d,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADD A,D
            0x83 => add_u8(
                self.registers.e,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADD A,E
            0x84 => add_u8(
                self.registers.h,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADD A,H
            0x85 => add_u8(
                self.registers.l,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADD A,L
            0x86 => add_u8(
                memory.read(self.registers.hl()),
                &mut self.registers.a,
                self.registers.f,
                1,
                8,
            ), // ADD A,(HL)
            0x87 => add_u8(
                self.registers.a,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADD A,A
            0x88 => adc(
                self.registers.b,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADC A,B
            0x89 => adc(
                self.registers.c,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADC A,C
            0x8a => adc(
                self.registers.d,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADC A,D
            0x8b => adc(
                self.registers.e,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADC A,E
            0x8c => adc(
                self.registers.h,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADC A,H
            0x8d => adc(
                self.registers.l,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADC A,L
            0x8e => adc(
                memory.read(self.registers.hl()),
                &mut self.registers.a,
                self.registers.f,
                1,
                8,
            ), // ADC A,(HL)
            0x8f => adc(
                self.registers.a,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // ADC A,A
            0x98 => sbc(self.registers.b, &mut self.registers, 1, 4), // SBC A,B
            0x99 => sbc(self.registers.c, &mut self.registers, 1, 4), // SBC A,C
            0x9a => sbc(self.registers.d, &mut self.registers, 1, 4), // SBC A,D
            0x9b => sbc(self.registers.e, &mut self.registers, 1, 4), // SBC A,E
            0x9c => sbc(self.registers.h, &mut self.registers, 1, 4), // SBC A,H
            0x9d => sbc(self.registers.l, &mut self.registers, 1, 4), // SBC A,L
            0x9e => sbc(memory.read(self.registers.hl()), &mut self.registers, 1, 8), // SBC A,(HL)
            0x9f => sbc(self.registers.a, &mut self.registers, 1, 4), // SBC A,A
            0x90 => sub(
                self.registers.b,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // SUB B
            0x91 => sub(
                self.registers.c,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // SUB C
            0x92 => sub(
                self.registers.d,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // SUB D
            0x93 => sub(
                self.registers.e,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // SUB E
            0x94 => sub(
                self.registers.h,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // SUB H
            0x95 => sub(
                self.registers.l,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // SUB L
            0x96 => sub(
                memory.read(self.registers.hl()),
                &mut self.registers.a,
                self.registers.f,
                1,
                8,
            ), // SUB (HL)
            0x97 => sub(
                self.registers.a,
                &mut self.registers.a,
                self.registers.f,
                1,
                4,
            ), // SUB A
            0xa0 => and(self.registers.b, &mut self.registers.a, 1, 4), // AND B
            0xa1 => and(self.registers.c, &mut self.registers.a, 1, 4), // AND C
            0xa2 => and(self.registers.d, &mut self.registers.a, 1, 4), // AND D
            0xa3 => and(self.registers.e, &mut self.registers.a, 1, 4), // AND E
            0xa4 => and(self.registers.h, &mut self.registers.a, 1, 4), // AND H
            0xa5 => and(self.registers.l, &mut self.registers.a, 1, 4), // AND L
            0xa6 => {
                let x = memory.read(self.registers.hl());
                and(x, &mut self.registers.a, 1, 8)
            } // AND (HL)
            0xa7 => and(self.registers.a, &mut self.registers.a, 1, 4), // AND A
            0xa8 => xor(self.registers.b, &mut self.registers.a, 1, 4), // XOR B
            0xa9 => xor(self.registers.c, &mut self.registers.a, 1, 4), // XOR C
            0xaa => xor(self.registers.d, &mut self.registers.a, 1, 4), // XOR D
            0xab => xor(self.registers.e, &mut self.registers.a, 1, 4), // XOR E
            0xac => xor(self.registers.h, &mut self.registers.a, 1, 4), // XOR H
            0xad => xor(self.registers.l, &mut self.registers.a, 1, 4), // XOR L
            0xae => {
                let x = memory.read(self.registers.hl());
                xor(x, &mut self.registers.a, 1, 8)
            } // XOR (HL)
            0xaf => xor(self.registers.a, &mut self.registers.a, 1, 4), // XOR A
            0xb0 => or(self.registers.b, &mut self.registers.a, 1, 4), // OR B
            0xb1 => or(self.registers.c, &mut self.registers.a, 1, 4), // OR C
            0xb2 => or(self.registers.d, &mut self.registers.a, 1, 4), // OR D
            0xb3 => or(self.registers.e, &mut self.registers.a, 1, 4), // OR E
            0xb4 => or(self.registers.h, &mut self.registers.a, 1, 4), // OR H
            0xb5 => or(self.registers.l, &mut self.registers.a, 1, 4), // OR L
            0xb6 => {
                let x = memory.read(self.registers.hl());
                or(x, &mut self.registers.a, 1, 8)
            } // OR (HL)
            0xb7 => or(self.registers.a, &mut self.registers.a, 1, 4), // OR A
            0xb8 => cp(self.registers.b, &mut self.registers, 1, 4), // CP B // rwtodo &mut? &?
            0xb9 => cp(self.registers.c, &mut self.registers, 1, 4), // CP C // rwtodo &mut? &?
            0xba => cp(self.registers.d, &mut self.registers, 1, 4), // CP D // rwtodo &mut? &?
            0xbb => cp(self.registers.e, &mut self.registers, 1, 4), // CP E // rwtodo &mut? &?
            0xbc => cp(self.registers.h, &mut self.registers, 1, 4), // CP H // rwtodo &mut? &?
            0xbd => cp(self.registers.l, &mut self.registers, 1, 4), // CP L // rwtodo &mut? &?
            0xbe => cp(memory.read(self.registers.hl()), &mut self.registers, 1, 8), // CP (HL)
            0xbf => cp(self.registers.a, &mut self.registers, 1, 4), // RES 7,A
            0xc0 => {
                if self.registers.f & Registers::FLAG_ZERO == 0 {
                    self.registers.pc = stack_pop(&mut self.registers.sp, memory);
                    CpuDiff::new(0, 20)
                } else {
                    CpuDiff::new(1, 8)
                }
            } // RET NZ
            0xc1 => {
                let new_bc = stack_pop(&mut self.registers.sp, memory);
                self.registers.set_bc(new_bc);
                CpuDiff::new(1, 12)
            } // POP BC
            0xc2 => {
                if self.registers.f & Registers::FLAG_ZERO == 0 {
                    self.registers.pc = immediate_u16();
                    CpuDiff::new(0, 16)
                } else {
                    CpuDiff::new(3, 12)
                }
            } // JP NZ,xx
            0xc3 => {
                self.registers.pc = immediate_u16();
                CpuDiff::new(0, 16)
            } // JP xx
            0xc4 => call(
                self.registers.f & Registers::FLAG_ZERO == 0,
                &mut self.registers,
                memory,
            ), // CALL NZ,xx
            0xc5 => {
                stack_push(self.registers.bc(), &mut self.registers.sp, memory);
                CpuDiff::new(1, 16)
            } // PUSH BC
            0xc6 => add_u8(
                immediate_u8(),
                &mut self.registers.a,
                self.registers.f,
                2,
                8,
            ), // ADD A,x
            0xc7 => rst(0x00, &mut self.registers, memory), // RST 00h
            0xc8 => {
                if self.registers.f & Registers::FLAG_ZERO != 0 {
                    self.registers.pc = stack_pop(&mut self.registers.sp, memory);
                    CpuDiff::new(0, 20)
                } else {
                    CpuDiff::new(1, 8)
                }
            } // RET Z
            0xc9 => {
                self.registers.pc = stack_pop(&mut self.registers.sp, memory);
                CpuDiff::new(0, 16)
            } // RET
            0xca => {
                if self.registers.f & Registers::FLAG_ZERO != 0 {
                    self.registers.pc = immediate_u16();
                    CpuDiff::new(0, 16)
                } else {
                    CpuDiff::new(3, 12)
                }
            } // JP Z,xx
            0xcb => cb::execute_cb_instruction(&mut self.registers, memory), // 0xcb prefixed opcodes
            0xcc => call(
                self.registers.f & Registers::FLAG_ZERO != 0,
                &mut self.registers,
                memory,
            ), // CALL Z,xx
            0xcd => call(true, &mut self.registers, memory),                 // CALL xx
            0xce => adc(
                immediate_u8(),
                &mut self.registers.a,
                self.registers.f,
                2,
                8,
            ), // ADC A,x
            0xcf => rst(0x08, &mut self.registers, memory),                  // RST 08h
            0xd0 => {
                if self.registers.f & Registers::FLAG_CARRY == 0 {
                    self.registers.pc = stack_pop(&mut self.registers.sp, memory);
                    CpuDiff::new(0, 20)
                } else {
                    CpuDiff::new(1, 8)
                }
            } // RET NC
            0xd1 => {
                let popped_value = stack_pop(&mut self.registers.sp, memory);
                self.registers.set_de(popped_value);
                CpuDiff::new(1, 12)
            }
            0xd2 => {
                if self.registers.f & Registers::FLAG_CARRY == 0 {
                    self.registers.pc = immediate_u16();
                    CpuDiff::new(0, 16)
                } else {
                    CpuDiff::new(3, 12)
                }
            } // JP NC,xx
            0xd3 => unreachable!("Invalid opcode"),

            0xd4 => call(
                self.registers.f & Registers::FLAG_CARRY == 0,
                &mut self.registers,
                memory,
            ), // CALL NC,xx
            0xd5 => {
                stack_push(self.registers.de(), &mut self.registers.sp, memory);
                CpuDiff::new(1, 16)
            } // PUSH DE
            0xd6 => sub(
                immediate_u8(),
                &mut self.registers.a,
                self.registers.f,
                2,
                8,
            ), // SUB x
            0xd7 => rst(0x10, &mut self.registers, memory), // RST 10h
            0xd8 => {
                if self.registers.f & Registers::FLAG_CARRY != 0 {
                    self.registers.pc = stack_pop(&mut self.registers.sp, memory);
                    CpuDiff::new(0, 20)
                } else {
                    CpuDiff::new(1, 8)
                }
            } // RET C
            0xd9 => {
                self.registers.pc = stack_pop(&mut self.registers.sp, memory);
                self.registers.ime = true;
                CpuDiff::new(0, 16)
            } // RETI
            0xda => {
                if self.registers.f & Registers::FLAG_CARRY != 0 {
                    self.registers.pc = immediate_u16();
                    CpuDiff::new(0, 16)
                } else {
                    CpuDiff::new(3, 12)
                }
            } // JP C,xx
            0xdb => unreachable!("Invalid opcode"),
            0xdc => call(
                self.registers.f & Registers::FLAG_CARRY != 0,
                &mut self.registers,
                memory,
            ), // CALL C,xx
            0xdd => unreachable!("Invalid opcode"),
            0xde => sbc(immediate_u8(), &mut self.registers, 2, 8), // SBC A,x
            0xdf => rst(0x18, &mut self.registers, memory),         // RST 18h
            0xe0 => {
                memory.write(0xff00 + u16::from(immediate_u8()), self.registers.a);
                CpuDiff::new(2, 12)
            } // LDH (ff00+x),A
            0xe1 => {
                let popped_value = stack_pop(&mut self.registers.sp, memory);
                self.registers.set_hl(popped_value);
                CpuDiff::new(1, 12)
            } // POP HL
            0xe2 => {
                memory.write(0xff00 + u16::from(self.registers.c), self.registers.a);
                CpuDiff::new(1, 8)
            } // LD (ff00+C),A
            0xe3 => unreachable!("Invalid opcode"),
            0xe4 => unreachable!("Invalid opcode"),
            0xe5 => {
                stack_push(self.registers.hl(), &mut self.registers.sp, memory);
                CpuDiff::new(1, 16)
            } // PUSH HL
            0xe6 => and(immediate_u8(), &mut self.registers.a, 2, 8), // AND x
            0xe7 => rst(0x20, &mut self.registers, memory),           // RST 20H
            0xe8 => {
                // rwtodo: This is likely wrong.
                let imm: i32 = (immediate_u8() as i8).into();

                // rwtodo: Investigate what happens with this double XOR.
                let sp32: i32 = self.registers.sp.into();
                let xor_result = sp32 ^ imm ^ (sp32 + imm);

                self.registers.sp = self
                    .registers
                    .sp
                    .wrapping_add_signed(imm.try_into().unwrap());

                CpuDiff::new(2, 16)
                    .flag_z(false)
                    .flag_n(false)
                    .flag_h(xor_result & 0x10 != 0)
                    .flag_c(xor_result & 0x100 != 0)
            } // ADD SP,s
            0xe9 => {
                self.registers.pc = self.registers.hl();
                CpuDiff::new(0, 4)
            } // JP (HL)
            0xea => {
                memory.write(immediate_u16(), self.registers.a);
                CpuDiff::new(3, 16)
            } // LD (x),A
            0xeb..=0xed => unreachable!("Invalid opcode"),
            0xee => xor(immediate_u8(), &mut self.registers.a, 2, 4), // XOR x
            0xef => rst(0x28, &mut self.registers, memory),           // RST 28H
            0xf0 => {
                self.registers.a = memory.read(0xff00 + u16::from(immediate_u8()));
                CpuDiff::new(2, 12)
            } // LDH A,(0xff00+x)
            0xf1 => {
                let new_af = stack_pop(&mut self.registers.sp, memory) & 0xfff0; // Lower nybble of F must stay 0.
                self.registers.set_af(new_af);
                CpuDiff::new(1, 12)
            } // POP AF
            0xf2 => {
                self.registers.a = memory.read(0xff00 + u16::from(self.registers.c));
                CpuDiff::new(1, 8)
            } // LD A,(ff00+C)
            0xf3 => {
                self.registers.ime = false;
                CpuDiff::new(1, 4)
            } // DI
            0xf4 => unreachable!("Invalid opcode"),
            0xf5 => {
                stack_push(self.registers.af(), &mut self.registers.sp, memory);
                CpuDiff::new(1, 16)
            } // PUSH AF
            0xf6 => or(immediate_u8(), &mut self.registers.a, 2, 8), // OR x
            0xf7 => rst(0x30, &mut self.registers, memory),          // RST 30H
            0xf8 => {
                // rwtodo: This is likely wrong.
                let imm: i32 = (immediate_u8() as i8).into();

                // rwtodo: Investigate what happens with this double XOR.
                let sp32: i32 = self.registers.sp.into();
                let xor_result = sp32 ^ imm ^ (sp32 + imm);

                self.registers.set_hl((sp32 + imm) as u16);

                CpuDiff::new(2, 12)
                    .flag_z(false)
                    .flag_n(false)
                    .flag_h(xor_result & 0x10 != 0)
                    .flag_c(xor_result & 0x100 != 0)
            } // LDHL SP,s
            0xf9 => {
                self.registers.sp = self.registers.hl();
                CpuDiff::new(1, 8)
            } // LD SP,HL
            0xfa => {
                let address = u16::from(immediate_u16());
                self.registers.a = memory.read(address);
                CpuDiff::new(3, 16)
            } // LD A,(xx)
            0xfb => {
                self.registers.ime = true;
                CpuDiff::new(1, 4)
            } // IE
            0xfc => unreachable!("Invalid opcode"),
            0xfd => unreachable!("Invalid opcode"),
            0xfe => {
                let imm = immediate_u8();

                let half_carry =
                    subtraction_produces_half_carry(self.registers.a, imm, self.registers.f, false);
                let full_carry = subtraction_produces_full_carry(self.registers.a, imm);

                let sub_result = self.registers.a.wrapping_sub(imm);

                CpuDiff::new(2, 8)
                    .flag_z(sub_result == 0)
                    .flag_n(true)
                    .flag_h(half_carry)
                    .flag_c(full_carry)
            } // CP x
            0xff => rst(0x38, &mut self.registers, memory), // RST 38H
        };

        self.registers.pc = self.registers.pc.wrapping_add_signed(diff.pc_delta);
        self.registers.update_flags(diff.flag_diff);
        diff.cycles
    }
}
