//! The experimental paged screens and focus-following camera.

mod common;

use astryx_core::paged::{CameraFollow, FlipButtons, Paged};
use astryx_core::testing::*;
use astryx_core::*;
use common::*;

const W: u32 = 400;
const H: u32 = 300;

/// `n` buttons, 36 points tall with 16 points between them, in a pager
/// with 16 points of padding.
fn buttons(n: usize) -> impl Fn() -> Element<usize> {
    move || {
        Paged::new()
            .gap(Space::S4)
            .padding(Space::S4)
            .children((0..n).map(|i| Button::new(format!("B{i}")).on_click(i)))
            .into()
    }
}

fn focused<M: Clone + 'static>(rt: &Runtime<View<M>>) -> Option<String> {
    a11y_nodes(&rt.accessibility()?)
        .into_iter()
        .find(|n| n.focused)
        .and_then(|n| n.label)
}

fn page_label<M: Clone + 'static>(rt: &Runtime<View<M>>) -> Option<String> {
    a11y_nodes(&rt.accessibility()?)
        .into_iter()
        .find(|n| n.live.is_some() && n.label.as_deref().is_some_and(|l| l.starts_with("Page ")))
        .and_then(|n| n.label)
}

/// The clip a resting pager draws its page into.
fn page_clip(list: &DisplayList) -> Rect {
    list.commands
        .iter()
        .find_map(|c| match c {
            DrawCmd::PushClip { rect, .. } => Some(*rect),
            _ => None,
        })
        .expect("a pager clip")
}

fn settle<M: Clone + 'static>(rt: &mut Runtime<View<M>>) -> DisplayList {
    draw(rt, W, H);
    rt.frame(
        &FrameInfo {
            width: W,
            height: H,
            scale: 1.0,
            dt: 1.0,
        },
        &measure(),
    )
}

#[test]
fn pages_fit_the_viewport_and_never_cut_an_item() {
    let mut rt = runtime(buttons(12));
    let list = settle(&mut rt);
    // The pager is the window less the 24 point indicator row: 276 points.
    // Five 32 point buttons fit above the bottom padding; the page ends
    // where the sixth starts, and the sixth is not drawn.
    let clip = page_clip(&list);
    let b5 = a11y(&rt, Role::Button)[5].bounds;
    assert_eq!(clip.height, b5.y - clip.y);
    assert!(b5.y + b5.height > 276.0 - 16.0, "a sixth would not fit");
    let shown: Vec<String> = a11y(&rt, Role::Button)
        .into_iter()
        .filter(|b| b.bounds.y >= clip.y && b.bounds.y + b.bounds.height <= clip.y + clip.height)
        .filter_map(|b| b.label)
        .collect();
    assert_eq!(shown, ["B0", "B1", "B2", "B3", "B4"]);
    assert_eq!(page_label(&rt).as_deref(), Some("Page 1 of 3"));
    // Three dots, the first one accent.
    let t = light();
    let dots: Vec<Color> = rects(&list)
        .into_iter()
        .filter(|r| r.0.width == t.space.s1_5 && r.0.height == t.space.s1_5)
        .map(|r| r.2)
        .collect();
    assert_eq!(
        dots,
        [c(t.color.accent), c(t.color.neutral), c(t.color.neutral)]
    );
}

#[test]
fn down_past_the_last_stop_flips_to_the_next_page() {
    let mut rt = runtime(buttons(12));
    settle(&mut rt);
    for _ in 0..4 {
        tap(&mut rt, Input::Down);
    }
    settle(&mut rt);
    assert_eq!(focused(&rt).as_deref(), Some("B4"));
    tap(&mut rt, Input::Down);
    draw(&mut rt, W, H);
    assert!(rt.wants_frames(), "the page slides");
    let list = settle(&mut rt);
    assert_eq!(
        focused(&rt).as_deref(),
        Some("B5"),
        "the first stop of the next page"
    );
    assert_eq!(page_label(&rt).as_deref(), Some("Page 2 of 3"));
    let clip = page_clip(&list);
    let b5 = a11y(&rt, Role::Button)
        .into_iter()
        .find(|b| b.label.as_deref() == Some("B5"))
        .unwrap();
    assert_eq!(
        b5.bounds.y, 16.0,
        "the page starts at its first item, under the padding"
    );
    assert_eq!(
        clip.y, 16.0,
        "the end of the page before stays hidden above it"
    );
    // Up from the page's first stop goes back, to the last stop there.
    tap(&mut rt, Input::Up);
    settle(&mut rt);
    assert_eq!(focused(&rt).as_deref(), Some("B4"));
    assert_eq!(page_label(&rt).as_deref(), Some("Page 1 of 3"));
}

