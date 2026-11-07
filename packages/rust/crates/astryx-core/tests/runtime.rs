//! Runtime tests: focus movement, activation, back, theme switching,
//! scrolling focus into view, accessibility ids, and screen-reader actions.

mod common;
use astryx_core::testing::{draw, tap};
use astryx_core::*;
use common::*;

fn buttons(n: usize) -> impl Fn() -> Element<usize> {
    move || {
        VStack::new()
            .gap(Space::S2)
            .children((0..n).map(|i| Button::new(format!("B{i}")).on_click(i)))
            .into()
    }
}

fn focused_label<M: Clone + 'static>(rt: &Runtime<View<M>>) -> Option<String> {
    a11y_nodes(&rt.accessibility()?)
        .into_iter()
        .find(|n| n.focused)
        .and_then(|n| n.label)
}

#[test]
fn first_frame_focuses_the_first_stop() {
    let mut rt = runtime(buttons(3));
    draw(&mut rt, 400, 600);
    assert_eq!(focused_label(&rt).as_deref(), Some("B0"));
}

#[test]
fn next_and_previous_follow_reading_order_and_wrap() {
    let mut rt = runtime(buttons(3));
    draw(&mut rt, 400, 600);
    tap(&mut rt, Input::Next);
    tap(&mut rt, Input::Next);
    draw(&mut rt, 400, 600);
    assert_eq!(focused_label(&rt).as_deref(), Some("B2"));
    tap(&mut rt, Input::Next);
    draw(&mut rt, 400, 600);
    assert_eq!(focused_label(&rt).as_deref(), Some("B0"));
    tap(&mut rt, Input::Previous);
    draw(&mut rt, 400, 600);
    assert_eq!(focused_label(&rt).as_deref(), Some("B2"));
}

#[test]
fn arrows_move_by_geometry() {
    // A 2 × 2 grid: Right then Down reaches the bottom right.
    let mut rt = runtime(|| {
        VStack::new()
            .gap(Space::S2)
            .child(
                HStack::new()
                    .gap(Space::S2)
                    .child(Button::new("TL").on_click(0))
                    .child(Button::new("TR").on_click(1)),
            )
            .child(
                HStack::new()
                    .gap(Space::S2)
                    .child(Button::new("BL").on_click(2))
                    .child(Button::new("BR").on_click(3)),
            )
            .into()
    });
    draw(&mut rt, 400, 600);
    tap(&mut rt, Input::Right);
    tap(&mut rt, Input::Down);
    draw(&mut rt, 400, 600);
    assert_eq!(focused_label(&rt).as_deref(), Some("BR"));
    tap(&mut rt, Input::Down);
    draw(&mut rt, 400, 600);
    assert_eq!(
        focused_label(&rt).as_deref(),
        Some("BR"),
        "focus stays at an edge"
    );
    tap(&mut rt, Input::Left);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().sent, vec![2]);
}

#[test]
fn a_activates_the_focused_control_only() {
    let mut rt = runtime(buttons(2));
    draw(&mut rt, 400, 600);
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().sent, vec![1]);
    // Moving focus while A is held cancels the press.
    rt.input(Input::A, InputPhase::Pressed);
    tap(&mut rt, Input::Up);
    rt.input(Input::A, InputPhase::Released);
    assert_eq!(rt.app().sent, vec![1]);
}

struct Back {
    backs: u32,
    choice: ThemeChoice,
}

#[derive(Clone)]
enum BackMsg {
    Back,
    Dark,
}

impl App for Back {
    type Msg = BackMsg;
    fn view(&self) -> Element<BackMsg> {
        VStack::new()
            .child(Text::new(format!("backs {}", self.backs)))
            .child(Button::new("Dark").on_click(BackMsg::Dark))
            .into()
    }
    fn update(&mut self, msg: BackMsg) -> Command {
        match msg {
            BackMsg::Back => {
                self.backs += 1;
                if self.backs == 2 {
                    return Command::Quit;
                }
            }
            BackMsg::Dark => self.choice = ThemeChoice::new(Theme::GOTHIC, Mode::Dark),
        }
        Command::None
    }
    fn theme(&self) -> ThemeChoice {
        self.choice
    }
    fn on_input(&self, input: Input) -> Option<BackMsg> {
        (input == Input::B).then_some(BackMsg::Back)
    }
}

#[test]
fn b_is_handed_to_the_app_and_its_command_returns() {
    let mut rt = Runtime::new(Back {
        backs: 0,
        choice: ThemeChoice::default(),
    });
    draw(&mut rt, 400, 600);
    assert_eq!(rt.input(Input::B, InputPhase::Pressed), Command::None);
    rt.input(Input::B, InputPhase::Released);
    assert_eq!(rt.input(Input::B, InputPhase::Pressed), Command::Quit);
    let list = draw(&mut rt, 400, 600);
    assert!(texts(&list).iter().any(|t| t.1 == "backs 2"));
    assert_eq!(rt.input(Input::X, InputPhase::Pressed), Command::None);
}

