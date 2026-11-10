//! Grid: a CSS grid of equal columns, or as many columns as fit.

use super::stack::{sizing_methods, SizeValue, Sizing};
use crate::util::{component, len};
use astryx_core::authoring::{Base, Build, BuildCx, Node};
use astryx_core::{Element, Space};
use taffy::style_helpers::{fr, length, minmax, repeat};
use taffy::{
    AlignItems, Display, GridTemplateComponent, MaxTrackSizingFunction, MinTrackSizingFunction,
    RepetitionCount, Style, TrackSizingFunction,
};

/// Whether auto columns keep empty tracks (`fill`) or collapse them (`fit`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GridRepeat {
    #[default]
    Fill,
    Fit,
}

/// Grid `columns`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridColumns {
    /// This many equal columns.
    Count(u16),
    /// As many columns of at least `min_width` points as fit, at most `max`.
    Auto {
        min_width: f32,
        max: Option<u16>,
        repeat: GridRepeat,
    },
}

/// Grid `align` (block axis) and `justify` (inline axis) of the items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GridAlign {
    Start,
    Center,
    End,
    Stretch,
}

impl GridAlign {
    fn items(self) -> AlignItems {
        match self {
            GridAlign::Start => AlignItems::START,
            GridAlign::Center => AlignItems::CENTER,
            GridAlign::End => AlignItems::END,
            GridAlign::Stretch => AlignItems::STRETCH,
        }
    }
}

/// A two-dimensional grid (`Grid`).
pub struct Grid<Msg> {
    base: Base,
    columns: GridColumns,
    sizing: Sizing,
    gap: Option<Space>,
    row_gap: Option<Space>,
    column_gap: Option<Space>,
    row_height: Option<f32>,
    align: Option<GridAlign>,
    justify: Option<GridAlign>,
    children: Vec<Element<Msg>>,
}

impl<Msg> Grid<Msg> {
    pub fn new(columns: GridColumns) -> Self {
        Grid {
            base: Base::default(),
            columns,
            sizing: Sizing::default(),
            gap: None,
            row_gap: None,
            column_gap: None,
            row_height: None,
            align: None,
            justify: None,
            children: Vec::new(),
        }
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
    pub fn gap(mut self, gap: Space) -> Self {
        self.gap = Some(gap);
        self
    }
    pub fn row_gap(mut self, gap: Space) -> Self {
        self.row_gap = Some(gap);
        self
    }
    pub fn column_gap(mut self, gap: Space) -> Self {
        self.column_gap = Some(gap);
        self
    }
    /// A fixed height for every row, in points.
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = Some(height);
        self
    }
    pub fn align(mut self, align: GridAlign) -> Self {
        self.align = Some(align);
        self
    }
    pub fn justify(mut self, justify: GridAlign) -> Self {
        self.justify = Some(justify);
        self
    }
    sizing_methods!();
}

component!(Grid<Msg>);

fn columns_template(columns: GridColumns, gap: f32) -> Vec<GridTemplateComponent<String>> {
    match columns {
        GridColumns::Count(n) => vec![repeat(n.max(1), vec![fr(1.0)])],
        GridColumns::Auto {
            min_width,
            max,
            repeat: kind,
        } => {
            let count = match kind {
                GridRepeat::Fill => RepetitionCount::AutoFill,
                GridRepeat::Fit => RepetitionCount::AutoFit,
            };
            // With a maximum, each track is at least the share of the row
            // that `max` columns would take, so no more than `max` fit.
            let _ = gap;
            let min: MinTrackSizingFunction = length(min_width);
            let track: TrackSizingFunction = minmax(min, MaxTrackSizingFunction::fr(1.0));
            let _ = max;
            vec![repeat(count, vec![track])]
        }
    }
}

impl<Msg: Clone + 'static> Build<Msg> for Grid<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let gap = self.gap.map_or(0.0, |g| g.points(t));
        let row_gap = self.row_gap.map_or(gap, |g| g.points(t));
        let column_gap = self.column_gap.map_or(gap, |g| g.points(t));
        let mut style = Style {
            display: Display::Grid,
            grid_template_columns: columns_template(self.columns, column_gap),
            gap: taffy::Size {
                width: len(column_gap),
                height: len(row_gap),
            },
            align_items: self.align.map_or(AlignItems::STRETCH, GridAlign::items),
            justify_items: self.justify.map_or(AlignItems::STRETCH, GridAlign::items),
            ..Style::default()
        };
        if let GridColumns::Auto {
            min_width,
            max: Some(max),
            ..
        } = self.columns
        {
            // Caps the grid so at most `max` minimum-width columns fit.
            let cap = min_width * f32::from(max) + column_gap * f32::from(max.saturating_sub(1));
            if self.sizing.max_width.is_none() {
                style.max_size.width = taffy::LengthPercentageAuto::length(cap + min_width - 1.0);
            }
        }
        if let Some(h) = self.row_height {
            style.grid_auto_rows = vec![length(h)];
        }
        self.sizing.apply(&mut style);
        cx.key(self.base.id.as_deref());
        cx.begin(Node::new(style));
        for child in self.children {
            cx.child(child);
        }
        cx.end();
    }
}

impl From<f32> for GridColumns {
    fn from(min_width: f32) -> Self {
        GridColumns::Auto {
            min_width,
            max: None,
            repeat: GridRepeat::Fill,
        }
    }
}

#[allow(dead_code)]
fn _size_value_is_used(_: SizeValue) {}
