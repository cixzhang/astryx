use super::*;
use astryx_core::testing::{frame, FixedMeasure};
use astryx_core::{A11yNode, App, Command, Point, Runtime, Surface, Text, VStack};

#[test]
fn window_covers_the_viewport_with_overscan() {
    let h = Heights::new(10_000, 40.0);
    let w = Window::compute(&h, 0.0, 400.0, 4);
    assert_eq!((w.start, w.end), (0, 14));
    assert_eq!(w.total, 400_000.0);
    let mid = Window::compute(&h, 200_000.0, 400.0, 4);
    assert_eq!((mid.start, mid.end), (4996, 5014));
    assert_eq!(mid.before, 4996.0 * 40.0);
    assert!(
        (mid.before + (mid.end - mid.start) as f32 * 40.0 + mid.after - mid.total).abs() < 0.01
    );
    let end = Window::compute(&h, 400_000.0 - 400.0, 400.0, 4);
    assert_eq!(end.end, 10_000);
}

#[test]
fn measured_heights_replace_estimates() {
    let mut h = Heights::new(100, 40.0);
    assert!(h.set(0, 80.0));
    assert!(!h.set(0, 80.2), "sub-point changes are noise");
    let w = Window::compute(&h, 0.0, 120.0, 0);
    assert_eq!(
        (w.start, w.end),
        (0, 2),
        "the tall first row leaves room for one more"
    );
    assert_eq!(w.total, 80.0 + 99.0 * 40.0);
}

struct Big {
    show: bool,
    memory: ScrollMemory,
}

impl App for Big {
    type Msg = ();
    fn view(&self) -> Element<()> {
        let mut v = VStack::new();
        if self.show {
            v = v.child(
                VirtualList::new("rows", 10_000, 30.0, |i| {
                    // Every third row wraps to two lines: variable heights.
                    let text = if i % 3 == 0 {
                        format!("Row {i} has a long label that wraps onto a second line here")
                    } else {
                        format!("Row {i}")
                    };
                    Text::new(text).into()
                })
                .kind(Kind::Grid)
                .label("Big table")
                .height(400.0)
                .memory(self.memory.clone()),
            );
        }
        v.into()
    }
    fn update(&mut self, _: ()) -> Command {
        Command::None
    }
}

fn step(rt: &mut Runtime<Big>) {
    let mut f = frame(300, 600);
    f.dt = 0.016;
    rt.frame(&f, &FixedMeasure::default());
}

fn count(n: &A11yNode, role: Role) -> usize {
    usize::from(n.role == role) + n.children.iter().map(|c| count(c, role)).sum::<usize>()
}

fn first(n: &A11yNode, role: Role) -> Option<&A11yNode> {
    if n.role == role {
        return Some(n);
    }
    n.children.iter().find_map(|c| first(c, role))
}

fn rt() -> Runtime<Big> {
    let mut rt = Runtime::new(Big {
        show: true,
        memory: ScrollMemory::new(),
    });
    for _ in 0..3 {
        step(&mut rt);
    }
    rt
}

#[test]
fn ten_thousand_rows_render_under_a_hundred_and_expose_their_count() {
    let rt = rt();
    let tree = rt.accessibility().unwrap();
    let rows = count(&tree, Role::Row);
    assert!(rows > 5 && rows < 100, "{rows} rows rendered");
    let grid = first(&tree, Role::Grid).unwrap();
    assert_eq!(grid.state.row_count, Some(10_000));
    assert_eq!(first(&tree, Role::Row).unwrap().state.row_index, Some(1));
}

#[test]
fn scrolling_renders_the_rows_in_view_with_their_index() {
    let mut rt = rt();
    // Wheel down a long way, then let the smooth scroll land.
    for _ in 0..50 {
        rt.scroll(Point { x: 50.0, y: 100.0 }, Point { x: 0.0, y: 2000.0 });
        step(&mut rt);
    }
    for _ in 0..60 {
        step(&mut rt);
    }
    let tree = rt.accessibility().unwrap();
    let row = first(&tree, Role::Row).unwrap();
    let index = row.state.row_index.unwrap();
    assert!(index > 1000, "row {index} of 10,000 is in view");
    assert!(count(&tree, Role::Row) < 100);
    // The rendered rows are the ones on screen: the first one starts at or
    // above the viewport top, within the overscan.
    let grid = first(&tree, Role::Grid).unwrap();
    assert!(row.bounds.y <= grid.bounds.y + 1.0);
}

#[test]
fn the_offset_comes_back_when_the_list_returns() {
    let mut rt = rt();
    for _ in 0..10 {
        rt.scroll(Point { x: 50.0, y: 100.0 }, Point { x: 0.0, y: 600.0 });
        step(&mut rt);
    }
    for _ in 0..60 {
        step(&mut rt);
    }
    let before = first(&rt.accessibility().unwrap(), Role::Row)
        .unwrap()
        .state
        .row_index;
    let (kept, _) = rt.app().memory.anchor();
    assert!(kept > 100, "{kept}");
    rt.app_mut().show = false;
    step(&mut rt);
    rt.app_mut().show = true;
    for _ in 0..3 {
        step(&mut rt);
    }
    let after = first(&rt.accessibility().unwrap(), Role::Row)
        .unwrap()
        .state
        .row_index;
    assert_eq!(before, after, "back at the same rows");
}
