use crate::address;
use crate::interrupt;
use crate::Memory;

//rwtodo: I can probably do something nifty with Rust attributes to make the "instruction" functions more ergonomic.

type CycleCount = u8;

struct Registers {
    af: u16, // rwtodo: union
    bc: u16, // rwtodo: union
    de: u16, // rwtodo: union
    hl: u16, // rwtodo: union
    sp: u16,
    pc: u16,
    ime: bool,
}

impl Registers {
    const FLAG_ZERO: u8 = 0x80; // "Z", Zero Flag
    const FLAG_SUBTRACTION: u8 = 0x40; // "N", Add/Sub-Flag (BCD)
    const FLAG_HALFCARRY: u8 = 0x20; // "H", Half Carry Flag (BCD)
    const FLAG_CARRY: u8 = 0x10; // "C", Carry Flag

    fn new() -> Self {
        Self {
            af: 0x01b0, // NOTE: This is different for Game Boy Pocket, Color etc.
            bc: 0x0013,
            de: 0x00d8,
            hl: 0x014d,
            sp: 0xfffe,
            pc: 0x0100,
            ime: true,
        }
    }

    fn a(&self) -> u8 {
        self.af.to_le_bytes()[1]
    }

    fn set_a(&mut self, a: u8) {
        self.af.to_le_bytes()[1] = a;
        assert!(self.a() == a);
    }

    fn f(&self) -> u8 {
        self.af.to_le_bytes()[0]
    }

    fn set_f(&mut self, f: u8) {
        self.af.to_le_bytes()[0] = f;
        assert!(self.f() == f);
    }
}

pub struct Cpu {
    registers: Registers, // rwtodo maybe just put the registers in the cpu without wrapping them in a struct
    is_halted: bool,
}

