use crate::address;
use crate::make_bit;
use crate::Lcd;
use crate::Memory; // rwtodo: how is this working? shouldn't it be memory::Memory?

const LCDC_WINDOW_TILE_MAP_SELECT: u8 = 0x01 << 6;
const LCDC_WINDOW_ENABLED: u8 = 0x01 << 5;
const LCDC_BG_AND_WINDOW_TILE_DATA_SELECT: u8 = 0x01 << 4;
const LCDC_BG_TILE_MAP_SELECT: u8 = 0x01 << 3;
const LCDC_DOUBLE_HEIGHT_OBJECTS: u8 = 0x01 << 2;
const LCDC_OBJECTS_ENABLED: u8 = 0x01 << 1;
const LCDC_BG_AND_WINDOW_ENABLED: u8 = 0x01;

const NUM_BYTES_PER_TILE: i32 = 16;
const NUM_BYTES_PER_TILE_LINE: i32 = 2;
const NUM_TILES_PER_BG_LINE: u8 = 32;
const TILE_WIDTH: u8 = 8;
const TILE_HEIGHT: u8 = 8;
type TileLine = [u8; TILE_WIDTH as usize];

const SHADE_0_FLAG: u8 = 0x04;

// rwtodo: investigate how best to remove the unwrap()s in this file.
// rwtodo: Look at how to minimize the integer casts. I can probably just have most stuff as usize.

fn tile_line_ref(offset: usize, screen_line: &mut [u8; Lcd::WIDTH]) -> &mut TileLine {
    // Grab a &[] where the tile should be written to.
    let tile_line = &mut screen_line[offset..(offset + usize::from(TILE_WIDTH))];

    // Turn it into a fixed-sized slice.
    tile_line
        .try_into()
        .expect("Tile destination should be of size TILE_WIDTH=8")
}

