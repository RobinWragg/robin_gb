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

mod address {
    pub const LCD_CONTROL: u16 = 0xff40; // "LCDC"
    pub const LCD_STATUS: u16 = 0xff41;
    pub const LCD_LY: u16 = 0xff44;
    pub const LCD_LYC: u16 = 0xff45;
    pub const INTERRUPT_FLAGS: u16 = 0xff0f;
    pub const INTERRUPT_ENABLE: u16 = 0xffff;
}

struct Timer {
    cycles_since_last_tima_increment: u16,
    incrementer_every_cycle: u16,
}
impl Timer {
    const DIVIDER_ADDRESS: u16 = 0xff04; // "DIV"
    const COUNTER_ADDRESS: u16 = 0xff05; // "TIMA"
    const MODULO_ADDRESS: u16 = 0xff06; // "TMA"
    const CONTROL_ADDRESS: u16 = 0xff07; // "TAC"

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

/* TODO: "Each bit is set to 1 automatically when an internal signal from that subsystem goes from '0' to '1', it doesn't matter if the corresponding bit in IE is set. This is specially important in the case of LCD STAT interrupt, as it will be explained in the video controller chapter." */

/* TODO: "When using a status interrupt in DMG or in CGB in DMG mode, register IF should be set to 0 after the value of the STAT register is set. (In DMG, setting the STAT register value changes the value of the IF register, and an interrupt is generated at the same time as interrupts are enabled.)" */

/*
TODO:
"8.7. STAT Interrupt
This interrupt can be configured with register STAT.
The STAT IRQ is trigged by an internal signal.
This signal is set to 1 if:
    ( (LY = LYC) AND (STAT.ENABLE_LYC_COMPARE = 1) ) OR
    ( (ScreenMode = 0) AND (STAT.ENABLE_HBL = 1) ) OR
    ( (ScreenMode = 2) AND (STAT.ENABLE_OAM = 1) ) OR
    ( (ScreenMode = 1) AND (STAT.ENABLE_VBL || STAT.ENABLE_OAM) ) -> Not only VBL!!??
-If LCD is off, the signal is 0.
-It seems that this interrupt needs less time to execute in DMG than in CGB? -DMG bug?"
*/
struct Lcd {
    elapsed_cycles: u32, // rwtodo if this needs to be i32, fine.
}
impl Lcd {
    const WIDTH: usize = 160; // rwtodo would there be any benefit to these being u8?
    const HEIGHT: usize = 144; // rwtodo would there be any benefit to these being u8?
    const PIXEL_COUNT: usize = Lcd::WIDTH * Lcd::HEIGHT;
    fn new() -> Self {
        Self { elapsed_cycles: 0 }
    }

    // rwtodo return an Option, "Some" if rendered?
    fn update(&mut self, newly_elapsed_cycles: u8, memory: &mut Memory) {
        const LCDC_ENABLED_BIT: u8 = 0x01 << 7;
        const NUM_CYCLES_PER_FULL_SCREEN_REFRESH: u32 = 70224; // Approximately 59.7275Hz
        const NUM_CYCLES_PER_LY_INCREMENT: u32 = 456;
        const LY_VBLANK_ENTRY_VALUE: u8 = 144;
        const LY_MAXIMUM_VALUE: u8 = 154;
        const MODE_0_CYCLE_DURATION: u32 = 204;
        const MODE_1_CYCLE_DURATION: u32 = 4560;
        const MODE_2_CYCLE_DURATION: u32 = 80;
        const MODE_3_CYCLE_DURATION: u32 = 172;

        if (memory.read(address::LCD_CONTROL) & LCDC_ENABLED_BIT) == 0 {
            // Bit 7 of the LCD control register is 0, so the LCD is switched off.
            // LY, the mode, and the LYC=LY flag should all be 0.
            memory.write(address::LCD_LY, 0x00);
            memory.write(address::LCD_STATUS, 0xf8);
            return;
        }

        // rwtodo I can do into() here to add u8 to i32, why couldn't I do it elsewhere?
        self.elapsed_cycles += u32::from(newly_elapsed_cycles);

        // Set LY.
        if self.elapsed_cycles >= NUM_CYCLES_PER_LY_INCREMENT {
            self.elapsed_cycles -= NUM_CYCLES_PER_LY_INCREMENT;
            // rwtodo: Won't this logic cause LY to skip its maximum value?
            let ly = memory.direct_access(address::LCD_LY);
            *ly += 1;
            if *ly >= LY_MAXIMUM_VALUE {
                *ly = 0;
            }
        }

        // Handle LYC.
        if memory.read(address::LCD_LY) == memory.read(address::LCD_LYC) {
            *memory.direct_access(address::LCD_STATUS) |= 0x04;
            if memory.read(address::LCD_STATUS) & 0x40 != 0 {
                interrupt::make_request(interrupt::FLAG_LCD_STAT, memory);
            }
        } else {
            *memory.direct_access(address::LCD_STATUS) &= !0x04;
        }

        // Set the mode.
        let previous_mode: u8;
        {
            let status = memory.direct_access(address::LCD_STATUS);
            previous_mode = *status & 0x03; // Get lower 2 bits only.
            *status &= 0xfc; // Discard the old mode.
        }

        if memory.read(address::LCD_LY) < LY_VBLANK_ENTRY_VALUE {
            /*
            Approx mode graph:
            Mode 2  2_____2_____2_____2_____2_____2___________________2____
            Mode 3  _33____33____33____33____33____33__________________3___
            Mode 0  ___000___000___000___000___000___000________________000
            Mode 1  ____________________________________11111111111111_____
            */

            if self.elapsed_cycles >= MODE_2_CYCLE_DURATION + MODE_3_CYCLE_DURATION {
                *memory.direct_access(address::LCD_STATUS) |= 0x00; // H-blank.

                if previous_mode != 0x00 && (memory.read(address::LCD_STATUS) & 0x08) != 0 {
                    interrupt::make_request(interrupt::FLAG_LCD_STAT, memory);
                }
            } else if self.elapsed_cycles >= MODE_2_CYCLE_DURATION {
                // Declare that the LCD is reading from both OAM and VRAM.
                *memory.direct_access(address::LCD_STATUS) |= 0x03;

                if previous_mode != 0x03 {
                    panic!(); // rwtodo robingb_render_screen_line();
                }
            } else {
                // Declare that the LCD is reading from OAM.
                *memory.direct_access(address::LCD_STATUS) |= 0x02;

                if previous_mode != 0x02 && (memory.read(address::LCD_STATUS) & 0x20) != 0 {
                    interrupt::make_request(interrupt::FLAG_LCD_STAT, memory);
                }
            }
        } else {
            *memory.direct_access(address::LCD_STATUS) |= 0x01; // V-blank.

            if previous_mode != 0x01 {
                interrupt::make_request(interrupt::FLAG_VBLANK, memory);

                if (memory.read(address::LCD_STATUS) & 0x10) != 0 {
                    interrupt::make_request(interrupt::FLAG_LCD_STAT, memory);
                }
            }
        }
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
        let previous_lcd_ly = *self.memory.direct_access(address::LCD_LY);

        let mut total_elapsed_cycles_this_h_blank: u32 = 0;

        // Execute instructions until a horizontal-blank occurs.
        while *self.memory.direct_access(address::LCD_LY) == previous_lcd_ly {
            let elapsed_cycles = self.cpu.execute_next_instruction(&mut self.memory);

            self.lcd.update(elapsed_cycles, &mut self.memory);
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