#[allow(non_snake_case)] // Disable warnings for instruction names
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

    #[must_use] // Returns the number of cycles the instruction took.
    pub fn execute_next_instruction(&mut self, memory: &mut Memory) -> u8 {
        let cycles = self.execute_next_instruction_inner(memory);
        self.handle_interrupt_requests(memory);
        cycles
    }

    #[must_use] // Returns the number of cycles the instruction took.
    pub fn execute_next_instruction_inner(&mut self, memory: &mut Memory) -> u8 {
        if self.is_halted {
            return 4;
        }

        let opcode = memory.read(self.registers.pc);

        return match opcode {
            0x00 /* NOP */ => return self.finish_instruction(1, 4),
            0xc3 /* JP xx */ => {
                self.registers.pc = memory.read_u16(self.registers.pc + 1);
                self.finish_instruction(0, 16)
            },
            _ => {
                unreachable!(
                    "Unknown opcode {:#04x} at address {:#06x}\n",
                    opcode, self.registers.pc
                )
            }
        };
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

    // rwtodo: These probably take u8 arguments, so I might end up calling .into() a lot...
    fn subtraction_produces_u8_full_carry(a: i16, b: i16) -> bool {
        a - b < 0
    }

    // rwtodo: These probably take u8 arguments, so I might end up calling .into() a lot...
    fn addition_produces_u8_full_carry(a: i16, b: i16) -> bool {
        a + b > 0xff
    }

    // rwtodo: These probably take u8 arguments, so I might end up calling .into() a lot...
    fn negate_produces_u8_half_carry(&self, a: i16, b: i16, include_carry: bool) -> bool {
        let optional_carry: i16 =
            if include_carry && (self.registers.f() & Registers::FLAG_CARRY) != 0 {
                1
            } else {
                0
            };

        (a & 0x0f) - (b & 0x0f) - optional_carry < 0
    }

    // rwtodo: These probably take u8 arguments, so I might end up calling .into() a lot...
    fn addition_produces_u8_half_carry(&self, a: i16, b: i16, include_carry: bool) -> bool {
        let optional_carry: i16 =
            if include_carry && (self.registers.f() & Registers::FLAG_CARRY) != 0 {
                1
            } else {
                0
            };

        (a & 0x0f) + (b & 0x0f) + optional_carry > 0x0f
    }

    #[must_use]
    fn finish_instruction(&mut self, pc_increment: i16, elapsed_cycles: CycleCount) -> CycleCount {
        self.registers.pc = self.registers.pc.wrapping_add_signed(pc_increment);
        elapsed_cycles
    }

    fn instruction_INC_u8(&mut self, value_to_increment: &mut u8, num_cycles: u8) -> CycleCount {
        let mut f = self.registers.f();

        if self.addition_produces_u8_half_carry((*value_to_increment).into(), 1, false) {
            f |= Registers::FLAG_HALFCARRY;
        } else {
            f &= !Registers::FLAG_HALFCARRY;
        }

        *value_to_increment += 1;

        if *value_to_increment != 0 {
            f &= !Registers::FLAG_ZERO;
        } else {
            f |= Registers::FLAG_ZERO;
        }

        f &= !Registers::FLAG_SUBTRACTION;

        self.registers.set_f(f);
        self.finish_instruction(1, num_cycles)
    }

    fn instruction_XOR(&mut self, xor_input: u8, num_cycles: u8) -> CycleCount {
        let result = self.registers.a() ^ xor_input;
        self.registers.set_a(result);

        let mut f = self.registers.f();

        if result == 0 {
            f |= Registers::FLAG_ZERO;
        } else {
            f &= !Registers::FLAG_ZERO;
        }

        f &= !Registers::FLAG_SUBTRACTION;
        f &= !Registers::FLAG_HALFCARRY;
        f &= !Registers::FLAG_CARRY;

        self.registers.set_f(f);
        self.finish_instruction(1, num_cycles)
    }

    fn instruction_OR(&mut self, or_input: u8, pc_increment: i16, num_cycles: u8) -> CycleCount {
        // rwtodo: I think pc_increment might always be 1, thereby allowing us to remove it as from the param list.

        let result = self.registers.a() | or_input;
        self.registers.set_a(result);

        let mut f = self.registers.f();

        if result == 0 {
            f |= Registers::FLAG_ZERO;
        } else {
            f &= !Registers::FLAG_ZERO;
        }

        f &= !Registers::FLAG_SUBTRACTION;
        f &= !Registers::FLAG_HALFCARRY;
        f &= !Registers::FLAG_CARRY;

        self.registers.set_f(f);
        self.finish_instruction(pc_increment, num_cycles)
    }

    fn instruction_AND(&mut self, and_input: u8, pc_increment: i16, num_cycles: u8) -> CycleCount {
        // rwtodo: If pc_increment is always 1, remove it as from the param list.

        let result = self.registers.a() & and_input;
        self.registers.set_a(result);

        let mut f = self.registers.f();

        if result == 0 {
            f |= Registers::FLAG_ZERO;
        } else {
            f &= !Registers::FLAG_ZERO;
        }

        f &= !Registers::FLAG_SUBTRACTION;
        f |= Registers::FLAG_HALFCARRY;
        f &= !Registers::FLAG_CARRY;

        self.registers.set_f(f);
        self.finish_instruction(pc_increment, num_cycles)
    }

    fn instruction_RST(&mut self, memory: &mut Memory, address_lower_byte: u8) -> CycleCount {
        self.stack_push(self.registers.pc + 1, memory);
        self.registers.pc = address_lower_byte as u16;
        self.finish_instruction(0, 16)
    }

    fn instruction_SET(
        &mut self,
        bit_to_set: u8,
        byte_to_set: &mut u8,
        num_cycles: u8,
    ) -> CycleCount {
        *byte_to_set |= 0x01 << bit_to_set;
        self.finish_instruction(1, num_cycles)
    }

    fn instruction_RES(
        &mut self,
        bit_to_reset: u8,
        byte_to_reset: &mut u8,
        num_cycles: u8,
    ) -> CycleCount {
        *byte_to_reset &= !(0x01 << bit_to_reset);
        self.finish_instruction(1, num_cycles)
    }

    fn instruction_CALL_condition_xx(
        &mut self,
        condition: bool,
        memory: &mut Memory,
    ) -> CycleCount {
        if condition {
            self.stack_push(self.registers.pc + 3, memory);
            self.registers.pc = memory.read_u16(self.registers.pc + 1);
            self.finish_instruction(0, 24)
        } else {
            self.finish_instruction(3, 12)
        }
    }

    fn instruction_BIT(
        &mut self,
        bit_to_check: u8,
        byte_to_check: u8,
        num_cycles: u8,
    ) -> CycleCount {
        let mut f = self.registers.f();

        if (byte_to_check & (0x01 << bit_to_check)) != 0 {
            f &= !Registers::FLAG_ZERO;
        } else {
            f |= Registers::FLAG_ZERO;
        }

        f &= !Registers::FLAG_SUBTRACTION;
        f |= Registers::FLAG_HALFCARRY;
        self.registers.set_f(f);

        self.finish_instruction(1, num_cycles)
    }

    fn instruction_SWAP(&mut self, byte_to_swap: &mut u8, num_cycles: u8) -> CycleCount {
        let upper_4_bits = *byte_to_swap & 0xf0;
        let lower_4_bits = *byte_to_swap & 0x0f;
        *byte_to_swap = upper_4_bits >> 4;
        *byte_to_swap |= lower_4_bits << 4;

        let mut f = self.registers.f();

        if *byte_to_swap == 0 {
            f |= Registers::FLAG_ZERO;
        } else {
            f &= !Registers::FLAG_ZERO;
        }

        f &= !Registers::FLAG_SUBTRACTION;
        f &= !Registers::FLAG_HALFCARRY;
        f &= !Registers::FLAG_CARRY;

        self.registers.set_f(f);
        self.finish_instruction(1, num_cycles)
    }
}
