//! SelectableCard and the experimental lab components.

mod common;

use astryx_core::lab::{HoldButton, HoldCard, Image, SelectableCardGroup, StatusBar};
use astryx_core::testing::*;
use astryx_core::*;
use common::*;

fn focused<M: Clone + 'static>(rt: &Runtime<View<M>>) -> Option<A11yNode> {
    a11y_nodes(&rt.accessibility()?)
        .into_iter()
        .find(|n| n.focused)
}

fn step<A: App>(rt: &mut Runtime<A>, dt: f32) -> DisplayList {
    rt.frame(
        &FrameInfo {
            width: 400,
            height: 300,
            scale: 1.0,
            dt,
        },
        &measure(),
    )
}

/// The hold fill: the pressed tint, made stronger.
fn hold_fill(t: &Tokens) -> Color {
    let p = c(t.color.overlay_pressed);
    p.with_alpha((p.a * 3.0).clamp(0.25, 0.5))
}

// ---- SelectableCard ----

#[test]
fn a_selectable_card_is_a_checkbox_with_an_inset_ring() {
    let t = light();
    let mut rt = runtime(|| {
        VStack::new()
            .gap(Space::S4)
            .padding(Space::S4)
            .child(
                SelectableCard::new("Pro plan", true)
                    .on_change(|on| on)
                    .child(Text::new("Pro")),
            )
            .child(
                SelectableCard::new("Free plan", false)
                    .on_change(|on| on)
                    .child(Text::new("Free")),
            )
            .into()
    });
    let list = draw(&mut rt, 400, 300);
    let cards = a11y(&rt, Role::Checkbox);
    assert_eq!(cards.len(), 2);
    assert_eq!(
        (cards[0].label.as_deref(), cards[0].checked),
        (Some("Pro plan"), Some(true))
    );
    assert_eq!(cards[1].checked, Some(false));
    // The selected card draws a 2pt ring in the accent over its content.
    let ring = rects(&list)
        .into_iter()
        .find(|r| r.3 == Some((c(t.color.accent), 2.0)));
    assert_eq!(ring.map(|r| r.0), Some(cards[0].bounds));
    // Both cards are the same height: the ring takes no layout.
    assert_eq!(cards[0].bounds.height, cards[1].bounds.height);
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().sent, [true], "the free card toggles on");
}

#[test]
fn a_disabled_selectable_card_keeps_focus_and_does_not_toggle() {
    let mut rt = runtime(|| {
        SelectableCard::new("Pro", false)
            .is_disabled(true)
            .on_change(|on| on)
            .child(Text::new("Pro"))
            .into()
    });
    draw(&mut rt, 300, 200);
    assert!(focused(&rt).is_some_and(|n| n.disabled));
    tap(&mut rt, Input::A);
    assert!(rt.app().sent.is_empty());
}

// ---- SelectableCardGroup ----

fn plans(value: Option<&'static str>) -> impl Fn() -> Element<String> {
    move || {
        let mut group = SelectableCardGroup::new("Plan").on_change(|v| v);
        if let Some(v) = value {
            group = group.value(v);
        }
        for (v, label) in [("free", "Free"), ("pro", "Pro"), ("team", "Team")] {
            group = group.option(v, SelectableCard::new(label, false).child(Text::new(label)));
        }
        VStack::new()
            .gap(Space::S4)
            .child(Button::new("Before").on_click("before".to_string()))
            .child(group)
            .child(Button::new("After").on_click("after".to_string()))
            .into()
    }
}

