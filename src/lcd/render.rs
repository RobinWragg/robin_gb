use crate::address;
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
const TILE_WIDTH: usize = 8; // rwtodo usize? u8?
const TILE_HEIGHT: usize = 8; // rwtodo usize? u8?

const SHADE_0_FLAG: u8 = 0x04;

// rwtodo: investigate how best to remove the unwrap()s in this file.
// rwtodo: Look at how to minimize the integer casts. I can probably just have most stuff as usize.

pub struct Renderer {
    // rwtodo Do we really need a Renderer struct with state? or just shade state? I also don't like the naming of render::Renderer.
    shade_0: u8,
    shade_1: u8,
    shade_2: u8,
    shade_3: u8,
    pub pixels: [u8; Lcd::PIXEL_COUNT],
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            shade_0: 0x00,
            shade_1: 0x00,
            shade_2: 0x00,
            shade_3: 0x00,
            pixels: [42; Lcd::PIXEL_COUNT], // rwtodo: probably want to init it to 0xff.
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
        line_out: &mut [u8; TILE_WIDTH],
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
                *line_out = [self.shade_0; TILE_WIDTH];
                return;
            }
            0x00ff => {
                *line_out = [self.shade_1; TILE_WIDTH];
                return;
            }
            0xff00 => {
                *line_out = [self.shade_2; TILE_WIDTH];
                return;
            }
            0xffff => {
                *line_out = [self.shade_3; TILE_WIDTH];
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
        line_out: &mut [u8; TILE_WIDTH],
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

    fn render_background_line(&mut self, memory: &Memory) {
        let ly = memory.read(address::LCD_LY);
        let control = memory.read(address::LCD_CONTROL);
        let bg_scroll_y = memory.read(0xff42); // rwtodo const
        let bg_scroll_x = memory.read(0xff43); // rwtodo const

        let bg_y = ly + bg_scroll_y;

        let tilegrid_y = bg_y / (TILE_HEIGHT as u8);
        let tile_line_index = bg_y - tilegrid_y * (TILE_HEIGHT as u8);

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

        // Get the slice of the screen representing the current horizontal line.
        let screen_line;
        {
            let first_pixel_of_line = usize::from(ly) * Lcd::WIDTH;
            screen_line = &self.pixels[first_pixel_of_line..(first_pixel_of_line + Lcd::WIDTH)];
        }

        for tilegrid_x in 0u8..NUM_TILES_PER_BG_LINE {
            let screen_x = tilegrid_x * (TILE_WIDTH as u8) - bg_scroll_x;

            if screen_x <= (Lcd::WIDTH - TILE_WIDTH) as u8 {
                // Get the portion of the screen line where the tile should appear.
                let screen_x: usize = screen_x.into();
                let tile_line_dst: &mut [u8; TILE_WIDTH] = &mut screen_line
                    [screen_x..(screen_x + TILE_WIDTH)]
                    .try_into()
                    .expect("Tile destination should be of size TILE_WIDTH=8");

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
    }

    pub fn render_screen_line(&mut self, memory: &Memory) {
        let lcd_control = memory.read(address::LCD_CONTROL);

        if (lcd_control & LCDC_BG_AND_WINDOW_ENABLED) != 0 {
            let bg_palette = memory.read(0xff47); // rwtodo const
            self.set_palette(bg_palette);

            self.render_background_line(memory);

            // if (lcd_control & LCDC_WINDOW_ENABLED) render_window_line();
        }

        // rwtodo: only implemented background rendering for now.
    }
}