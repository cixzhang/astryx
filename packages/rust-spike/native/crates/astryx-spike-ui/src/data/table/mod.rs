//! Table: columns over typed rows, with plugins that add selection,
//! sorting, filtering, pagination, row index, status, grouping, tree data,
//! row expansion, column settings, column resize, sticky columns and
//! virtualization.
//!
//! Astryx's plugins transform DOM render props. Here a plugin is a trait
//! object with typed hooks: it can change the columns, the row model (which
//! rows show, in what order, at what depth) and the parts of header and
//! body rows and cells.

mod plugins;

pub use plugins::*;

use crate::controls::ListDensity;
use crate::util::{color, len, len_auto, radii};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Interact, Node,
    Semantics,
};
use astryx_core::contract::{Color, FontFamily, Role};
use astryx_core::Element;
use std::rc::Rc;
use taffy::{
    AlignItems, Dimension, FlexDirection, GridTemplateComponent, LengthPercentageAuto, MinMax,
    Style,
};

/// TableColumn `width`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnWidth {
    /// A share of the free width (`{type: 'proportional'}`).
    Proportional { value: f32, min_width: Option<f32> },
    /// A fixed width in points (`{type: 'pixel'}`).
    Pixel(f32),
}

impl Default for ColumnWidth {
    fn default() -> Self {
        ColumnWidth::Proportional {
            value: 1.0,
            min_width: None,
        }
    }
}

/// TableColumn `align`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColumnAlign {
    #[default]
    Start,
    Center,
    End,
}

/// Table `dividers`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableDividers {
    #[default]
    Rows,
    Columns,
    Grid,
    None,
}

/// Table `verticalAlign`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableVerticalAlign {
    #[default]
    Middle,
    Top,
    Bottom,
}

/// Table `textOverflow`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextOverflow {
    #[default]
    Wrap,
    Truncate,
}

/// The value a column shows for a row, which also sorts and filters it.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum CellValue {
    Text(String),
    Number(f64),
    Empty,
}

impl CellValue {
    pub fn text(&self) -> String {
        match self {
            CellValue::Text(s) => s.clone(),
            CellValue::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            CellValue::Empty => String::new(),
        }
    }
    fn order(&self, other: &CellValue) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (self, other) {
            (CellValue::Number(a), CellValue::Number(b)) => {
                a.partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (CellValue::Empty, CellValue::Empty) => Ordering::Equal,
            (CellValue::Empty, _) => Ordering::Greater,
            (_, CellValue::Empty) => Ordering::Less,
            (a, b) => a.text().to_lowercase().cmp(&b.text().to_lowercase()),
        }
    }
}

impl From<&str> for CellValue {
    fn from(s: &str) -> Self {
        CellValue::Text(s.into())
    }
}
impl From<String> for CellValue {
    fn from(s: String) -> Self {
        CellValue::Text(s)
    }
}
impl From<f64> for CellValue {
    fn from(n: f64) -> Self {
        CellValue::Number(n)
    }
}
impl From<i64> for CellValue {
    fn from(n: i64) -> Self {
        CellValue::Number(n as f64)
    }
}
impl From<u32> for CellValue {
    fn from(n: u32) -> Self {
        CellValue::Number(f64::from(n))
    }
}

type ValueFn<T> = Rc<dyn Fn(&T) -> CellValue>;
/// Draws a cell for a row.
pub type CellFn<T, Msg> = Rc<dyn Fn(&T) -> Element<Msg>>;

/// One column (`TableColumn`): a key, a header, how wide, and what each
/// row shows. `value` gives the cell's data (shown as text, and used to
/// sort and filter); `render_cell` replaces the text with any element.
pub struct TableColumn<T, Msg> {
    pub key: String,
    pub header: Option<String>,
    pub width: ColumnWidth,
    pub align: ColumnAlign,
    pub resizable: bool,
    /// The sort key, when the column sorts (`sortable`).
    pub sort_key: Option<String>,
    /// The filter field, when the column filters (`filter`).
    pub filter: Option<String>,
    pub is_row_header: bool,
    pub value: Option<ValueFn<T>>,
    pub render_cell: Option<CellFn<T, Msg>>,
    /// Header content other than text (a select-all checkbox).
    pub(crate) header_element: Option<Rc<dyn Fn() -> Element<Msg>>>,
    /// Set by StickyColumns.
    pub(crate) sticky: bool,
}

