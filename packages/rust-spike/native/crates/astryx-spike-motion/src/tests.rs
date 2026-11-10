//! The ledger's Motion exercises through the core runtime: a Switch-like
//! thumb on a spring, a layer's enter and exit, Collapsible's height, and
//! Reduce Motion.

use super::*;
use astryx_core::authoring::{element, Build, BoxDraw, Draw, Node};
use astryx_core::testing::{frame, FixedMeasure};
use astryx_core::{
    App, Color, Command, DisplayList, DrawCmd, Element, Preferences, Radii, Runtime, Surface,
    VStack,
};
use astryx_motion::Animated;
use taffy::Style;

fn sized(w: f32, h: f32) -> Style {
    Style {
        size: taffy::Size {
            width: taffy::Dimension::length(w),
            height: taffy::Dimension::length(h),
        },
        ..Style::default()
    }
}

/// A 20 x 20 box: a thumb at `x`, or a block of `h` points.
struct Block {
    anims: Vec<Anim>,
    exit: Option<Exit>,
    resize: Option<Resize>,
    h: f32,
    children: Vec<Element<Msg>>,
    key: Option<&'static str>,
}

impl Build<Msg> for Block {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let mut style = sized(20.0, self.h);
        if self.resize.is_some() {
            style.size.height = taffy::Dimension::auto();
            style.flex_direction = taffy::FlexDirection::Column;
        }
        let mut node = Node::new(style).draw(Draw::Box(BoxDraw::fill(
            Color::rgba(1.0, 0.0, 0.0, 1.0),
            Radii::default(),
        )));
        node.anims = self.anims;
        node.exit = self.exit;
        node.resize = self.resize;
        if node.resize.is_some() {
            node.clip = Some(Radii::default());
        }
        cx.key(self.key);
        cx.begin(node);
        for c in self.children {
            cx.child(c);
        }
        cx.end();
    }
}

fn block(h: f32) -> Block {
    Block {
        anims: vec![],
        exit: None,
        resize: None,
        h,
        children: vec![],
        key: None,
    }
}

#[derive(Clone)]
enum Msg {}

struct Scene {
    on: bool,
    open: bool,
    expanded: bool,
}

impl App for Scene {
    type Msg = Msg;
    fn view(&self) -> Element<Msg> {
        let t = motion_tokens::DEFAULT;
        let mut v = VStack::new();
        let mut thumb = block(20.0);
        thumb.anims = vec![state_change(t, Channel::TranslateX, if self.on { 20.0 } else { 0.0 })];
        v = v.child(element(thumb));
        if self.open {
            let mut layer = block(40.0);
            layer.anims = layer_enter(t, From::Below, false);
            layer.exit = Some(layer_exit(t));
            // Identity by key: the blocks after it keep theirs.
            layer.key = Some("layer");
            v = v.child(element(layer));
        }
        let mut panel = block(0.0);
        panel.resize = Some(collapse(t));
        if self.expanded {
            panel.children = vec![element(block(100.0))];
        }
        v = v.child(element(panel)).child(element(block(10.0)));
        v.into()
    }
    fn update(&mut self, msg: Msg) -> Command {
        match msg {}
    }
}

fn runtime() -> Runtime<Scene> {
    let mut rt = Runtime::new(Scene {
        on: false,
        open: false,
        expanded: false,
    });
    step(&mut rt, 0.0);
    rt
}

fn step(rt: &mut Runtime<Scene>, ms: f32) -> DisplayList {
    let mut f = frame(400, 400);
    f.dt = ms / 1000.0;
    rt.frame(&f, &FixedMeasure::default())
}

/// The x translation of the first transform in `list` (the thumb's).
fn thumb_x(list: &DisplayList) -> f32 {
    list.commands
        .iter()
        .find_map(|c| match c {
            DrawCmd::PushTransform { translate, scale, .. } if *scale == 1.0 => Some(translate.x),
            _ => None,
        })
        .unwrap_or(0.0)
}

/// The y of the last block (the one after the collapsible panel).
fn last_y(list: &DisplayList) -> f32 {
    list.commands
        .iter()
        .rev()
        .find_map(|c| match c {
            DrawCmd::Rect { rect, .. } if rect.height == 10.0 => Some(rect.y),
            _ => None,
        })
        .expect("last block")
}

#[test]
fn a_toggle_retargets_from_where_it_is_without_a_jump() {
    let mut rt = runtime();
    rt.app_mut().on = true;
    let mut xs = vec![thumb_x(&step(&mut rt, 16.0))];
    for _ in 0..6 {
        xs.push(thumb_x(&step(&mut rt, 16.0)));
    }
    let mid = *xs.last().unwrap();
    assert!(mid > 2.0 && mid < 19.0, "mid-way: {xs:?}");
    rt.app_mut().on = false;
    let after = thumb_x(&step(&mut rt, 16.0));
    // The first frame after the retarget continues from the same place: the
    // step is no larger than the steps before it.
    let last_step = (xs[6] - xs[5]).abs();
    assert!((after - mid).abs() <= last_step * 1.5 + 0.5, "{mid} -> {after}, step {last_step}");
    for _ in 0..120 {
        step(&mut rt, 16.0);
    }
    assert!(thumb_x(&step(&mut rt, 16.0)).abs() < 0.01, "settles back at 0");
    assert!(!rt.wants_frames());
}

