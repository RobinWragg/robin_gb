use std::{cell::RefCell, rc::Rc};

struct CpuRegisters {
    sp: u16,   // rwtodo more descriptive names than the z80 shorthand
    pc: u16,   // rwtodo more descriptive names than the z80 shorthand
    ime: bool, // rwtodo more descriptive names than the z80 shorthand
}
impl CpuRegisters {
    fn new() -> Self {
        Self {
            // registers.af = 0x01b0; /* NOTE: This is different for Game Boy Pocket, Color etc. */
            // registers.bc = 0x0013;
            // registers.de = 0x00d8;
            // registers.hl = 0x014d;
            sp: 0xfffe,
            pc: 0x0100,
            ime: true,
        }
    }
}

struct Timer {}
impl Timer {
    fn new() -> Self {
        Self {}
    }
}

struct Memory {
    bytes: [u8; Self::ADDRESS_SPACE_SIZE],
    joypad: Joypad, // rwtodo: move back to GameBoy struct.
}

impl Memory {
    const ADDRESS_SPACE_SIZE: usize = 1024 * 64;
    fn new() -> Self {
        Self {
            bytes: [0; Self::ADDRESS_SPACE_SIZE],
            joypad: Joypad::new(),
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
        if (address < 0x8000) {
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

    fn read(&self, address: u16) -> u8 {
        // rwtodo rom banks
        // if address >= 0x4000 && address < 0x8000 {
        //     return robingb_romb_read_switchable_bank(address);
        // } else {
        self.bytes[address as usize]
        // }
    }

    fn read_u16(&self, address: u16) -> u16 {
        let byte_0 = self.read(address) as u16;
        let byte_1 = self.read(address + 1) as u16;
        (byte_0 << 8) | byte_1
    }

    fn request_interrupt(&mut self, interrupt_flag: u8) {
        // Combine with the existing request flags
        // rwtodo can do this all in one call
        self.bytes[INTERRUPT_FLAGS_ADDRESS] |= interrupt_flag;
        // Top 3 bits are always 1
        self.bytes[INTERRUPT_FLAGS_ADDRESS] |= 0xe0; // rwtodo is there binary syntax for this?
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

    fn instruction_RST(&mut self, memory: &mut Memory, address_lower_byte: u8) {
        self.stack_push(self.registers.pc + 1, memory);
        self.registers.pc = address_lower_byte as u16;
        self.finish_instruction(0, 16);
    }

    fn instruction_SET(&mut self, bit_to_set: u8, register_to_set: &mut u8, num_cycles: u8) {
        *register_to_set |= 0x01 << bit_to_set;
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

    Ok(GameBoy {
        lcd: Lcd::new(),
        memory: Memory::new(),
        cpu: Cpu::new(),
        timer: Timer::new(),
    })
}
