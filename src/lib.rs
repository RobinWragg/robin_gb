#![allow(unused_variables)]
#![allow(dead_code)]

mod cpu;
mod memory;
mod render;

use memory::Memory;

// rwtodo: interrupt responsibility is shared here and in the Cpu impl, which is icky.
mod interrupt {
    use crate::address;
    use crate::memory::Memory;

    pub const FLAG_VBLANK: u8 = 0x01;
    pub const FLAG_LCD_STAT: u8 = 0x02;
    pub const FLAG_TIMER: u8 = 0x04;
    pub const FLAG_SERIAL: u8 = 0x08;
    pub const FLAG_JOYPAD: u8 = 0x10;

    pub fn make_request(interrupt_flag: u8, memory: &mut Memory) {
        // Combine with the existing request flags
        // rwtodo can do this all in one call
        *memory.direct_access(address::INTERRUPT_FLAGS) |= interrupt_flag;
        // Top 3 bits are always 1
        *memory.direct_access(address::INTERRUPT_FLAGS) |= 0xe0; // rwtodo is there binary syntax for this?
    }
}

// rwtodo: still not sure if it's most ergonomic/safe for these to be usize, or whether they should be u16 and just do .into() when u16 won't suffice..
mod address {
    pub const LCD_CONTROL: usize = 0xff40; // "LCDC"
    pub const LCD_STATUS: usize = 0xff41;
    pub const LCD_LY: usize = 0xff44;
    pub const LCD_LYC: usize = 0xff45;
    pub const INTERRUPT_FLAGS: u16 = 0xff0f;
    pub const INTERRUPT_ENABLE: u16 = 0xffff;
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

    fn update(&mut self, elapsed_cycles: u8) {
        panic!();
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

    fn update(&mut self, elapsed_cycles: u8) {
        panic!();
    }

    fn pixel_data(&self) -> [u8; Lcd::PIXEL_COUNT] {
        [127; Lcd::PIXEL_COUNT] // rwtodo just returning grey for now
    }
}

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
    // rwtodo: returns true if not vblank. not a fan. enum?
    fn emulate_next_lcd_line(&mut self) -> bool {
        let previous_lcd_ly = *self.memory.direct_access(address::LCD_LY as u16);

        let mut total_elapsed_cycles_this_h_blank: u32 = 0;

        // Execute instructions until a horizontal-blank occurs.
        while *self.memory.direct_access(address::LCD_LY as u16) == previous_lcd_ly {
            let elapsed_cycles = self.cpu.execute_next_instruction(&mut self.memory);

            self.lcd.update(elapsed_cycles);
            self.timer.update(elapsed_cycles);

            // rwtodo discuss why I can't do .into() here.
            total_elapsed_cycles_this_h_blank += u32::from(elapsed_cycles);
        }

        // rwtodo: update audio right here.

        // Return false if LY has advanced past the vblank stage.
        previous_lcd_ly < 144
    }

    pub fn emulate_next_frame(&mut self) -> [u8; Lcd::PIXEL_COUNT] {
        // rwtodo run cpu etc

        // Call the function until the vblank phase is exited.
        while self.emulate_next_lcd_line() == false {}

        // Call the function until the vblank phase is entered again.
        while self.emulate_next_lcd_line() != true {}

        // The screen has now been fully updated.
        self.lcd.pixel_data()
    }

    // Inform the emulator of button state with this function. All buttons are up (unpressed) when emulation starts.
    pub fn set_button(&mut self, button: &Button, is_down: bool) {
        // rwtodo
    }
}

pub fn load_rom_file(rom_file_data: &[u8]) -> Result<GameBoy, std::io::Error> {
    let mut memory = Memory::new(&rom_file_data);
    let timer = Timer::new(&mut memory);

    Ok(GameBoy {
        lcd: Lcd::new(),
        memory,
        cpu: cpu::Cpu::new(),
        timer,
    })
}