#[test]
fn a_card_group_chooses_one_and_is_one_step_for_next() {
    let mut rt = runtime(plans(Some("pro")));
    draw(&mut rt, 400, 600);
    let group = a11y(&rt, Role::RadioGroup);
    assert_eq!(group[0].label.as_deref(), Some("Plan"));
    let options = a11y(&rt, Role::Radio);
    assert_eq!(
        options.iter().map(|o| o.checked).collect::<Vec<_>>(),
        [Some(false), Some(true), Some(false)]
    );
    // Next from Before lands on the chosen card; Next again leaves the group.
    tap(&mut rt, Input::Next);
    draw(&mut rt, 400, 600);
    assert_eq!(focused(&rt).and_then(|n| n.label).as_deref(), Some("Pro"));
    tap(&mut rt, Input::Next);
    draw(&mut rt, 400, 600);
    assert_eq!(focused(&rt).and_then(|n| n.label).as_deref(), Some("After"));
    tap(&mut rt, Input::Previous);
    draw(&mut rt, 400, 600);
    assert_eq!(
        focused(&rt).and_then(|n| n.label).as_deref(),
        Some("Pro"),
        "back to the chosen card"
    );
    // The arrows move between cards; A chooses.
    tap(&mut rt, Input::Down);
    draw(&mut rt, 400, 600);
    assert_eq!(focused(&rt).and_then(|n| n.label).as_deref(), Some("Team"));
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().sent, ["team"]);
}

#[test]
fn with_nothing_chosen_the_group_is_entered_at_its_edge() {
    let mut rt = runtime(plans(None));
    draw(&mut rt, 400, 600);
    tap(&mut rt, Input::Next);
    draw(&mut rt, 400, 600);
    assert_eq!(focused(&rt).and_then(|n| n.label).as_deref(), Some("Free"));
    tap(&mut rt, Input::Next);
    tap(&mut rt, Input::Previous);
    draw(&mut rt, 400, 600);
    assert_eq!(
        focused(&rt).and_then(|n| n.label).as_deref(),
        Some("Team"),
        "entered from below at the last card"
    );
}

// ---- HoldButton ----

#[test]
fn a_hold_button_confirms_only_when_held_to_the_end() {
    let t = light();
    let mut rt = runtime(|| {
        HoldButton::new("Delete")
            .duration_ms(1000)
            .on_confirm("deleted")
            .into()
    });
    step(&mut rt, 0.0);
    let button = a11y(&rt, Role::Button)[0].clone();
    assert_eq!(button.description.as_deref(), Some("Hold to confirm"));
    // Let go early: nothing is sent, and the fill drains.
    rt.input(Input::A, InputPhase::Pressed);
    step(&mut rt, 0.0);
    let half = step(&mut rt, 0.5);
    let fill = rects(&half)
        .into_iter()
        .find(|r| r.2 == hold_fill(t))
        .expect("a fill");
    assert!(
        (fill.0.width - button.bounds.width / 2.0).abs() < button.bounds.width * 0.05,
        "{:?}",
        fill.0
    );
    assert_eq!(fill.0.x, button.bounds.x, "it grows from inline start");
    rt.input(Input::A, InputPhase::Released);
    step(&mut rt, 1.0);
    assert!(rt.app().sent.is_empty(), "released early");
    assert!(!rt.wants_frames());
    // Held to the end: sent once, at the end, not on release.
    rt.input(Input::A, InputPhase::Pressed);
    step(&mut rt, 0.0);
    step(&mut rt, 0.6);
    assert!(rt.app().sent.is_empty());
    step(&mut rt, 0.5);
    assert_eq!(rt.app().sent, ["deleted"]);
    rt.input(Input::A, InputPhase::Released);
    step(&mut rt, 0.1);
    assert_eq!(rt.app().sent, ["deleted"], "release sends nothing more");
}

#[test]
fn reduced_motion_fills_a_hold_in_quarter_steps() {
    let t = light();
    let mut rt = runtime(|| HoldButton::new("Delete").on_confirm(()).into());
    rt.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    step(&mut rt, 0.0);
    let width = a11y(&rt, Role::Button)[0].bounds.width;
    rt.input(Input::A, InputPhase::Pressed);
    step(&mut rt, 0.0);
    let list = step(&mut rt, 0.6);
    let fill = rects(&list)
        .into_iter()
        .find(|r| r.2 == hold_fill(t))
        .expect("a fill");
    assert!(
        (fill.0.width - width / 2.0).abs() < 0.01,
        "0.6 s shows the half step: {}",
        fill.0.width
    );
}

