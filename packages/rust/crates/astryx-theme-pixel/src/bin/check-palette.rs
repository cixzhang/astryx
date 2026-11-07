//! Checks that art uses only the pixel palette (`tokens/pixel-palette.json`):
//! every pixel an opaque swatch or fully clear.
//!
//! `tools/check-palette <png...>` prints each file with off-palette pixels
//! and where the first ones are, and exits with 1 when any file has them,
//! so it can run in CI.

use astryx_theme_pixel::PALETTE;
use std::process::ExitCode;

fn main() -> ExitCode {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() || files.iter().any(|f| f == "-h" || f == "--help") {
        eprintln!("usage: check-palette <png...>");
        eprintln!(
            "Checks every pixel against the {} swatches of {:?}.",
            PALETTE.swatches.len(),
            PALETTE.name
        );
        return ExitCode::from(2);
    }
    let mut failed = 0;
    for file in &files {
        let image = match std::fs::read(file)
            .map_err(|e| e.to_string())
            .and_then(|bytes| astryx_image::decode(&bytes).map_err(|e| format!("{e:?}")))
        {
            Ok(image) => image,
            Err(e) => {
                println!("{file}: cannot read: {e}");
                failed += 1;
                continue;
            }
        };
        if let Err(off) = PALETTE.check(&image) {
            println!("{file}: {off}");
            failed += 1;
        }
    }
    if failed > 0 {
        println!("{failed} of {} files are off the palette", files.len());
        ExitCode::FAILURE
    } else {
        println!("{} files on the palette", files.len());
        ExitCode::SUCCESS
    }
}
