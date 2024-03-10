use crate::address;
use crate::interrupt;
use crate::Memory;

mod render;
use render::Renderer;

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
pub struct Lcd {
    renderer: Renderer,
    elapsed_cycles: u32, // rwtodo if this needs to be i32, fine.
}

impl Lcd {
    pub const WIDTH: usize = 160; // rwtodo would there be any benefit to these being u8?
    pub const HEIGHT: usize = 144; // rwtodo would there be any benefit to these being u8?
    pub const PIXEL_COUNT: usize = Lcd::WIDTH * Lcd::HEIGHT;

    pub fn new() -> Self {
        Self {
            // rwtodo: Not sure what the shades should initialize to.
            renderer: render::Renderer::new(),
            elapsed_cycles: 0,
        }
    }

    // rwtodo return an Option, "Some" if rendered?
    pub fn update(&mut self, newly_elapsed_cycles: u8, memory: &mut Memory) {
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
                    self.renderer.render_screen_line(memory);
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

    pub fn pixels(&self) -> &[u8; Lcd::PIXEL_COUNT] {
        &self.renderer.pixels
    }
}