// ---- StatusBar ----

#[test]
fn the_status_bar_shows_the_device_and_reads_as_one_line() {
    let mut rt = runtime(|| Element::<()>::from(StatusBar::new()));
    let device = DeviceStatus {
        wifi: Radio::Connected,
        battery_percent: Some(82),
        charging: true,
        time: (19, 42),
        ..DeviceStatus::default()
    };
    let list = draw(&mut rt, 400, 100);
    assert!(
        list.commands.is_empty() || a11y_nodes(&rt.accessibility().unwrap()).len() == 1,
        "nothing without a device"
    );
    rt.set_preferences(&Preferences {
        device: Some(device),
        ..Preferences::default()
    });
    let list = draw(&mut rt, 400, 100);
    let nodes = a11y_nodes(&rt.accessibility().unwrap());
    let bar = nodes
        .iter()
        .find(|n| n.label.as_deref().is_some_and(|l| l.starts_with("Wi-Fi")))
        .expect("a status bar");
    assert_eq!(
        bar.label.as_deref(),
        Some("Wi-Fi connected, battery 82% charging, 7:42 PM")
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|n| n.label.as_deref() == Some("82%"))
            .count(),
        0,
        "one line, not its parts"
    );
    // The digits and PM draw apart, so PM can shrink without wrapping.
    assert!(texts(&list).iter().any(|t| t.1 == "7:42"));
    assert!(texts(&list).iter().any(|t| t.1 == " PM"));
    assert!(texts(&list).iter().any(|t| t.1 == "82%"));
    let t = light();
    assert!(
        rects(&list).iter().any(|r| r.2 == c(t.color.icon_success)),
        "a charging gauge is green"
    );
}

#[test]
fn the_status_bar_shows_a_muted_speaker_while_sounds_are_off() {
    let device = DeviceStatus {
        wifi: Radio::Connected,
        battery_percent: Some(82),
        time: (7, 5),
        ..DeviceStatus::default()
    };
    let paths = |list: &DisplayList| {
        list.commands
            .iter()
            .filter(|c| matches!(c, DrawCmd::Path { .. }))
            .count()
    };
    let mut rt = runtime(|| Element::<()>::from(StatusBar::new()));
    rt.set_preferences(&Preferences {
        device: Some(device.clone()),
        ..Preferences::default()
    });
    let sounding = draw(&mut rt, 400, 100);
    rt.set_preferences(&Preferences {
        device: Some(device.clone()),
        sound_off: true,
        ..Preferences::default()
    });
    let muted = draw(&mut rt, 400, 100);
    assert_eq!(paths(&muted), paths(&sounding) + 1, "one more icon");
    let nodes = a11y_nodes(&rt.accessibility().unwrap());
    assert!(nodes
        .iter()
        .any(|n| n.label.as_deref() == Some("Wi-Fi connected, sounds off, battery 82%, 7:05 AM")));
    // A preview can show either state whatever the shell says.
    let shown =
        paint(move || Element::<()>::from(StatusBar::new().device(device.clone()).sound_off(true)));
    assert_eq!(paths(&shown), paths(&sounding) + 1);
}

#[test]
fn wifi_is_crossed_out_whenever_it_is_not_connected() {
    let paths = |wifi: Radio| {
        let device = DeviceStatus {
            wifi,
            time: (9, 0),
            ..DeviceStatus::default()
        };
        let list = paint(move || Element::<()>::from(StatusBar::new().device(device.clone())));
        list.commands
            .iter()
            .find_map(|c| match c {
                DrawCmd::Path { commands, .. } => Some(commands.len()),
                _ => None,
            })
            .unwrap()
    };
    let connected = paths(Radio::Connected);
    assert!(
        paths(Radio::On) > connected,
        "on but not connected: crossed out"
    );
    assert_eq!(paths(Radio::On), paths(Radio::Off));
}

