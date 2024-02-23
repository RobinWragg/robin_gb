use robin_gb;
use std::fs;

// rwtodo: This will become a simple app that loads multiple instances of the emulator and loads a different game in each one.
fn main() {
    println!("Hello, world!");
    let rom_file_data = fs::read("roms/Tetris.gb").unwrap();
    let mut gb = robin_gb::load_rom_file(&rom_file_data[..]).unwrap();

    for i in 0..100 {
        let frame = gb.emulate_next_frame();
    }
}
