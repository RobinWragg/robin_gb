use crate::address;
use crate::interrupt;
use crate::Memory;
use crate::{bit, make_u16};

mod instructions;

//rwtodo: I can probably do something nifty with Rust attributes to make the "instruction" functions more ergonomic.

// rwtodo: Apparently STOP is like HALT except the LCD is inoperational as well, and the "stopped" state is only exited when a button is pressed. Look for better documentation on it.

fn stack_push(value_to_push: u16, sp: &mut u16, memory: &mut Memory) {
    let bytes = value_to_push.to_le_bytes();
    *sp -= 2;

    // rwtodo: this can be memory.write_u16, right?
    memory.write(*sp, bytes[0]);
    memory.write(*sp + 1, bytes[1]);
}

fn stack_pop(sp: &mut u16, memory: &Memory) -> u16 {
    let popped_value = memory.read_u16(*sp);
    *sp += 2;
    popped_value
}

type CycleCount = u8;

pub struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    pc: u16,
    sp: u16,
    ime: bool,
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
        make_u16(self.c, self.d)
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
        let elapsed_cycles = self.execute_next_instruction_inner(memory);
        self.handle_interrupt_requests(memory);
        elapsed_cycles
    }

    #[must_use] // Returns the number of cycles the instruction took.
    fn execute_next_instruction_inner(&mut self, memory: &mut Memory) -> u8 {
        if self.is_halted {
            return 4;
        }

        // print_instruction(self.registers.pc, memory); rwtodo

        let opcode = memory.read(self.registers.pc);

        use instructions::*;

        let finish: Finish = match opcode {
            0x00 => nop(), // NOP
            0x01 => {
                self.registers
                    .set_bc(memory.read_u16(self.registers.pc + 1));
                Finish {
                    pc_increment: 3,
                    elapsed_cycles: 12,
                }
            } // LD BC,xx
            0x02 => {
                memory.write(self.registers.bc(), self.registers.a);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (BC),A
            0x03 => {
                self.registers.set_bc(self.registers.bc() + 1);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // INC BC
            0x04 => inc_u8(&mut self.registers.b, &mut self.registers.f, 4), // INC B
            0x05 => dec_reg8(&mut self.registers.b, &mut self.registers.f), // DEC B
            0x06 => ld_reg8_mem8(&mut self.registers.b, memory.read(self.registers.pc + 1)), // LD B,x
            0x07 => {
                if self.registers.a & bit(7) != 0 {
                    self.registers.f |= Registers::FLAG_CARRY;
                    self.registers.a <<= 1;
                    self.registers.a |= bit(0);
                } else {
                    self.registers.f &= !Registers::FLAG_CARRY;
                    self.registers.a <<= 1;
                    self.registers.a &= !bit(0);
                }

                self.registers.f &= !Registers::FLAG_ZERO;
                self.registers.f &= !Registers::FLAG_SUBTRACTION;
                self.registers.f &= !Registers::FLAG_HALFCARRY;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // RLCA
            0x08 => {
                memory.write_u16(memory.read_u16(self.registers.pc + 1), self.registers.sp);
                Finish {
                    pc_increment: 3,
                    elapsed_cycles: 20,
                }
            } // LD (xx),SP
            0x09 => {
                let mut hl = self.registers.hl();
                let finish = add_reg16(self.registers.bc(), &mut hl, &mut self.registers.f);
                self.registers.set_hl(hl);
                finish
            } // ADD HL,BC
            0x0a => {
                self.registers.a = memory.read(self.registers.bc());
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD A,(BC)
            0x0b => {
                let bc = self.registers.bc().wrapping_sub(1);
                self.registers.set_bc(bc);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // DEC BC
            0x0c => inc_u8(&mut self.registers.c, &mut self.registers.f, 4), // INC C
            0x0d => dec_reg8(&mut self.registers.c, &mut self.registers.f),  // DEC C
            0x0e => ld_reg8_mem8(&mut self.registers.c, memory.read(self.registers.pc + 1)), // LD C,x
            0x11 => {
                self.registers
                    .set_de(memory.read_u16(self.registers.pc + 1));
                Finish {
                    pc_increment: 3,
                    elapsed_cycles: 12,
                }
            } // LD DE,xx
            0x15 => dec_reg8(&mut self.registers.d, &mut self.registers.f), // DEC D
            0x16 => ld_reg8_mem8(&mut self.registers.d, memory.read(self.registers.pc + 1)), // LD D,x
            0x1d => dec_reg8(&mut self.registers.e, &mut self.registers.f), // DEC E
            0x1e => ld_reg8_mem8(&mut self.registers.e, memory.read(self.registers.pc + 1)), // LD E,x
            0x20 => {
                if (self.registers.f & Registers::FLAG_ZERO) != 0 {
                    Finish {
                        pc_increment: 2,
                        elapsed_cycles: 8,
                    }
                } else {
                    Finish {
                        pc_increment: 2 + i16::from(memory.read(self.registers.pc + 1) as i8),
                        elapsed_cycles: 12,
                    }
                }
            } // JR NZ,s
            0x21 => {
                self.registers
                    .set_hl(memory.read_u16(self.registers.pc + 1));
                Finish {
                    pc_increment: 3,
                    elapsed_cycles: 12,
                }
            } // LD HL,xx
            0x25 => dec_reg8(&mut self.registers.h, &mut self.registers.f), // DEC H
            0x26 => ld_reg8_mem8(&mut self.registers.h, memory.read(self.registers.pc + 1)), // LD H,x
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
                        new_a -= 0x06;
                        if self.registers.f & Registers::FLAG_CARRY == 0 {
                            new_a &= 0xff;
                        }
                    }

                    if self.registers.f & Registers::FLAG_CARRY != 0 {
                        new_a -= 0x60;
                    }
                }

                self.registers.a = new_a as u8;

                self.registers.f &= !(Registers::FLAG_HALFCARRY | Registers::FLAG_ZERO);
                if new_a & 0x100 != 0 {
                    self.registers.f |= Registers::FLAG_CARRY;
                }
                if self.registers.a == 0 {
                    self.registers.f |= Registers::FLAG_ZERO;
                }

                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // DAA
            0x28 => {
                if self.registers.f & Registers::FLAG_ZERO != 0 {
                    let x = memory.read(self.registers.pc + 1) as i8;
                    Finish {
                        pc_increment: (2 + x).into(),
                        elapsed_cycles: 12,
                    }
                } else {
                    Finish {
                        pc_increment: 2,
                        elapsed_cycles: 8,
                    }
                }
            } // JR Z,s
            0x29 => {
                let mut hl = self.registers.hl();
                let finish = add_reg16(self.registers.hl(), &mut hl, &mut self.registers.f);
                self.registers.set_hl(hl);
                finish
            } // ADD HL,HL
            0x2a => {
                self.registers.a = memory.read(self.registers.hl());
                self.registers.set_hl(self.registers.hl() + 1);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD A,(HL+)
            0x2c => inc_u8(&mut self.registers.l, &mut self.registers.f, 4), // INC L
            0x2d => dec_reg8(&mut self.registers.l, &mut self.registers.f),  // DEC L
            0x2e => ld_reg8_mem8(&mut self.registers.l, memory.read(self.registers.pc + 1)), // LD L,x
            0x2f => {
                self.registers.a ^= 0xff;
                self.registers.f |= Registers::FLAG_SUBTRACTION;
                self.registers.f |= Registers::FLAG_HALFCARRY;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // CPL
            0x30 => {
                if (self.registers.f & Registers::FLAG_CARRY) == 0 {
                    Finish {
                        pc_increment: 2 + i16::from(memory.read(self.registers.pc + 1) as i8),
                        elapsed_cycles: 12,
                    }
                } else {
                    Finish {
                        pc_increment: 2,
                        elapsed_cycles: 8,
                    }
                }
            } // JR NC,s
            0x31 => {
                self.registers.sp = memory.read_u16(self.registers.pc + 1);
                Finish {
                    pc_increment: 3,
                    elapsed_cycles: 12,
                }
            } // LD SP,xx
            0x32 => {
                memory.write(self.registers.hl(), self.registers.a);
                self.registers.set_hl(self.registers.hl() - 1);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (HL-),A
            0x36 => {
                memory.write(self.registers.hl(), memory.read(self.registers.pc + 1));
                Finish {
                    pc_increment: 2,
                    elapsed_cycles: 12,
                }
            } // LD (HL),x
            0x3d => dec_reg8(&mut self.registers.a, &mut self.registers.f), // DEC A
            0x3e => ld_reg8_mem8(&mut self.registers.a, memory.read(self.registers.pc + 1)), // LD A,x
            0x53 => ld_reg8_reg8(&mut self.registers.d, self.registers.e), // LD D,E
            0x40 => nop(),                                                 // LD B,B
            0x41 => ld_reg8_reg8(&mut self.registers.b, self.registers.c), // LD B,C
            0x42 => ld_reg8_reg8(&mut self.registers.b, self.registers.d), // LD B,D
            0x43 => ld_reg8_reg8(&mut self.registers.b, self.registers.e), // LD B,E
            0x44 => ld_reg8_reg8(&mut self.registers.b, self.registers.h), // LD B,H
            0x45 => ld_reg8_reg8(&mut self.registers.b, self.registers.l), // LD B,L
            0x47 => ld_reg8_reg8(&mut self.registers.b, self.registers.a), // LD B,A
            0x48 => ld_reg8_reg8(&mut self.registers.c, self.registers.b), // LD C,B
            0x49 => nop(),                                                 // LD C,C
            0x4a => ld_reg8_reg8(&mut self.registers.c, self.registers.d), // LD C,D
            0x4b => ld_reg8_reg8(&mut self.registers.c, self.registers.e), // LD C,E
            0x4c => ld_reg8_reg8(&mut self.registers.c, self.registers.h), // LD C,H
            0x4d => ld_reg8_reg8(&mut self.registers.c, self.registers.l), // LD C,L
            0x4f => ld_reg8_reg8(&mut self.registers.c, self.registers.a), // LD C,A
            0x50 => ld_reg8_reg8(&mut self.registers.d, self.registers.b), // LD D,B
            0x51 => ld_reg8_reg8(&mut self.registers.d, self.registers.c), // LD D,C
            0x52 => nop(),                                                 // LD D,D
            0x54 => ld_reg8_reg8(&mut self.registers.d, self.registers.h), // LD D,H
            0x55 => ld_reg8_reg8(&mut self.registers.d, self.registers.l), // LD D,L
            0x57 => ld_reg8_reg8(&mut self.registers.d, self.registers.a), // LD D,A
            0x58 => ld_reg8_reg8(&mut self.registers.e, self.registers.b), // LD E,B
            0x59 => ld_reg8_reg8(&mut self.registers.e, self.registers.c), // LD E,C
            0x5a => ld_reg8_reg8(&mut self.registers.e, self.registers.d), // LD E,D
            0x5b => nop(),                                                 // LD E,E
            0x5c => ld_reg8_reg8(&mut self.registers.e, self.registers.h), // LD E,H
            0x5d => ld_reg8_reg8(&mut self.registers.e, self.registers.l), // LD E,L
            0x5f => ld_reg8_reg8(&mut self.registers.e, self.registers.a), // LD E,A
            0x60 => ld_reg8_reg8(&mut self.registers.h, self.registers.b), // LD H,B
            0x61 => ld_reg8_reg8(&mut self.registers.h, self.registers.c), // LD H,C
            0x62 => ld_reg8_reg8(&mut self.registers.h, self.registers.d), // LD H,D
            0x63 => ld_reg8_reg8(&mut self.registers.h, self.registers.e), // LD H,E
            0x64 => nop(),                                                 // LD H,H
            0x65 => ld_reg8_reg8(&mut self.registers.h, self.registers.l), // LD H,L
            0x67 => ld_reg8_reg8(&mut self.registers.h, self.registers.a), // LD H,A
            0x68 => ld_reg8_reg8(&mut self.registers.l, self.registers.b), // LD L,B
            0x69 => ld_reg8_reg8(&mut self.registers.l, self.registers.c), // LD L,C
            0x6a => ld_reg8_reg8(&mut self.registers.l, self.registers.d), // LD L,D
            0x6b => ld_reg8_reg8(&mut self.registers.l, self.registers.e), // LD L,E
            0x6c => ld_reg8_reg8(&mut self.registers.l, self.registers.h), // LD L,H
            0x6d => nop(),                                                 // LD L,L
            0x6f => ld_reg8_reg8(&mut self.registers.l, self.registers.a), // LD L,A
            0x70 => {
                memory.write(self.registers.hl(), self.registers.b);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (HL),B
            0x71 => {
                memory.write(self.registers.hl(), self.registers.c);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (HL),C
            0x72 => {
                memory.write(self.registers.hl(), self.registers.d);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (HL),D
            0x73 => {
                memory.write(self.registers.hl(), self.registers.e);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (HL),E
            0x74 => {
                memory.write(self.registers.hl(), self.registers.h);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (HL),H
            0x75 => {
                memory.write(self.registers.hl(), self.registers.l);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (HL),L
            0x77 => {
                memory.write(self.registers.hl(), self.registers.a);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (HL),A
            0x78 => {
                self.registers.a = self.registers.b;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // LD A,B
            0x79 => {
                self.registers.a = self.registers.c;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // LD A,C
            0x7a => {
                self.registers.a = self.registers.d;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // LD A,D
            0x7b => {
                self.registers.a = self.registers.e;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // LD A,E
            0x7c => {
                self.registers.a = self.registers.h;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // LD A,H
            0x7d => {
                self.registers.a = self.registers.l;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // LD A,L
            0x7e => {
                self.registers.a = memory.read(self.registers.hl());
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD A,(HL)
            0x7f => {
                self.registers.a = self.registers.a;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // LD A,A
            0x80 => add_u8(self.registers.b, &mut self.registers, 1, 4),   // ADD A,B
            0x81 => add_u8(self.registers.c, &mut self.registers, 1, 4),   // ADD A,C
            0x82 => add_u8(self.registers.d, &mut self.registers, 1, 4),   // ADD A,D
            0x83 => add_u8(self.registers.e, &mut self.registers, 1, 4),   // ADD A,E
            0x84 => add_u8(self.registers.h, &mut self.registers, 1, 4),   // ADD A,H
            0x85 => add_u8(self.registers.l, &mut self.registers, 1, 4),   // ADD A,L
            0x86 => add_u8(memory.read(self.registers.hl()), &mut self.registers, 1, 8), // ADD A,(HL)
            0x87 => add_u8(self.registers.a, &mut self.registers, 1, 4),                 // ADD A,A
            0x88 => adc(self.registers.b, &mut self.registers, 1, 4),                    // ADC A,B
            0x89 => adc(self.registers.c, &mut self.registers, 1, 4),                    // ADC A,C
            0x8a => adc(self.registers.d, &mut self.registers, 1, 4),                    // ADC A,D
            0x8b => adc(self.registers.e, &mut self.registers, 1, 4),                    // ADC A,E
            0x8c => adc(self.registers.h, &mut self.registers, 1, 4),                    // ADC A,H
            0x8d => adc(self.registers.l, &mut self.registers, 1, 4),                    // ADC A,L
            0x8e => adc(memory.read(self.registers.hl()), &mut self.registers, 1, 8), // ADC A,(HL)
            0x8f => adc(self.registers.a, &mut self.registers, 1, 4),                 // ADC A,A
            0x90 => sub(self.registers.b, &mut self.registers, 1, 4),                 // SUB B
            0x91 => sub(self.registers.c, &mut self.registers, 1, 4),                 // SUB C
            0x92 => sub(self.registers.d, &mut self.registers, 1, 4),                 // SUB D
            0x93 => sub(self.registers.e, &mut self.registers, 1, 4),                 // SUB E
            0x94 => sub(self.registers.h, &mut self.registers, 1, 4),                 // SUB H
            0x95 => sub(self.registers.l, &mut self.registers, 1, 4),                 // SUB L
            0x96 => sub(memory.read(self.registers.hl()), &mut self.registers, 1, 8), // SUB (HL)
            0x97 => sub(self.registers.a, &mut self.registers, 1, 4),                 // SUB A
            0x98 => sbc(self.registers.b, &mut self.registers, 1, 4),                 // SBC A,B
            0x99 => sbc(self.registers.c, &mut self.registers, 1, 4),                 // SBC A,C
            0x9a => sbc(self.registers.d, &mut self.registers, 1, 4),                 // SBC A,D
            0x9b => sbc(self.registers.e, &mut self.registers, 1, 4),                 // SBC A,E
            0x9c => sbc(self.registers.h, &mut self.registers, 1, 4),                 // SBC A,H
            0x9d => sbc(self.registers.l, &mut self.registers, 1, 4),                 // SBC A,L
            0x9e => sbc(memory.read(self.registers.hl()), &mut self.registers, 1, 8), // SBC A,(HL)
            0x9f => sbc(self.registers.a, &mut self.registers, 1, 4),                 // SBC A,A
            0xa0 => and(self.registers.b, &mut self.registers, 1, 4),                 // AND B
            0xa1 => and(self.registers.c, &mut self.registers, 1, 4),                 // AND C
            0xa2 => and(self.registers.d, &mut self.registers, 1, 4),                 // AND D
            0xa3 => and(self.registers.e, &mut self.registers, 1, 4),                 // AND E
            0xa4 => and(self.registers.h, &mut self.registers, 1, 4),                 // AND H
            0xa5 => and(self.registers.l, &mut self.registers, 1, 4),                 // AND L
            0xa6 => and(memory.read(self.registers.hl()), &mut self.registers, 1, 8), // AND (HL)
            0xa7 => and(self.registers.a, &mut self.registers, 1, 4),                 // AND A
            0xa8 => xor(self.registers.b, &mut self.registers, 4),                    // XOR B
            0xa9 => xor(self.registers.c, &mut self.registers, 4),                    // XOR C
            0xaa => xor(self.registers.d, &mut self.registers, 4),                    // XOR D
            0xab => xor(self.registers.e, &mut self.registers, 4),                    // XOR E
            0xac => xor(self.registers.h, &mut self.registers, 4),                    // XOR H
            0xad => xor(self.registers.l, &mut self.registers, 4),                    // XOR L
            0xae => xor(memory.read(self.registers.hl()), &mut self.registers, 8),    // XOR (HL)
            0xaf => xor(self.registers.a, &mut self.registers, 4),                    // XOR A
            0xb0 => or(self.registers.b, &mut self.registers, 1, 4),                  // OR B
            0xb1 => or(self.registers.c, &mut self.registers, 1, 4),                  // OR C
            0xb2 => or(self.registers.d, &mut self.registers, 1, 4),                  // OR D
            0xb3 => or(self.registers.e, &mut self.registers, 1, 4),                  // OR E
            0xb4 => or(self.registers.h, &mut self.registers, 1, 4),                  // OR H
            0xb5 => or(self.registers.l, &mut self.registers, 1, 4),                  // OR L
            0xb6 => or(memory.read(self.registers.hl()), &mut self.registers, 1, 8),  // OR (HL)
            0xb7 => or(self.registers.a, &mut self.registers, 1, 4),                  // OR A
            0xb8 => cp(self.registers.b, &mut self.registers, 1, 4),                  // CP B
            0xb9 => cp(self.registers.c, &mut self.registers, 1, 4),                  // CP C
            0xba => cp(self.registers.d, &mut self.registers, 1, 4),                  // CP D
            0xbb => cp(self.registers.e, &mut self.registers, 1, 4),                  // CP E
            0xbc => cp(self.registers.h, &mut self.registers, 1, 4),                  // CP H
            0xbd => cp(self.registers.l, &mut self.registers, 1, 4),                  // CP L
            0xbe => cp(memory.read(self.registers.hl()), &mut self.registers, 1, 8),  // CP (HL)
            0xbf => cp(self.registers.a, &mut self.registers, 1, 4),                  // RES 7,A
            0xc0 => {
                if self.registers.f & Registers::FLAG_ZERO != 0 {
                    Finish {
                        pc_increment: 1,
                        elapsed_cycles: 8,
                    }
                } else {
                    self.registers.pc = stack_pop(&mut self.registers.sp, memory);
                    Finish {
                        pc_increment: 0,
                        elapsed_cycles: 20,
                    }
                }
            } // RET NZ
            0xc1 => {
                let new_bc = stack_pop(&mut self.registers.sp, memory);
                self.registers.set_bc(new_bc);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 12,
                }
            } // POP BC
            0xc2 => {
                if self.registers.f & Registers::FLAG_ZERO == 0 {
                    self.registers.pc = memory.read_u16(self.registers.pc + 1);
                    Finish {
                        pc_increment: 0,
                        elapsed_cycles: 16,
                    }
                } else {
                    Finish {
                        pc_increment: 3,
                        elapsed_cycles: 12,
                    }
                }
            } // JP NZ,xx
            0xc3 => {
                self.registers.pc = memory.read_u16(self.registers.pc + 1);
                Finish {
                    pc_increment: 0,
                    elapsed_cycles: 16,
                }
            } // JP xx
            0xc4 => call_condition(
                self.registers.f & Registers::FLAG_ZERO == 0,
                &mut self.registers,
                memory,
            ), // CALL NZ,xx
            0xc5 => {
                stack_push(self.registers.bc(), &mut self.registers.sp, memory);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 16,
                }
            } // PUSH BC
            0xc6 => add_u8(
                memory.read(self.registers.pc + 1),
                &mut self.registers,
                2,
                8,
            ), // ADD A,x
            0xc7 => rst(0x00, &mut self.registers, memory),                           // RST 00h
            0xc8 => {
                if self.registers.f & Registers::FLAG_ZERO != 0 {
                    self.registers.pc = stack_pop(&mut self.registers.sp, memory);
                    Finish {
                        pc_increment: 0,
                        elapsed_cycles: 20,
                    }
                } else {
                    Finish {
                        pc_increment: 1,
                        elapsed_cycles: 8,
                    }
                }
            } // RET Z
            0xc9 => {
                self.registers.pc = stack_pop(&mut self.registers.sp, memory);
                Finish {
                    pc_increment: 0,
                    elapsed_cycles: 16,
                }
            } // RET
            0xca => {
                if self.registers.f & Registers::FLAG_ZERO != 0 {
                    self.registers.pc = memory.read_u16(self.registers.pc + 1);
                    Finish {
                        pc_increment: 0,
                        elapsed_cycles: 16,
                    }
                } else {
                    Finish {
                        pc_increment: 3,
                        elapsed_cycles: 12,
                    }
                }
            } // JP Z,xx
            0xcb => todo!("No 0xcb instructions yet"),
            0xcc => call_condition(
                self.registers.f & Registers::FLAG_ZERO != 0,
                &mut self.registers,
                memory,
            ), // CALL Z,xx
            0xcd => call_condition(true, &mut self.registers, memory), // CALL xx
            0xce => {
                let x = memory.read(self.registers.pc + 1);
                adc(x, &mut self.registers, 2, 8)
            } // ADC A,x
            0xcf => rst(0x08, &mut self.registers, memory),            // RST 08h
            0xd0 => {
                if self.registers.f & Registers::FLAG_CARRY == 0 {
                    self.registers.pc = stack_pop(&mut self.registers.sp, memory);
                    Finish {
                        pc_increment: 0,
                        elapsed_cycles: 20,
                    }
                } else {
                    Finish {
                        pc_increment: 1,
                        elapsed_cycles: 8,
                    }
                }
            } // RET NC
            0xd1 => {
                let popped_value = stack_pop(&mut self.registers.sp, memory);
                self.registers.set_de(popped_value);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 12,
                }
            }
            0xd2 => {
                if self.registers.f & Registers::FLAG_CARRY == 0 {
                    self.registers.pc = memory.read_u16(self.registers.pc + 1);
                    Finish {
                        pc_increment: 0,
                        elapsed_cycles: 16,
                    }
                } else {
                    Finish {
                        pc_increment: 3,
                        elapsed_cycles: 12,
                    }
                }
            } // JP NC,xx
            0xd3 => unreachable!("Invalid opcode"),

            0xd4 => call_condition(
                self.registers.f & Registers::FLAG_CARRY == 0,
                &mut self.registers,
                memory,
            ), // CALL NC,xx
            0xd5 => {
                stack_push(self.registers.de(), &mut self.registers.sp, memory);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 16,
                }
            } // PUSH DE
            0xe0 => {
                memory.write(
                    0xff00 + u16::from(memory.read(self.registers.pc + 1)),
                    self.registers.a,
                );
                Finish {
                    pc_increment: 2,
                    elapsed_cycles: 12,
                }
            } // LDH (ff00+x),A
            0xe1 => {
                let popped_value = stack_pop(&mut self.registers.sp, memory);
                self.registers.set_hl(popped_value);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 12,
                }
            } // POP HL
            0xe2 => {
                memory.write(0xff00 + u16::from(self.registers.c), self.registers.a);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (ff00+C),A
            0xe3 => unreachable!("Invalid opcode"),
            0xe4 => unreachable!("Invalid opcode"),
            0xe5 => {
                stack_push(self.registers.hl(), &mut self.registers.sp, memory);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 16,
                }
            } // PUSH HL
            0xe6 => {
                let x = memory.read(self.registers.pc + 1);
                and(x, &mut self.registers, 2, 8)
            } // AND x
            0xe8 => {
                // rwtodo: This is likely wrong.
                let x: i32 = (memory.read(self.registers.pc + 1) as i8).into();

                // rwtodo: Investigate what happens with this double XOR.
                let sp32: i32 = self.registers.sp.into();
                let xor_result = sp32 ^ x ^ (sp32 + x);

                self.registers.f = 0;
                if (xor_result & 0x10) != 0 {
                    self.registers.f |= Registers::FLAG_HALFCARRY;
                }
                if (xor_result & 0x100) != 0 {
                    self.registers.f |= Registers::FLAG_CARRY;
                }

                self.registers.sp = self.registers.sp.wrapping_add_signed(x.try_into().unwrap());

                Finish {
                    pc_increment: 2,
                    elapsed_cycles: 16,
                }
            } // ADD SP,s
            0xe9 => {
                self.registers.pc = self.registers.hl();
                Finish {
                    pc_increment: 0,
                    elapsed_cycles: 4,
                }
            } // JP (HL)
            0xea => {
                memory.write(memory.read_u16(self.registers.pc + 1), self.registers.a);
                Finish {
                    pc_increment: 3,
                    elapsed_cycles: 16,
                }
            } // LD (x),A
            0xf0 => {
                self.registers.a =
                    memory.read(0xff00 + u16::from(memory.read(self.registers.pc + 1)));
                Finish {
                    pc_increment: 2,
                    elapsed_cycles: 12,
                }
            } // LDH A,(0xff00+x)
            0xf3 => {
                self.registers.ime = false;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // DI
            0xf4 => unreachable!("Invalid opcode"),
            0xf5 => {
                stack_push(self.registers.af(), &mut self.registers.sp, memory);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 16,
                }
            } // PUSH AF
            0xf6 => {
                let x = memory.read(self.registers.pc + 1);
                or(x, &mut self.registers, 2, 8)
            } // OR x
            0xf7 => rst(0x30, &mut self.registers, memory), // RST 30h
            0xf8 => {
                // rwtodo: This is likely wrong.
                let x: i32 = (memory.read(self.registers.pc + 1) as i8).into();

                // rwtodo: Investigate what happens with this double XOR.
                let sp32: i32 = self.registers.sp.into();
                let xor_result = sp32 ^ x ^ (sp32 + x);

                self.registers.f = 0;
                if (xor_result & 0x10) != 0 {
                    self.registers.f |= Registers::FLAG_HALFCARRY;
                }
                if (xor_result & 0x100) != 0 {
                    self.registers.f |= Registers::FLAG_CARRY;
                }

                self.registers.set_hl((sp32 + x) as u16);

                Finish {
                    pc_increment: 2,
                    elapsed_cycles: 12,
                }
            } // LDHL SP,s
            0xf9 => {
                self.registers.sp = self.registers.hl();
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD SP,HL
            0xfa => {
                let address = u16::from(memory.read_u16(self.registers.pc + 1));
                self.registers.a = memory.read(address);
                Finish {
                    pc_increment: 3,
                    elapsed_cycles: 16,
                }
            } // LD A,(xx)
            0xfb => {
                self.registers.ime = true;
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 4,
                }
            } // IE
            0xfc => unreachable!("Invalid opcode"),
            0xfd => unreachable!("Invalid opcode"),
            0xfe => {
                let byte_0 = memory.read(self.registers.pc + 1);

                if subtraction_produces_u8_half_carry(
                    self.registers.a,
                    byte_0,
                    self.registers.f,
                    false,
                ) {
                    self.registers.f |= Registers::FLAG_HALFCARRY;
                } else {
                    self.registers.f &= !Registers::FLAG_HALFCARRY;
                }

                if subtraction_produces_u8_full_carry(self.registers.a, byte_0) {
                    self.registers.f |= Registers::FLAG_CARRY;
                } else {
                    self.registers.f &= !Registers::FLAG_CARRY;
                }

                // Set the add/sub flag high, indicating subtraction.
                self.registers.f |= Registers::FLAG_SUBTRACTION;

                let sub_result = self.registers.a.wrapping_sub(byte_0);
                if sub_result == 0 {
                    self.registers.f |= Registers::FLAG_ZERO;
                } else {
                    self.registers.f &= !Registers::FLAG_ZERO;
                }

                Finish {
                    pc_increment: 2,
                    elapsed_cycles: 8,
                }
            } // CP x
            _ => todo!(
                "Unknown opcode {:#04x} at address {:#06x}\n",
                opcode,
                self.registers.pc
            ),
        };

        self.registers.pc = self.registers.pc.wrapping_add_signed(finish.pc_increment);
        finish.elapsed_cycles
    }
}