#[test]
fn a_low_battery_gauge_turns_to_the_error_color() {
    let t = light();
    let device = DeviceStatus {
        battery_percent: Some(12),
        time: (9, 0),
        ..DeviceStatus::default()
    };
    let list = paint(move || Element::<()>::from(StatusBar::new().device(device.clone())));
    assert!(rects(&list).iter().any(|r| r.2 == c(t.color.icon_error)));
}

// ---- Image ----

#[test]
fn images_fill_the_width_at_their_aspect_ratio() {
    let id = ImageId(42);
    let mut rt = runtime(move || {
        VStack::<()>::new()
            .padding(Space::S4)
            .child(Image::new(id, "A beach at sunset").fit(ImageFit::Contain))
            .child(Image::new(ImageId(43), ""))
            .into()
    })
    .with_image(
        id,
        Some(ImageData {
            width: 4,
            height: 2,
            rgba: vec![255; 32].into(),
        }),
    );
    let list = draw(&mut rt, 400, 600);
    let images = a11y(&rt, Role::Image);
    assert_eq!(images.len(), 1, "the decorative image is not read");
    assert_eq!(images[0].label.as_deref(), Some("A beach at sunset"));
    assert_eq!(
        images[0].bounds,
        Rect::new(16.0, 16.0, 368.0, 184.0),
        "full width at the image's 2:1"
    );
    let drawn = list.commands.iter().find_map(|c| match c {
        DrawCmd::Image { rect, fit, .. } => Some((*rect, *fit)),
        _ => None,
    });
    assert_eq!(drawn, Some((images[0].bounds, ImageFit::Contain)));
    // The unregistered one is a 16:9 placeholder.
    let t = light();
    let placeholder = rect_filled(&list, c(t.color.background_muted)).unwrap().0;
    assert_eq!(placeholder.height, 207.0);
}

#[test]
fn pixel_perfect_images_scale_by_whole_pixels() {
    let id = ImageId(44);
    let mut rt = runtime(move || {
        Element::<()>::from(
            Image::new(id, "Sprite")
                .aspect_ratio(1.0)
                .pixel_perfect(true),
        )
    })
    .with_image(
        id,
        Some(ImageData {
            width: 16,
            height: 16,
            rgba: vec![255; 16 * 16 * 4].into(),
        }),
    );
    let list = rt.frame(
        &FrameInfo {
            width: 200,
            height: 300,
            scale: 2.0,
            dt: 0.0,
        },
        &measure(),
    );
    let drawn = list.commands.iter().find_map(|c| match c {
        DrawCmd::Image { rect, .. } => Some(*rect),
        _ => None,
    });
    // 100 x 100 points is 200 device pixels: 12 whole copies of 16 pixels.
    let r = drawn.unwrap();
    assert_eq!((r.width, r.height), (96.0, 96.0));
    assert!(list
        .commands
        .iter()
        .any(|c| matches!(c, DrawCmd::PushPixelGrid { cell } if *cell == 0.5)));
}

// ---- Kbd and contained rows ----

