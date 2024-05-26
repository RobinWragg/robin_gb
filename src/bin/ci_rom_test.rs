// This file produces a binary that loads multiple roms and emulates them simultaneously,
// rendering them in a grid using wgpu.

use clap::{builder::OsStr, Parser};
use robin_gb::GameBoy;
use std::path::PathBuf;

// #[command(version, about, long_about = None)]
#[derive(Parser, Debug)]
struct CliArgs {
    /// One or more paths to rom files to test
    #[arg(value_name = "ROM_PATH", num_args = 1..)]
    roms: Vec<PathBuf>,
}

fn main() {
    let args = CliArgs::parse();

    for path in args.roms {
        let path_str = path.to_str().expect("Path does not convert to str");
        assert_eq!(path.exists(), true, "Path does not exist: {}", path_str);
        assert_eq!(path.is_file(), true, "Path is not a file: {}", path_str);
        let extension = path.extension().and_then(std::ffi::OsStr::to_str);
        let extension = extension.unwrap_or_else(|| panic!("No extension found: {}", path_str));
        assert_eq!(extension.to_lowercase(), "gb");
    }
    // rwtodo: make this a command line argument.
    // let path = fs::read_dir("roms/romonly").unwrap();
    // let num_frames = 60 * 60; // 1 minute.

    // rwtodo: Load the rom, and emulate for the set time. Then collect the serial data.

    // Boot up the game boy.
    // let game_boy = GameBoy::new(&rom);

    // for i in 0..num_frames {
    //     let _ = game_boy.emulate_next_frame();
    // }
}
