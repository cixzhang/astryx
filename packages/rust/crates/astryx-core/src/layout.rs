//! Layout on taffy. Text leaves are measured with the shell's
//! [`TextMeasure`], so layout and pixels agree.

use crate::contract::{Rect, TextMeasure};
use crate::element::{Axis, Draw, Node, TextDraw};
use std::collections::HashMap;
use taffy::{AvailableSpace, LayoutInput, LayoutOutput, NodeId, TaffyTree};

const ELLIPSIS: char = '\u{2026}';

/// Laid-out geometry for the current node list.
pub(crate) struct Layout {
    tree: TaffyTree<u32>,
    ids: Vec<NodeId>,
    /// Absolute boxes before scrolling.
    pub boxes: Vec<Rect>,
    /// Absolute boxes as painted, after scroll offsets.
    pub rects: Vec<Rect>,
    /// How far each scroll container's content reaches past its own box.
    pub overflow: Vec<f32>,
    /// Text as painted, when it differs from the node's own (truncated).
    pub texts: Vec<Option<String>>,
}

impl Layout {
    pub fn new<Msg>(nodes: &[Node<Msg>]) -> Self {
        let mut tree = TaffyTree::with_capacity(nodes.len());
        let mut ids = vec![NodeId::new(0); nodes.len()];
        // Children follow their parents, so building back to front sees
        // every child before its parent.
        for (ix, node) in nodes.iter().enumerate().rev() {
            let mut style = node.style.clone();
            // Every node is a containing block for its absolute children, so
            // an overlay sits on the component that owns it.
            if style.position == taffy::Position::Static {
                style.position = taffy::Position::Relative;
            }
            ids[ix] = if node.children.is_empty() {
                match node.draw {
                    Draw::Text(_) => tree.new_leaf_with_context(style, ix as u32),
                    _ => tree.new_leaf(style),
                }
            } else {
                let children: Vec<NodeId> =
                    node.children.iter().map(|&c| ids[c as usize]).collect();
                tree.new_with_children(style, &children)
            }
            .expect("taffy node");
        }
        Layout {
            tree,
            ids,
            boxes: Vec::new(),
            rects: Vec::new(),
            overflow: Vec::new(),
            texts: Vec::new(),
        }
    }

    /// Lays out the tree inside `viewport` (logical points).
    pub fn compute<Msg>(
        &mut self,
        nodes: &[Node<Msg>],
        viewport: (f32, f32),
        text: &dyn TextMeasure,
    ) {
        if nodes.is_empty() {
            return;
        }
        let root = self.ids[0];
        let mut style = nodes[0].style.clone();
        style.position = taffy::Position::Relative;
        style.size = taffy::Size {
            width: taffy::Dimension::length(viewport.0),
            height: taffy::Dimension::length(viewport.1),
        };
        self.tree.set_style(root, style).expect("root style");
        let available = taffy::Size {
            width: AvailableSpace::Definite(viewport.0),
            height: AvailableSpace::Definite(viewport.1),
        };
        self.tree
            .compute_layout_with_measure(root, available, |input, _, ix, style| match ix {
                Some(ix) => match &nodes[*ix as usize].draw {
                    Draw::Text(t) => measure_leaf(input, style, t, text),
                    _ => empty_leaf(input, style),
                },
                None => empty_leaf(input, style),
            })
            .expect("taffy layout");

        let n = nodes.len();
        self.boxes = vec![Rect::default(); n];
        self.overflow = vec![0.0; n];
        self.texts = vec![None; n];
        for (ix, node) in nodes.iter().enumerate() {
            let l = self.tree.layout(self.ids[ix]).expect("layout");
            let origin = node.parent.map_or((0.0, 0.0), |p| {
                (self.boxes[p as usize].x, self.boxes[p as usize].y)
            });
            self.boxes[ix] = Rect::new(
                origin.0 + l.location.x,
                origin.1 + l.location.y,
                l.size.width,
                l.size.height,
            );
        }
        for (ix, node) in nodes.iter().enumerate() {
            if let Some(axis) = node.scroll {
                let b = self.boxes[ix];
                let pad = self.tree.layout(self.ids[ix]).expect("layout").padding;
                // A right-to-left row overflows past its left edge instead.
                let rtl_row = axis == Axis::Horizontal && node.rtl;
                let end = node
                    .children
                    .iter()
                    .map(|&c| {
                        let r = self.boxes[c as usize];
                        match axis {
                            Axis::Vertical => r.y + r.height - b.y + pad.bottom,
                            Axis::Horizontal if rtl_row => b.x + b.width - r.x + pad.left,
                            Axis::Horizontal => r.x + r.width - b.x + pad.right,
                        }
                    })
                    .fold(0.0, f32::max);
                let size = if axis == Axis::Vertical {
                    b.height
                } else {
                    b.width
                };
                self.overflow[ix] = (end - size).max(0.0);
            }
            if let Draw::Text(t) = &node.draw {
                self.texts[ix] = truncate(t, self.boxes[ix].width, text);
            }
        }
    }