impl<T, Msg> Clone for TableColumn<T, Msg> {
    fn clone(&self) -> Self {
        TableColumn {
            key: self.key.clone(),
            header: self.header.clone(),
            width: self.width,
            align: self.align,
            resizable: self.resizable,
            sort_key: self.sort_key.clone(),
            filter: self.filter.clone(),
            is_row_header: self.is_row_header,
            value: self.value.clone(),
            render_cell: self.render_cell.clone(),
            header_element: self.header_element.clone(),
            sticky: self.sticky,
        }
    }
}

impl<T, Msg> TableColumn<T, Msg> {
    pub fn new(key: impl Into<String>, header: impl Into<String>) -> Self {
        TableColumn {
            key: key.into(),
            header: Some(header.into()),
            width: ColumnWidth::default(),
            align: ColumnAlign::Start,
            resizable: false,
            sort_key: None,
            filter: None,
            is_row_header: false,
            value: None,
            render_cell: None,
            header_element: None,
            sticky: false,
        }
    }
    pub fn value<V: Into<CellValue>>(mut self, f: impl Fn(&T) -> V + 'static) -> Self {
        self.value = Some(Rc::new(move |t| f(t).into()));
        self
    }
    pub fn render_cell(mut self, f: impl Fn(&T) -> Element<Msg> + 'static) -> Self {
        self.render_cell = Some(Rc::new(f));
        self
    }
    pub fn width(mut self, w: ColumnWidth) -> Self {
        self.width = w;
        self
    }
    pub fn align(mut self, a: ColumnAlign) -> Self {
        self.align = a;
        self
    }
    pub fn resizable(mut self, on: bool) -> Self {
        self.resizable = on;
        self
    }
    /// Sorts by this column's value; the sort key is the column key.
    pub fn sortable(mut self) -> Self {
        self.sort_key = Some(self.key.clone());
        self
    }
    pub fn sort_key(mut self, k: impl Into<String>) -> Self {
        self.sort_key = Some(k.into());
        self
    }
    pub fn filter(mut self, field: impl Into<String>) -> Self {
        self.filter = Some(field.into());
        self
    }
    /// The cell names its row for assistive technology (`<th scope=row>`).
    pub fn is_row_header(mut self, on: bool) -> Self {
        self.is_row_header = on;
        self
    }
    pub fn cell_value(&self, item: &T) -> CellValue {
        self.value.as_ref().map_or(CellValue::Empty, |f| f(item))
    }
}

/// A row as the row model shows it: which item, and how deep (tree data,
/// groups).
#[derive(Clone, Debug, PartialEq)]
pub enum RowEntry {
    Item {
        index: usize,
        depth: u32,
    },
    /// A group header row (GroupedRows).
    Group {
        key: String,
        count: usize,
        collapsed: bool,
    },
}

/// The parts of a header cell a plugin can change.
pub struct HeaderCellParts<Msg> {
    pub before: Vec<Element<Msg>>,
    pub content: Option<Element<Msg>>,
    pub after: Vec<Element<Msg>>,
    pub below: Vec<Element<Msg>>,
    /// Pressing the header sends this (a sort).
    pub on_press: Option<Msg>,
    /// `aria-sort`.
    pub sort: Option<SortDirection>,
}

/// The parts of a body row a plugin can change.
pub struct BodyRowParts<Msg> {
    pub is_selected: bool,
    pub ground: Option<Color>,
    pub on_press: Option<Msg>,
    /// Content under the row across all columns (row expansion).
    pub after_row: Vec<Element<Msg>>,
    /// A leading mark in the first cell (tree depth, expand toggle).
    pub first_cell_before: Vec<Element<Msg>>,
    pub indent: f32,
}

/// What the table hands to plugins about where they are.
pub struct TableEnv<'a, T> {
    pub data: &'a [T],
    pub id_of: &'a dyn Fn(&T) -> String,
}