#[test]
fn theme_switches_at_run_time() {
    let mut rt = Runtime::new(Back {
        backs: 0,
        choice: ThemeChoice::default(),
    });
    let before = draw(&mut rt, 400, 600);
    assert_eq!(
        before.clear,
        Color::from(Theme::ASTRYX.light.color.background_body)
    );
    tap(&mut rt, Input::A);
    assert!(rt.wants_frames());
    let after = draw(&mut rt, 400, 600);
    let gothic = &Theme::GOTHIC.dark;
    assert_eq!(after.clear, Color::from(gothic.color.background_body));
    assert_eq!(texts(&after)[0].3, Color::from(gothic.color.text_primary));
    assert!(
        rings(&after, gothic.color.accent.into()).len() == 1,
        "focus survives the rebuild"
    );
}

#[test]
fn themed_subtree_paints_with_its_own_tokens() {
    let dark = &Theme::ASTRYX.dark;
    let list = paint(|| {
        VStack::<()>::new()
            .child(Text::new("light"))
            .child(
                Element::from(Text::new("dark"))
                    .themed(ThemeChoice::new(Theme::ASTRYX, Mode::Dark)),
            )
            .into()
    });
    assert_eq!(
        text(&list, "light").2,
        Color::from(Theme::ASTRYX.light.color.text_primary)
    );
    assert_eq!(text(&list, "dark").2, Color::from(dark.color.text_primary));
    assert!(rect_filled(&list, dark.color.background_body.into()).is_some());
}

#[test]
fn focus_follows_its_id_across_rebuilds_and_finds_a_neighbor_when_removed() {
    use std::cell::Cell;
    use std::rc::Rc;
    let count = Rc::new(Cell::new(3usize));
    let shown = count.clone();
    let mut rt = runtime(move || {
        VStack::new()
            .gap(Space::S2)
            .children(
                (0..shown.get())
                    .map(|i| Button::new(format!("B{i}")).id(format!("b{i}")).on_click(i)),
            )
            .into()
    });
    draw(&mut rt, 400, 600);
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::Down);
    draw(&mut rt, 400, 600);
    let id = rt.focused();
    // Rebuilding with the same elements keeps the same id focused.
    rt.app_mut();
    draw(&mut rt, 400, 600);
    assert_eq!(rt.focused(), id);
    // Removing the focused button moves focus to the nearest one.
    count.set(2);
    rt.app_mut();
    draw(&mut rt, 400, 600);
    assert_eq!(focused_label(&rt).as_deref(), Some("B1"));
}

#[test]
fn accessibility_ids_are_stable_and_keyed_ids_survive_insertions() {
    use std::cell::Cell;
    use std::rc::Rc;
    let extra = Rc::new(Cell::new(false));
    let flag = extra.clone();
    let mut rt = runtime(move || {
        let mut v = VStack::<()>::new();
        if flag.get() {
            v = v.child(Text::new("inserted"));
        }
        v.child(Text::new("keyed").id("k"))
            .child(Text::new("plain"))
            .into()
    });
    draw(&mut rt, 400, 600);
    let ids = |rt: &Runtime<View<()>>| -> Vec<(String, u64)> {
        a11y_nodes(&rt.accessibility().unwrap())
            .into_iter()
            .filter_map(|n| Some((n.label?, n.id)))
            .collect()
    };
    let first = ids(&rt);
    draw(&mut rt, 400, 600);
    assert_eq!(ids(&rt), first, "same frame, same ids");
    extra.set(true);
    rt.app_mut();
    draw(&mut rt, 400, 600);
    let after = ids(&rt);
    let id_of = |v: &[(String, u64)], l: &str| v.iter().find(|(x, _)| x == l).unwrap().1;
    assert_eq!(id_of(&first, "keyed"), id_of(&after, "keyed"));
    assert_ne!(
        id_of(&first, "plain"),
        id_of(&after, "plain"),
        "unkeyed ids follow position"
    );
}

#[test]
fn a11y_tree_has_bounds_states_and_reading_order() {
    let mut rt = runtime(|| {
        VStack::new()
            .child(Heading::new(1, "Title"))
            .child(Switch::new("On", true).on_change(|v| v))
            .child(Button::new("Off").is_disabled(true).on_click(false))
            .into()
    });
    draw(&mut rt, 400, 600);
    let root = rt.accessibility().unwrap();
    assert_eq!(root.role, Role::Group);
    assert_eq!(root.bounds, Rect::new(0.0, 0.0, 400.0, 600.0));
    let roles: Vec<Role> = root.children.iter().map(|n| n.role).collect();
    assert_eq!(roles, vec![Role::Heading(1), Role::Switch, Role::Button]);
    assert!(root.children[1].focused && root.children[1].checked == Some(true));
    assert!(root.children[2].disabled);
    assert!(root
        .children
        .iter()
        .all(|n| n.bounds.width > 0.0 && n.bounds.height > 0.0));
    // A labeled control speaks for its own words; only live regions remain inside.
    assert!(root
        .children
        .iter()
        .all(|n| n.children.iter().all(|c| c.live.is_some())));
}

