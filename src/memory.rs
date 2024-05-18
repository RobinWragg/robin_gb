use num_enum::TryFromPrimitive;

// rwtodo: ensure LY is never written to by the game.

use crate::address;
use crate::interrupt;
use crate::make_u16;
use crate::Joypad;

const ROM_BANK_SIZE: usize = 16384; // 16kB // rwtodo rename to just BANK_SIZE?

#[derive(TryFromPrimitive)]
#[repr(u8)]
enum CartKind {
    RomOnly = 0x00,
    Mbc1 = 0x01,
    Mbc1Ram = 0x02,
    Mbc1RamBattery = 0x03,
    Mbc2 = 0x05,
    Mbc2Battery = 0x06,
    Ram = 0x08,
    RamBattery = 0x09,
    Mmm01 = 0x0b,
    Mmm01Ram = 0x0c,
    Mmm01RamBattery = 0x0d,
    Mbc3TimerBattery = 0x0f,
    Mbc3TimerRamBattery = 0x10,
    Mbc3 = 0x11,
    Mbc3Ram = 0x12,
    Mbc3RamBattery = 0x13,
    Mbc4 = 0x15,
    Mbc4Ram = 0x16,
    Mbc4RamBattery = 0x17,
    Mbc5 = 0x19,
    Mbc5Ram = 0x1a,
    Mbc5RamBattery = 0x1b,
    Mbc5Rumble = 0x1c,
    Mbc5RumbleRam = 0x1d,
    Mbc5RumbleRamBattery = 0x1e,
    PocketCamera = 0xfc,
    BandaiTama5 = 0xfd,
    HuC3 = 0xfe,
    HuC1RamBattery = 0xff,
}

#[derive(PartialEq)]
enum Mbc {
    None,
    Mbc1,
    Mbc2,
    Mbc3,
}

type CachedBank = [u8; ROM_BANK_SIZE];

mod bank_ranges {
    use std::ops::RangeInclusive;
    pub const ROM_0: RangeInclusive<u16> = 0x0000..=0x3fff;
    pub const ROM_1: RangeInclusive<u16> = 0x4000..=0x7fff;
    pub const VIDEO_RAM: RangeInclusive<u16> = 0x8000..=0x9fff;
    pub const EXTERNAL_RAM: RangeInclusive<u16> = 0xa000..=0xbfff;
    pub const WORK_RAM_STATIC: RangeInclusive<u16> = 0xc000..=0xcfff;
    pub const WORK_RAM_SWITCHABLE: RangeInclusive<u16> = 0xd000..=0xdfff;
    pub const ECHO_RAM: RangeInclusive<u16> = 0xe000..=0xfdff;
    pub const OBJECT_ATTRIBUTES: RangeInclusive<u16> = 0xfe00..=0xfe9f;
    pub const PROHIBITED: RangeInclusive<u16> = 0xfea0..=0xfeff;
    pub const IO_REGISTERS: RangeInclusive<u16> = 0xff00..=0xff7f;
    pub const HIGH_RAM: RangeInclusive<u16> = 0xff80..=0xfffe;
    pub const INTERRUPT_ENABLE: RangeInclusive<u16> = 0xffff..=0xffff;
}

struct Banker {
    mbc: Mbc,
    has_ram: bool, // rwtodo do I really need this as well as ram_bank_count?
    ram_bank_count: u8,
    ram_is_enabled: bool,
    active_switchable_rom_bank: u8,
    cached_banks: Vec<CachedBank>,
}

impl Banker {
    // rwtodo: maybe Memory API access to banks should redirect through the Banker, and Banker could hold the currently active bank data, then I wouldn't need this ugliness where the Banker is partially responsible for data living outside the Banker.
    fn new(bank_slots_in_memory: &mut [u8; ROM_BANK_SIZE * 2], file_data: &[u8]) -> Banker {
        const CART_KIND_ADDRESS: usize = 0x0147;

        let cart_kind =
            CartKind::try_from(file_data[CART_KIND_ADDRESS]).expect("Couldn't get cart kind");
        let mbc = Self::detect_mbc(cart_kind);
        let cached_banks = Self::load_cache(file_data);

        let banker = Banker {
            mbc,
            has_ram: false,        // rwtodo
            ram_bank_count: 0,     // rwtodo
            ram_is_enabled: false, // rwtodo
            active_switchable_rom_bank: 1,
            cached_banks,
        };

        // Init first 2 banks
        let banks = &file_data[..(ROM_BANK_SIZE * 2)];
        bank_slots_in_memory.copy_from_slice(banks);

        banker
    }

    fn load_cache(file_data: &[u8]) -> Vec<CachedBank> {
        const BANK_COUNT_ID_ADDRESS: usize = 0x0148;
        let bank_count_id = file_data[BANK_COUNT_ID_ADDRESS];

        let mut cached_banks = vec![];
        let total_bank_count: usize;

        if bank_count_id <= 0x08 {
            total_bank_count = 2 << bank_count_id;
        } else {
            match bank_count_id {
                0x52 => total_bank_count = 72,
                0x53 => total_bank_count = 80,
                0x54 => total_bank_count = 96,
                _ => panic!("Unrecognized bank count ID"),
            }
        }

        if total_bank_count > 2 {
            let cached_bank_count = total_bank_count - 2;

            println!("Loading the remaining {} ROM banks...\n", cached_bank_count);

            let file_offset: usize = ROM_BANK_SIZE * 2; // Offset of 2, as the first 2 banks are already loaded

            for bank_index in 0..cached_bank_count {
                let bank_start = file_offset + ROM_BANK_SIZE * bank_index;
                let mut bank_data: CachedBank = [0; ROM_BANK_SIZE];
                bank_data.copy_from_slice(&file_data[bank_start..bank_start + ROM_BANK_SIZE]);
                cached_banks.push(bank_data);
            }

            assert!(cached_banks.len() == total_bank_count - 2);
            println!("Done");
        }

        cached_banks
    }

