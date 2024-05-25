// This file produces a binary that loads multiple roms and emulates them simultaneously,
// rendering them in a grid using wgpu.

use robin_gb::GameBoy;
use std::fs;

fn main() {
    // rwtodo: make this a command line argument.
    let path = fs::read_dir("roms/romonly").unwrap();
    let num_frames = 60 * 60; // 1 minute.

    // rwtodo: Load the rom, and emulate for the set time. Then collect the serial data.

    // Boot up the game boy.
    // let game_boy = GameBoy::new(&rom);

    // for i in 0..num_frames {
    //     let _ = game_boy.emulate_next_frame();
    // }
}