#[test]
fn the_spring_settles_within_ten_percent_of_astryx_spring() {
    // The runtime steps the spring per frame; Astryx's animator solves it
    // in closed form. Both must agree within 10% of the travel at every
    // frame, and settle at the same time within 10%.
    let t = motion_tokens::DEFAULT;
    let spring = t.spring(springs::STATE_CHANGE).spring();
    let curve = spring.solve(0.0, 20.0, 0.0);
    let mut stepped = Animated::new(0.0f32);
    stepped.animate_to(20.0, Motion::Spring(spring));
    let mut worst: f64 = 0.0;
    let mut settled_at = None;
    for i in 1..=120 {
        stepped.tick(1.0 / 60.0);
        let exact = curve.value(i as f64 / 60.0);
        worst = worst.max((stepped.value() as f64 - exact).abs());
        if settled_at.is_none() && !stepped.is_animating() {
            settled_at = Some(i as f64 / 60.0);
        }
    }
    assert!(worst <= 2.0, "worst deviation {worst} of 20");
    let expected = curve.settle_time(0.01);
    let got = settled_at.expect("settles");
    assert!((got - expected).abs() <= expected * 0.1 + 1.0 / 60.0, "{got} vs {expected}");
}

#[test]
fn a_layer_enters_and_its_exit_plays_once() {
    let mut rt = runtime();
    rt.app_mut().open = true;
    let first = step(&mut rt, 0.0);
    let opacity = first.commands.iter().find_map(|c| match c {
        DrawCmd::PushOpacity(o) => Some(*o),
        _ => None,
    });
    assert_eq!(opacity, Some(0.0), "enters from transparent");
    for _ in 0..30 {
        step(&mut rt, 16.0);
    }
    let open = step(&mut rt, 16.0);
    assert!(!open.commands.iter().any(|c| matches!(c, DrawCmd::PushOpacity(_))), "fully in");
    // Close: one exit copy fades, then it is gone.
    rt.app_mut().open = false;
    // The layer is closed, so every 40-point block drawn is an exit copy.
    let count_copies = |l: &DisplayList| {
        l.commands
            .iter()
            .filter(|c| matches!(c, DrawCmd::Rect { rect, .. } if rect.height == 40.0))
            .count()
    };
    let closing = step(&mut rt, 16.0);
    assert_eq!(count_copies(&closing), 1);
    // Open and close again mid-exit: still one copy.
    rt.app_mut().open = true;
    step(&mut rt, 16.0);
    rt.app_mut().open = false;
    let again = step(&mut rt, 16.0);
    assert_eq!(count_copies(&again), 1, "a second exit replaces the first");
    for _ in 0..40 {
        step(&mut rt, 16.0);
    }
    assert_eq!(count_copies(&step(&mut rt, 16.0)), 0, "removed when it settles");
    assert!(!rt.wants_frames());
}

#[test]
fn collapsible_height_animates_and_content_below_reflows() {
    let mut rt = runtime();
    let closed = last_y(&step(&mut rt, 16.0));
    rt.app_mut().expanded = true;
    let mut ys = Vec::new();
    for _ in 0..40 {
        ys.push(last_y(&step(&mut rt, 16.0)));
    }
    assert!(ys.windows(2).all(|w| w[1] >= w[0] - 0.01), "moves down steadily: {ys:?}");
    assert!(ys[3] > closed + 1.0 && ys[3] < closed + 99.0, "in between: {ys:?}");
    assert!((ys.last().unwrap() - (closed + 100.0)).abs() < 0.5, "lands at the full height");
    // Collapse reverses; interrupting mid-way turns around without a jump.
    rt.app_mut().expanded = false;
    let a = last_y(&step(&mut rt, 16.0));
    let b = last_y(&step(&mut rt, 16.0));
    assert!(b < a && a <= closed + 100.0);
}

#[test]
fn reduce_motion_lands_travel_at_once() {
    let mut rt = runtime();
    rt.set_preferences(&Preferences {
        reduced_motion: true,
        ..Preferences::default()
    });
    step(&mut rt, 0.0);
    rt.app_mut().on = true;
    let list = step(&mut rt, 16.0);
    assert_eq!(thumb_x(&list), 20.0, "no transform animation");
    rt.app_mut().open = true;
    let entered = step(&mut rt, 0.0);
    assert!(
        !entered
            .commands
            .iter()
            .any(|c| matches!(c, DrawCmd::PushOpacity(o) if *o < 1.0)),
        "fine pointer: no fade either"
    );
    rt.app_mut().expanded = true;
    let open = last_y(&step(&mut rt, 16.0));
    let next = last_y(&step(&mut rt, 16.0));
    assert_eq!(open, next, "height lands at once");
    // On a touch screen the entrance cross-fades.
    let mut touch = runtime();
    touch.set_preferences(&Preferences {
        reduced_motion: true,
        coarse_pointer: true,
        ..Preferences::default()
    });
    step(&mut touch, 0.0);
    touch.app_mut().open = true;
    let first = step(&mut touch, 0.0);
    assert!(first
        .commands
        .iter()
        .any(|c| matches!(c, DrawCmd::PushOpacity(o) if *o < 0.5)));
    assert!(!first.commands.iter().any(|c| matches!(c,
        DrawCmd::PushTransform { translate, scale, .. } if translate.y != 0.0 || *scale != 1.0)));
}