/// A Table plugin (`TablePlugin`). Every hook has a default that changes
/// nothing; plugins run in the order they are added.
pub trait TablePlugin<T, Msg> {
    fn columns(
        &self,
        columns: Vec<TableColumn<T, Msg>>,
        _env: &TableEnv<T>,
    ) -> Vec<TableColumn<T, Msg>> {
        columns
    }
    fn rows(&self, rows: Vec<RowEntry>, _env: &TableEnv<T>) -> Vec<RowEntry> {
        rows
    }
    fn header_cell(
        &self,
        _column: &TableColumn<T, Msg>,
        _parts: &mut HeaderCellParts<Msg>,
        _env: &TableEnv<T>,
    ) {
    }
    fn body_row(
        &self,
        _item: &T,
        _index: usize,
        _parts: &mut BodyRowParts<Msg>,
        _env: &TableEnv<T>,
    ) {
    }
    /// Rows to draw when only a window shows (virtualization): the range
    /// and the space above and below it, for the body's scroll offset and
    /// height.
    fn window(&self, _row_count: usize, _scroll_top: f32, _viewport: f32) -> Option<RowWindow> {
        None
    }
    /// Whether the table draws only a window of rows.
    fn windows(&self) -> bool {
        false
    }
    /// Content under the table (a pager).
    fn footer(&self, _total_rows: usize, _env: &TableEnv<T>) -> Option<Element<Msg>> {
        None
    }
    /// A group header row's content.
    fn group_row(&self, _key: &str, _count: usize, _collapsed: bool) -> Option<Element<Msg>> {
        None
    }
    /// Column widths that override the columns' own (column resize).
    fn width_of(&self, _key: &str) -> Option<f32> {
        None
    }
}

/// The rows a virtualized table draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowWindow {
    pub start: usize,
    pub end: usize,
    pub before: f32,
    pub after: f32,
}

/// A table of typed rows (`Table`).
pub struct Table<T, Msg> {
    base: Base,
    data: Rc<Vec<T>>,
    columns: Vec<TableColumn<T, Msg>>,
    id_of: Rc<dyn Fn(&T) -> String>,
    density: ListDensity,
    dividers: TableDividers,
    is_striped: bool,
    has_hover: bool,
    vertical_align: TableVerticalAlign,
    text_overflow: TextOverflow,
    plugins: Vec<Rc<dyn TablePlugin<T, Msg>>>,
    label: Option<String>,
    max_height: Option<f32>,
}

impl<T: 'static, Msg: 'static> Table<T, Msg> {
    /// `id_of` gives each row a stable id (`idKey`).
    pub fn new(
        data: impl Into<Rc<Vec<T>>>,
        columns: Vec<TableColumn<T, Msg>>,
        id_of: impl Fn(&T) -> String + 'static,
    ) -> Self {
        Table {
            base: Base::default(),
            data: data.into(),
            columns,
            id_of: Rc::new(id_of),
            density: ListDensity::Balanced,
            dividers: TableDividers::Rows,
            is_striped: false,
            has_hover: false,
            vertical_align: TableVerticalAlign::Middle,
            text_overflow: TextOverflow::Wrap,
            plugins: Vec::new(),
            label: None,
            max_height: None,
        }
    }
    pub fn density(mut self, d: ListDensity) -> Self {
        self.density = d;
        self
    }
    pub fn dividers(mut self, d: TableDividers) -> Self {
        self.dividers = d;
        self
    }
    pub fn is_striped(mut self, on: bool) -> Self {
        self.is_striped = on;
        self
    }
    pub fn has_hover(mut self, on: bool) -> Self {
        self.has_hover = on;
        self
    }
    pub fn vertical_align(mut self, a: TableVerticalAlign) -> Self {
        self.vertical_align = a;
        self
    }
    pub fn text_overflow(mut self, o: TextOverflow) -> Self {
        self.text_overflow = o;
        self
    }
    pub fn plugin(mut self, p: impl TablePlugin<T, Msg> + 'static) -> Self {
        self.plugins.push(Rc::new(p));
        self
    }
    /// Sorting (`useTableSortable`).
    pub fn sortable(self, s: Sortable<Msg>) -> Self
    where
        Msg: Clone,
    {
        self.plugin(SortableColumns::from(s))
    }
    /// Filtering (`useTableFiltering`).
    pub fn filtering(self, f: Filtering<Msg>) -> Self
    where
        Msg: Clone,
    {
        self.plugin(FilteringColumns::from(f))
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    /// Scrolls the body under a sticky header past this height.
    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = Some(h);
        self
    }
}

impl<T: 'static, Msg: Clone + 'static> From<Table<T, Msg>> for Element<Msg> {
    fn from(t: Table<T, Msg>) -> Self {
        astryx_core::authoring::element(t)
    }
}

fn cell_pad(cx_space: &astryx_tokens::Tokens, d: ListDensity) -> (f32, f32) {
    let s = &cx_space.space;
    match d {
        ListDensity::Compact => (s.s2, s.s1),
        ListDensity::Balanced => (s.s3, s.s2),
        ListDensity::Spacious => (s.s4, s.s3),
    }
}