#[test]
fn screen_reader_actions_focus_and_activate() {
    let mut rt = runtime(buttons(3));
    draw(&mut rt, 400, 600);
    let id = a11y(&rt, Role::Button)[2].id;
    assert_eq!(rt.a11y_action(id, A11yAction::Focus), Command::None);
    draw(&mut rt, 400, 600);
    assert_eq!(focused_label(&rt).as_deref(), Some("B2"));
    let first = a11y(&rt, Role::Button)[0].id;
    rt.a11y_action(first, A11yAction::Activate);
    assert_eq!(rt.app().sent, vec![0]);
    rt.a11y_action(12345, A11yAction::Activate);
    assert_eq!(rt.app().sent, vec![0], "unknown ids do nothing");
}

#[test]
fn focus_scrolls_into_view() {
    let mut rt = runtime(|| {
        VStack::new()
            .gap(Space::S2)
            .is_scrollable(true)
            .children((0..30).map(|i| Button::new(format!("B{i}")).on_click(i)))
            .into()
    });
    draw(&mut rt, 300, 200);
    for _ in 0..20 {
        tap(&mut rt, Input::Next);
    }
    let list = draw(&mut rt, 300, 200);
    let focused = a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.focused)
        .unwrap();
    assert_eq!(focused.label.as_deref(), Some("B20"));
    let margin = FOCUS_RING_OFFSET + FOCUS_RING_WIDTH;
    assert!(
        focused.bounds.y >= margin
            && focused.bounds.y + focused.bounds.height <= 200.0 - margin + 0.5,
        "{:?}",
        focused.bounds
    );
    assert!(has(&list, |c| matches!(c, DrawCmd::PushClip { .. })));
    // Back to the top scrolls back.
    for _ in 0..20 {
        tap(&mut rt, Input::Previous);
    }
    draw(&mut rt, 300, 200);
    assert_eq!(a11y(&rt, Role::Button)[0].bounds.y, 0.0);
}

#[test]
fn wants_frames_only_while_something_changes() {
    let mut rt = runtime(buttons(1));
    assert!(rt.wants_frames());
    draw(&mut rt, 400, 600);
    assert!(!rt.wants_frames());
    tap(&mut rt, Input::Down);
    assert!(rt.wants_frames());
    draw(&mut rt, 400, 600);
    assert!(!rt.wants_frames());
}

#[test]
fn layout_follows_the_frame_size_and_scale() {
    let mut rt = runtime(|| Button::<()>::new("Fill").width(ButtonWidth::Fill).into());
    let small = draw(&mut rt, 200, 100);
    let frame = FrameInfo {
        width: 800,
        height: 400,
        scale: 2.0,
        dt: 0.016,
    };
    let big = rt.frame(&frame, &measure());
    let width = |l: &DisplayList| rects(l)[0].0.width;
    assert_eq!(width(&small), 200.0);
    assert_eq!(width(&big), 400.0, "logical points, not pixels");
}

#[test]
fn preferences_drive_a_system_theme_but_not_a_pinned_one() {
    let dark = Preferences {
        color_scheme: ColorScheme::Dark,
        ..Preferences::default()
    };
    let mut rt = runtime(|| Text::new("x").into_el());
    draw(&mut rt, 100, 100);
    rt.set_preferences(&dark);
    assert!(rt.wants_frames());
    let list = draw(&mut rt, 100, 100);
    assert_eq!(
        list.clear,
        Color::from(Theme::ASTRYX.dark.color.background_body)
    );
    let mut pinned = Runtime::new(Back {
        backs: 0,
        choice: ThemeChoice::new(Theme::ASTRYX, Mode::Light),
    });
    pinned.set_preferences(&dark);
    let list = draw(&mut pinned, 100, 100);
    assert_eq!(
        list.clear,
        Color::from(Theme::ASTRYX.light.color.background_body)
    );
}

#[test]
fn more_contrast_emphasizes_borders() {
    let t = light();
    let mut rt = runtime(|| Card::<()>::new().into());
    rt.set_preferences(&Preferences {
        more_contrast: true,
        ..Preferences::default()
    });
    let list = draw(&mut rt, 200, 200);
    assert_eq!(
        rects(&list)[0].3,
        Some((Color::from(t.color.border_emphasized), 1.0))
    );
}

#[test]
fn reduced_motion_stops_spinners_and_slides() {
    let mut rt = runtime(|| {
        VStack::<()>::new()
            .child(Button::new("Busy").is_loading(true))
            .child(ProgressBar::new("Wait").is_indeterminate(true))
            .into()
    });
    rt.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    let first = draw(&mut rt, 300, 200);
    assert!(!rt.wants_frames(), "nothing animates");
    let frame = FrameInfo {
        width: 300,
        height: 200,
        scale: 1.0,
        dt: 0.25,
    };
    let later = rt.frame(&frame, &measure());
    assert_eq!(first, later, "frames do not change over time");
    // The indeterminate fill spans the track instead of sliding.
    let t = light();
    let (track, _, _) = rect_filled(&later, c(t.color.background_muted)).unwrap();
    assert_eq!(
        rect_filled(&later, c(t.color.accent)).unwrap().0.width,
        track.width
    );
}

