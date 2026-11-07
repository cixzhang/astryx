//! `astryx-headless`: render fixtures to PNG and compare PNG files.

use std::path::PathBuf;
use std::process::ExitCode;

use astryx_core::Input;
use astryx_headless::fixtures::{FixtureSurface, FIXTURES, FIXTURE_SIZE};
use astryx_headless::golden::compare_png;
use astryx_headless::render_png;

const USAGE: &str = "\
usage:
  astryx-headless list
  astryx-headless render <fixture|all> [--width W] [--height H] [--scale S]
                         [--input next,next,a] [--out PATH]
  astryx-headless diff <actual.png> <expected.png> [--threshold N] [--out diff.png]

render writes <fixture>.png into --out (a directory, default .), or to --out
itself when it ends in .png and one fixture is named.
diff exits 1 when any pixel differs by more than the threshold (default 0).";

fn parse_input(name: &str) -> Option<Input> {
    Some(match name.to_ascii_lowercase().as_str() {
        "up" => Input::Up,
        "down" => Input::Down,
        "left" => Input::Left,
        "right" => Input::Right,
        "next" | "tab" => Input::Next,
        "previous" | "prev" => Input::Previous,
        "a" | "enter" => Input::A,
        "b" | "escape" => Input::B,
        "x" => Input::X,
        "y" => Input::Y,
        "start" => Input::Start,
        "select" => Input::Select,
        "menu" => Input::Menu,
        "backspace" => Input::Backspace,
        "delete" => Input::Delete,
        "home" => Input::Home,
        "end" => Input::End,
        _ => return None,
    })
}

struct Args {
    positional: Vec<String>,
    flags: Vec<(String, String)>,
}

impl Args {
    fn parse(raw: impl Iterator<Item = String>) -> Result<Self, String> {
        let (mut positional, mut flags) = (Vec::new(), Vec::new());
        let mut raw = raw.peekable();
        while let Some(arg) = raw.next() {
            if let Some(name) = arg.strip_prefix("--") {
                let value = raw
                    .next()
                    .ok_or_else(|| format!("--{name} needs a value"))?;
                flags.push((name.to_owned(), value));
            } else {
                positional.push(arg);
            }
        }
        Ok(Self { positional, flags })
    }

    fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    fn number<T: std::str::FromStr>(&self, name: &str, default: T) -> Result<T, String> {
        match self.flag(name) {
            Some(v) => v
                .parse()
                .map_err(|_| format!("--{name}: not a number: {v}")),
            None => Ok(default),
        }
    }
}

fn render(args: &Args) -> Result<(), String> {
    let which = args
        .positional
        .get(1)
        .ok_or("render needs a fixture name or all")?;
    let names: Vec<&str> = if which == "all" {
        FIXTURES.to_vec()
    } else {
        vec![which.as_str()]
    };
    let width = args.number("width", FIXTURE_SIZE.0)?;
    let height = args.number("height", FIXTURE_SIZE.1)?;
    let scale = args.number("scale", 1.0f32)?;
    let inputs = match args.flag("input") {
        Some(list) => list
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| parse_input(s).ok_or_else(|| format!("unknown input: {s}")))
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };
    let out = PathBuf::from(args.flag("out").unwrap_or("."));
    for name in names {
        let mut surface =
            FixtureSurface::new(name).ok_or_else(|| format!("unknown fixture: {name}"))?;
        let png = render_png(&mut surface, width, height, scale, &inputs);
        let path = if out.extension().is_some_and(|e| e == "png") && which != "all" {
            out.clone()
        } else {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            out.join(format!("{name}.png"))
        };
        std::fs::write(&path, png).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}

fn diff(args: &Args) -> Result<bool, String> {
    let [_, actual, expected] = &args.positional[..] else {
        return Err("diff needs two PNG paths".into());
    };
    let read = |p: &str| std::fs::read(p).map_err(|e| format!("{p}: {e}"));
    let threshold = args.number("threshold", 0u8)?;
    let cmp = compare_png(&read(actual)?, &read(expected)?, threshold)?;
    println!(
        "{}x{}: max difference {}, mean {:.3}, {} pixels over {}",
        cmp.width, cmp.height, cmp.max_diff, cmp.mean_diff, cmp.differing, threshold
    );
    if let Some(out) = args.flag("out") {
        cmp.diff_image
            .save_png(out)
            .map_err(|e| format!("{out}: {e}"))?;
    }
    Ok(cmp.differing == 0)
}

fn main() -> ExitCode {
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result = match args.positional.first().map(String::as_str) {
        Some("list") => {
            FIXTURES.iter().for_each(|f| println!("{f}"));
            Ok(true)
        }
        Some("render") => render(&args).map(|_| true),
        Some("diff") => diff(&args),
        _ => Err(USAGE.to_owned()),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
