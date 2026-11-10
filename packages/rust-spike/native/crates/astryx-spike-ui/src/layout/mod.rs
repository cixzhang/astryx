//! Wave 2: layout primitives and containers.

mod aspect_ratio;
mod center;
mod empty_state;
mod frame;
mod grid;
mod section;
mod stack;

pub use aspect_ratio::{AspectRatio, AspectRatioFit, AspectRatioShape};
pub use center::{Center, CenterAxis};
pub use empty_state::EmptyState;
pub use frame::{Layout, LayoutContent, LayoutFooter, LayoutHeader, LayoutHeight, LayoutPanel};
pub use grid::{Grid, GridAlign, GridColumns, GridRepeat};
pub(crate) use section::{card_ground, elevation_layers};
pub use section::{Card, CardVariant, Section, SectionDivider, SectionVariant};
pub(crate) use stack::{size_auto, size_dim};
pub use stack::{
    HStack, SizeValue, Sizing, Stack, StackAlign, StackAxisAlign, StackDirection, StackItem,
    StackItemSize, StackJustify, StackWrap, VStack,
};

use crate::util::len;
use astryx_core::authoring::BuildCx;
use astryx_core::Space;
use taffy::LengthPercentage;

/// Logical padding props, most specific first: `inline_start` beats
/// `inline`, which beats `all`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Padding {
    pub all: Option<Space>,
    pub inline: Option<Space>,
    pub inline_start: Option<Space>,
    pub inline_end: Option<Space>,
    pub block: Option<Space>,
    pub block_start: Option<Space>,
    pub block_end: Option<Space>,
}

impl Padding {
    pub fn all(step: Space) -> Self {
        Padding {
            all: Some(step),
            ..Self::default()
        }
    }
    /// (inline start, inline end, block start, block end) in points.
    pub fn resolve(&self, tokens: &astryx_core::Tokens) -> (f32, f32, f32, f32) {
        let pick = |a: Option<Space>, b: Option<Space>| {
            a.or(b).or(self.all).map_or(0.0, |s| s.points(tokens))
        };
        (
            pick(self.inline_start, self.inline),
            pick(self.inline_end, self.inline),
            pick(self.block_start, self.block),
            pick(self.block_end, self.block),
        )
    }
}

/// The physical padding rect for logical padding in the region's direction.
pub(crate) fn pad_rect<Msg>(p: &Padding, cx: &BuildCx<Msg>) -> taffy::Rect<LengthPercentage> {
    let (is, ie, bs, be) = p.resolve(cx.tokens);
    let (left, right) = cx.inline(is, ie);
    taffy::Rect {
        left: len(left),
        right: len(right),
        top: len(bs),
        bottom: len(be),
    }
}