#[test]
fn a11y_marks_focus_stops_and_live_regions() {
    let mut rt = runtime(|| {
        VStack::new()
            .child(Text::new("plain"))
            .child(Button::new("Save").is_loading(true).on_click(()))
            .child(Switch::new("S", true).status(Status::error("Broken")))
            .child(TextInput::new("Name", "").status(Status::success("Saved")))
            .into()
    });
    draw(&mut rt, 400, 600);
    let nodes = a11y_nodes(&rt.accessibility().unwrap());
    let find = |label: &str| {
        nodes
            .iter()
            .find(|n| n.label.as_deref() == Some(label))
            .unwrap()
            .clone()
    };
    assert!(!find("plain").focusable);
    assert!(find("Save").focusable && find("S").focusable);
    assert!(nodes
        .iter()
        .any(|n| n.role == Role::TextInput && n.focusable));
    // Button keeps a polite region holding "Loading" while busy.
    let loading = find("Save")
        .children
        .into_iter()
        .find(|n| n.live.is_some())
        .expect("loading region");
    assert_eq!(
        (loading.label.as_deref(), loading.live),
        (Some("Loading"), Some(Live::Polite))
    );
    assert_eq!(find("Broken").live, Some(Live::Assertive));
    assert_eq!(find("Saved").live, Some(Live::Polite));
}

#[test]
fn idle_buttons_keep_an_empty_loading_region() {
    let mut rt = runtime(|| Button::new("Save").on_click(()).into());
    draw(&mut rt, 400, 600);
    let button = &a11y(&rt, Role::Button)[0];
    let region = button
        .children
        .iter()
        .find(|n| n.live.is_some())
        .expect("persistent region");
    assert_eq!(region.label, None);
}

trait El {
    fn into_el(self) -> Element<()>;
}
impl<T: Into<Element<()>>> El for T {
    fn into_el(self) -> Element<()> {
        self.into()
    }
}

/// Measures like `FixedMeasure`, but its glyphs grow when a font "loads".
struct Loading {
    advance: std::cell::Cell<f32>,
    generation: std::cell::Cell<u64>,
}

impl TextMeasure for Loading {
    fn measure(&self, text: &str, style: &TextStyle, max: Option<f32>) -> Size {
        testing::FixedMeasure {
            advance: self.advance.get(),
        }
        .measure(text, style, max)
    }
    fn generation(&self) -> u64 {
        self.generation.get()
    }
}

#[test]
fn a_new_measure_generation_relays_out() {
    let mut rt = runtime(|| HStack::<()>::new().child(Text::new("abcd")).into());
    let m = Loading {
        advance: 0.5.into(),
        generation: 0.into(),
    };
    let frame = testing::frame(400, 100);
    let before = texts(&rt.frame(&frame, &m))[0].0.width;
    m.advance.set(1.0);
    assert_eq!(
        texts(&rt.frame(&frame, &m))[0].0.width,
        before,
        "same generation, cached sizes"
    );
    m.generation.set(1);
    assert_eq!(texts(&rt.frame(&frame, &m))[0].0.width, before * 2.0);
}

/// Each focus ring in `list` with the clip rect active where it is drawn.
fn rings_with_clips(list: &DisplayList, color: Color) -> Vec<(Rect, Option<Rect>)> {
    let mut clips: Vec<Rect> = Vec::new();
    let mut out = Vec::new();
    for cmd in &list.commands {
        match cmd {
            DrawCmd::PushClip { rect, .. } => clips.push(*rect),
            DrawCmd::PopClip => {
                clips.pop();
            }
            DrawCmd::Rect {
                rect,
                fill,
                stroke: Some((c, w)),
                ..
            } if *c == color && fill.a == 0.0 && *w == FOCUS_RING_WIDTH => {
                out.push((*rect, clips.last().copied()))
            }
            _ => {}
        }
    }
    out
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x - 0.01
        && inner.y >= outer.y - 0.01
        && inner.x + inner.width <= outer.x + outer.width + 0.01
        && inner.y + inner.height <= outer.y + outer.height + 0.01
}

#[test]
fn focus_rings_at_scroll_and_card_edges_are_never_clipped() {
    let accent = Color::from(light().color.accent);
    // A button flush against a scroll container's top-left edge, and one
    // flush inside a card with no padding.
    let mut rt = runtime(|| {
        VStack::new()
            .is_scrollable(true)
            .child(Button::new("Edge").on_click(0))
            .child(
                Card::new()
                    .padding(Space::S0)
                    .child(Button::new("In card").on_click(1)),
            )
            .children((0..20).map(|i| Text::new(format!("line {i}"))))
            .into()
    });
    for stop in 0..2 {
        let list = draw(&mut rt, 300, 200);
        let rings = rings_with_clips(&list, accent);
        assert_eq!(rings.len(), 1, "stop {stop}");
        let (ring, clip) = rings[0];
        let clip = clip.expect("the scroll viewport, grown by the ring");
        assert!(contains(clip, ring), "ring {ring:?} cut by clip {clip:?}");
        let reach = FOCUS_RING_OFFSET + FOCUS_RING_WIDTH;
        assert_eq!(
            clip,
            Rect::new(-reach, -reach, 300.0 + 2.0 * reach, 200.0 + 2.0 * reach)
        );
        // Rings paint after all content: only clip and opacity pops follow.
        let last = list
            .commands
            .iter()
            .rposition(|c| matches!(c, DrawCmd::Rect { stroke: Some((c, _)), .. } if *c == accent))
            .unwrap();
        assert!(list.commands[last + 1..]
            .iter()
            .all(|c| matches!(c, DrawCmd::PopClip | DrawCmd::PopOpacity)));
        tap(&mut rt, Input::Next);
    }
}