#[test]
fn kbd_draws_a_keycap_per_key_and_reads_as_one_name() {
    let t = light();
    let mut rt = runtime(|| {
        HStack::<()>::new()
            .child(Kbd::gamepad("B"))
            .child(Text::new("Back"))
            .into()
    });
    let list = draw(&mut rt, 300, 100);
    let caps = a11y(&rt, Role::Image);
    assert_eq!(caps[0].label.as_deref(), Some("B button"));
    assert!(
        a11y_nodes(&rt.accessibility().unwrap())
            .iter()
            .all(|n| n.label.as_deref() != Some("B")),
        "the glyph is not read twice"
    );
    let face = rect_filled(&list, c(t.color.neutral)).expect("a face");
    let edge = rect_filled(&list, c(t.color.border_emphasized)).expect("a bottom edge");
    assert_eq!(
        edge.0.y + edge.0.height,
        face.0.y + face.0.height,
        "the edge runs along the bottom"
    );
    assert_eq!(edge.0.height, 2.0);
    assert!(face.0.width >= t.space.s5 && face.0.height >= t.space.s5);
    let (glyph, style, color) = text(&list, "B");
    assert_eq!(
        (style.size, style.weight, color),
        (
            t.font.supporting.size,
            t.font.weight.medium,
            c(t.color.text_primary)
        )
    );
    assert!(
        glyph.x > face.0.x && glyph.x + glyph.width < face.0.x + face.0.width,
        "padded inside the cap"
    );
    let list = paint(|| Element::<()>::from(Kbd::gamepad("L1 R1")));
    assert_eq!(
        rects(&list)
            .iter()
            .filter(|r| r.2 == c(t.color.neutral))
            .count(),
        2
    );
}

#[test]
fn contained_rows_keep_their_surface_and_border_when_unselected() {
    let t = light();
    let mut rt = runtime(|| {
        VStack::new()
            .gap(Space::S2)
            .child(ListItem::new("Basil").is_contained(true).on_click(0))
            .child(
                ListItem::new("Tomato")
                    .is_contained(true)
                    .is_selected(true)
                    .on_click(1),
            )
            .child(ListItem::new("Plain").on_click(2))
            .into()
    });
    let list = draw(&mut rt, 300, 300);
    let rows = a11y(&rt, Role::Button);
    let at = |r: Rect| {
        rects(&list)
            .into_iter()
            .find(|p| p.0 == r)
            .expect("a row surface")
    };
    let unselected = at(rows[0].bounds);
    assert_eq!(unselected.2, c(t.color.background_card));
    assert_eq!(unselected.3, Some((c(t.color.border), t.size.border_width)));
    let selected = at(rows[1].bounds);
    assert_eq!(
        (selected.2, selected.3.map(|b| b.0)),
        (c(t.color.accent_muted), Some(c(t.color.accent)))
    );
    assert_eq!(
        rows[0].bounds.height, rows[2].bounds.height,
        "the border takes no extra room"
    );
}

// ---- Pointer presses ----

#[test]
fn a_pointer_press_runs_the_same_press_as_a() {
    let mut rt = runtime(|| {
        VStack::new()
            .child(Button::new("One").on_click("one"))
            .child(Button::new("Two").on_click("two"))
            .into()
    });
    step(&mut rt, 0.0);
    let two = a11y(&rt, Role::Button)[1].id;
    rt.a11y_action(two, A11yAction::PressStart);
    step(&mut rt, 0.0);
    assert!(
        focused(&rt).is_some_and(|n| n.id == two),
        "the press focuses"
    );
    assert!(rt.app().sent.is_empty(), "nothing on press");
    rt.a11y_action(two, A11yAction::PressEnd);
    assert_eq!(rt.app().sent, ["two"]);
    rt.a11y_action(two, A11yAction::PressStart);
    rt.a11y_action(two, A11yAction::PressCancel);
    step(&mut rt, 0.1);
    assert_eq!(rt.app().sent, ["two"], "a cancelled press does nothing");
}

