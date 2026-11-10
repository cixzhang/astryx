//! Embeds the QA lane's case files (`packages/rust-spike/qa/cases/*.json`)
//! when they exist, so web and desktop builds render the same cases.

use std::fmt::Write;
use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../qa/cases");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut out = String::from("pub static QA_CASES: &[(&str, &str)] = &[\n");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut files: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        for f in files {
            println!("cargo:rerun-if-changed={}", f.display());
            let family = f.file_stem().unwrap().to_string_lossy().to_string();
            let _ = writeln!(
                out,
                "    ({:?}, include_str!({:?})),",
                family,
                f.canonicalize().unwrap().display().to_string()
            );
        }
    }
    out.push_str("];\n");
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("qa_cases.rs");
    std::fs::write(dest, out).unwrap();
}
