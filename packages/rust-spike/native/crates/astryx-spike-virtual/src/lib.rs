//! List windowing: a long list renders only the rows that cover its
//! viewport, plus a few past each edge, and sizes its scroll range for the
//! whole list, as Astryx's list-windowing module specifies (one shared
//! module for Table, Selector and Chat).
//!
//! - [`Window`]: which rows render and where, from row heights (measured
//!   where known, estimated elsewhere) and the scroll offset.
//! - [`VirtualList`]: the element. Rows carry their position in the full
//!   list for assistive technology (`aria-rowindex` / `aria-posinset`), and
//!   the list its full count (`aria-rowcount` / `aria-setsize`).
//! - [`ScrollMemory`]: keeps the offset when the list leaves and returns.

use astryx_core::authoring::{Build, BuildCx, LocalHandle, Node, Semantics};
use astryx_core::{Element, Role};
use std::cell::Cell;
use std::rc::Rc;
use taffy::Style;

/// Row heights: measured where a row has been drawn, else the estimate.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heights {
    pub estimate: f32,
    measured: Vec<Option<f32>>,
}

impl Heights {
    pub fn new(count: usize, estimate: f32) -> Self {
        Heights {
            estimate,
            measured: vec![None; count],
        }
    }

    pub fn len(&self) -> usize {
        self.measured.len()
    }

    pub fn is_empty(&self) -> bool {
        self.measured.is_empty()
    }

    /// Resizes to `count` rows, keeping known heights.
    pub fn resize(&mut self, count: usize) {
        self.measured.resize(count, None);
    }

    pub fn get(&self, i: usize) -> f32 {
        self.measured
            .get(i)
            .copied()
            .flatten()
            .unwrap_or(self.estimate)
    }

    /// Records a measured height; true when it changed.
    pub fn set(&mut self, i: usize, h: f32) -> bool {
        match self.measured.get_mut(i) {
            Some(slot) if slot.is_none_or(|old| (old - h).abs() > 0.5) => {
                *slot = Some(h);
                true
            }
            _ => false,
        }
    }

    pub fn measured_count(&self) -> usize {
        self.measured.iter().filter(|m| m.is_some()).count()
    }
}

/// The rows to render for one scroll offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    /// First and one-past-last rendered row.
    pub start: usize,
    pub end: usize,
    /// Space above the first rendered row and below the last.
    pub before: f32,
    pub after: f32,
    /// The whole list's height.
    pub total: f32,
}

impl Window {
    /// The rows covering `[offset, offset + viewport]`, with `overscan`
    /// rows past each edge.
    pub fn compute(heights: &Heights, offset: f32, viewport: f32, overscan: usize) -> Window {
        let n = heights.len();
        let mut y = 0.0f32;
        let mut first = n;
        let mut last = n;
        for i in 0..n {
            let h = heights.get(i);
            if first == n && y + h > offset {
                first = i;
            }
            if first != n && y >= offset + viewport {
                last = i;
                break;
            }
            y += h;
        }
        if first == n {
            first = n.saturating_sub(1);
        }
        let start = first.saturating_sub(overscan);
        let end = (last + overscan).min(n);
        let before: f32 = (0..start).map(|i| heights.get(i)).sum();
        let shown: f32 = (start..end).map(|i| heights.get(i)).sum();
        let total: f32 = before + shown + (end..n).map(|i| heights.get(i)).sum::<f32>();
        Window {
            start,
            end,
            before,
            after: total - before - shown,
            total,
        }
    }
}

/// Keeps a list's place while the list is away (a tab switch, a route
/// change), so it comes back at the same rows. It keeps the row at the top
/// of the viewport and how far into it the viewport starts, not a pixel
/// offset: row heights measured before may not be known when the list
/// returns. Clone it into app state.
#[derive(Clone, Debug, Default)]
pub struct ScrollMemory(Rc<Cell<(usize, f32)>>);

impl ScrollMemory {
    pub fn new() -> Self {
        Self::default()
    }
    /// The row at the top of the viewport, and the offset into it.
    pub fn anchor(&self) -> (usize, f32) {
        self.0.get()
    }
}

impl Heights {
    /// The offset of row `i`'s top.
    pub fn offset_of(&self, i: usize) -> f32 {
        (0..i.min(self.len())).map(|r| self.get(r)).sum()
    }

    /// The row at `offset`, and how far into it `offset` is.
    pub fn row_at(&self, offset: f32) -> (usize, f32) {
        let mut y = 0.0;
        for i in 0..self.len() {
            let h = self.get(i);
            if y + h > offset {
                return (i, offset - y);
            }
            y += h;
        }
        (self.len().saturating_sub(1), 0.0)
    }
}

/// How the list is exposed to assistive technology.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A grid (Table): rows with `aria-rowindex`, the grid with
    /// `aria-rowcount`.
    Grid,
    /// A list: items with `aria-posinset`, the list with `aria-setsize`.
    List,
}

type RowFn<Msg> = Rc<dyn Fn(usize) -> Element<Msg>>;

/// A windowed list of `count` rows.
pub struct VirtualList<Msg> {
    id: String,
    count: usize,
    row: RowFn<Msg>,
    estimate: f32,
    height: f32,
    overscan: usize,
    kind: Kind,
    label: Option<String>,
    /// Rows above the data rows (a header row), for row indexes.
    header_rows: u32,
    memory: Option<ScrollMemory>,
}

