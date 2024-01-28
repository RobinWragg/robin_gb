#![allow(unused_variables)]
#![allow(dead_code)]

fn make_u16(a: u8, b: u8) -> u16 {
    let byte_0 = a as u16;
    let byte_1 = b as u16;
    (byte_0 << 8) | byte_1
}

struct Renderer {
    shade_0: u8,
    shade_1: u8,
    shade_2: u8,
    shade_3: u8,
}
impl Renderer {
    const SHADE_0_FLAG: u8 = 0x04;
    fn set_palette(&mut self, palette: u8) {
        // SHADE_0_FLAG ensures shade_0 is unique, which streamlines the process of
        // shade-0-dependent blitting. The flag is discarded in the final step of the render.
        self.shade_0 = (palette & 0x03) | Self::SHADE_0_FLAG;
        self.shade_1 = (palette & 0x0c) >> 2;
        self.shade_2 = (palette & 0x30) >> 4;
        self.shade_3 = (palette & 0xc0) >> 6;
    }
}

// rwtodo more descriptive names than the z80 shorthand
struct CpuRegisters {
    af: u16, // rwtodo: union
    bc: u16, // rwtodo: union
    de: u16, // rwtodo: union
    hl: u16, // rwtodo: union
    sp: u16,
    pc: u16,
    ime: bool,
}
impl CpuRegisters {
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

struct Timer {
    cycles_since_last_tima_increment: u16,
    incrementer_every_cycle: u16,
}
impl Timer {
    const DIVIDER_ADDRESS: u16 = 0xff04; /* "DIV" */
    const COUNTER_ADDRESS: u16 = 0xff05; /* "TIMA" */
    const MODULO_ADDRESS: u16 = 0xff06; /* "TMA" */
    const CONTROL_ADDRESS: u16 = 0xff07; /* "TAC" */

    fn new(memory: &mut Memory) -> Self {
        // rwtodo: why is it ok for this to be immutable? Surely it can be mutated after it is returned from this function? is it because the bchecker has concluded it's not being mutated outside of this function?
        let mut new_timer = Self {
            cycles_since_last_tima_increment: 0,
            incrementer_every_cycle: 0xabcc,
        };

        let div_byte = new_timer.incrementer_every_cycle.to_le_bytes()[1];
        assert!(div_byte == 0xab);
        *memory.direct_access(Self::DIVIDER_ADDRESS) = div_byte;
        *memory.direct_access(Self::COUNTER_ADDRESS) = 0x00;
        *memory.direct_access(Self::MODULO_ADDRESS) = 0x00;
        *memory.direct_access(Self::CONTROL_ADDRESS) = 0x00;

        new_timer
    }
}

struct Memory {
    bytes: [u8; Self::ADDRESS_SPACE_SIZE],
    joypad: Joypad,                  // rwtodo: move back to GameBoy struct.
    current_switchable_rom_bank: u8, // rwtodo rename to "active..."
}

impl Memory {
    const ADDRESS_SPACE_SIZE: usize = 1024 * 64;
    const ROM_BANK_SIZE: usize = 16384; // 16kB

    fn new() -> Self {
        Self {
            bytes: [0; Self::ADDRESS_SPACE_SIZE],
            joypad: Joypad::new(),
            current_switchable_rom_bank: 1,
        }
    }

    fn direct_access(&mut self, address: u16) -> &mut u8 {
        &mut self.bytes[address as usize]
    }

    fn get_joypad_register_write_result(&mut self, mut register_value: u8) -> u8 {
        const ACTION_BUTTON_REQUEST: u8 = 0x20;
        const DIRECTION_BUTTON_REQUEST: u8 = 0x10;

        register_value |= 0xc0; // bits 6 and 7 are always 1.
        register_value |= 0x0f; // unpressed buttons are 1.

        if (register_value & ACTION_BUTTON_REQUEST) == 0x00 {
            register_value &= self.joypad.action_buttons;
            self.request_interrupt(INTERRUPT_FLAG_JOYPAD);
        }

        if (register_value & DIRECTION_BUTTON_REQUEST) == 0x00 {
            register_value &= self.joypad.direction_buttons;
            self.request_interrupt(INTERRUPT_FLAG_JOYPAD);
        }

        return register_value;
    }