fn template<T, Msg>(
    cols: &[TableColumn<T, Msg>],
    widths: &dyn Fn(&str) -> Option<f32>,
) -> Vec<GridTemplateComponent<String>> {
    cols.iter()
        .map(|c| {
            let w = widths(&c.key).map(ColumnWidth::Pixel).unwrap_or(c.width);
            match w {
                ColumnWidth::Pixel(px) => GridTemplateComponent::Single(taffy::prelude::length(px)),
                ColumnWidth::Proportional { value, min_width } => {
                    GridTemplateComponent::Single(MinMax {
                        min: taffy::MinTrackSizingFunction::length(min_width.unwrap_or(0.0)),
                        max: taffy::MaxTrackSizingFunction::fr(value),
                    })
                }
            }
        })
        .collect()
}

impl<T: 'static, Msg: Clone + 'static> Build<Msg> for Table<T, Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let c = &t.color;
        let id_of = self.id_of.clone();
        let env = TableEnv {
            data: &self.data,
            id_of: &*id_of,
        };
        let mut columns = self.columns;
        for p in &self.plugins {
            columns = p.columns(columns, &env);
        }
        let mut rows: Vec<RowEntry> = (0..self.data.len())
            .map(|index| RowEntry::Item { index, depth: 0 })
            .collect();
        for p in &self.plugins {
            rows = p.rows(rows, &env);
        }
        let plugins = self.plugins.clone();
        let widths = move |k: &str| plugins.iter().find_map(|p| p.width_of(k));
        let tpl = template(&columns, &widths);
        let (px, py) = cell_pad(t, self.density);
        let bw = t.size.border_width;
        let row_rule = matches!(self.dividers, TableDividers::Rows | TableDividers::Grid);
        let col_rule = matches!(self.dividers, TableDividers::Columns | TableDividers::Grid);
        let n_cols = columns.len();
        let row_style = |ground: bool| Style {
            display: taffy::Display::Grid,
            grid_template_columns: tpl.clone(),
            align_items: match self.vertical_align {
                TableVerticalAlign::Middle => AlignItems::CENTER,
                TableVerticalAlign::Top => AlignItems::FLEX_START,
                TableVerticalAlign::Bottom => AlignItems::FLEX_END,
            },
            border: taffy::Rect {
                left: len(0.0),
                right: len(0.0),
                top: len(0.0),
                bottom: len(if row_rule && ground { bw } else { 0.0 }),
            },
            ..Style::default()
        };
        let cell_style = |i: usize, align: ColumnAlign| Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            justify_content: match align {
                ColumnAlign::Start => taffy::JustifyContent::FLEX_START,
                ColumnAlign::Center => taffy::JustifyContent::CENTER,
                ColumnAlign::End => taffy::JustifyContent::FLEX_END,
            },
            gap: taffy::Size {
                width: len(t.space.s1),
                height: len(0.0),
            },
            padding: taffy::Rect {
                left: len(px),
                right: len(px),
                top: len(py),
                bottom: len(py),
            },
            border: taffy::Rect {
                left: len(0.0),
                right: len(if col_rule && i + 1 < n_cols { bw } else { 0.0 }),
                top: len(0.0),
                bottom: len(0.0),
            },
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: LengthPercentageAuto::auto(),
            },
            align_self: Some(AlignItems::STRETCH),
            ..Style::default()
        };
        let rule_ink = color(c.border);
        let sem = match &self.label {
            Some(l) => Semantics::new(Role::Table, l.clone()),
            None => Semantics::role(Role::Table),
        };
        cx.key(self.base.id.as_deref());
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                ..Style::default()
            })
            .sem(sem.with_state(|s| s.row_count = Some(rows.len() as u32 + 1))),
        );
        // Header row.
        cx.begin(Node::new(row_style(true)).sem(Semantics::role(Role::Row)));
        if row_rule {
            rule(cx, rule_ink, bw, false);
        }
        for (i, col) in columns.iter().enumerate() {
            let mut parts = HeaderCellParts {
                before: Vec::new(),
                content: None,
                after: Vec::new(),
                below: Vec::new(),
                on_press: None,
                sort: None,
            };
            for p in &self.plugins {
                p.header_cell(col, &mut parts, &env);
            }
            let label = col.header.clone().unwrap_or_default();
            let mut sem = Semantics::new(Role::ColumnHeader, label.clone());
            sem.state.sort = parts.sort.map(|d| match d {
                SortDirection::Ascending => astryx_core::contract::Sort::Ascending,
                SortDirection::Descending => astryx_core::contract::Sort::Descending,
            });
            let node = Node::new(Style {
                flex_direction: FlexDirection::Column,
                justify_content: taffy::JustifyContent::CENTER,
                align_items: match col.align {
                    ColumnAlign::Start => AlignItems::FLEX_START,
                    ColumnAlign::Center => AlignItems::CENTER,
                    ColumnAlign::End => AlignItems::FLEX_END,
                },
                ..cell_style(i, col.align)
            })
            .sem(sem);
            cx.begin(node);
            if col_rule && i + 1 < n_cols {
                rule(cx, rule_ink, bw, true);
            }
            cx.begin(Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s1),
                    height: len(0.0),
                },
                ..Style::default()
            }));
            for b in parts.before {
                cx.child(b);
            }
            if let Some(content) = parts.content {
                cx.child(content);
            } else if let Some(h) = &col.header_element {
                cx.child(h());
            } else {
                let mut style = type_style(t.font.label, FontFamily::Body);
                style.weight = t.font.weight.semibold;
                let mut n = text_node(label, style, color(c.text_secondary));
                n.sem = None;
                if self.text_overflow == TextOverflow::Truncate {
                    text_draw(&mut n).max_lines = Some(1);
                }
                cx.leaf(n);
            }
            for a in parts.after {
                cx.child(a);
            }
            cx.end();
            for b in parts.below {
                cx.child(b);
            }
            cx.end();
        }
        cx.end();
        // Body.
        // The body scrolls under the header; a windowing plugin draws only
        // the rows in view, rebuilt as the body scrolls.
        let windows = self.plugins.iter().any(|p| p.windows());
        cx.key(Some("body"));
        let body_id = cx.next_id();
        let scroll_top = cx.scroll_offset(body_id);
        let viewport = self.max_height.unwrap_or(400.0);
        let window = self
            .plugins
            .iter()
            .find_map(|p| p.window(rows.len(), scroll_top, viewport));
        let (start, end) = window.map_or((0, rows.len()), |w| (w.start, w.end.min(rows.len())));
        let mut body = Node::new(Style {
            flex_direction: FlexDirection::Column,
            max_size: taffy::Size {
                width: LengthPercentageAuto::auto(),
                height: self
                    .max_height
                    .map_or(LengthPercentageAuto::auto(), LengthPercentageAuto::length),
            },
            overflow: taffy::Point {
                x: taffy::Overflow::Visible,
                y: if self.max_height.is_some() {
                    taffy::Overflow::Scroll
                } else {
                    taffy::Overflow::Visible
                },
            },
            ..Style::default()
        });
        body.rebuild_on_scroll = windows;
        if self.max_height.is_some() {
            body.scroll = Some(astryx_core::authoring::Axis::Vertical);
        }
        cx.key(Some("body"));
        cx.begin(body);
        if let Some(w) = window {
            cx.leaf(Node::new(Style {
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(w.before),
                },
                flex_shrink: 0.0,
                ..Style::default()
            }));
        }
        let last = end.saturating_sub(1);
        for (ri, entry) in rows.iter().enumerate().take(end).skip(start) {
            match entry {
                RowEntry::Group {
                    key,
                    count,
                    collapsed,
                } => {
                    let content = self
                        .plugins
                        .iter()
                        .find_map(|p| p.group_row(key, *count, *collapsed));
                    cx.key_slot(ri as u64);
                    cx.begin(
                        Node::new(Style {
                            flex_direction: FlexDirection::Row,
                            padding: taffy::Rect {
                                left: len(px),
                                right: len(px),
                                top: len(py),
                                bottom: len(py),
                            },
                            ..Style::default()
                        })
                        .draw(Draw::Box(BoxDraw::fill(
                            color(c.background_muted),
                            radii(0.0),
                        )))
                        .sem(Semantics::role(Role::Row)),
                    );
                    if row_rule {
                        rule(cx, rule_ink, bw, false);
                    }
                    if let Some(e) = content {
                        cx.child(e);
                    } else {
                        cx.child(crate::primitives::Text::new(format!("{key} ({count})")).into());
                    }
                    cx.end();
                }
                RowEntry::Item { index, depth } => {
                    let item = &self.data[*index];
                    let mut parts = BodyRowParts {
                        is_selected: false,
                        ground: None,
                        on_press: None,
                        after_row: Vec::new(),
                        first_cell_before: Vec::new(),
                        indent: *depth as f32 * t.space.s4,
                    };
                    for p in &self.plugins {
                        p.body_row(item, *index, &mut parts, &env);
                    }
                    let striped = self.is_striped && ri % 2 == 1;
                    let ground = if parts.is_selected {
                        color(c.accent_muted)
                    } else if let Some(g) = parts.ground {
                        g
                    } else if striped {
                        color(c.background_muted)
                    } else {
                        Color::TRANSPARENT
                    };
                    let rule_below = row_rule && ri != last;
                    let mut sem = Semantics::role(Role::Row);
                    sem.selected = parts.is_selected.then_some(true);
                    let id = (id_of)(item);
                    cx.key(Some(&id));
                    let mut node = Node::new(row_style(rule_below))
                        .draw(Draw::Box(BoxDraw::fill(ground, radii(0.0))))
                        .sem(sem);
                    if self.has_hover && !parts.is_selected {
                        node = node.variant(
                            astryx_core::layer::When::Hovered,
                            Draw::Box(BoxDraw::fill(color(c.background_muted), radii(0.0))),
                        );
                    }
                    if let Some(m) = parts.on_press.take() {
                        node = node.interact(Interact::new(Some(m), false));
                    }
                    cx.begin(node);
                    if rule_below {
                        rule(cx, rule_ink, bw, false);
                    }
                    let mut first_before = Some(parts.first_cell_before);
                    for (i, col) in columns.iter().enumerate() {
                        let value = col.cell_value(item);
                        let label = value.text();
                        let role = if col.is_row_header {
                            Role::RowHeader
                        } else {
                            Role::Cell
                        };
                        let mut node = Node::new(cell_style(i, col.align))
                            .sem(Semantics::new(role, label.clone()));
                        if i == 0 && parts.indent > 0.0 {
                            node.style.padding.left = len(px + parts.indent);
                        }
                        cx.begin(node);
                        if col_rule && i + 1 < n_cols {
                            rule(cx, rule_ink, bw, true);
                        }
                        if i == 0 {
                            for b in first_before.take().unwrap_or_default() {
                                cx.child(b);
                            }
                        }
                        if let Some(r) = &col.render_cell {
                            cx.child(r(item));
                        } else {
                            let mut n = text_node(
                                label,
                                type_style(t.font.body, FontFamily::Body),
                                color(c.text_primary),
                            );
                            n.sem = None;
                            n.style.flex_shrink = 1.0;
                            n.style.min_size.width = len_auto(0.0);
                            if self.text_overflow == TextOverflow::Truncate {
                                text_draw(&mut n).max_lines = Some(1);
                            }
                            cx.leaf(n);
                        }
                        cx.end();
                    }
                    cx.end();
                    for a in parts.after_row {
                        cx.child(a);
                    }
                }
            }
        }
        if let Some(w) = window {
            cx.leaf(Node::new(Style {
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(w.after),
                },
                flex_shrink: 0.0,
                ..Style::default()
            }));
        }
        cx.end();
        let total = rows.len();
        let footers: Vec<Element<Msg>> = self
            .plugins
            .iter()
            .filter_map(|p| p.footer(total, &env))
            .collect();
        for f in footers {
            cx.child(f);
        }
        cx.end();
    }
}

/// A one-point rule along a cell's or row's bottom (`side` false) or end
/// (`side` true) edge.
fn rule<Msg>(cx: &mut BuildCx<Msg>, ink: Color, width: f32, end: bool) {
    let a = LengthPercentageAuto::length(0.0);
    let auto = LengthPercentageAuto::auto();
    cx.leaf(
        Node::new(Style {
            position: taffy::Position::Absolute,
            inset: if end {
                taffy::Rect {
                    left: auto,
                    right: a,
                    top: a,
                    bottom: a,
                }
            } else {
                taffy::Rect {
                    left: a,
                    right: a,
                    top: auto,
                    bottom: a,
                }
            },
            size: if end {
                taffy::Size {
                    width: Dimension::length(width),
                    height: Dimension::auto(),
                }
            } else {
                taffy::Size {
                    width: Dimension::auto(),
                    height: Dimension::length(width),
                }
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(ink, radii(0.0)))),
    );
}