    /// Applies scroll offsets (keyed by node id) to get painted rects.
    pub fn place<Msg>(&mut self, nodes: &[Node<Msg>], scroll: &HashMap<u64, f32>) {
        self.rects = self.placed(nodes, scroll);
    }

    /// The painted rects under the given scroll offsets.
    pub fn placed<Msg>(&self, nodes: &[Node<Msg>], scroll: &HashMap<u64, f32>) -> Vec<Rect> {
        let mut shift = vec![(0.0f32, 0.0f32); nodes.len()];
        let mut rects = self.boxes.clone();
        for (ix, node) in nodes.iter().enumerate() {
            let (mut dx, mut dy) = node.parent.map_or((0.0, 0.0), |p| shift[p as usize]);
            rects[ix].x -= dx;
            rects[ix].y -= dy;
            if let Some(axis) = node.scroll {
                let offset = scroll
                    .get(&node.id)
                    .copied()
                    .unwrap_or(0.0)
                    .clamp(0.0, self.overflow[ix]);
                match axis {
                    Axis::Vertical => dy += offset,
                    // Scrolling toward inline end moves the content right.
                    Axis::Horizontal if node.rtl => dx -= offset,
                    Axis::Horizontal => dx += offset,
                }
            }
            shift[ix] = (dx, dy);
        }
        rects
    }
}

fn empty_leaf(input: LayoutInput, style: &taffy::Style) -> LayoutOutput {
    taffy::compute_leaf_layout(input, style, |_, _| 0.0, |_, _| taffy::Size::ZERO)
}

fn measure_leaf(
    input: LayoutInput,
    style: &taffy::Style,
    t: &TextDraw,
    text: &dyn TextMeasure,
) -> LayoutOutput {
    taffy::compute_leaf_layout(
        input,
        style,
        |_, _| 0.0,
        |known, available| {
            let size = measure_text(t, known.width, available.width, text);
            taffy::Size {
                width: known.width.unwrap_or(size.0),
                height: known.height.unwrap_or(size.1),
            }
        },
    )
}

pub(crate) fn line_height(t: &TextDraw) -> f32 {
    t.style.size * t.style.leading
}

/// The size of a text leaf: wrapped at the width it is offered, with height
/// in whole lines of the style's line height.
fn measure_text(
    t: &TextDraw,
    known: Option<f32>,
    available: AvailableSpace,
    text: &dyn TextMeasure,
) -> (f32, f32) {
    let lh = line_height(t);
    let limit = match (known, available) {
        (Some(w), _) => Some(w),
        (None, AvailableSpace::Definite(w)) => Some(w),
        (None, AvailableSpace::MaxContent) => None,
        (None, AvailableSpace::MinContent) => {
            // Text that truncates can shrink to nothing; other text keeps
            // its widest word, as CSS min-content does.
            if t.max_lines.is_some() {
                return (0.0, ceil(lh));
            }
            let widest = t
                .text
                .split_whitespace()
                .map(|w| text.measure(w, &t.style, None).width)
                .fold(0.0, f32::max);
            let widest = ceil(widest);
            Some(widest)
        }
    };
    let measured = text.measure(&t.text, &t.style, limit);
    let lines = lines(measured.height, lh);
    let shown = t.max_lines.map_or(lines, |max| lines.min(max.max(1)));
    let width = ceil(measured.width);
    let width = limit.map_or(width, |l| width.min(l.max(0.0)));
    (width, ceil(shown as f32 * lh))
}

/// Rounds up to whole points, ignoring float noise from token products such
/// as 14 × 1.4286.
fn ceil(v: f32) -> f32 {
    (v - 0.01).ceil().max(0.0)
}

