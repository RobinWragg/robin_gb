#![allow(unused_variables)]
#![allow(dead_code)]

mod cpu;
mod memory;
mod render;

use memory::Memory;

mod interrupt {
    pub const FLAG_VBLANK: u8 = 0x01;
    pub const FLAG_LCD_STAT: u8 = 0x02;
    pub const FLAG_TIMER: u8 = 0x04;
    pub const FLAG_SERIAL: u8 = 0x08;
    pub const FLAG_JOYPAD: u8 = 0x10;

    pub const FLAGS_ADDRESS: u16 = 0xff0f; // rwtodo move to address module
    pub const ENABLE_ADDRESS: u16 = 0xffff; // rwtodo move to address module
}

mod address {
    pub const LCD_CONTROL: u16 = 0xff40; // "LCDC"
    pub const LCD_STATUS: u16 = 0xff41;
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

pub enum Button {
    A,
    B,
    START,
    SELECT,
}

pub struct GameBoy {
    lcd: Lcd,
    memory: Memory,
    cpu: cpu::Cpu,
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
        cpu: cpu::Cpu::new(),
        timer,
    })
}
