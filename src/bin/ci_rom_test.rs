// This file produces a binary that receives rom files, collects their serial output, and builds an HTML table of the results with ✅ or ❌.

use clap::Parser;
use robin_gb::GameBoy;
use std::path::PathBuf;

// #[command(version, about, long_about = None)]
#[derive(Parser, Debug)]
struct CliArgs {
    /// One or more paths to rom files to test
    #[arg(value_name = "ROM_PATH", num_args = 1..)]
    roms: Vec<PathBuf>,
}

fn run_rom_test(path: PathBuf) -> Result<String, String> {
    // Validate the path.
    let path_str = path.to_str().ok_or("Path does not convert to &str")?;
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path_str));
    }
    let extension = path.extension().and_then(std::ffi::OsStr::to_str);
    let extension = extension.ok_or(format!("No extension found for path: {}", path_str))?;
    if extension.to_lowercase() != "gb" {
        return Err(format!("Expected extension 'gb' for path: {}", path_str));
    }

    Ok("Path is valid. (rom test TODO)".to_owned())
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

        html += &format!("<tr>\n<td>{}</td><td>{}</td>\n</tr>\n", name, result);
    }

    html += "</table>\n</body>\n</html>";
    println!("{}", html);
}
