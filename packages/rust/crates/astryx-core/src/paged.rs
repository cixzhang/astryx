//! Experimental: paged screens and a camera that follows focus, for
//! focus-first devices.
//!
//! On a D-pad, a screen laid out into whole pages reads better than a
//! scrolling one. [`Paged`] cuts its content into pages that fit its
//! viewport exactly: it never splits an item across pages, and long text
//! breaks between lines. The D-pad moves focus inside a page; pressing past
//! the last focus stop flips to the next page, and L2 and R2 flip directly.
//! A small row of dots at the bottom edge shows the page, as Astryx
//! Pagination's `dots` variant draws them.
//!
//! [`CameraFollow`] is the one scrolling form: a viewport over a 2D board or
//! map that follows the focused item, moving only when focus nears an edge.
//!
//! Neither is an Astryx component yet; the names and behavior may change.

use crate::contract::Rect;
use crate::element::{base_props, Axis, Base, Build, BuildCx, Draw, Element, Node, Space};
use crate::Tokens;
use taffy::prelude::*;
use taffy::Overflow;

/// A vertical list of children laid out into whole pages.
pub struct Paged<Msg> {
    base: Base,
    gap: Space,
    padding: Space,
    children: Vec<Element<Msg>>,
}

impl<Msg> Default for Paged<Msg> {
    fn default() -> Self {
        Paged {
            base: Base::default(),
            gap: Space::S0,
            padding: Space::S0,
            children: Vec::new(),
        }
    }
}

impl<Msg> Paged<Msg> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn gap(mut self, gap: Space) -> Self {
        self.gap = gap;
        self
    }
    /// Space around each page's content.
    pub fn padding(mut self, padding: Space) -> Self {
        self.padding = padding;
        self
    }
    pub fn child(mut self, child: impl Into<Element<Msg>>) -> Self {
        self.children.push(child.into());
        self
    }
    pub fn children<E: Into<Element<Msg>>>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }
}

base_props!(Paged<Msg>);

/// The height of the page indicator row under a pager.
pub(crate) fn indicator_height(t: &Tokens) -> f32 {
    t.space.s6
}

impl<Msg> Build<Msg> for Paged<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let pad = length(self.padding.points(t));
        let gap = length(self.gap.points(t));
        // The pager fills the height its parent offers; pages are cut to it.
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: Size {
                width: length(0.0),
                height: length(0.0),
            },
            ..Style::default()
        }));
        let mut view = Node::new(Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: Size {
                width: length(0.0),
                height: length(0.0),
            },
            gap: Size {
                width: gap,
                height: gap,
            },
            padding: taffy::Rect {
                left: pad,
                right: pad,
                top: pad,
                bottom: pad,
            },
            overflow: taffy::Point {
                x: Overflow::Visible,
                y: Overflow::Scroll,
            },
            ..Style::default()
        });
        view.scroll = Some(Axis::Vertical);
        view.pager = true;
        cx.begin(view);
        for child in self.children {
            cx.child(child);
        }
        cx.end();
        // The runtime draws the page dots here once it knows the page count.
        cx.leaf(Node::new(Style {
            size: Size {
                width: Dimension::auto(),
                height: length(indicator_height(t)),
            },
            flex_shrink: 0.0,
            ..Style::default()
        }));
        cx.end();
    }
}

/// A viewport over content larger than itself in both directions, such as
/// a board or a map, that follows the focused item. It moves only when
/// focus comes within a quarter of the viewport of an edge, and shows no
/// scroll indicator.
pub struct CameraFollow<Msg> {
    base: Base,
    child: Element<Msg>,
}

impl<Msg> CameraFollow<Msg> {
    pub fn new(child: impl Into<Element<Msg>>) -> Self {
        CameraFollow {
            base: Base::default(),
            child: child.into(),
        }
    }
}

base_props!(CameraFollow<Msg>);

