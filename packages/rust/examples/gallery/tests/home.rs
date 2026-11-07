//! The gallery's home screen: sections as rows, A opens one, B comes back
//! to the same row.

mod settle;

use astryx_core::testing::{draw, tap};
use astryx_core::*;
use astryx_gallery::{Gallery, Section};

fn focused(rt: &Runtime<Gallery>) -> Option<String> {
    fn find(n: &A11yNode) -> Option<String> {
        if n.focused {
            return n.label.clone();
        }
        n.children.iter().find_map(find)
    }
    find(&rt.accessibility()?)
}

#[test]
fn home_lists_every_section() {
    let mut rt = Runtime::new(Gallery::new());
    draw(&mut rt, 960, 640);
    fn walk(n: &A11yNode, out: &mut Vec<String>) {
        if n.role == Role::Button && n.description.is_some() {
            out.push(n.label.clone().unwrap_or_default());
        }
        n.children.iter().for_each(|c| walk(c, out));
    }
    let mut rows = Vec::new();
    walk(&rt.accessibility().unwrap(), &mut rows);
    let titles: Vec<&str> = Section::ALL.iter().map(|s| s.title()).collect();
    assert_eq!(rows, titles);
}

#[test]
fn a_opens_a_section_and_b_returns_to_its_row() {
    let mut rt = Runtime::new(Gallery::new());
    draw(&mut rt, 960, 640);
    // Past the three header selectors, then down the list to the fifth row.
    for _ in 0..3 {
        tap(&mut rt, Input::Next);
    }
    for _ in 0..4 {
        tap(&mut rt, Input::Down);
    }
    draw(&mut rt, 960, 640);
    assert_eq!(focused(&rt).as_deref(), Some(Section::ALL[4].title()));
    tap(&mut rt, Input::A);
    draw(&mut rt, 960, 640);
    assert_eq!(rt.app().page, Some(Section::ALL[4]));
    assert_ne!(focused(&rt).as_deref(), Some(Section::ALL[4].title()));
    // Move around inside the page, then go back.
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::B);
    draw(&mut rt, 960, 640);
    assert_eq!(rt.app().page, None);
    assert_eq!(
        focused(&rt).as_deref(),
        Some(Section::ALL[4].title()),
        "focus is back on the row"
    );
}

#[test]
fn the_back_button_and_theme_shortcuts_work_on_a_page() {
    let mut rt = Runtime::new(Gallery::new());
    draw(&mut rt, 960, 640);
    rt.send(astryx_gallery::Msg::Open(Section::Buttons));
    draw(&mut rt, 960, 640);
    tap(&mut rt, Input::R1);
    assert_eq!(rt.app().theme.name, Theme::ALL[1].name);
    assert_eq!(
        rt.app().page,
        Some(Section::Buttons),
        "theme shortcuts stay on the page"
    );
    // The visible back button returns too, for pointer and keyboard people.
    fn back(n: &A11yNode) -> Option<u64> {
        if n.label.as_deref() == Some("All components") {
            return Some(n.id);
        }
        n.children.iter().find_map(back)
    }
    let id = back(&rt.accessibility().unwrap()).expect("back button");
    rt.a11y_action(id, A11yAction::Activate);
    assert_eq!(rt.app().page, None);
}

#[test]
fn home_goldens() {
    use astryx_headless::golden::{assert_golden, Tolerance};
    use Input::{Down, Next, A};
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens");
    let tolerance = Tolerance {
        channel: 2,
        pixels: 64,
    };
    for (inputs, name) in [
        (&[][..], "home.png"),
        (
            &[Next, Next, Down, Down, Down, Down, A][..],
            "home-opened-section.png",
        ),
        (
            &[Next, Next, Down, Down, Down, Down, A, Down, Input::B][..],
            "home-back-on-row.png",
        ),
    ] {
        let mut rt = Runtime::new(Gallery::new());
        assert_golden(
            &settle::render_png(&mut rt, 960, 640, 1.0, inputs),
            dir.join(name),
            tolerance,
        );
    }
}

#[test]
fn gallery_text_fields_take_typing() {
    let mut rt = Runtime::new(Gallery::section(Section::TextInputs));
    draw(&mut rt, 720, 900);
    rt.text_input("Grace");
    assert_eq!(
        rt.app().fields.get("Name").map(String::as_str),
        Some("Grace")
    );
    draw(&mut rt, 720, 900);
    tap(&mut rt, Input::Backspace);
    assert_eq!(
        rt.app().fields.get("Name").map(String::as_str),
        Some("Grac")
    );
}