fn lines(height: f32, line_height: f32) -> u32 {
    if line_height <= 0.0 {
        return 1;
    }
    ((height / line_height).round() as u32).max(1)
}

/// The text with an ellipsis when it needs more than `max_lines` lines at
/// `width`, or `None` when it fits.
fn truncate(t: &TextDraw, width: f32, text: &dyn TextMeasure) -> Option<String> {
    let max = t.max_lines?.max(1);
    let lh = line_height(t);
    let fits = |s: &str| lines(text.measure(s, &t.style, Some(width)).height, lh) <= max;
    // One line never wraps, so it is measured unwrapped against the width.
    let fits_one = |s: &str| text.measure(s, &t.style, None).width <= width + 0.5;
    let check = |s: &str| if max == 1 { fits_one(s) } else { fits(s) };
    if check(&t.text) {
        return None;
    }
    let bounds: Vec<usize> = t.text.char_indices().map(|(i, _)| i).collect();
    let candidate = |k: usize| {
        format!(
            "{}{ELLIPSIS}",
            t.text[..bounds.get(k).copied().unwrap_or(t.text.len())].trim_end()
        )
    };
    // The longest prefix that fits with an ellipsis; the full text does not.
    let (mut lo, mut hi) = (0usize, bounds.len().saturating_sub(1));
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if check(&candidate(mid)) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    Some(candidate(lo))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{FontFamily, TextAlign, TextStyle};
    use crate::testing::FixedMeasure;

    fn body(text: &str, max_lines: Option<u32>) -> TextDraw {
        TextDraw {
            text: text.into(),
            style: TextStyle {
                direction: None,
                family: FontFamily::Body,
                size: 14.0,
                weight: 400,
                leading: 1.4286,
            },
            color: crate::contract::Color::TRANSPARENT,
            align: TextAlign::Start,
            max_lines,
            strikethrough: false,
        }
    }

    const M: FixedMeasure = FixedMeasure { advance: 0.5 };

    #[test]
    fn max_content_is_one_line() {
        // 11 characters at 7 points each; one 20 point line.
        assert_eq!(
            measure_text(
                &body("hello world", None),
                None,
                AvailableSpace::MaxContent,
                &M
            ),
            (77.0, 20.0)
        );
    }

    #[test]
    fn definite_width_wraps_to_whole_lines() {
        assert_eq!(
            measure_text(
                &body("hello world", None),
                None,
                AvailableSpace::Definite(50.0),
                &M
            ),
            (35.0, 40.0)
        );
        assert_eq!(
            measure_text(
                &body("hello world", None),
                Some(60.0),
                AvailableSpace::MaxContent,
                &M
            ),
            (35.0, 40.0)
        );
    }

    #[test]
    fn min_content_is_the_widest_word_unless_it_truncates() {
        assert_eq!(
            measure_text(
                &body("a longest b", None),
                None,
                AvailableSpace::MinContent,
                &M
            )
            .0,
            49.0
        );
        assert_eq!(
            measure_text(
                &body("a longest b", Some(1)),
                None,
                AvailableSpace::MinContent,
                &M
            ),
            (0.0, 20.0)
        );
    }

    #[test]
    fn max_lines_caps_the_height() {
        let t = body("one two three four five six", Some(2));
        assert_eq!(
            measure_text(&t, Some(40.0), AvailableSpace::MaxContent, &M).1,
            40.0
        );
    }

    #[test]
    fn truncation_adds_an_ellipsis_only_when_needed() {
        assert_eq!(truncate(&body("short", Some(1)), 100.0, &M), None);
        let cut = truncate(&body("a sentence that is too long", Some(1)), 70.0, &M).unwrap();
        assert!(cut.ends_with(ELLIPSIS));
        assert!(M.measure(&cut, &body("", None).style, None).width <= 70.0);
        assert_eq!(
            truncate(&body("wraps fine here", None), 10.0, &M),
            None,
            "no max lines, no ellipsis"
        );
        let two = truncate(
            &body("one two three four five six seven", Some(2)),
            50.0,
            &M,
        )
        .unwrap();
        assert!(two.ends_with(ELLIPSIS) && two.len() < 33);
    }

    #[test]
    fn whole_points_ignore_token_float_noise() {
        assert_eq!(ceil(14.0 * 1.4286), 20.0);
        assert_eq!(ceil(20.2), 21.0);
        assert_eq!(lines(40.0004, 20.0004), 2);
    }
}
