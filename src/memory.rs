use crate::address;
use crate::interrupt;
use crate::Joypad;

fn make_u16(a: u8, b: u8) -> u16 {
    let byte_0 = a as u16;
    let byte_1 = b as u16;
    (byte_0 << 8) | byte_1
}

pub struct Memory {
    bytes: [u8; Self::ADDRESS_SPACE_SIZE],
    joypad: Joypad,                  // rwtodo: move back to GameBoy struct.
    current_switchable_rom_bank: u8, // rwtodo rename to "active..."
}

impl Memory {
    const ADDRESS_SPACE_SIZE: usize = 1024 * 64;
    const ROM_BANK_SIZE: usize = 16384; // 16kB

    pub fn new() -> Self {
        let mut bytes = [0; Self::ADDRESS_SPACE_SIZE];

        // Set all nonzero bytes.
        bytes[0xff10] = 0x80;
        bytes[0xff11] = 0xbf;
        bytes[0xff12] = 0xf3;
        bytes[0xff14] = 0xbf;
        bytes[0xff16] = 0x3f;
        bytes[0xff19] = 0xbf;
        bytes[0xff1a] = 0x7f;
        bytes[0xff1b] = 0xff;
        bytes[0xff1c] = 0x9f;
        bytes[0xff1e] = 0xbf;
        bytes[0xff00] = 0xff;
        bytes[0xff20] = 0xff;
        bytes[0xff23] = 0xbf;
        bytes[0xff24] = 0x77;
        bytes[0xff25] = 0xf3;
        bytes[0xff26] = 0xf1; // NOTE: This is different for Game Boy Color etc.
        bytes[address::LCD_CONTROL] = 0x91;
        bytes[address::LCD_STATUS] = 0x85;
        bytes[0xff47] = 0xfc;
        bytes[0xff48] = 0xff;
        bytes[0xff49] = 0xff;
        bytes[address::INTERRUPT_FLAGS] = 0xe1; // TODO: Might be acceptable for this to be 0xe0

        let new_mem = Self {
            bytes,
            joypad: Joypad::new(),
            current_switchable_rom_bank: 1,
        };

        // init_cart_state(); // rwtodo: Lots to do to get this working.

        new_mem
    }

    pub fn direct_access(&mut self, address: u16) -> &mut u8 {
        &mut self.bytes[address as usize]
    }

    fn get_joypad_register_write_result(&mut self, mut register_value: u8) -> u8 {
        const ACTION_BUTTON_REQUEST: u8 = 0x20;
        const DIRECTION_BUTTON_REQUEST: u8 = 0x10;

        register_value |= 0xc0; // bits 6 and 7 are always 1.
        register_value |= 0x0f; // unpressed buttons are 1.

        if (register_value & ACTION_BUTTON_REQUEST) == 0x00 {
            register_value &= self.joypad.action_buttons;
            self.request_interrupt(interrupt::FLAG_JOYPAD);
        }

        if (register_value & DIRECTION_BUTTON_REQUEST) == 0x00 {
            register_value &= self.joypad.direction_buttons;
            self.request_interrupt(interrupt::FLAG_JOYPAD);
        }

        return register_value;
    }

    pub fn write(&mut self, address: u16, value: u8) {
        // rwtodo: convert to match statement?
        if address < 0x8000 {
            // perform_cart_control(address, value); rwtodo
        } else if address == 0xff00 {
            // rwtodo: label 0xff00 as a constant?
            self.bytes[address as usize] = self.get_joypad_register_write_result(value);
        } else if address == 0xff04 {
            // rwtodo: label 0xff04 as a constant?
            self.bytes[address as usize] = 0x00; // Reset timer DIV register. rwtodo: move this responibility into Timer struct
        } else if address == 0xff46 {
            // Perform OAM DMA transfer. rwtodo: copying twice here, unless the compiler optimizes it out. Use copy_within on self.memory directly.
            const SIZE_OF_TRANSFER: usize = 160;

            let mut bytes_to_transfer: [u8; SIZE_OF_TRANSFER] = [0; SIZE_OF_TRANSFER];

            {
                let src_range_start: usize = (value as usize) * 0x100;
                let src_range_end: usize = src_range_start + SIZE_OF_TRANSFER;
                let src_slice = &self.bytes[src_range_start..src_range_end];
                bytes_to_transfer.copy_from_slice(src_slice);
            }

            let dst_range_start: usize = 0xfe00;
            let dst_range_end: usize = dst_range_start + SIZE_OF_TRANSFER;
            let dst_slice = &mut self.bytes[dst_range_start..dst_range_end];

            dst_slice.copy_from_slice(&bytes_to_transfer);
        } else {
            self.bytes[address as usize] = value;

            // Memory is duplicated when writing to these registers
            if address >= 0xc000 && address < 0xde00 {
                let echo_address = address - 0xc000 + 0xe000;
                self.bytes[echo_address as usize] = value;
            } else if address >= 0xe000 && address < 0xfe00 {
                let echo_address = address - 0xe000 + 0xc000;
                self.bytes[echo_address as usize] = value;
            }

            // rwtodo: implement cart_state stuff so we can do this.
            // rwtodo: Also handle the below for MBC3.
            // if cart_state.mbc_type == MBC_1 && address >= 0xa000 && address < 0xc000 {
            //     // RAM was written to.
            //     cart_state.save_file_is_outdated = true;
            // }
        }
    }

    fn write_u16(&mut self, address: u16, value: u16) {
        let bytes = value.to_le_bytes();
        self.write(address, bytes[0]);
        self.write(address + 1, bytes[1]);
    }

    pub fn read(&self, address: u16) -> u8 {
        // rwtodo rom banks
        // if address >= 0x4000 && address < 0x8000 {
        //     return robingb_romb_read_switchable_bank(address);
        // } else {
        self.bytes[address as usize]
        // }
    }

    pub fn read_u16(&self, address: u16) -> u16 {
        make_u16(self.read(address), self.read(address + 1))
    }

    // rwtodo move to the interrupt module? bonus: FLAGS_ADDRESS can be private.
    fn request_interrupt(&mut self, interrupt_flag: u8) {
        // Combine with the existing request flags
        // rwtodo can do this all in one call
        self.bytes[address::INTERRUPT_FLAGS] |= interrupt_flag;
        // Top 3 bits are always 1
        self.bytes[address::INTERRUPT_FLAGS] |= 0xe0; // rwtodo is there binary syntax for this?
    }

    fn init_first_rom_banks(&mut self, file_data: &[u8]) {
        let banks_src = &file_data[..(Self::ROM_BANK_SIZE * 2)];
        let banks_dst = &mut self.bytes[..(Self::ROM_BANK_SIZE * 2)];
        banks_dst.copy_from_slice(banks_src);
        self.current_switchable_rom_bank = 1;
    }
}