#[test]
fn a_disabled_focused_control_dims_its_ring() {
    let mut rt = runtime(|| Button::new("Off").is_disabled(true).on_click(()).into_el());
    let list = draw(&mut rt, 200, 100);
    let ring = list
        .commands
        .iter()
        .position(|c| matches!(c, DrawCmd::Rect { stroke: Some((_, w)), fill, .. } if *w == FOCUS_RING_WIDTH && fill.a == 0.0))
        .unwrap();
    assert_eq!(list.commands[ring - 1], DrawCmd::PushOpacity(0.5));
}

// ---- D-pad scrolling ----

fn offset_of<M: Clone + 'static>(rt: &Runtime<View<M>>, label: &str) -> f32 {
    a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.label.as_deref() == Some(label))
        .unwrap()
        .bounds
        .y
}

/// Two buttons with a long text block between them, in a 300 × 200 window.
fn long_page() -> Runtime<View<&'static str>> {
    let mut rt = runtime(|| {
        VStack::new()
            .is_scrollable(true)
            .padding(Space::S4)
            .gap(Space::S4)
            .child(Button::new("Top").on_click("top"))
            .children(
                (0..40).map(|i| Text::new(format!("Paragraph line {i} that cannot take focus"))),
            )
            .child(Button::new("Bottom").on_click("bottom"))
            .into()
    });
    draw(&mut rt, 300, 200);
    rt
}

#[test]
fn down_scrolls_through_text_a_third_at_a_time_then_reaches_the_next_button() {
    let mut rt = long_page();
    let start = offset_of(&rt, "Paragraph line 0 that cannot take focus");
    tap(&mut rt, Input::Down);
    draw(&mut rt, 300, 200);
    assert_eq!(
        focused_label(&rt).as_deref(),
        Some("Top"),
        "focus stays while the far button is out of view"
    );
    let moved = start - offset_of(&rt, "Paragraph line 0 that cannot take focus");
    assert!(
        (moved - 200.0 / 3.0).abs() < 1.0,
        "scrolled a third of the viewport: {moved}"
    );
    let mut presses = 1;
    while focused_label(&rt).as_deref() == Some("Top") {
        tap(&mut rt, Input::Down);
        draw(&mut rt, 300, 200);
        presses += 1;
        assert!(presses < 100, "never reached the bottom button");
    }
    assert_eq!(focused_label(&rt).as_deref(), Some("Bottom"));
    let bottom = a11y(&rt, Role::Button)[1].bounds;
    assert!(
        bottom.y >= 0.0 && bottom.y + bottom.height <= 200.0,
        "the new focus is in view"
    );
    // At the end of the content Down does nothing.
    let before = offset_of(&rt, "Bottom");
    tap(&mut rt, Input::Down);
    draw(&mut rt, 300, 200);
    assert_eq!(
        (focused_label(&rt).as_deref(), offset_of(&rt, "Bottom")),
        (Some("Bottom"), before)
    );
    // Up works the same way back.
    let mut presses = 0;
    while focused_label(&rt).as_deref() == Some("Bottom") {
        tap(&mut rt, Input::Up);
        draw(&mut rt, 300, 200);
        presses += 1;
        assert!(presses < 100);
    }
    assert_eq!(focused_label(&rt).as_deref(), Some("Top"));
    assert!(offset_of(&rt, "Top") >= 0.0);
}

#[test]
fn close_neighbours_in_view_take_focus_at_once() {
    let mut rt = runtime(buttons(3));
    draw(&mut rt, 400, 600);
    tap(&mut rt, Input::Down);
    draw(&mut rt, 400, 600);
    assert_eq!(focused_label(&rt).as_deref(), Some("B1"));
}

#[test]
fn paging_scrolls_a_viewport_and_brings_focus_into_view() {
    let mut rt = runtime(|| {
        VStack::new()
            .gap(Space::S2)
            .is_scrollable(true)
            .children((0..30).map(|i| Button::new(format!("B{i}")).on_click(i)))
            .into()
    });
    draw(&mut rt, 300, 200);
    let first = offset_of(&rt, "B0");
    tap(&mut rt, Input::R2);
    draw(&mut rt, 300, 200);
    let paged = first - offset_of(&rt, "B0");
    let overlap = light().space.s6;
    assert_eq!(paged, 200.0 - overlap, "one viewport less a small overlap");
    let focused = a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.focused)
        .unwrap();
    assert!(
        focused.bounds.y >= 0.0 && focused.bounds.y + focused.bounds.height <= 200.0,
        "{:?}",
        focused.bounds
    );
    assert_ne!(
        focused.label.as_deref(),
        Some("B0"),
        "the old focus scrolled away"
    );
    tap(&mut rt, Input::L2);
    draw(&mut rt, 300, 200);
    assert_eq!(offset_of(&rt, "B0"), first);
    // Paging past the start does nothing.
    let focus = rt.focused();
    tap(&mut rt, Input::L2);
    assert_eq!(rt.focused(), focus);
}