pub struct Renderer {
    // rwtodo Do we really need a Renderer struct with state? or just shade state? I also don't like the naming of render::Renderer.
    shade_0: u8,
    shade_1: u8,
    shade_2: u8,
    shade_3: u8,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            shade_0: 0x00,
            shade_1: 0x00,
            shade_2: 0x00,
            shade_3: 0x00,
        }
    }

    fn set_palette(&mut self, palette: u8) {
        // SHADE_0_FLAG ensures shade_0 is unique, which streamlines the process of
        // shade-0-dependent blitting. The flag is discarded in the final step of the render.
        self.shade_0 = (palette & 0x03) | SHADE_0_FLAG;
        self.shade_1 = (palette & 0x0c) >> 2;
        self.shade_2 = (palette & 0x30) >> 4;
        self.shade_3 = (palette & 0xc0) >> 6;
    }

    // rwtodo: Unsure of the int types here.
    fn get_tile_line(
        &self,
        memory: &Memory,
        tile_bank_address: u16,
        tile_index: i16, // Must be signed!
        tile_line_index: u8,
        line_out: &mut TileLine,
    ) {
        // Convert to i32 to do arithmetic
        let tile_bank_address = tile_bank_address as i32;
        let tile_index = tile_index as i32;
        let tile_line_index = tile_line_index as i32;

        let tile_address = tile_bank_address + tile_index * NUM_BYTES_PER_TILE;
        let tile_line_address = tile_address + tile_line_index * NUM_BYTES_PER_TILE_LINE;
        let line_data = memory.read_u16(tile_line_address as u16);

        match line_data {
            0x0000 => {
                *line_out = [self.shade_0; TILE_WIDTH as usize];
                return;
            }
            0x00ff => {
                *line_out = [self.shade_1; TILE_WIDTH as usize];
                return;
            }
            0xff00 => {
                *line_out = [self.shade_2; TILE_WIDTH as usize];
                return;
            }
            0xffff => {
                *line_out = [self.shade_3; TILE_WIDTH as usize];
                return;
            }
            _ => (),
        }

        match line_data & 0x8080 {
            0x0000 => line_out[0] = self.shade_0,
            0x0080 => line_out[0] = self.shade_1,
            0x8000 => line_out[0] = self.shade_2,
            0x8080 => line_out[0] = self.shade_3,
            _ => unreachable!(),
        }

        match line_data & 0x4040 {
            0x0000 => line_out[1] = self.shade_0,
            0x0040 => line_out[1] = self.shade_1,
            0x4000 => line_out[1] = self.shade_2,
            0x4040 => line_out[1] = self.shade_3,
            _ => unreachable!(),
        }

        match line_data & 0x2020 {
            0x0000 => line_out[2] = self.shade_0,
            0x0020 => line_out[2] = self.shade_1,
            0x2000 => line_out[2] = self.shade_2,
            0x2020 => line_out[2] = self.shade_3,
            _ => unreachable!(),
        }

        match line_data & 0x1010 {
            0x0000 => line_out[3] = self.shade_0,
            0x0010 => line_out[3] = self.shade_1,
            0x1000 => line_out[3] = self.shade_2,
            0x1010 => line_out[3] = self.shade_3,
            _ => unreachable!(),
        }

        match line_data & 0x0808 {
            0x0000 => line_out[4] = self.shade_0,
            0x0008 => line_out[4] = self.shade_1,
            0x0800 => line_out[4] = self.shade_2,
            0x0808 => line_out[4] = self.shade_3,
            _ => unreachable!(),
        }

        match line_data & 0x0404 {
            0x0000 => line_out[5] = self.shade_0,
            0x0004 => line_out[5] = self.shade_1,
            0x0400 => line_out[5] = self.shade_2,
            0x0404 => line_out[5] = self.shade_3,
            _ => unreachable!(),
        }

        match line_data & 0x0202 {
            0x0000 => line_out[6] = self.shade_0,
            0x0002 => line_out[6] = self.shade_1,
            0x0200 => line_out[6] = self.shade_2,
            0x0202 => line_out[6] = self.shade_3,
            _ => unreachable!(),
        }

        match line_data & 0x0101 {
            0x0000 => line_out[7] = self.shade_0,
            0x0001 => line_out[7] = self.shade_1,
            0x0100 => line_out[7] = self.shade_2,
            0x0101 => line_out[7] = self.shade_3,
            _ => unreachable!(),
        }
    }

    fn get_bg_tile_line(
        &self,
        memory: &Memory,
        coord_x: u8,
        coord_y: u8,
        tile_map_address_space: u16,
        tile_data_bank_address: u16,
        tile_line_index: u8,
        line_out: &mut TileLine,
    ) {
        // Convert to i32 to do arithmetic
        let tile_map_address_space = tile_map_address_space as u16;

        let tile_map_index: u16 =
            u16::from(coord_x) + u16::from(coord_y) * u16::from(NUM_TILES_PER_BG_LINE);
        let address = tile_map_address_space + tile_map_index;
        let tile_data_index: u8 = memory.read(address); // rwtodo: this should be a direct read. consider having "direct_ref" and "direct_read" instead of the hand-wavy "direct_access".

        if tile_data_bank_address == 0x9000 {
            // bank 0x9000 uses signed addressing, hence the "as i8" below.
            self.get_tile_line(
                memory,
                tile_data_bank_address,
                (tile_data_index as i8).into(),
                tile_line_index,
                line_out,
            );
        } else {
            self.get_tile_line(
                memory,
                tile_data_bank_address,
                tile_data_index.into(),
                tile_line_index,
                line_out,
            );
        }
    }

    fn render_background_line(&mut self, memory: &Memory) -> [u8; Lcd::WIDTH] {
        let ly = memory.read(address::LCD_LY);
        let control = memory.read(address::LCD_CONTROL);
        let bg_scroll_y = memory.read(0xff42); // rwtodo const
        let bg_scroll_x = memory.read(0xff43); // rwtodo const

        let bg_y = ly + bg_scroll_y;

        let tilegrid_y = bg_y / TILE_HEIGHT;
        let tile_line_index = bg_y - tilegrid_y * TILE_HEIGHT;

        let tile_map_address_space: u16 = if (control & LCDC_BG_TILE_MAP_SELECT) != 0 {
            0x9c00
        } else {
            0x9800
        };

        let tile_data_address_space: u16 = if (control & LCDC_BG_AND_WINDOW_TILE_DATA_SELECT) != 0 {
            0x8000
        } else {
            0x9000
        };

        let mut screen_line: [u8; Lcd::WIDTH] = [42; Lcd::WIDTH]; // rwtodo 42

        for tilegrid_x in 0u8..NUM_TILES_PER_BG_LINE {
            let screen_x = tilegrid_x * TILE_WIDTH - bg_scroll_x;

            if screen_x <= Lcd::WIDTH as u8 - TILE_WIDTH {
                // Get the portion of the screen line where the tile should appear.
                let screen_x: usize = screen_x.into();
                let tile_line_dst = tile_line_ref(screen_x, &mut screen_line);

                self.get_bg_tile_line(
                    memory,
                    tilegrid_x,
                    tilegrid_y,
                    tile_map_address_space,
                    tile_data_address_space,
                    tile_line_index,
                    tile_line_dst,
                );
            } /*else if screen_x <= Lcd::WIDTH as u8 {

                  uint8_t tile_line[TILE_WIDTH];
                  get_bg_tile_line(tilegrid_x, tilegrid_y, tile_map_address_space, tile_data_address_space, tile_line_index, tile_line);

                  uint8_t tile_x;
                  for (tile_x = 0; tile_x < Lcd::WIDTH-screen_x; tile_x++) {
                      screen_line[screen_x + tile_x] = tile_line[tile_x];
                  }
              } else if screen_x - Lcd::WIDTH < TILE_WIDTH {
                  uint8_t pixel_count_to_render = screen_x - Lcd::WIDTH;
                  screen_x = 0;

                  uint8_t tile_line[TILE_WIDTH];
                  get_bg_tile_line(bg_scroll_x/TILE_WIDTH, tilegrid_y, tile_map_address_space, tile_data_address_space, tile_line_index, tile_line);

                  uint8_t tile_x;
                  for (tile_x = TILE_WIDTH - pixel_count_to_render; tile_x < TILE_WIDTH; tile_x++) {
                      screen_line[screen_x++] = tile_line[tile_x];
                  }
              }
              */
        }

        screen_line
    }

    fn render_objects(&mut self, screen_line: &mut [u8; Lcd::WIDTH], memory: &Memory) {
        let control = memory.read(address::LCD_CONTROL);
        let ly = memory.read(address::LCD_LY);

        let object_height = if control & LCDC_DOUBLE_HEIGHT_OBJECTS != 0 {
            16
        } else {
            8
        };

        for object_address in (0xfe00..=0xfe9c).step_by(4).rev() {
            let ly: i16 = ly.into();
            let translate_y = i16::from(memory.read(object_address)) - i16::from(TILE_HEIGHT) * 2;

            if ly >= translate_y && ly < translate_y + object_height {
                let translate_x: i16 =
                    i16::from(memory.read(object_address + 1)) - i16::from(TILE_WIDTH);

                // Ignore the lowest bit of the index if in double-height mode.
                let tile_data_index = if object_height > 8 {
                    memory.read(object_address + 2) & 0xfe
                } else {
                    memory.read(object_address + 2)
                };

                let object_flags = memory.read(object_address + 3);
                let choose_palette_1 = object_flags & make_bit(4) != 0;
                let flip_x = object_flags & make_bit(5) != 0;
                let flip_y = object_flags & make_bit(6) != 0;
                let behind_background = object_flags & make_bit(7) != 0;

                let object_palette = if choose_palette_1 {
                    memory.read(0xff49)
                } else {
                    memory.read(0xff48)
                };
                self.set_palette(object_palette);

                let mut tile_line: TileLine = [0; TILE_WIDTH as usize];
                {
                    // rwtodo: redo this whole block, could have esoteric i8/u8 casting.
                    // int8_t tile_line_index = flip_y ? (translate_y+7 - *ly) : *ly - translate_y; rwtodo
                    let tile_line_index = (ly - translate_y) as u8; // rwtodo wrong; see above
                    self.get_tile_line(
                        memory,
                        0x8000,
                        tile_data_index.into(),
                        tile_line_index,
                        &mut tile_line,
                    );
                }

                let screen_x_start = if translate_x < 0 { 0 } else { translate_x };
                let screen_x_end = translate_x + i16::from(TILE_WIDTH);

                if flip_x {
                    // uint8_t tile_pixel_index = translate_x < 0 ? (TILE_WIDTH-1)+translate_x : (TILE_WIDTH-1);

                    // if (behind_background) {
                    //     uint8_t screen_x;
                    //     for (screen_x = screen_x_start; screen_x < screen_x_end; screen_x++) {
                    //         uint8_t tile_pixel = tile_line[tile_pixel_index--];

                    //         if (!(tile_pixel & SHADE_0_FLAG) && screen_line[screen_x] & SHADE_0_FLAG) {
                    //             screen_line[screen_x] = tile_pixel;
                    //         }
                    //     }
                    // } else {
                    //     uint8_t screen_x;
                    //     for (screen_x = screen_x_start; screen_x < screen_x_end; screen_x++) {
                    //         uint8_t tile_pixel = tile_line[tile_pixel_index--];

                    //         if (!(tile_pixel & SHADE_0_FLAG)) {
                    //             screen_line[screen_x] = tile_pixel;
                    //         }
                    //     }
                    // }
                } else {
                    let mut tile_pixel_index = if translate_x < 0 {
                        -translate_x as usize
                    } else {
                        0
                    };

                    if behind_background {
                        for screen_x in screen_x_start..screen_x_end {
                            let tile_pixel = tile_line[tile_pixel_index];
                            tile_pixel_index += 1;

                            if (tile_pixel & SHADE_0_FLAG) == 0
                                && screen_line[screen_x as usize] & SHADE_0_FLAG != 0
                            {
                                screen_line[screen_x as usize] = tile_pixel;
                            }
                        }
                    } else {
                        for screen_x in screen_x_start..screen_x_end {
                            let tile_pixel = tile_line[tile_pixel_index];
                            tile_pixel_index += 1;

                            if (tile_pixel & SHADE_0_FLAG) == 0 {
                                screen_line[screen_x as usize] = tile_pixel;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn render_screen_line(&mut self, memory: &Memory) -> [u8; Lcd::WIDTH] {
        let lcd_control = memory.read(address::LCD_CONTROL);

        let mut screen_line = if (lcd_control & LCDC_BG_AND_WINDOW_ENABLED) != 0 {
            let bg_palette = memory.read(0xff47); // rwtodo const
            self.set_palette(bg_palette);

            self.render_background_line(memory)

            // if (lcd_control & LCDC_WINDOW_ENABLED) render_window_line(); rwtodo
        } else {
            // rwtodo: render white here.
            [0; Lcd::WIDTH]
        };

        if lcd_control & LCDC_OBJECTS_ENABLED != 0 {
            self.render_objects(&mut screen_line, memory);
        }

        // Convert from game boy 2-bit (with SHADE_0_FLAG) to target 8-bit.
        for pixel in &mut screen_line {
            // The '& 0x03' below is to discard the SHADE_0_FLAG bit, which has already served its purpose in render_objects(). rwtodo move this to render_objects()?
            let mut pixel_i16 = i16::from(*pixel & 0x03);

            // Flip the values and multiply to make white == 255.
            pixel_i16 -= 3;
            pixel_i16 *= -85;

            *pixel = pixel_i16 as u8;
        }

        screen_line
    }
}
