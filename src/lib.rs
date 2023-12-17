struct Registers {
    sp: u16,
    pc: u16,
    ime: bool,
}
impl Registers {
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

    fn respond_to_div_register(&mut self) -> u8 {
        0x00 // rwtodo: Revisit this.
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

const MEMORY_ADDRESS_SPACE_SIZE: usize = 1024 * 64;
type Memory = [u8; MEMORY_ADDRESS_SPACE_SIZE];

struct Cpu {
    num_cycles_for_finish: u8, // rwtodo: I could perhaps just implement this as return values from all the functions.
}
impl Cpu {
    fn new() -> Self {
        Self {
            num_cycles_for_finish: 0,
        }
    }

    fn execute_next_instruction(&mut self, registers: &mut Registers, memory: &mut Memory)
    /*rwtodo do I need to return anything here? Handle lcd completion state elsewhere? */
    {
    }

    fn subtraction_produces_u8_full_carry(a: i16, b: i16) -> bool {
        a - b < 0
    }

    fn addition_produces_u8_full_carry(a: i16, b: i16) -> bool {
        a + b > 0xff
    }

    fn finish_instruction(
        &mut self,
        registers: &mut Registers,
        pc_increment: i16,
        num_cycles_param: u8,
    ) {
        registers.pc = registers.pc.wrapping_add_signed(pc_increment);
        self.num_cycles_for_finish = num_cycles_param;
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
const IE_ADDRESS: usize = 0xffff;

pub struct GameBoy {
    joypad: Joypad,
    lcd: Lcd,
    memory: Memory,
    cpu: Cpu,
    registers: Registers,
    timer: Timer,
}

impl GameBoy {
    pub fn emulate_next_frame(&mut self) -> [u8; Lcd::PIXEL_COUNT] {
        // rwtodo run cpu etc
        self.cpu
            .execute_next_instruction(&mut self.registers, &mut self.memory);
        self.lcd.pixel_data()
    }

    // Inform the emulator of button state with this function. All buttons are up (unpressed) when emulation starts.
    pub fn set_button(&mut self, button: &Button, is_down: bool) {
        // rwtodo
    }

    fn respond_to_joypad_register_write(&mut self, mut register_value: u8) -> u8 {
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

    fn request_interrupt(&mut self, interrupt_flag: u8) {
        // Combine with the existing request flags
        self.memory[INTERRUPT_FLAGS_ADDRESS] |= interrupt_flag;
        // Top 3 bits are always 1
        self.memory[INTERRUPT_FLAGS_ADDRESS] |= 0xe0;
    }

    fn memory_write(&mut self, address: usize, value: u8) {
        // rwtodo: convert to match statement?
        if (address < 0x8000) {
            // perform_cart_control(address, value); rwtodo
        } else if address == 0xff00 {
            // rwtodo: label as a constant?
            self.memory[address] = self.respond_to_joypad_register_write(value);
        } else if address == 0xff04 {
            // rwtodo: label as a constant?
            self.memory[address] = self.timer.respond_to_div_register();
        } else if address == 0xff46 {
            // Perform OAM DMA transfer. rwtodo: copying twice here, unless the compiler optimizes it out. Use copy_within on self.memory directly.
            const SIZE_OF_TRANSFER: usize = 160;

            let mut bytes_to_transfer: [u8; SIZE_OF_TRANSFER] = [0; SIZE_OF_TRANSFER];

            {
                let src_range_start: usize = (value as usize) * 0x100;
                let src_range_end: usize = src_range_start + SIZE_OF_TRANSFER;
                let src_slice = &self.memory[src_range_start..src_range_end];
                bytes_to_transfer.copy_from_slice(src_slice);
            }

            let dst_range_start: usize = 0xfe00;
            let dst_range_end: usize = dst_range_start + SIZE_OF_TRANSFER;
            let dst_slice = &mut self.memory[dst_range_start..dst_range_end];

            dst_slice.copy_from_slice(&bytes_to_transfer);
        } else {
            self.memory[address] = value;

            // Memory is duplicated when writing to these registers
            if address >= 0xc000 && address < 0xde00 {
                let echo_address = address - 0xc000 + 0xe000;
                self.memory[echo_address] = value;
            } else if address >= 0xe000 && address < 0xfe00 {
                let echo_address = address - 0xe000 + 0xc000;
                self.memory[echo_address] = value;
            }

            // rwtodo: implement cart_state stuff so we can do this.
            // rwtodo: Also handle the below for MBC3.
            // if cart_state.mbc_type == MBC_1 && address >= 0xa000 && address < 0xc000 {
            //     // RAM was written to.
            //     cart_state.save_file_is_outdated = true;
            // }
        }
    }
}

pub fn load_rom(rom_path: &str) -> Result<GameBoy, std::io::Error> {
    // rwtodo use a file-like object instead of &str?
    let rom_data = fs::read(&rom_path)?; // rwtodo
    Ok(GameBoy {
        lcd: Lcd::new(),
        joypad: Joypad::new(),
        memory: [0; MEMORY_ADDRESS_SPACE_SIZE],
        cpu: Cpu::new(),
        registers: Registers::new(),
        timer: Timer::new(),
    })
}