#[test]
fn scrolling_is_smooth_and_shows_an_indicator_that_fades() {
    let neutral = Color::from(light().color.neutral);
    let mut rt = long_page();
    let frame = |dt: f32| FrameInfo {
        width: 300,
        height: 200,
        scale: 1.0,
        dt,
    };
    tap(&mut rt, Input::R2);
    assert!(rt.wants_frames());
    let line = |list: &DisplayList| {
        texts(list)
            .iter()
            .find(|t| t.1.starts_with("Paragraph line 0"))
            .map(|t| t.0.y)
    };
    let start = line(&rt.frame(&frame(0.0), &measure()));
    let mid = rt.frame(&frame(0.05), &measure());
    let end_list = rt.frame(&frame(0.2), &measure());
    let (y0, y1) = (start.unwrap(), line(&mid).unwrap_or(f32::MIN));
    assert!(y1 < y0, "moving after 50 ms");
    assert!(
        line(&end_list).is_none_or(|y2| y2 < y1),
        "and further once settled"
    );
    let thumb = rects(&end_list)
        .into_iter()
        .find(|r| r.2 == neutral && r.0.width == light().space.s1);
    let thumb = thumb.expect("indicator thumb");
    assert_eq!(thumb.0.x + thumb.0.width, 300.0 - light().space.s0_5);
    // It stays a second, then fades and goes; then frames stop.
    let held = rt.frame(&frame(0.5), &measure());
    assert!(
        rects(&held)
            .iter()
            .any(|r| r.2 == neutral && r.0.width == light().space.s1),
        "still shown"
    );
    let fading = rt.frame(&frame(0.3), &measure());
    assert!(fading
        .commands
        .iter()
        .any(|c| matches!(c, DrawCmd::PushOpacity(o) if *o < 1.0)));
    let gone = rt.frame(&frame(0.2), &measure());
    assert!(!rects(&gone)
        .iter()
        .any(|r| r.2 == neutral && r.0.width == light().space.s1));
    assert!(!rt.wants_frames());
}

#[test]
fn reduced_motion_scrolls_at_once() {
    let mut rt = long_page();
    rt.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    draw(&mut rt, 300, 200);
    tap(&mut rt, Input::R2);
    let list = rt.frame(
        &FrameInfo {
            width: 300,
            height: 200,
            scale: 1.0,
            dt: 0.0,
        },
        &measure(),
    );
    let y = texts(&list)
        .iter()
        .find(|t| t.1 == "Bottom" || t.1.starts_with("Paragraph line 8"))
        .map(|t| t.0.y);
    assert!(y.is_some());
    let a11y_y = offset_of(&rt, "Paragraph line 0 that cannot take focus");
    let painted = texts(&list)
        .iter()
        .find(|t| t.1.starts_with("Paragraph line 0"))
        .map(|t| t.0.y);
    assert!(
        painted.is_none_or(|p| p == a11y_y),
        "painted where it is, not on the way"
    );
}

// ---- Wheel scrolling ----

#[test]
fn wheel_scrolls_the_innermost_container_that_can_move() {
    let mut rt = runtime(|| {
        VStack::<()>::new()
            .is_scrollable(true)
            .child(
                HStack::new()
                    .is_scrollable(true)
                    .gap(Space::S2)
                    .children((0..30).map(|i| Text::new(format!("chip{i}")))),
            )
            .children((0..40).map(|i| Text::new(format!("row {i}"))))
            .into()
    });
    draw(&mut rt, 300, 200);
    let x_of = |rt: &Runtime<View<()>>| offset_of_x(rt, "chip0");
    // Over the chips, a horizontal wheel moves the chips; a vertical one
    // passes to the page, which can move that way.
    assert!(rt.scroll(Point { x: 10.0, y: 5.0 }, Point { x: 50.0, y: 0.0 }));
    draw(&mut rt, 300, 200);
    assert_eq!(x_of(&rt), -50.0);
    assert!(rt.scroll(Point { x: 10.0, y: 5.0 }, Point { x: 0.0, y: 30.0 }));
    draw(&mut rt, 300, 200);
    assert_eq!(offset_of(&rt, "row 0"), 20.0 - 30.0);
    assert!(rt.wants_frames(), "the indicator shows");
    // Clamped at the start: scrolling further up does nothing.
    assert!(rt.scroll(Point { x: 10.0, y: 100.0 }, Point { x: 0.0, y: -1000.0 }));
    assert!(!rt.scroll(Point { x: 10.0, y: 100.0 }, Point { x: 0.0, y: -10.0 }));
    // Outside every container nothing moves.
    assert!(!rt.scroll(Point { x: 500.0, y: 500.0 }, Point { x: 0.0, y: 10.0 }));
}

fn offset_of_x<M: Clone + 'static>(rt: &Runtime<View<M>>, label: &str) -> f32 {
    a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.label.as_deref() == Some(label))
        .unwrap()
        .bounds
        .x
}