/// Rows past each edge rendered by default.
pub const OVERSCAN: usize = 4;

impl<Msg: 'static> VirtualList<Msg> {
    /// `id` names the list (and its rows, `"{id}-row-{i}"`); `row(i)` builds
    /// row `i`; `estimate` is a row's height before it is measured.
    pub fn new(
        id: impl Into<String>,
        count: usize,
        estimate: f32,
        row: impl Fn(usize) -> Element<Msg> + 'static,
    ) -> Self {
        VirtualList {
            id: id.into(),
            count,
            row: Rc::new(row),
            estimate,
            height: 400.0,
            overscan: OVERSCAN,
            kind: Kind::List,
            label: None,
            header_rows: 0,
            memory: None,
        }
    }

    /// The viewport's height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }

    pub fn overscan(mut self, rows: usize) -> Self {
        self.overscan = rows;
        self
    }

    pub fn kind(mut self, kind: Kind) -> Self {
        self.kind = kind;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn header_rows(mut self, rows: u32) -> Self {
        self.header_rows = rows;
        self
    }

    /// Restores the offset from `memory` when the list appears, and keeps
    /// it there as it scrolls.
    pub fn memory(mut self, memory: ScrollMemory) -> Self {
        self.memory = Some(memory);
        self
    }
}

fn spacer<Msg>(h: f32) -> Node<Msg> {
    Node::new(Style {
        size: taffy::Size {
            width: taffy::Dimension::auto(),
            height: taffy::Dimension::length(h),
        },
        flex_shrink: 0.0,
        ..Style::default()
    })
}

impl<Msg: 'static> Build<Msg> for VirtualList<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let this = *self;
        cx.key(Some(&this.id));
        let id = cx.next_id();
        let heights: LocalHandle<Heights> = cx.local_handle();
        let mut h = heights.get();
        if h.len() != this.count || h.estimate != this.estimate {
            h.estimate = this.estimate;
            h.resize(this.count);
        }
        // Heights of the rows drawn last frame. Rows above the one at the
        // top of the viewport that change height move the offset by the
        // same amount, so what is in view stays put (scroll anchoring).
        let offset = cx.scroll_offset(id);
        let (top_row, _) = h.row_at(offset);
        let mut changed = false;
        let mut adjust = 0.0;
        for i in 0..this.count.min(h.len()) {
            if let Some(r) = cx.last_rect(&format!("{}-row-{i}", this.id)) {
                let old = h.get(i);
                if h.set(i, r.height) {
                    changed = true;
                    if i < top_row {
                        adjust += r.height - old;
                    }
                }
            }
        }
        if changed {
            heights.set(h.clone());
        }
        let offset = offset + adjust;
        let w = Window::compute(&h, offset, this.height, this.overscan);
        let (role, count_state): (Role, fn(&mut astryx_core::A11yState, u32)) = match this.kind {
            Kind::Grid => (Role::Grid, |s, n| s.row_count = Some(n)),
            Kind::List => (Role::List, |s, n| s.set_size = Some(n)),
        };
        let total_rows = this.count as u32 + this.header_rows;
        let sem = Semantics {
            label: this.label.clone(),
            ..Semantics::role(role)
        }
        .with_state(|s| count_state(s, total_rows));
        let mut list = Node::new(Style {
            flex_direction: taffy::FlexDirection::Column,
            size: taffy::Size {
                width: taffy::Dimension::auto(),
                height: taffy::Dimension::length(this.height),
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Visible,
                y: taffy::Overflow::Scroll,
            },
            ..Style::default()
        })
        .sem(sem);
        list.scroll = Some(astryx_core::authoring::Axis::Vertical);
        list.rebuild_on_scroll = true;
        list.scroll_adjust = adjust;
        if let Some(memory) = &this.memory {
            let (row, into) = memory.anchor();
            list.initial_scroll = Some(h.offset_of(row) + into);
            let memory = memory.clone();
            let heights = Rc::new(h.clone());
            list.scroll_report = Some(Rc::new(move |y| memory.0.set(heights.row_at(y))));
        }
        cx.key(Some(&this.id));
        cx.begin(list);
        cx.leaf(spacer(w.before));
        for i in w.start..w.end {
            let index = i as u32 + this.header_rows + 1;
            let row_sem = match this.kind {
                Kind::Grid => Semantics::role(Role::Row).with_state(|s| s.row_index = Some(index)),
                Kind::List => Semantics::role(Role::ListItem).with_state(|s| {
                    s.pos_in_set = Some(index);
                    s.set_size = Some(total_rows);
                }),
            };
            let name = format!("{}-row-{i}", this.id);
            cx.key(Some(&name));
            cx.begin(
                Node::new(Style {
                    flex_direction: taffy::FlexDirection::Column,
                    flex_shrink: 0.0,
                    ..Style::default()
                })
                .sem(row_sem),
            );
            cx.child((this.row)(i));
            cx.end();
        }
        cx.leaf(spacer(w.after));
        cx.end();
    }
}

impl<Msg: 'static> From<VirtualList<Msg>> for Element<Msg> {
    fn from(v: VirtualList<Msg>) -> Self {
        astryx_core::authoring::element(v)
    }
}

#[cfg(test)]
mod tests;