#[test]
fn right_flips_only_from_the_last_stop_on_the_page() {
    let mut rt = runtime(|| {
        Paged::new()
            .gap(Space::S4)
            .padding(Space::S4)
            .children((0..6).map(|r| {
                HStack::new().gap(Space::S2).children(
                    (0..2).map(move |c| Button::new(format!("R{r}C{c}")).on_click(r * 2 + c)),
                )
            }))
            .into()
    });
    settle(&mut rt);
    tap(&mut rt, Input::Right);
    settle(&mut rt);
    assert_eq!(focused(&rt).as_deref(), Some("R0C1"));
    // Nothing to the right, but stops remain below: no flip.
    tap(&mut rt, Input::Right);
    settle(&mut rt);
    assert_eq!(page_label(&rt).as_deref(), Some("Page 1 of 2"));
    for _ in 0..4 {
        tap(&mut rt, Input::Down);
    }
    settle(&mut rt);
    assert_eq!(focused(&rt).as_deref(), Some("R4C1"));
    tap(&mut rt, Input::Right);
    settle(&mut rt);
    assert_eq!(
        focused(&rt).as_deref(),
        Some("R5C0"),
        "from the last stop, Right flips"
    );
    assert_eq!(page_label(&rt).as_deref(), Some("Page 2 of 2"));
}

#[test]
fn l2_and_r2_flip_pages_directly() {
    let mut rt = runtime(buttons(12));
    settle(&mut rt);
    tap(&mut rt, Input::R2);
    settle(&mut rt);
    assert_eq!(page_label(&rt).as_deref(), Some("Page 2 of 3"));
    assert_eq!(focused(&rt).as_deref(), Some("B5"));
    tap(&mut rt, Input::R2);
    tap(&mut rt, Input::R2);
    settle(&mut rt);
    assert_eq!(
        page_label(&rt).as_deref(),
        Some("Page 3 of 3"),
        "no page past the last"
    );
    assert_eq!(focused(&rt).as_deref(), Some("B10"));
    tap(&mut rt, Input::L2);
    settle(&mut rt);
    assert_eq!(page_label(&rt).as_deref(), Some("Page 2 of 3"));
    assert_eq!(
        focused(&rt).as_deref(),
        Some("B5"),
        "the first stop on the page"
    );
}

/// An app that hears the buttons nothing else took.
struct Shoulders {
    heard: Vec<Input>,
    buttons: FlipButtons,
}

impl App for Shoulders {
    type Msg = usize;
    fn view(&self) -> Element<usize> {
        Paged::new()
            .gap(Space::S4)
            .padding(Space::S4)
            .flip_buttons(self.buttons)
            .children((0..12).map(|i| Button::new(format!("B{i}")).on_click(i)))
            .into()
    }
    fn update(&mut self, msg: usize) -> Command {
        match msg {
            100 => self.heard.push(Input::L1),
            101 => self.heard.push(Input::R1),
            _ => {}
        }
        Command::None
    }
    fn on_input(&self, input: Input) -> Option<usize> {
        match input {
            Input::L1 => Some(100),
            Input::R1 => Some(101),
            _ => None,
        }
    }
}

fn shoulders(buttons: FlipButtons) -> Runtime<Shoulders> {
    let mut rt = Runtime::new(Shoulders {
        heard: Vec::new(),
        buttons,
    });
    draw(&mut rt, W, H);
    rt
}

fn page_of(rt: &Runtime<Shoulders>) -> Option<String> {
    a11y_nodes(&rt.accessibility()?)
        .into_iter()
        .find(|n| n.live.is_some() && n.label.as_deref().is_some_and(|l| l.starts_with("Page ")))
        .and_then(|n| n.label)
}

fn settle_app<A: App>(rt: &mut Runtime<A>) {
    draw(rt, W, H);
    rt.frame(
        &FrameInfo {
            width: W,
            height: H,
            scale: 1.0,
            dt: 1.0,
        },
        &measure(),
    );
}

#[test]
fn l1_and_r1_flip_a_pager_set_to_them_and_l2_and_r2_then_do_nothing() {
    let mut rt = shoulders(FlipButtons::L1R1);
    tap(&mut rt, Input::R1);
    settle_app(&mut rt);
    assert_eq!(page_of(&rt).as_deref(), Some("Page 2 of 3"));
    tap(&mut rt, Input::R2);
    settle_app(&mut rt);
    assert_eq!(
        page_of(&rt).as_deref(),
        Some("Page 2 of 3"),
        "R2 is not a flip button here"
    );
    tap(&mut rt, Input::L1);
    settle_app(&mut rt);
    assert_eq!(page_of(&rt).as_deref(), Some("Page 1 of 3"));
    assert!(
        !rt.app().heard.contains(&Input::R1) && !rt.app().heard.contains(&Input::L1),
        "the pager took L1 and R1: {:?}",
        rt.app().heard
    );
}