    fn write(&mut self, address: u16, value: u8) {
        // rwtodo: convert to match statement?
        if address < 0x8000 {
            // perform_cart_control(address, value); rwtodo
        } else if address == 0xff00 {
            // rwtodo: label 0xff00 as a constant?
            self.bytes[address as usize] = self.get_joypad_register_write_result(value);
        } else if address == 0xff04 {
            // rwtodo: label 0xff04 as a constant?
            self.bytes[address as usize] = 0x00; // Reset timer DIV register. rwtodo: move this responibility into Timer struct
        } else if address == 0xff46 {
            // Perform OAM DMA transfer. rwtodo: copying twice here, unless the compiler optimizes it out. Use copy_within on self.memory directly.
            const SIZE_OF_TRANSFER: usize = 160;

            let mut bytes_to_transfer: [u8; SIZE_OF_TRANSFER] = [0; SIZE_OF_TRANSFER];

            {
                let src_range_start: usize = (value as usize) * 0x100;
                let src_range_end: usize = src_range_start + SIZE_OF_TRANSFER;
                let src_slice = &self.bytes[src_range_start..src_range_end];
                bytes_to_transfer.copy_from_slice(src_slice);
            }

            let dst_range_start: usize = 0xfe00;
            let dst_range_end: usize = dst_range_start + SIZE_OF_TRANSFER;
            let dst_slice = &mut self.bytes[dst_range_start..dst_range_end];

            dst_slice.copy_from_slice(&bytes_to_transfer);
        } else {
            self.bytes[address as usize] = value;

            // Memory is duplicated when writing to these registers
            if address >= 0xc000 && address < 0xde00 {
                let echo_address = address - 0xc000 + 0xe000;
                self.bytes[echo_address as usize] = value;
            } else if address >= 0xe000 && address < 0xfe00 {
                let echo_address = address - 0xe000 + 0xc000;
                self.bytes[echo_address as usize] = value;
            }

            // rwtodo: implement cart_state stuff so we can do this.
            // rwtodo: Also handle the below for MBC3.
            // if cart_state.mbc_type == MBC_1 && address >= 0xa000 && address < 0xc000 {
            //     // RAM was written to.
            //     cart_state.save_file_is_outdated = true;
            // }
        }
    }

    fn write_u16(&mut self, address: u16, value: u16) {
        let bytes = value.to_le_bytes();
        self.write(address, bytes[0]);
        self.write(address + 1, bytes[1]);
    }

    fn read(&self, address: u16) -> u8 {
        // rwtodo rom banks
        // if address >= 0x4000 && address < 0x8000 {
        //     return robingb_romb_read_switchable_bank(address);
        // } else {
        self.bytes[address as usize]
        // }
    }

    fn read_u16(&self, address: u16) -> u16 {
        make_u16(self.read(address), self.read(address + 1))
    }

    fn request_interrupt(&mut self, interrupt_flag: u8) {
        // Combine with the existing request flags
        // rwtodo can do this all in one call
        self.bytes[INTERRUPT_FLAGS_ADDRESS] |= interrupt_flag;
        // Top 3 bits are always 1
        self.bytes[INTERRUPT_FLAGS_ADDRESS] |= 0xe0; // rwtodo is there binary syntax for this?
    }

    fn init_first_rom_banks(&mut self, file_data: &[u8]) {
        let banks_src = &file_data[..(Self::ROM_BANK_SIZE * 2)];
        let banks_dst = &mut self.bytes[..(Self::ROM_BANK_SIZE * 2)];
        banks_dst.copy_from_slice(banks_src);
        self.current_switchable_rom_bank = 1;
    }
}

struct Lcd {}
impl Lcd {
    const WIDTH: usize = 160; // rwtodo would there be any benefit to these being u8?
    const HEIGHT: usize = 144; // rwtodo would there be any benefit to these being u8?
    const PIXEL_COUNT: usize = Lcd::WIDTH * Lcd::HEIGHT;
    fn new() -> Self {
        Self {}
    }
    fn pixel_data(&self) -> [u8; Lcd::PIXEL_COUNT] {
        [127; Lcd::PIXEL_COUNT] // rwtodo just returning grey for now
    }
}

use std::fs;

struct Joypad {
    action_buttons: u8,
    direction_buttons: u8,
}
impl Joypad {
    fn new() -> Self {
        Self {
            action_buttons: 0xff,
            direction_buttons: 0xff,
        }
    }
}

struct Cpu {
    registers: CpuRegisters, // rwtodo maybe just put the registers in the cpu without wrapping them in a struct
    num_cycles_for_finish: u8, // rwtodo: I could perhaps just implement this as return values from all the functions.
}

#[allow(non_snake_case)] // Disable warnings for instruction names
impl Cpu {
    fn new() -> Self {
        Self {
            registers: CpuRegisters::new(),
            num_cycles_for_finish: 0,
        }
    }

