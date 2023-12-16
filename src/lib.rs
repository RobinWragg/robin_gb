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

struct Joypad {}
impl Joypad {
    fn new() -> Self {
        Self {}
    }

    fn set_button(&mut self, button: &Button, is_down: bool) {}
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

pub struct GameBoy {
    joypad: Joypad,
    lcd: Lcd,
    memory: Memory,
    cpu: Cpu,
    registers: Registers,
    timer: Timer,
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

impl GameBoy {
    pub fn emulate_next_frame(&mut self) -> [u8; Lcd::PIXEL_COUNT] {
        // rwtodo run cpu etc
        self.cpu
            .execute_next_instruction(&mut self.registers, &mut self.memory);
        self.lcd.pixel_data()
    }

    // Inform the emulator of button state with this function. All buttons are up (unpressed) when emulation starts.
    pub fn set_button(&mut self, button: &Button, is_down: bool) {
        self.joypad.set_button(button, is_down);
    }

    fn request_interrupt(&mut self, interrupt_flag: u8) {
        // Combine with the existing request flags
        self.memory[INTERRUPT_FLAGS_ADDRESS] |= interrupt_flag;
        // Top 3 bits are always 1
        self.memory[INTERRUPT_FLAGS_ADDRESS] |= 0xe0;
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
