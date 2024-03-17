#![allow(dead_code)] // rwtodo: remove.

mod cpu;
mod lcd;
mod memory;

use cpu::Cpu;
use lcd::Lcd;
use memory::Memory;

fn make_u16(lower_nibble: u8, upper_nibble: u8) -> u16 {
    let lower_nibble = u16::from(lower_nibble);
    let upper_nibble = u16::from(upper_nibble);
    lower_nibble | (upper_nibble << 8)
}

fn bit(index: u8) -> u8 {
    0x01 << index
}

// rwtodo: interrupt responsibility is shared here and in the Cpu impl, which is icky.
mod interrupt {
    use crate::address;
    use crate::Memory;

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

mod address {
    pub const LCD_CONTROL: u16 = 0xff40; // "LCDC"
    pub const LCD_STATUS: u16 = 0xff41;
    pub const LCD_LY: u16 = 0xff44;
    pub const LCD_LYC: u16 = 0xff45;
    pub const INTERRUPT_FLAGS: u16 = 0xff0f;
    pub const INTERRUPT_ENABLE: u16 = 0xffff;
}

struct Timer {
    cycles_since_last_tima_increment: u16, // rwtodo: rename to cycles_since_last_counter_increment? search everywhere for "tima".
    incrementer_every_cycle: u16,
}
impl Timer {
    // rwtodo: put in address module?
    const DIVIDER_ADDRESS: u16 = 0xff04; // "DIV"
    const COUNTER_ADDRESS: u16 = 0xff05; // "TIMA"
    const MODULO_ADDRESS: u16 = 0xff06; // "TMA"
    const CONTROL_ADDRESS: u16 = 0xff07; // "TAC"

    const MINIMUM_CYCLES_PER_COUNTER_INCREMENT: u16 = 16;

    fn new(memory: &mut Memory) -> Self {
        let new_timer = Self {
            cycles_since_last_tima_increment: 0,
            incrementer_every_cycle: 0xabcc,
        };

        let div_byte = new_timer.incrementer_every_cycle.to_le_bytes()[1];
        assert!(div_byte == 0xab);
        memory.write(Self::DIVIDER_ADDRESS, div_byte);
        assert!(memory.read(Self::COUNTER_ADDRESS) == 0x00);
        assert!(memory.read(Self::MODULO_ADDRESS) == 0x00);
        assert!(memory.read(Self::CONTROL_ADDRESS) == 0x00);

        new_timer
    }

    fn update(&mut self, elapsed_cycles: u8, memory: &mut Memory) {
        let elapsed_cycles: u16 = elapsed_cycles.into();
        let control_value = memory.read(Self::CONTROL_ADDRESS);

        // Update the incrementer and keep the DIV register in sync.
        self.incrementer_every_cycle = self.incrementer_every_cycle.wrapping_add(elapsed_cycles);
        let div_byte = self.incrementer_every_cycle.to_le_bytes()[1];
        memory.write(Self::DIVIDER_ADDRESS, div_byte);

        // If the timer is enabled, update TIMA and potentially request an interrupt.
        if control_value & 0x04 != 0 {
            self.cycles_since_last_tima_increment += elapsed_cycles;

            if self.cycles_since_last_tima_increment >= Self::MINIMUM_CYCLES_PER_COUNTER_INCREMENT {
                // Calculate actual cycles per TIMA increment from the lowest 2 bits.
                let cycles_per_tima_increment = match control_value & 0x03 {
                    0x00 => 1024,
                    0x01 => 16,
                    0x02 => 64,
                    0x03 => 256,
                    _ => unreachable!(),
                };

                if self.cycles_since_last_tima_increment >= cycles_per_tima_increment {
                    let counter = memory.read(Self::COUNTER_ADDRESS);
                    let previous_tima_value = counter;
                    memory.write(Self::COUNTER_ADDRESS, counter + 1);

                    // Check for overflow.
                    if previous_tima_value > counter {
                        memory.write(Self::CONTROL_ADDRESS, memory.read(Self::MODULO_ADDRESS));
                        interrupt::make_request(interrupt::FLAG_TIMER, memory);
                    }

                    self.cycles_since_last_tima_increment -= cycles_per_tima_increment;
                }
            }
        }
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
    cpu: Cpu,
    timer: Timer,
}

impl GameBoy {
    pub fn new(rom_file_data: &[u8]) -> GameBoy {
        let mut memory = Memory::new(&rom_file_data);
        let timer = Timer::new(&mut memory);

        GameBoy {
            lcd: Lcd::new(),
            memory,
            cpu: Cpu::new(),
            timer,
        }
    }

    // rwtodo: returns true if not vblank. not a fan. enum?
    fn emulate_next_lcd_line(&mut self) -> bool {
        let previous_lcd_ly = *self.memory.direct_access(address::LCD_LY);

        let mut total_elapsed_cycles_this_h_blank: u32 = 0;

        // Execute instructions until a horizontal-blank occurs.
        while *self.memory.direct_access(address::LCD_LY) == previous_lcd_ly {
            let elapsed_cycles = self.cpu.execute_next_instruction(&mut self.memory);

            self.lcd.update(elapsed_cycles, &mut self.memory);
            self.timer.update(elapsed_cycles, &mut self.memory);

            total_elapsed_cycles_this_h_blank += u32::from(elapsed_cycles);
        }

        // rwtodo: update audio right here.

        // Return false if LY has advanced past the vblank stage.
        previous_lcd_ly < 144
    }

    pub fn emulate_next_frame(&mut self) -> &[u8; Lcd::PIXEL_COUNT] {
        // rwtodo run cpu etc

        // Call the function until the vblank phase is exited.
        while self.emulate_next_lcd_line() == false {}

        // Call the function until the vblank phase is entered again.
        while self.emulate_next_lcd_line() == true {}

        // The screen has now been fully updated.
        self.lcd.pixels()
    }

    // Inform the emulator of button state with this function. All buttons are up (unpressed) when emulation starts.
    pub fn set_button(&mut self, button: &Button, is_down: bool) {
        // rwtodo
    }
}