    fn detect_mbc(cart: CartKind) -> Mbc {
        use CartKind::*;

        match cart {
            RomOnly | Ram | RamBattery => Mbc::None,
            Mbc1 | Mbc1Ram | Mbc1RamBattery => Mbc::Mbc1,
            Mbc2 | Mbc2Battery => Mbc::Mbc2,
            Mbc3 | Mbc3Ram | Mbc3RamBattery | Mbc3TimerBattery | Mbc3TimerRamBattery => Mbc::Mbc3,
            _ => panic!("Unsupported CartKind value"),
        }
    }
}

pub struct Memory {
    bytes: [u8; Self::ADDRESS_SPACE_SIZE],
    joypad: Joypad, // rwtodo: move back to GameBoy struct.
    banker: Banker,
    print_serial: bool,
}

impl Memory {
    const ADDRESS_SPACE_SIZE: usize = 1024 * 64;

    // rwtodo: Supply rom file data here so we can initialise the banks at the same time as the rest of the memory.
    pub fn new(file_data: &[u8]) -> Self {
        let mut bytes: [u8; Self::ADDRESS_SPACE_SIZE] = [0; Self::ADDRESS_SPACE_SIZE];

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
        bytes[usize::from(address::LCD_CONTROL)] = 0x91;
        bytes[usize::from(address::LCD_STATUS)] = 0x85;
        bytes[0xff47] = 0xfc;
        bytes[0xff48] = 0xff;
        bytes[0xff49] = 0xff;
        bytes[usize::from(address::INTERRUPT_FLAGS)] = 0xe1; // TODO: Might be acceptable for this to be 0xe0

        let bank_slots = &mut bytes[..ROM_BANK_SIZE * 2];
        let banker = Banker::new(bank_slots.try_into().unwrap(), file_data);

        let new_mem = Self {
            bytes,
            joypad: Joypad::new(),
            banker,
            print_serial: false,
        };

        new_mem
    }

    pub fn redirect_serial_to_stdout(&mut self, redirect: bool) {
        self.print_serial = redirect
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
            interrupt::make_request(interrupt::FLAG_JOYPAD, self);
        }

        if (register_value & DIRECTION_BUTTON_REQUEST) == 0x00 {
            register_value &= self.joypad.direction_buttons;
            interrupt::make_request(interrupt::FLAG_JOYPAD, self);
        }

        return register_value;
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            x if bank_ranges::ROM_0.contains(&x) || bank_ranges::ROM_1.contains(&x) => { // perform_cart_control(address, value); rwtodo
            }
            0xff00 => {
                self.bytes[address as usize] = self.get_joypad_register_write_result(value);
            }
            address::SERIAL_CONTROL => {
                self.bytes[address::SERIAL_CONTROL as usize] = value;

                // Write data that would have gone to the serial port to stdout.
                // No plans for emulating actual serial communication yet.
                if value == 0x81 && self.print_serial {
                    let serial_byte = self.bytes[address::SERIAL_BYTE as usize];
                    if serial_byte < 128 {
                        print!("{}", serial_byte as char);
                    }
                }
            }
            0xff04 => {
                // rwtodo: label 0xff04 as a constant?
                self.bytes[address as usize] = 0x00; // Reset timer DIV register. rwtodo: move this responsibility into Timer struct
            }
            0xff46 => {
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
            }
            x if bank_ranges::WORK_RAM_STATIC.contains(&x) => {
                self.bytes[address as usize] = value;
                // Copy to echo RAM
                let echo_address =
                    address - bank_ranges::WORK_RAM_STATIC.start() + bank_ranges::ECHO_RAM.start();
                self.bytes[echo_address as usize] = value;
            }
            x if bank_ranges::ECHO_RAM.contains(&x) => {
                self.bytes[address as usize] = value;
                // Copy to work RAM static
                let echo_address =
                    address - bank_ranges::ECHO_RAM.start() + bank_ranges::WORK_RAM_STATIC.start();
                self.bytes[echo_address as usize] = value;
            }
            _ => self.bytes[address as usize] = value,
        }

        // rwtodo: implement cart_state stuff so we can do this.
        // rwtodo: Also handle the below for MBC3.
        // if cart_state.mbc_type == MBC_1 && address >= 0xa000 && address < 0xc000 {
        //     // RAM was written to.
        //     cart_state.save_file_is_outdated = true;
        // }
    }

    pub fn write_u16(&mut self, address: u16, value: u16) {
        let bytes = value.to_le_bytes();
        self.write(address, bytes[0]);
        self.write(address + 1, bytes[1]);
    }

    pub fn read(&self, address: u16) -> u8 {
        // rwtodo rom bank slot at address >= 0x4000 && address < 0x8000
        // rwtodo match statement?
        if address >= 0xfea0 && address <= 0xfeff {
            panic!("Attempted to read from a prohibited region");
        } else {
            self.bytes[address as usize]
        }
    }

    pub fn read_u16(&self, address: u16) -> u16 {
        make_u16(self.read(address), self.read(address + 1))
    }
}