#[test]
fn a_held_direction_repeats_after_a_delay() {
    let mut rt = runtime(buttons(10));
    let frame = |dt: f32| FrameInfo {
        width: 400,
        height: 600,
        scale: 1.0,
        dt,
    };
    rt.frame(&frame(0.0), &measure());
    rt.input(Input::Down, InputPhase::Pressed);
    // A platform key repeat for the held key is ignored.
    rt.input(Input::Down, InputPhase::Pressed);
    rt.frame(&frame(0.0), &measure());
    assert_eq!(focused_label(&rt).as_deref(), Some("B1"));
    rt.frame(&frame(0.3), &measure());
    assert_eq!(
        focused_label(&rt).as_deref(),
        Some("B1"),
        "no repeat before 400 ms"
    );
    rt.frame(&frame(0.15), &measure());
    rt.frame(&frame(0.0), &measure());
    assert_eq!(focused_label(&rt).as_deref(), Some("B2"));
    rt.frame(&frame(0.1), &measure());
    rt.frame(&frame(0.0), &measure());
    assert_eq!(
        focused_label(&rt).as_deref(),
        Some("B3"),
        "then every 90 ms"
    );
    rt.input(Input::Down, InputPhase::Released);
    rt.frame(&frame(0.5), &measure());
    rt.frame(&frame(0.0), &measure());
    assert_eq!(focused_label(&rt).as_deref(), Some("B3"));
    assert!(!rt.wants_frames());
}

// ---- Right to left ----

struct Dir<M: Clone + 'static> {
    view: Box<dyn Fn() -> Element<M>>,
    sent: Vec<M>,
}

impl<M: Clone + 'static> App for Dir<M> {
    type Msg = M;
    fn view(&self) -> Element<M> {
        (self.view)()
    }
    fn update(&mut self, msg: M) -> Command {
        self.sent.push(msg);
        Command::None
    }
    fn direction(&self) -> Direction {
        Direction::Rtl
    }
}

fn rtl<M: Clone + 'static>(view: impl Fn() -> Element<M> + 'static) -> Runtime<Dir<M>> {
    Runtime::new(Dir {
        view: Box::new(view),
        sent: Vec::new(),
    })
}

fn labels_x<A: App>(rt: &Runtime<A>) -> Vec<(String, f32)> {
    a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .filter_map(|n| Some((n.label?, n.bounds.x)))
        .collect()
}

#[test]
fn rows_padding_and_text_mirror() {
    let mut rt = rtl(|| {
        VStack::<()>::new()
            .padding_inline_start(Space::S8)
            .child(
                HStack::new()
                    .gap(Space::S2)
                    .child(Text::new("first"))
                    .child(Text::new("second")),
            )
            .child(Text::new("aligned"))
            .into()
    });
    let list = draw(&mut rt, 400, 300);
    let x = labels_x(&rt);
    let at = |l: &str| x.iter().find(|(n, _)| n == l).unwrap().1;
    assert!(
        at("first") > at("second"),
        "the first child sits on the right"
    );
    let first = texts(&list).into_iter().find(|t| t.1 == "first").unwrap().0;
    assert_eq!(
        first.x + first.width,
        400.0 - light().space.s8,
        "inline-start padding is on the right"
    );
    let aligned = list.commands.iter().find_map(|c| match c {
        DrawCmd::Text {
            text, rect, align, ..
        } if text == "aligned" => Some((*rect, *align)),
        _ => None,
    });
    let (rect, align) = aligned.unwrap();
    assert_eq!(align, TextAlign::End, "start alignment is the right edge");
    assert_eq!(rect.x, 0.0);
    assert_eq!(
        rect.width,
        400.0 - light().space.s8,
        "inline-start padding is on the right"
    );
}

#[test]
fn switch_thumbs_travel_toward_inline_end() {
    let t = light();
    for (on, inset) in [(false, 4.0), (true, 2.0)] {
        let mut rt = rtl(move || Switch::<()>::new("S", on).into());
        let list = draw(&mut rt, 300, 100);
        let track_color = if on {
            t.color.accent
        } else {
            t.color.background_gray
        };
        let (track, _, _) = rect_filled(&list, c(track_color)).unwrap();
        let (thumb, _, _) = rect_filled(&list, c(t.color.background_surface)).unwrap();
        if on {
            assert_eq!(thumb.x, track.x + inset, "on: at the left, the inline end");
        } else {
            assert_eq!(
                thumb.x + thumb.width,
                track.x + track.width - inset,
                "off: at the right, the inline start"
            );
        }
        assert_eq!(track.x + track.width, 300.0, "the track leads on the right");
    }
}

#[test]
fn right_to_left_rows_scroll_toward_the_left() {
    let mut rt = rtl(|| {
        // Avatars keep their size, so the row overflows.
        HStack::new()
            .gap(Space::S2)
            .is_scrollable(true)
            .children((0..20).map(|i| Avatar::new().alt(format!("B{i}")).on_click(i)))
            .into()
    });
    draw(&mut rt, 200, 100);
    let at =
        |rt: &Runtime<Dir<i32>>, l: &str| labels_x(rt).into_iter().find(|(n, _)| n == l).unwrap().1;
    let first = at(&rt, "B0");
    assert!(
        first > 150.0,
        "the first button is at the right edge: {first}"
    );
    // Left is toward inline end: focus and scroll move left through the row.
    for _ in 0..10 {
        tap(&mut rt, Input::Left);
        draw(&mut rt, 200, 100);
    }
    let focused = a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.focused)
        .unwrap();
    assert_ne!(focused.label.as_deref(), Some("B0"));
    assert!(
        focused.bounds.x >= 0.0 && focused.bounds.x + focused.bounds.width <= 200.0,
        "{:?}",
        focused.bounds
    );
    assert!(
        at(&rt, "B0") > first,
        "the content moved right: {} then {}",
        first,
        at(&rt, "B0")
    );
    // A wheel scrolling toward the left (negative x) moves farther along too.
    let before = at(&rt, "B0");
    assert!(rt.scroll(Point { x: 100.0, y: 10.0 }, Point { x: -40.0, y: 0.0 }));
    draw(&mut rt, 200, 100);
    assert_eq!(at(&rt, "B0"), before + 40.0);
}

