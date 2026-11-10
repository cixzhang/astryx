//! Accessibility-tree goldens: what assistive technology reads for every
//! gallery section (roles, names, values, states, and nesting; not
//! geometry). The goldens were written from the tree before the spike's
//! accessibility changes, so a difference is a change in what a screen
//! reader hears. `ASTRYX_BLESS=1` rewrites them.

use astryx_core::testing::draw;
use astryx_core::{A11yNode, Surface};
use astryx_gallery::{runtime, Gallery, Section};
use std::fmt::Write;
use std::path::PathBuf;

fn dump(n: &A11yNode, depth: usize, out: &mut String) {
    let mut line = format!("{}{:?}", "  ".repeat(depth), n.role);
    for (k, v) in [
        ("label", &n.label),
        ("value", &n.value),
        ("description", &n.description),
    ] {
        if let Some(v) = v {
            let _ = write!(line, " {k}={v:?}");
        }
    }
    for (k, v) in [("checked", n.checked), ("selected", n.selected)] {
        if let Some(v) = v {
            let _ = write!(line, " {k}={v}");
        }
    }
    for (k, on) in [
        ("focusable", n.focusable),
        ("focused", n.focused),
        ("disabled", n.disabled),
        ("secure", n.secure),
        ("rtl", n.rtl),
    ] {
        if on {
            let _ = write!(line, " {k}");
        }
    }
    if let Some(live) = n.live {
        let _ = write!(line, " live={live:?}");
    }
    out.push_str(&line);
    out.push('\n');
    for c in &n.children {
        dump(c, depth + 1, out);
    }
}

#[test]
fn every_section_reads_as_before() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/a11y");
    let bless = std::env::var_os("ASTRYX_BLESS").is_some();
    let mut failed = Vec::new();
    for section in Section::ALL {
        let mut rt = runtime(Gallery::section(section));
        draw(&mut rt, 800, 1200);
        draw(&mut rt, 800, 1200);
        let mut text = String::new();
        if let Some(tree) = rt.accessibility() {
            dump(&tree, 0, &mut text);
        }
        let path = dir.join(format!("{}.txt", section.slug()));
        if bless {
            std::fs::create_dir_all(&dir).expect("golden dir");
            std::fs::write(&path, &text).expect("write golden");
            continue;
        }
        let want = std::fs::read_to_string(&path).unwrap_or_default();
        if want != text {
            failed.push(format!(
                "{}:\n--- golden\n{want}\n--- now\n{text}",
                section.slug()
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
