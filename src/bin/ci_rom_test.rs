// This file produces a binary that receives rom files, collects their serial output, and builds an HTML table of the results with ✅ or ❌.

use clap::Parser;
use regex::Regex;
use robin_gb::GameBoy;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct CliArgs {
    /// One or more paths to rom files to test
    #[arg(value_name = "ROM_PATH", num_args = 1..)]
    roms: Vec<PathBuf>,
}

fn run_rom_test(path: PathBuf) -> Result<String, String> {
    // Validate the path.
    let extension = path.extension().and_then(std::ffi::OsStr::to_str);
    let extension = extension.ok_or("No extension found for path")?;
    if extension.to_lowercase() != "gb" {
        return Err("Expected extension 'gb' for path".to_owned());
    }

    // Load the data and boot the Game Boy with serial output enabled.
    let rom_bytes = fs::read(path).map_err(|e| e.to_string())?;
    let mut game_boy = GameBoy::new(&rom_bytes);
    game_boy.record_serial_output(true);

    // Emulate 1 minute's worth of frames (Game Boy runs at 60 FPS).
    let frame_count = 60 * 60;
    let mut frame: [u8; 160 * 144] = [0; 160 * 144];
    for _ in 0..frame_count {
        let _ = game_boy.emulate_next_frame(&mut frame);
    }

    let mut serial_string = String::new();
    if let Some(serial) = game_boy.serial_buffer() {
        for serial_byte in serial {
            // Grab any ASCII bytes and put them in a string.
            if *serial_byte < 128 {
                serial_string.push(*serial_byte as char);
            }
        }

        // Shrink all occurrences of whitespace to one space character, for readability.
        let re = Regex::new(r"\s+").unwrap();
        serial_string = re.replace_all(&serial_string, " ").to_string();
        serial_string = serial_string.trim().to_owned();

        if serial_string.to_lowercase().contains("passed") {
            Ok(serial_string)
        } else {
            Err(serial_string)
        }
    } else {
        return Err("Couldn't read serial data".to_owned());
    }
}

fn sanitize_html_text(text: &str) -> String {
    let text = text.replace("&", "&amp");
    let text = text.replace("<", "&lt");
    let text = text.replace(">", "&gt");
    let text = text.replace("\"", "&quot");
    text.replace("'", "&#39")
}

fn main() {
    let args = CliArgs::parse();

    // Open a table in UTF-8 HTML.
    let mut html =
        String::from("<html>\n<head>\n<meta charset=\"UTF-8\">\n</head>\n<body>\n<table>\n");

    for path in args.roms {
        let name = sanitize_html_text(
            path.file_name()
                .and_then(std::ffi::OsStr::to_str)
                .expect("Expected file name"),
        );
        let result = sanitize_html_text(&match run_rom_test(path) {
            Ok(o) => format!("✅ {}", o),
            Err(e) => format!("❌ {}", e),
        });

        // Add a row to the table.
        html += &format!("<tr>\n<td>{}</td><td>{}</td>\n</tr>\n", name, result);
    }

    // Add all the closing tags.
    html += "</table>\n</body>\n</html>";
    println!("{}", html);
}
