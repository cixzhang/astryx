//! Renders one section after scripted input:
//! `cargo run -p astryx-gallery --example shot -- <section-slug> <out.png> [inputs...]`,
//! where inputs are names such as `Down`, `Next`, `A`, `B`.

use astryx_core::{Input, Runtime};
use astryx_gallery::{Gallery, Section};
use astryx_headless::render_png;

fn input(name: &str) -> Input {
    match name {
        "Up" => Input::Up,
        "Down" => Input::Down,
        "Left" => Input::Left,
        "Right" => Input::Right,
        "Next" => Input::Next,
        "Previous" => Input::Previous,
        "A" => Input::A,
        "B" => Input::B,
        other => panic!("unknown input {other}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let section = Section::ALL
        .into_iter()
        .find(|s| s.slug() == args[0])
        .expect("section slug");
    let inputs: Vec<Input> = args[2..].iter().map(|a| input(a)).collect();
    let mut rt = Runtime::new(Gallery::section(section));
    std::fs::write(&args[1], render_png(&mut rt, 720, 600, 1.0, &inputs)).unwrap();
}