#[test]
fn click_and_hold_confirms_a_hold_button_and_a_quick_click_does_not() {
    let mut rt = runtime(|| {
        HoldButton::new("Delete")
            .duration_ms(500)
            .on_confirm("deleted")
            .into()
    });
    step(&mut rt, 0.0);
    let id = a11y(&rt, Role::Button)[0].id;
    rt.a11y_action(id, A11yAction::PressStart);
    step(&mut rt, 0.1);
    rt.a11y_action(id, A11yAction::PressEnd);
    step(&mut rt, 0.5);
    assert!(rt.app().sent.is_empty(), "a quick click does not confirm");
    rt.a11y_action(id, A11yAction::PressStart);
    step(&mut rt, 0.0);
    step(&mut rt, 0.6);
    assert_eq!(rt.app().sent, ["deleted"], "held long enough");
    rt.a11y_action(id, A11yAction::PressEnd);
    // Assistive technology activates at once.
    rt.a11y_action(id, A11yAction::Activate);
    assert_eq!(rt.app().sent, ["deleted", "deleted"]);
}

// ---- Hover and feedback ----

#[derive(Default)]
struct Cues {
    cues: Vec<FeedbackKind>,
    pressed: u32,
}

#[derive(Clone, Debug)]
enum CueMsg {
    Cue(FeedbackKind),
    Press,
}

impl App for Cues {
    type Msg = CueMsg;
    fn view(&self) -> Element<CueMsg> {
        VStack::new()
            .gap(Space::S2)
            .child(Button::new("One").on_click(CueMsg::Press))
            .child(Button::new("Off").is_disabled(true).on_click(CueMsg::Press))
            .child(
                HoldButton::new("Hold")
                    .duration_ms(400)
                    .on_confirm(CueMsg::Press),
            )
            .into()
    }
    fn update(&mut self, msg: CueMsg) -> Command {
        match msg {
            CueMsg::Cue(k) => self.cues.push(k),
            CueMsg::Press => self.pressed += 1,
        }
        Command::None
    }
    fn on_feedback(&self, kind: FeedbackKind) -> Option<CueMsg> {
        Some(CueMsg::Cue(kind))
    }
}

#[test]
fn interactions_send_feedback_cues() {
    use FeedbackKind::*;
    let mut rt = Runtime::new(Cues::default());
    step(&mut rt, 0.0);
    let tap = |rt: &mut Runtime<Cues>, input: Input| {
        rt.input(input, InputPhase::Pressed);
        rt.input(input, InputPhase::Released);
        step(rt, 0.0);
    };
    tap(&mut rt, Input::A);
    tap(&mut rt, Input::Up);
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::A);
    tap(&mut rt, Input::Down);
    rt.input(Input::A, InputPhase::Pressed);
    for _ in 0..30 {
        step(&mut rt, 1.0 / 60.0);
    }
    rt.input(Input::A, InputPhase::Released);
    assert_eq!(
        rt.app().cues,
        [Select, Blocked, Move, Blocked, Move, Progress, Progress, Progress, Success],
        "press, top edge, down, disabled press, down, three quarters of the hold, done"
    );
    assert_eq!(rt.app().pressed, 2);
}

#[test]
fn the_pointer_hovers_the_control_under_it() {
    let t = light();
    let mut rt = runtime(|| {
        VStack::new()
            .child(
                Button::new("One")
                    .variant(ButtonVariant::Secondary)
                    .on_click("one"),
            )
            .child(
                Button::new("Off")
                    .variant(ButtonVariant::Secondary)
                    .is_disabled(true)
                    .on_click("off"),
            )
            .into()
    });
    let list = draw(&mut rt, 300, 200);
    let buttons = a11y(&rt, Role::Button);
    let hover = c(t.color.overlay_hover);
    assert!(rect_filled(&list, hover).is_none());
    let at = |r: Rect| Point {
        x: r.x + r.width / 2.0,
        y: r.y + r.height / 2.0,
    };
    rt.pointer_move(Some(at(buttons[0].bounds)));
    let list = draw(&mut rt, 300, 200);
    let tint = rect_filled(&list, hover).expect("a hover tint");
    assert_eq!(tint.0, buttons[0].bounds);
    rt.pointer_move(Some(at(buttons[1].bounds)));
    let list = draw(&mut rt, 300, 200);
    assert!(
        rect_filled(&list, hover).is_none(),
        "a disabled control does not hover"
    );
    rt.pointer_move(None);
    let list = draw(&mut rt, 300, 200);
    assert!(
        rect_filled(&list, hover).is_none(),
        "leaving the window ends hover"
    );
}

