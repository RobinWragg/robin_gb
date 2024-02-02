use crate::Memory;

struct Renderer {
    // rwtodo Do we really need a Renderer struct with state? or just shade state? I also don't like the naming of render::Renderer.
    shade_0: u8,
    shade_1: u8,
    shade_2: u8,
    shade_3: u8,
}

impl Renderer {
    const SHADE_0_FLAG: u8 = 0x04;
    const NUM_BYTES_PER_TILE: i32 = 16;
    const NUM_BYTES_PER_TILE_LINE: i32 = 2;
    const NUM_TILES_PER_BG_LINE: i32 = 32;
    const TILE_WIDTH: usize = 8;

    fn set_palette(&mut self, palette: u8) {
        // SHADE_0_FLAG ensures shade_0 is unique, which streamlines the process of
        // shade-0-dependent blitting. The flag is discarded in the final step of the render.
        self.shade_0 = (palette & 0x03) | Self::SHADE_0_FLAG;
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
        line_out: &mut [u8; Self::TILE_WIDTH],
    ) {
        // Convert to i32 to do arithmetic
        let tile_bank_address = tile_bank_address as i32;
        let tile_index = tile_index as i32;
        let tile_line_index = tile_line_index as i32;

        let tile_address = tile_bank_address + tile_index * Self::NUM_BYTES_PER_TILE;
        let tile_line_address = tile_address + tile_line_index * Self::NUM_BYTES_PER_TILE_LINE;
        let line_data = memory.read_u16(tile_line_address as u16);

        match line_data {
            0x0000 => {
                *line_out = [self.shade_0; Self::TILE_WIDTH];
                return;
            }
            0x00ff => {
                *line_out = [self.shade_1; Self::TILE_WIDTH];
                return;
            }
            0xff00 => {
                *line_out = [self.shade_2; Self::TILE_WIDTH];
                return;
            }
            0xffff => {
                *line_out = [self.shade_3; Self::TILE_WIDTH];
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
        line_out: &mut [u8; Self::TILE_WIDTH],
    ) {
        // Convert to i32 to do arithmetic
        let coord_x = coord_x as i32;
        let coord_y = coord_y as i32;
        let tile_map_address_space = tile_map_address_space as i32;

        let tile_map_index = coord_x + coord_y * Self::NUM_TILES_PER_BG_LINE;
        let address = tile_map_address_space + tile_map_index;
        let tile_data_index: u8 = memory.read(address as u16); // rwtodo: this should be a direct read. consider having "direct_ref" and "direct_read" instead of the hand-wavy "direct_access".

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
}
