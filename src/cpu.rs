use crate::address;
use crate::interrupt;
use crate::make_u16;
use crate::Memory;

//rwtodo: I can probably do something nifty with Rust attributes to make the "instruction" functions more ergonomic.

// rwtodo: Apparently STOP is like HALT except the LCD is inoperational as well, and the "stopped" state is only exited when a button is pressed. Look for better documentation on it.

fn print_instruction(pc: u16, memory: &mut Memory) {
    let opcode = memory.read(pc);
    match opcode {
        0x00 => println!("NOP"), // NOP
        0x11 => println!("LD DE,{:#06x}", memory.read_u16(pc + 1)),
        0x2c => println!("INC L"),
        0x53 => println!("LD D,E"),
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
        0xc3 => println!("JP {:#06x}", memory.read_u16(pc + 1)),
        0xaf => println!("XOR A"),
        _ => println!("op {:#04x} at address {:#06x}", opcode, pc),
    };
}

type CycleCount = u8;

struct Registers {
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

mod instructions {
    use crate::cpu::Registers;

    pub struct Finish {
        pub pc_increment: i16,
        pub elapsed_cycles: u8,
    }

    // rwtodo: These probably take u8 arguments, so I might end up calling .into() a lot...
    fn subtraction_produces_u8_full_carry(a: i16, b: i16) -> bool {
        a - b < 0
    }

    // rwtodo: These probably take u8 arguments, so I might end up calling .into() a lot...
    fn addition_produces_u8_full_carry(a: i16, b: i16) -> bool {
        a + b > 0xff
    }

    // rwtodo: These probably take u8 arguments, so I might end up calling .into() a lot...
    fn subtraction_produces_u8_half_carry(
        a: i16,
        b: i16,
        register_f: u8,
        include_carry: bool,
    ) -> bool {
        let optional_carry: i16 = if include_carry && (register_f & Registers::FLAG_CARRY) != 0 {
            1
        } else {
            0
        };

        (a & 0x0f) - (b & 0x0f) - optional_carry < 0
    }

    // rwtodo: These probably take u8 arguments, so I might end up calling .into() a lot...
    fn addition_produces_u8_half_carry(
        a: i16,
        b: i16,
        register_f: u8,
        include_carry: bool,
    ) -> bool {
        let optional_carry: i16 = if include_carry && (register_f & Registers::FLAG_CARRY) != 0 {
            1
        } else {
            0
        };

        (a & 0x0f) + (b & 0x0f) + optional_carry > 0x0f
    }

    pub fn inc_u8(value_to_increment: &mut u8, register_f: &mut u8, elapsed_cycles: u8) -> Finish {
        if addition_produces_u8_half_carry((*value_to_increment).into(), 1, *register_f, false) {
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

    pub fn xor(
        xor_input: u8,
        register_a: &mut u8,
        register_f: &mut u8,
        elapsed_cycles: u8,
    ) -> Finish {
        *register_a ^= xor_input;

        if *register_a == 0 {
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

    fn or(
        or_input: u8,
        register_a: &mut u8,
        register_f: &mut u8,
        pc_increment: i16,
        elapsed_cycles: u8,
    ) -> Finish {
        // rwtodo: I think pc_increment might always be 1, thereby allowing us to remove it as from the param list.

        *register_a |= or_input;

        if *register_a == 0 {
            *register_f |= Registers::FLAG_ZERO;
        } else {
            *register_f &= !Registers::FLAG_ZERO;
        }

        *register_f &= !Registers::FLAG_SUBTRACTION;
        *register_f &= !Registers::FLAG_HALFCARRY;
        *register_f &= !Registers::FLAG_CARRY;

        Finish {
            pc_increment,
            elapsed_cycles,
        }
    }

    fn and(
        and_input: u8,
        register_a: &mut u8,
        register_f: &mut u8,
        pc_increment: i16,
        elapsed_cycles: u8,
    ) -> Finish {
        // rwtodo: If pc_increment is always 1, remove it as from the param list.

        *register_a &= and_input;

        if *register_a == 0 {
            *register_f |= Registers::FLAG_ZERO;
        } else {
            *register_f &= !Registers::FLAG_ZERO;
        }

        *register_f &= !Registers::FLAG_SUBTRACTION;
        *register_f |= Registers::FLAG_HALFCARRY;
        *register_f &= !Registers::FLAG_CARRY;

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
        if subtraction_produces_u8_half_carry(i16::from(*register_to_dec), 1, *register_f, false) {
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
                self.stack_push(self.registers.pc, memory);

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

    fn stack_push(&mut self, value_to_push: u16, memory: &mut Memory) {
        let bytes = value_to_push.to_le_bytes();
        self.registers.sp -= 2;

        memory.write(self.registers.sp, bytes[0]);
        memory.write(self.registers.sp + 1, bytes[1]);
    }

    fn stack_pop(&mut self, memory: &Memory) -> u16 {
        let popped_value = memory.read_u16(self.registers.sp);
        self.registers.sp += 2;
        popped_value
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

        print_instruction(self.registers.pc, memory);

        let opcode = memory.read(self.registers.pc);

        use instructions::*;

        let finish: Finish = match opcode {
            0x00 => nop(),                                                                   // NOP
            0x05 => dec_reg8(&mut self.registers.b, &mut self.registers.f), // DEC B
            0x06 => ld_reg8_mem8(&mut self.registers.b, memory.read(self.registers.pc + 1)), // LD B,x
            0x0d => dec_reg8(&mut self.registers.c, &mut self.registers.f), // DEC C
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
            0x21 => {
                self.registers
                    .set_hl(memory.read_u16(self.registers.pc + 1));
                Finish {
                    pc_increment: 3,
                    elapsed_cycles: 12,
                }
            }
            0x25 => dec_reg8(&mut self.registers.h, &mut self.registers.f), // DEC H
            0x26 => ld_reg8_mem8(&mut self.registers.h, memory.read(self.registers.pc + 1)), // LD H,x
            0x2c => inc_u8(&mut self.registers.l, &mut self.registers.f, 4), // INC L
            0x2d => dec_reg8(&mut self.registers.l, &mut self.registers.f),  // DEC L
            0x2e => ld_reg8_mem8(&mut self.registers.l, memory.read(self.registers.pc + 1)), // LD L,x
            0x32 => {
                memory.write(self.registers.hl(), self.registers.a);
                self.registers.set_hl(self.registers.hl() - 1);
                Finish {
                    pc_increment: 1,
                    elapsed_cycles: 8,
                }
            } // LD (HL-),A
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
            0xaf => xor(
                self.registers.a,
                &mut self.registers.a,
                &mut self.registers.f,
                4,
            ), // XOR A
            0xc3 => {
                self.registers.pc = memory.read_u16(self.registers.pc + 1);
                Finish {
                    pc_increment: 0,
                    elapsed_cycles: 16,
                }
            } // JP xx
            _ => unreachable!(
                "Unknown opcode {:#04x} at address {:#06x}\n",
                opcode, self.registers.pc
            ),
        };

        self.registers.pc = self.registers.pc.wrapping_add_signed(finish.pc_increment);
        finish.elapsed_cycles
    }
}