// ---- Cue commands, HoldCard, and blocked moves ----

/// An app that answers each cue with a sound and logs the cues.
struct Sounds<F: Fn() -> Element<String>> {
    view: F,
    cues: Vec<FeedbackKind>,
    sent: Vec<String>,
}

impl<F: Fn() -> Element<String>> App for Sounds<F> {
    type Msg = String;
    fn view(&self) -> Element<String> {
        (self.view)()
    }
    fn update(&mut self, msg: String) -> Command {
        if let Some(kind) = msg.strip_prefix("cue:") {
            let kind = match kind {
                "Move" => FeedbackKind::Move,
                "Select" => FeedbackKind::Select,
                "Blocked" => FeedbackKind::Blocked,
                "Progress" => FeedbackKind::Progress,
                "Success" => FeedbackKind::Success,
                _ => FeedbackKind::Back,
            };
            self.cues.push(kind);
            return Command::PlaySound(SoundId(kind as u32));
        }
        self.sent.push(msg);
        Command::None
    }
    fn on_feedback(&self, kind: FeedbackKind) -> Option<String> {
        Some(format!("cue:{kind:?}"))
    }
}

fn sounds<F: Fn() -> Element<String>>(view: F) -> Runtime<Sounds<F>> {
    Runtime::new(Sounds {
        view,
        cues: Vec::new(),
        sent: Vec::new(),
    })
}

#[test]
fn a_cues_command_comes_back_with_the_input_or_after_the_frame() {
    let mut rt = sounds(|| Button::new("Go").on_click("go".to_string()).into());
    step(&mut rt, 0.0);
    rt.input(Input::A, InputPhase::Pressed);
    let command = rt.input(Input::A, InputPhase::Released);
    assert_eq!(
        command,
        Command::PlaySound(SoundId(FeedbackKind::Select as u32))
    );
    assert_eq!(rt.app().sent, ["go"]);
    // A hold completing in a frame: the shell takes the command after it.
    let mut rt = sounds(|| {
        HoldButton::new("Hold")
            .duration_ms(100)
            .on_confirm("held".to_string())
            .into()
    });
    step(&mut rt, 0.0);
    rt.input(Input::A, InputPhase::Pressed);
    for _ in 0..12 {
        step(&mut rt, 1.0 / 60.0);
    }
    let Command::Batch(commands) = rt.take_command() else {
        panic!("a batch of the cues' sounds");
    };
    assert_eq!(
        commands.last(),
        Some(&Command::PlaySound(SoundId(FeedbackKind::Success as u32)))
    );
    assert_eq!(rt.take_command(), Command::None, "taken once");
    assert_eq!(Command::None.and(Command::Quit), Command::Quit);
}