#[test]
fn by_default_l1_and_r1_reach_the_app_and_the_pager_stays() {
    let mut rt = shoulders(FlipButtons::default());
    tap(&mut rt, Input::R1);
    settle_app(&mut rt);
    assert_eq!(page_of(&rt).as_deref(), Some("Page 1 of 3"));
    assert_eq!(rt.app().heard, [Input::R1]);
    tap(&mut rt, Input::R2);
    settle_app(&mut rt);
    assert_eq!(page_of(&rt).as_deref(), Some("Page 2 of 3"));
}

#[test]
fn flips_are_instant_under_reduced_motion() {
    let mut rt = runtime(buttons(12));
    rt.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    settle(&mut rt);
    tap(&mut rt, Input::R2);
    draw(&mut rt, W, H);
    assert!(!rt.wants_frames());
    let b5 = a11y(&rt, Role::Button)
        .into_iter()
        .find(|b| b.label.as_deref() == Some("B5"))
        .unwrap();
    assert_eq!(b5.bounds.y, 16.0);
}

const LONG: &str = "Pages break between lines of long text so every line is whole on one page and none is cut in half at the bottom edge of the screen.";

#[test]
fn text_breaks_between_lines_and_text_pages_flip_with_down() {
    let mut rt = runtime(|| {
        Paged::new()
            .padding(Space::S4)
            .gap(Space::S4)
            .child(Button::new("Start").on_click(0))
            .children((0..6).map(|_| Text::new(LONG)))
            .child(Button::new("End").on_click(1))
            .into()
    });
    let list = settle(&mut rt);
    let clip = page_clip(&list);
    // Each line of body text is 20 points; the clip ends on a line boundary
    // of whichever paragraph it cuts.
    let cut = clip.y + clip.height;
    let lines_whole = texts(&list).into_iter().filter(|t| t.1 == LONG).all(|t| {
        let (top, bottom) = (t.0.y, t.0.y + t.0.height);
        cut <= top + 0.5 || cut >= bottom - 0.5 || ((cut - top) / 20.0).fract().abs() < 0.01
    });
    assert!(lines_whole);
    let count: usize = page_label(&rt)
        .unwrap()
        .rsplit(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(count >= 3, "{count} pages");
    // From Start, Down has no stop on this page: the page flips, and on
    // pages of text alone focus rests on the page itself.
    tap(&mut rt, Input::Down);
    settle(&mut rt);
    assert_eq!(
        page_label(&rt).as_deref(),
        Some(format!("Page 2 of {count}").as_str())
    );
    let mut pages = 2;
    while focused(&rt).is_none() {
        tap(&mut rt, Input::Down);
        settle(&mut rt);
        pages += 1;
        assert!(pages <= count);
    }
    assert_eq!(focused(&rt).as_deref(), Some("End"));
    assert_eq!(
        page_label(&rt).as_deref(),
        Some(format!("Page {count} of {count}").as_str())
    );
    // Up from the last page's first stop flips back to a text page.
    tap(&mut rt, Input::Up);
    settle(&mut rt);
    assert_eq!(focused(&rt), None);
    assert_eq!(
        page_label(&rt).as_deref(),
        Some(format!("Page {} of {count}", count - 1).as_str())
    );
}

#[test]
fn a_wheel_scrolls_freely_and_the_d_pad_snaps_back_to_a_page() {
    let mut rt = runtime(buttons(12));
    settle(&mut rt);
    assert!(rt.scroll(Point { x: 100.0, y: 100.0 }, Point { x: 0.0, y: 70.0 }));
    let list = settle(&mut rt);
    let b0 = a11y(&rt, Role::Button)[0].bounds;
    assert_eq!(b0.y, 16.0 - 70.0, "free scrolling under a pointer");
    assert!(
        !list
            .commands
            .iter()
            .any(|c| matches!(c, DrawCmd::PushClip { rect, .. } if rect.height < 276.0)),
        "no page clip"
    );
    tap(&mut rt, Input::Down);
    settle(&mut rt);
    assert_eq!(
        a11y(&rt, Role::Button)[0].bounds.y,
        16.0,
        "back on the page focus is on"
    );
}

/// A 10 x 10 board of 48 point avatars with 16 points between them.
fn board() -> Element<usize> {
    CameraFollow::new(VStack::new().gap(Space::S4).children((0..10).map(|r| {
        HStack::new().gap(Space::S4).children((0..10).map(move |c| {
            Avatar::new()
                .alt(format!("{r},{c}"))
                .size(AvatarSize::Lg)
                .on_click(r * 10 + c)
        }))
    })))
    .into()
}

fn avatar_x<M: Clone + 'static>(rt: &Runtime<View<M>>, label: &str) -> f32 {
    a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.label.as_deref() == Some(label))
        .unwrap()
        .bounds
        .x
}