impl<Msg> Build<Msg> for CameraFollow<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let mut node = Node::new(Style {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FLEX_START,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: Size {
                width: length(0.0),
                height: length(0.0),
            },
            overflow: taffy::Point {
                x: Overflow::Hidden,
                y: Overflow::Hidden,
            },
            ..Style::default()
        });
        node.clip = Some(Default::default());
        node.camera = true;
        cx.key(self.base.id.as_deref());
        cx.begin(node);
        cx.child(self.child);
        cx.end();
    }
}

/// The pages of pager `ix`: each page's scroll offset and the height of
/// content it shows, measured from the pager's top edge.
///
/// A page ends at the last place that cuts no item: between children of
/// plain vertical stacks, or between lines of text. A single item taller
/// than the page is cut at the page's end, since nothing else fits.
pub(crate) fn pages<Msg>(
    nodes: &[Node<Msg>],
    boxes: &[Rect],
    ix: usize,
    padding: (f32, f32),
) -> Vec<(f32, f32)> {
    let view = boxes[ix];
    let (pad_top, pad_bottom) = padding;
    let height = view.height;
    let mut inside = Vec::new();
    let mut stack: Vec<usize> = nodes[ix].children.iter().map(|&c| c as usize).collect();
    while let Some(n) = stack.pop() {
        inside.push(n);
        stack.extend(nodes[n].children.iter().map(|&c| c as usize));
    }
    let span = |n: usize| (boxes[n].y - view.y, boxes[n].y + boxes[n].height - view.y);
    let content_end = inside.iter().map(|&n| span(n).1).fold(0.0f32, f32::max);
    // Text may break at each line boundary.
    let line = |n: usize| match &nodes[n].draw {
        Draw::Text(t) => Some((t.style.size * t.style.leading - 0.01).ceil()),
        _ => None,
    };
    let mut candidates: Vec<f32> = Vec::new();
    for &n in &inside {
        let (top, bottom) = span(n);
        candidates.push(top);
        if let Some(lh) = line(n) {
            let mut y = top + lh;
            while y < bottom - 0.5 {
                candidates.push(y);
                y += lh;
            }
        }
    }
    candidates.sort_by(|a, b| b.total_cmp(a));
    candidates.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    let cuts = |y: f32, n: usize| {
        let (top, bottom) = span(n);
        if !(top < y - 0.5 && bottom > y + 0.5) {
            return false;
        }
        if let Some(lh) = line(n) {
            let k = (y - top) / lh;
            return (k - k.round()).abs() > 0.01;
        }
        !is_plain_column(&nodes[n])
    };

    let mut out = Vec::new();
    let mut offset = 0.0f32;
    loop {
        let limit = offset + height - pad_bottom;
        if content_end <= limit + 0.5 || out.len() > 1000 {
            out.push((offset, height));
            return out;
        }
        let floor = offset + pad_top + 0.5;
        let cut = candidates
            .iter()
            .copied()
            .filter(|&y| y <= limit + 0.01 && y > floor)
            .find(|&y| !inside.iter().any(|&n| cuts(y, n)))
            .unwrap_or(limit);
        out.push((offset, cut - offset));
        offset = cut - pad_top;
    }
}

/// A stack that only arranges its children vertically: cutting between
/// its children cuts no item.
fn is_plain_column<Msg>(node: &Node<Msg>) -> bool {
    matches!(node.draw, Draw::None)
        && node.interact.is_none()
        && node.indicator.is_none()
        && node.clip.is_none()
        && node.scroll.is_none()
        && node.style.flex_direction == FlexDirection::Column
}

/// The page of `pages` that shows content at `y` (from the pager's top).
pub(crate) fn page_at(pages: &[(f32, f32)], y: f32) -> usize {
    pages
        .iter()
        .rposition(|&(offset, height)| y >= offset - 0.5 && y < offset + height + 0.5)
        .unwrap_or(0)
}

/// Whether content from `top` to `bottom` lies wholly on `page`.
pub(crate) fn on_page(page: (f32, f32), top: f32, bottom: f32) -> bool {
    top >= page.0 - 0.5 && bottom <= page.0 + page.1 + 0.5
}