#[test]
fn a_hold_card_fills_in_the_accent_and_changes_only_when_held_out() {
    let t = light();
    let card = || {
        HoldCard::new(
            SelectableCard::new("Send", false)
                .on_change(|on| format!("send {on}"))
                .child(Text::new("Send it")),
        )
        .duration_ms(400)
        .into()
    };
    let mut rt = sounds(card);
    step(&mut rt, 0.0);
    let node = a11y_nodes(&rt.accessibility().unwrap())
        .into_iter()
        .find(|n| n.role == Role::Checkbox)
        .unwrap();
    assert_eq!(node.description.as_deref(), Some("Hold to confirm"));
    rt.input(Input::A, InputPhase::Pressed);
    let mut list = step(&mut rt, 0.0);
    for _ in 0..12 {
        list = step(&mut rt, 1.0 / 60.0);
    }
    let fill = rects(&list)
        .into_iter()
        .find(|r| r.2 == c(t.color.accent_muted))
        .expect("an accent fill");
    assert!(fill.0.width > 0.0 && fill.0.width < node.bounds.width);
    rt.input(Input::A, InputPhase::Released);
    for _ in 0..30 {
        step(&mut rt, 1.0 / 60.0);
    }
    assert!(rt.app().sent.is_empty(), "let go early: nothing sent");
    rt.input(Input::A, InputPhase::Pressed);
    for _ in 0..30 {
        step(&mut rt, 1.0 / 60.0);
    }
    rt.input(Input::A, InputPhase::Released);
    assert_eq!(rt.app().sent, ["send true"]);
    let after_release = rt
        .app()
        .cues
        .iter()
        .skip_while(|k| **k != FeedbackKind::Progress)
        .copied()
        .collect::<Vec<_>>();
    assert!(after_release.ends_with(&[
        FeedbackKind::Progress,
        FeedbackKind::Progress,
        FeedbackKind::Progress,
        FeedbackKind::Success
    ]));
}

#[test]
fn edges_of_a_card_group_and_a_pager_are_blocked() {
    use astryx_core::paged::Paged;
    let mut rt = sounds(plans_view);
    step(&mut rt, 0.0);
    // Focus starts on Free; right moves to Pro and Team, then nothing is
    // to the right.
    for _ in 0..3 {
        rt.input(Input::Right, InputPhase::Pressed);
        rt.input(Input::Right, InputPhase::Released);
        step(&mut rt, 0.0);
    }
    assert_eq!(
        rt.app().cues,
        [
            FeedbackKind::Move,
            FeedbackKind::Move,
            FeedbackKind::Blocked
        ]
    );
    let mut rt = sounds(|| {
        Paged::new()
            .children((0..12).map(|i| Button::new(format!("B{i}")).on_click(format!("{i}"))))
            .into()
    });
    step(&mut rt, 0.0);
    rt.input(Input::Up, InputPhase::Pressed);
    rt.input(Input::Up, InputPhase::Released);
    rt.input(Input::L2, InputPhase::Pressed);
    rt.input(Input::L2, InputPhase::Released);
    step(&mut rt, 0.0);
    assert_eq!(
        rt.app().cues,
        [FeedbackKind::Blocked, FeedbackKind::Blocked],
        "first page: up and previous page go nowhere"
    );
}

fn plans_view() -> Element<String> {
    let mut group = SelectableCardGroup::new("Plan")
        .on_change(|v| v)
        .value("pro")
        .is_horizontal(true);
    for (v, label) in [("free", "Free"), ("pro", "Pro"), ("team", "Team")] {
        group = group.option(v, SelectableCard::new(label, false).child(Text::new(label)));
    }
    group.into()
}

#[test]
fn the_phone_connection_shows_and_reads_only_without_wifi() {
    let label = |wifi, tether| {
        let mut rt = runtime(move || {
            Element::<()>::from(StatusBar::new().device(DeviceStatus {
                wifi,
                time: (9, 41),
                tether,
                ..DeviceStatus::default()
            }))
        });
        draw(&mut rt, 400, 100);
        a11y_nodes(&rt.accessibility().unwrap())
            .iter()
            .find_map(|n| n.label.clone().filter(|l| l.starts_with("Wi-Fi")))
            .unwrap()
    };
    assert_eq!(
        label(Radio::On, Radio::Connected),
        "Wi-Fi on, not connected, online through your phone, 9:41 AM"
    );
    assert_eq!(
        label(Radio::Off, Radio::On),
        "Wi-Fi off, connecting through your phone, 9:41 AM"
    );
    assert_eq!(
        label(Radio::Connected, Radio::Connected),
        "Wi-Fi connected, 9:41 AM",
        "Wi-Fi in use: the phone is not mentioned"
    );
}