#[test]
fn the_camera_follows_focus_only_near_an_edge() {
    let mut rt = runtime(board);
    settle(&mut rt);
    assert_eq!(avatar_x(&rt, "0,0"), 0.0);
    // Focus moves right through the middle of the view: the board holds still.
    tap(&mut rt, Input::Right);
    tap(&mut rt, Input::Right);
    tap(&mut rt, Input::Right);
    settle(&mut rt);
    assert_eq!(focused(&rt).as_deref(), Some("0,3"));
    assert_eq!(avatar_x(&rt, "0,0"), 0.0, "inside the dead zone");
    // 0,4 ends at 304, inside the right quarter (past 300): the board moves.
    tap(&mut rt, Input::Right);
    draw(&mut rt, W, H);
    assert!(rt.wants_frames(), "smoothly");
    settle(&mut rt);
    assert_eq!(
        avatar_x(&rt, "0,4") + 48.0,
        300.0,
        "its edge rests at the dead zone's edge"
    );
    // Back left: the board holds until focus nears the left quarter.
    tap(&mut rt, Input::Left);
    settle(&mut rt);
    let x3 = avatar_x(&rt, "0,3");
    assert!(x3 >= 100.0, "{x3}");
    assert_eq!(avatar_x(&rt, "0,4") + 48.0, 300.0, "still");
    // Down moves the board vertically the same way, and no scroll indicator shows.
    for _ in 0..4 {
        tap(&mut rt, Input::Down);
    }
    let list = settle(&mut rt);
    let row = a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.focused)
        .unwrap()
        .bounds;
    assert!(row.y + row.height <= 300.0 - 75.0 + 0.5, "{row:?}");
    let t = light();
    assert!(rect_filled(&list, c(t.color.neutral)).is_none_or(|r| r.0.width != t.space.s1));
}

#[test]
fn the_camera_never_shows_past_the_board() {
    let mut rt = runtime(board);
    settle(&mut rt);
    for _ in 0..12 {
        tap(&mut rt, Input::Right);
    }
    settle(&mut rt);
    assert_eq!(focused(&rt).as_deref(), Some("0,9"));
    // The board is 624 wide; the camera stops with its right edge at the view's.
    assert_eq!(avatar_x(&rt, "0,9") + 48.0, 400.0);
}

#[test]
fn the_camera_keeps_focus_inside_its_own_box_above_a_sibling() {
    let mut rt = runtime(|| {
        VStack::new()
            .child(board())
            .child(
                Card::new()
                    .padding(Space::S4)
                    .child(Text::new("A caption card under the board")),
            )
            .into()
    });
    settle(&mut rt);
    let nodes = a11y_nodes(&rt.accessibility().unwrap());
    let caption = nodes
        .iter()
        .find(|n| n.label.as_deref() == Some("A caption card under the board"))
        .unwrap()
        .bounds;
    // The camera's own box ends where the card begins.
    let view_bottom = caption.y - light().space.s4;
    for _ in 0..9 {
        tap(&mut rt, Input::Down);
        settle(&mut rt);
        let f = a11y_nodes(&rt.accessibility().unwrap())
            .into_iter()
            .find(|n| n.focused)
            .unwrap()
            .bounds;
        assert!(
            f.y + f.height <= view_bottom + 0.5,
            "focus {f:?} hidden under the card at {view_bottom}"
        );
    }
    let f = a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.focused)
        .unwrap();
    assert_eq!(f.label.as_deref(), Some("9,0"));
}

/// A camera whose parent cannot shrink it, so its own box runs past the
/// window: focus stays inside the part of the box on screen.
#[test]
fn the_camera_keeps_focus_on_screen_when_its_box_runs_past_the_window() {
    let mut rt = runtime(|| {
        VStack::new()
            .padding(Space::S4)
            .child(
                VStack::new()
                    .gap(Space::S4)
                    .child(board())
                    .child(Card::new().padding(Space::S4).child(Text::new("Caption"))),
            )
            .into()
    });
    settle(&mut rt);
    for _ in 0..9 {
        tap(&mut rt, Input::Down);
        settle(&mut rt);
        let f = a11y_nodes(&rt.accessibility().unwrap())
            .into_iter()
            .find(|n| n.focused)
            .unwrap()
            .bounds;
        assert!(
            f.y >= 16.0 && f.y + f.height <= H as f32,
            "focus {f:?} off screen"
        );
    }
}