#[test]
fn menus_line_up_with_the_trigger_start() {
    let mut rt = rtl(|| {
        HStack::new()
            .h_align(StackJustify::Start)
            .child(Selector::new("Fruit", ["apple", "banana"]).on_change(|v: String| v))
            .into()
    });
    let t = light();
    draw(&mut rt, 400, 300);
    tap(&mut rt, Input::A);
    let list = draw(&mut rt, 400, 300);
    let (menu, _, _) = rect_filled(&list, c(t.color.background_popover)).unwrap();
    assert_eq!(
        menu.x + menu.width,
        400.0,
        "aligned to the trigger's right edge"
    );
}

#[test]
fn spinners_off_screen_cost_no_frames() {
    let list = |n: usize| {
        move || {
            let mut stack = VStack::new();
            for i in 0..n {
                stack = stack.child(Button::new(format!("Item {i}")).on_click(i));
            }
            stack.child(Button::new("Saving").is_loading(true)).into()
        }
    };
    let mut rt = runtime(list(1));
    draw(&mut rt, 400, 600);
    assert!(rt.wants_frames(), "a visible spinner animates");
    let mut rt = runtime(list(60));
    draw(&mut rt, 400, 600);
    assert!(!rt.wants_frames(), "a spinner below the viewport does not");
}

#[test]
fn accessibility_nodes_carry_their_direction() {
    let mut rt = rtl(|| {
        VStack::<()>::new()
            .child(Text::new("rtl"))
            .child(Element::from(Text::new("ltr")).direction(Direction::Ltr))
            .into()
    });
    draw(&mut rt, 300, 200);
    let tree = rt.accessibility().unwrap();
    assert!(tree.rtl);
    let nodes = a11y_nodes(&tree);
    let dir = |l: &str| {
        nodes
            .iter()
            .find(|n| n.label.as_deref() == Some(l))
            .unwrap()
            .rtl
    };
    assert!(dir("rtl"));
    assert!(!dir("ltr"), "a nested region keeps its own direction");
}

#[test]
fn text_takes_its_paragraph_direction_from_its_region() {
    let mut rt = rtl(|| {
        VStack::<()>::new()
            .child(Text::new("Astryx שלום"))
            .child(Element::from(Text::new("plain")).direction(Direction::Ltr))
            .into()
    });
    let list = draw(&mut rt, 300, 200);
    let dir = |s: &str| {
        list.commands.iter().find_map(|c| match c {
            DrawCmd::Text { text, style, .. } if text == s => Some(style.direction),
            _ => None,
        })
    };
    assert_eq!(dir("Astryx שלום"), Some(Some(TextDirection::Rtl)));
    assert_eq!(dir("plain"), Some(Some(TextDirection::Ltr)));
}

struct Follow {
    seen: Vec<Option<String>>,
    removed: bool,
}

impl App for Follow {
    type Msg = Option<String>;
    fn view(&self) -> Element<Self::Msg> {
        let mut stack = VStack::new().child(Button::new("One").on_click(None).id("one"));
        if !self.removed {
            stack = stack.child(Button::new("Two").on_click(None).id("two"));
        }
        stack.child(Button::new("Plain").on_click(None)).into()
    }
    fn update(&mut self, msg: Self::Msg) -> Command {
        self.seen.push(msg);
        Command::None
    }
    fn on_focus(&self, id: Option<&str>) -> Option<Self::Msg> {
        Some(id.map(String::from))
    }
}

#[test]
fn on_focus_reports_each_focus_change_with_the_id_prop() {
    let mut rt = Runtime::new(Follow {
        seen: Vec::new(),
        removed: false,
    });
    draw(&mut rt, 300, 300);
    assert_eq!(rt.app().seen, [Some("one".to_string())], "launch focus");
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::Down);
    draw(&mut rt, 300, 300);
    // An element without an id reports None; no change, no report.
    tap(&mut rt, Input::Down);
    assert_eq!(
        rt.app().seen,
        [Some("one".into()), Some("two".into()), None]
    );
    // An accessibility action moves focus too.
    let two = a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.label.as_deref() == Some("Two"))
        .unwrap()
        .id;
    rt.a11y_action(two, A11yAction::Focus);
    assert_eq!(rt.app().seen.last(), Some(&Some("two".to_string())));
    // A rebuild that removes the focused element moves focus and reports it.
    rt.app_mut().removed = true;
    draw(&mut rt, 300, 300);
    let last = rt.app().seen.last().cloned().flatten();
    assert_ne!(last.as_deref(), Some("two"));
}