    fn execute_next_instruction(&mut self)
    /*rwtodo do I need to return anything here? Handle lcd completion state elsewhere? */
    {
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

    fn subtraction_produces_u8_full_carry(a: i16, b: i16) -> bool {
        a - b < 0
    }

    fn addition_produces_u8_full_carry(a: i16, b: i16) -> bool {
        a + b > 0xff
    }

    fn finish_instruction(&mut self, pc_increment: i16, num_cycles_param: u8) {
        self.registers.pc = self.registers.pc.wrapping_add_signed(pc_increment);
        self.num_cycles_for_finish = num_cycles_param;
    }

    fn instruction_XOR(&mut self, xor_input: u8, num_cycles: u8) {
        let result = self.registers.a() ^ xor_input;
        self.registers.set_a(result);

        let mut f = self.registers.f();

        if result == 0 {
            f |= CpuRegisters::FLAG_ZERO;
        } else {
            f &= !CpuRegisters::FLAG_ZERO;
        }

        f &= !CpuRegisters::FLAG_SUBTRACTION;
        f &= !CpuRegisters::FLAG_HALFCARRY;
        f &= !CpuRegisters::FLAG_CARRY;

        self.registers.set_f(f);
        self.finish_instruction(1, num_cycles);
    }

    fn instruction_OR(&mut self, or_input: u8, pc_increment: i16, num_cycles: u8) {
        // rwtodo: I think pc_increment might always be 1, thereby allowing us to remove it as from the param list.

        let result = self.registers.a() | or_input;
        self.registers.set_a(result);

        let mut f = self.registers.f();

        if result == 0 {
            f |= CpuRegisters::FLAG_ZERO;
        } else {
            f &= !CpuRegisters::FLAG_ZERO;
        }

        f &= !CpuRegisters::FLAG_SUBTRACTION;
        f &= !CpuRegisters::FLAG_HALFCARRY;
        f &= !CpuRegisters::FLAG_CARRY;

        self.registers.set_f(f);
        self.finish_instruction(pc_increment, num_cycles);
    }

    fn instruction_AND(&mut self, and_input: u8, pc_increment: i16, num_cycles: u8) {
        // rwtodo: If pc_increment is always 1, remove it as from the param list.

        let result = self.registers.a() & and_input;
        self.registers.set_a(result);

        let mut f = self.registers.f();

        if result == 0 {
            f |= CpuRegisters::FLAG_ZERO;
        } else {
            f &= !CpuRegisters::FLAG_ZERO;
        }

        f &= !CpuRegisters::FLAG_SUBTRACTION;
        f |= CpuRegisters::FLAG_HALFCARRY;
        f &= !CpuRegisters::FLAG_CARRY;

        self.registers.set_f(f);
        self.finish_instruction(pc_increment, num_cycles);
    }

    fn instruction_RST(&mut self, memory: &mut Memory, address_lower_byte: u8) {
        self.stack_push(self.registers.pc + 1, memory);
        self.registers.pc = address_lower_byte as u16;
        self.finish_instruction(0, 16);
    }

    fn instruction_SET(&mut self, bit_to_set: u8, byte_to_set: &mut u8, num_cycles: u8) {
        *byte_to_set |= 0x01 << bit_to_set;
        self.finish_instruction(1, num_cycles);
    }

    fn instruction_RES(&mut self, bit_to_reset: u8, byte_to_reset: &mut u8, num_cycles: u8) {
        *byte_to_reset &= !(0x01 << bit_to_reset);
        self.finish_instruction(1, num_cycles);
    }

    fn instruction_CALL_condition_xx(&mut self, condition: bool, memory: &mut Memory) {
        if condition {
            self.stack_push(self.registers.pc + 3, memory);
            self.registers.pc = memory.read_u16(self.registers.pc + 1);
            self.finish_instruction(0, 24);
        } else {
            self.finish_instruction(3, 12);
        }
    }

    fn instruction_BIT(&mut self, bit_to_check: u8, byte_to_check: u8, num_cycles: u8) {
        let mut f = self.registers.f();

        if (byte_to_check & (0x01 << bit_to_check)) != 0 {
            f &= !CpuRegisters::FLAG_ZERO;
        } else {
            f |= CpuRegisters::FLAG_ZERO;
        }

        f &= !CpuRegisters::FLAG_SUBTRACTION;
        f |= CpuRegisters::FLAG_HALFCARRY;
        self.registers.set_f(f);

        self.finish_instruction(1, num_cycles);
    }
}

pub enum Button {
    A,
    B,
    START,
    SELECT,
}

// rwtodo: enums?
const INTERRUPT_FLAG_VBLANK: u8 = 0x01;
const INTERRUPT_FLAG_LCD_STAT: u8 = 0x02;
const INTERRUPT_FLAG_TIMER: u8 = 0x04;
const INTERRUPT_FLAG_SERIAL: u8 = 0x08;
const INTERRUPT_FLAG_JOYPAD: u8 = 0x10;

const INTERRUPT_FLAGS_ADDRESS: usize = 0xff0f;
const INTERRUPT_ENABLE_ADDRESS: usize = 0xffff;

pub struct GameBoy {
    lcd: Lcd,
    memory: Memory,
    cpu: Cpu,
    timer: Timer,
}

impl GameBoy {
    pub fn emulate_next_frame(&mut self) -> [u8; Lcd::PIXEL_COUNT] {
        // rwtodo run cpu etc
        self.cpu.execute_next_instruction();
        self.lcd.pixel_data()
    }

    // Inform the emulator of button state with this function. All buttons are up (unpressed) when emulation starts.
    pub fn set_button(&mut self, button: &Button, is_down: bool) {
        // rwtodo
    }
}

pub fn load_rom(rom_path: &str) -> Result<GameBoy, std::io::Error> {
    // rwtodo use a file-like object instead of &str?
    let rom_data = fs::read(&rom_path)?; // rwtodo

    let mut memory = Memory::new();
    let timer = Timer::new(&mut memory);

    Ok(GameBoy {
        lcd: Lcd::new(),
        memory,
        cpu: Cpu::new(),
        timer,
    })
}
