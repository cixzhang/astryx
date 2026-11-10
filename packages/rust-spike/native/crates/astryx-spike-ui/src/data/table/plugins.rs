//! Table plugins.

use super::*;
use crate::composites::{Pagination, PaginationVariant};
use crate::controls::{Button, ButtonVariant, CheckboxInput, TextInput};
use crate::primitives::{CheckboxState, Icon, IconColor, IconSize, Text};
use astryx_spike_foundations::ComponentSize;
use std::collections::{BTreeMap, HashSet};

/// Gives a row's status marker.
pub type StatusFn<T> = Rc<dyn Fn(&T) -> Option<RowStatusMark>>;
/// Gives a tree row's child rows, as data indices.
pub type ChildrenFn<T> = Rc<dyn Fn(&T) -> Vec<usize>>;

/// A sort direction (`TableSortDirection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// One sort key and direction (`TableSortEntry`); the sort is a list so
/// several columns can sort at once.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SortEntry {
    pub sort_key: String,
    pub direction: SortDirection,
}

/// Sorting (`useTableSortable`): sortable column headers become buttons
/// with a direction arrow; pressing one cycles ascending → descending →
/// unsorted and sends the new sort.
pub struct Sortable<Msg> {
    pub sort: Vec<SortEntry>,
    pub on_sort_change: Rc<dyn Fn(Vec<SortEntry>) -> Msg>,
    pub allow_unsorted_state: bool,
}

impl<Msg> Sortable<Msg> {
    pub fn new(
        sort: Vec<SortEntry>,
        on_sort_change: impl Fn(Vec<SortEntry>) -> Msg + 'static,
    ) -> Self {
        Sortable {
            sort,
            on_sort_change: Rc::new(on_sort_change),
            allow_unsorted_state: true,
        }
    }
    pub fn allow_unsorted_state(mut self, on: bool) -> Self {
        self.allow_unsorted_state = on;
        self
    }
}

fn next_direction(current: Option<SortDirection>, allow_unsorted: bool) -> Option<SortDirection> {
    match current {
        None => Some(SortDirection::Ascending),
        Some(SortDirection::Ascending) => Some(SortDirection::Descending),
        Some(SortDirection::Descending) => (!allow_unsorted).then_some(SortDirection::Ascending),
    }
}

/// The rows sorted by `sort`, stable, using each column's value.
pub fn sort_rows<T, Msg>(
    rows: &mut [RowEntry],
    data: &[T],
    columns: &[TableColumn<T, Msg>],
    sort: &[SortEntry],
) {
    let keyed: Vec<(&TableColumn<T, Msg>, SortDirection)> = sort
        .iter()
        .filter_map(|e| {
            columns
                .iter()
                .find(|c| c.sort_key.as_deref() == Some(&e.sort_key))
                .map(|c| (c, e.direction))
        })
        .collect();
    if keyed.is_empty() {
        return;
    }
    rows.sort_by(|a, b| {
        let (RowEntry::Item { index: ia, .. }, RowEntry::Item { index: ib, .. }) = (a, b) else {
            return std::cmp::Ordering::Equal;
        };
        for (col, dir) in &keyed {
            let o = col
                .cell_value(&data[*ia])
                .order(&col.cell_value(&data[*ib]));
            let o = if *dir == SortDirection::Descending {
                o.reverse()
            } else {
                o
            };
            if o != std::cmp::Ordering::Equal {
                return o;
            }
        }
        std::cmp::Ordering::Equal
    });
}

/// Sorting needs the columns, which a plugin sees in `columns`; the row
/// model then sorts with the columns kept here.
pub struct SortableColumns<T, Msg> {
    inner: Sortable<Msg>,
    columns: std::cell::RefCell<Vec<TableColumn<T, Msg>>>,
}

impl<T, Msg> From<Sortable<Msg>> for SortableColumns<T, Msg> {
    fn from(inner: Sortable<Msg>) -> Self {
        SortableColumns {
            inner,
            columns: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl<T: 'static, Msg: Clone + 'static> TablePlugin<T, Msg> for SortableColumns<T, Msg> {
    fn columns(
        &self,
        columns: Vec<TableColumn<T, Msg>>,
        _env: &TableEnv<T>,
    ) -> Vec<TableColumn<T, Msg>> {
        *self.columns.borrow_mut() = columns.clone();
        columns
    }
    fn rows(&self, mut rows: Vec<RowEntry>, env: &TableEnv<T>) -> Vec<RowEntry> {
        sort_rows(
            &mut rows,
            env.data,
            &self.columns.borrow(),
            &self.inner.sort,
        );
        rows
    }
    fn header_cell(
        &self,
        column: &TableColumn<T, Msg>,
        parts: &mut HeaderCellParts<Msg>,
        _env: &TableEnv<T>,
    ) {
        let Some(key) = &column.sort_key else { return };
        let current = self
            .inner
            .sort
            .iter()
            .find(|e| &e.sort_key == key)
            .map(|e| e.direction);
        parts.sort = current;
        let glyph = match current {
            Some(SortDirection::Ascending) => "arrowUp",
            Some(SortDirection::Descending) => "arrowDown",
            None => "arrowsUpDown",
        };
        parts.after.push(
            Icon::new(glyph)
                .size(IconSize::Xsm)
                .color(if current.is_some() {
                    IconColor::Primary
                } else {
                    IconColor::Secondary
                })
                .into(),
        );
        let next = next_direction(current, self.inner.allow_unsorted_state);
        let sort = match next {
            Some(direction) => vec![SortEntry {
                sort_key: key.clone(),
                direction,
            }],
            None => Vec::new(),
        };
        let label = column.header.clone().unwrap_or_default();
        let after = std::mem::take(&mut parts.after);
        parts.content = Some(astryx_core::authoring::element(SortHeader {
            label,
            marks: after,
            msg: (self.inner.on_sort_change)(sort),
        }));
    }
}

/// Row selection (`useTableSelection`): a checkbox column, a select-all
/// header checkbox, and selected rows on the accent-muted ground.
pub struct Selection<Msg> {
    pub selected: HashSet<String>,
    pub on_change: Rc<dyn Fn(HashSet<String>) -> Msg>,
    pub is_single: bool,
    pub has_row_press: bool,
}

impl<Msg> Selection<Msg> {
    pub fn new(
        selected: HashSet<String>,
        on_change: impl Fn(HashSet<String>) -> Msg + 'static,
    ) -> Self {
        Selection {
            selected,
            on_change: Rc::new(on_change),
            is_single: false,
            has_row_press: false,
        }
    }
    /// One row at a time; no select-all.
    pub fn single(mut self) -> Self {
        self.is_single = true;
        self
    }
    /// Pressing anywhere on a row toggles it.
    pub fn row_press(mut self) -> Self {
        self.has_row_press = true;
        self
    }
}

impl<T: 'static, Msg: Clone + 'static> TablePlugin<T, Msg> for Selection<Msg> {
    fn columns(
        &self,
        columns: Vec<TableColumn<T, Msg>>,
        env: &TableEnv<T>,
    ) -> Vec<TableColumn<T, Msg>> {
        let ids: Vec<String> = env.data.iter().map(|t| (env.id_of)(t)).collect();
        let n_sel = ids.iter().filter(|i| self.selected.contains(*i)).count();
        let state = if n_sel == 0 {
            CheckboxState::Unchecked
        } else if n_sel == ids.len() {
            CheckboxState::Checked
        } else {
            CheckboxState::Indeterminate
        };
        let all: HashSet<String> = ids.into_iter().collect();
        let on = self.on_change.clone();
        let next = if state == CheckboxState::Checked {
            HashSet::new()
        } else {
            all
        };
        let single = self.is_single;
        let mut col = TableColumn::new("__select", "");
        col.header = Some(String::new());
        col.width = ColumnWidth::Pixel(44.0);
        if !single {
            let msg = Rc::new(move || on(next.clone()));
            col.header_element = Some(Rc::new(move || {
                let m = msg.clone();
                CheckboxInput::new("Select all rows", state)
                    .is_label_hidden(true)
                    .on_change(move |_| m())
                    .into()
            }));
        }
        let mut out = vec![col];
        out.extend(columns);
        out
    }
    fn body_row(&self, item: &T, _index: usize, parts: &mut BodyRowParts<Msg>, env: &TableEnv<T>) {
        let id = (env.id_of)(item);
        let on = self.selected.contains(&id);
        parts.is_selected = on;
        let mut next = if self.is_single {
            HashSet::new()
        } else {
            self.selected.clone()
        };
        if on {
            next.remove(&id);
        } else {
            next.insert(id.clone());
        }
        let f = self.on_change.clone();
        let next2 = next.clone();
        parts.first_cell_before.push(
            CheckboxInput::new(
                format!("Select row {id}"),
                if on {
                    CheckboxState::Checked
                } else {
                    CheckboxState::Unchecked
                },
            )
            .is_label_hidden(true)
            .on_change(move |_| f(next2.clone()))
            .into(),
        );
        if self.has_row_press {
            parts.on_press = Some((self.on_change)(next));
        }
    }
}

/// A Selection's checkbox sits in the select column; the table puts row
/// marks in the first cell, which is that column.
/// Bulk actions over the selection (`TableSelectionToolbar`).
pub struct TableSelectionToolbar<Msg> {
    count: usize,
    on_clear: Msg,
    start_content: Vec<Element<Msg>>,
    label: String,
}

impl<Msg: Clone + 'static> TableSelectionToolbar<Msg> {
    pub fn new(count: usize, on_clear: Msg) -> Self {
        TableSelectionToolbar {
            count,
            on_clear,
            start_content: Vec::new(),
            label: "Bulk actions".into(),
        }
    }
    pub fn start_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.start_content.push(e.into());
        self
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = l.into();
        self
    }
}

impl<Msg: Clone + 'static> From<TableSelectionToolbar<Msg>> for Element<Msg> {
    fn from(t: TableSelectionToolbar<Msg>) -> Self {
        let mut bar = crate::composites::Toolbar::new(t.label)
            .size(ComponentSize::Sm)
            .variant(crate::layout::SectionVariant::Muted)
            .start_content(Text::new(format!("{} selected", t.count)));
        for e in t.start_content {
            bar = bar.start_content(e);
        }
        bar.end_content(
            Button::new("Unselect All")
                .variant(ButtonVariant::Ghost)
                .on_click(t.on_clear),
        )
        .into()
    }
}

/// Filtering (`useTableFiltering`): rows whose filtered columns contain
/// the filter text, case-insensitive. The `Inline` variant draws a filter
/// field under each filterable header.
pub struct Filtering<Msg> {
    pub filters: BTreeMap<String, String>,
    pub on_filter_change: Rc<dyn Fn(String, Option<String>) -> Msg>,
    pub variant: FilterVariant,
    columns: std::cell::RefCell<Vec<(String, String)>>,
}

/// useTableFiltering `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FilterVariant {
    #[default]
    Popover,
    Inline,
    InlineCompact,
}

impl<Msg> Filtering<Msg> {
    pub fn new(
        filters: BTreeMap<String, String>,
        on_filter_change: impl Fn(String, Option<String>) -> Msg + 'static,
    ) -> Self {
        Filtering {
            filters,
            on_filter_change: Rc::new(on_filter_change),
            variant: FilterVariant::Popover,
            columns: std::cell::RefCell::new(Vec::new()),
        }
    }
    pub fn variant(mut self, v: FilterVariant) -> Self {
        self.variant = v;
        self
    }
}

impl<T: 'static, Msg: Clone + 'static> TablePlugin<T, Msg> for Filtering<Msg> {
    fn columns(
        &self,
        columns: Vec<TableColumn<T, Msg>>,
        _env: &TableEnv<T>,
    ) -> Vec<TableColumn<T, Msg>> {
        *self.columns.borrow_mut() = columns
            .iter()
            .filter_map(|c| c.filter.clone().map(|f| (f, c.key.clone())))
            .collect();
        columns
    }
    fn rows(&self, rows: Vec<RowEntry>, _env: &TableEnv<T>) -> Vec<RowEntry> {
        // Matching needs the columns' values; FilteringValues does it.
        rows
    }
    fn header_cell(
        &self,
        column: &TableColumn<T, Msg>,
        parts: &mut HeaderCellParts<Msg>,
        _env: &TableEnv<T>,
    ) {
        let Some(field) = &column.filter else { return };
        let value = self.filters.get(field).cloned().unwrap_or_default();
        let f = self.on_filter_change.clone();
        let field2 = field.clone();
        let input = TextInput::new(
            format!("Filter {}", column.header.clone().unwrap_or_default()),
            value,
        )
        .is_label_hidden(true)
        .size(ComponentSize::Sm)
        .placeholder("Filter")
        .on_change(move |v: String| f(field2.clone(), (!v.is_empty()).then_some(v)));
        match self.variant {
            FilterVariant::Inline | FilterVariant::InlineCompact => parts.below.push(input.into()),
            FilterVariant::Popover => {
                let active = self.filters.contains_key(field);
                parts.after.push(
                    crate::overlays::Popover::new(
                        format!("filter-{field}"),
                        Icon::new("filter").size(IconSize::Xsm).color(if active {
                            IconColor::Accent
                        } else {
                            IconColor::Secondary
                        }),
                        input,
                    )
                    .label(format!(
                        "Filter {}",
                        column.header.clone().unwrap_or_default()
                    ))
                    .into(),
                );
            }
        }
    }
}

/// Keeps rows whose filtered columns contain each filter's text.
pub fn filter_rows<T, Msg>(
    rows: Vec<RowEntry>,
    data: &[T],
    columns: &[TableColumn<T, Msg>],
    filters: &BTreeMap<String, String>,
) -> Vec<RowEntry> {
    let active: Vec<(&TableColumn<T, Msg>, String)> = filters
        .iter()
        .filter_map(|(field, text)| {
            columns
                .iter()
                .find(|c| c.filter.as_deref() == Some(field))
                .map(|c| (c, text.to_lowercase()))
        })
        .collect();
    if active.is_empty() {
        return rows;
    }
    rows.into_iter()
        .filter(|r| match r {
            RowEntry::Item { index, .. } => active.iter().all(|(c, text)| {
                c.cell_value(&data[*index])
                    .text()
                    .to_lowercase()
                    .contains(text)
            }),
            RowEntry::Group { .. } => true,
        })
        .collect()
}

/// Filtering with its row model: the table hands the columns to `columns`
/// first, so this keeps them for `rows`.
pub struct FilteringColumns<T, Msg> {
    inner: Filtering<Msg>,
    columns: std::cell::RefCell<Vec<TableColumn<T, Msg>>>,
}

impl<T, Msg> From<Filtering<Msg>> for FilteringColumns<T, Msg> {
    fn from(inner: Filtering<Msg>) -> Self {
        FilteringColumns {
            inner,
            columns: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl<T: 'static, Msg: Clone + 'static> TablePlugin<T, Msg> for FilteringColumns<T, Msg> {
    fn columns(
        &self,
        columns: Vec<TableColumn<T, Msg>>,
        env: &TableEnv<T>,
    ) -> Vec<TableColumn<T, Msg>> {
        *self.columns.borrow_mut() = columns.clone();
        TablePlugin::<T, Msg>::columns(&self.inner, columns, env)
    }
    fn rows(&self, rows: Vec<RowEntry>, env: &TableEnv<T>) -> Vec<RowEntry> {
        filter_rows(rows, env.data, &self.columns.borrow(), &self.inner.filters)
    }
    fn header_cell(
        &self,
        column: &TableColumn<T, Msg>,
        parts: &mut HeaderCellParts<Msg>,
        env: &TableEnv<T>,
    ) {
        TablePlugin::<T, Msg>::header_cell(&self.inner, column, parts, env);
    }
}

/// Pagination (`useTablePagination`): one page of rows and a pager under
/// the table.
pub struct TablePagination<Msg> {
    pub page: u32,
    pub page_size: u32,
    pub on_change: Rc<dyn Fn(u32) -> Msg>,
    pub variant: PaginationVariant,
}

impl<Msg> TablePagination<Msg> {
    pub fn new(page: u32, page_size: u32, on_change: impl Fn(u32) -> Msg + 'static) -> Self {
        TablePagination {
            page: page.max(1),
            page_size: page_size.max(1),
            on_change: Rc::new(on_change),
            variant: PaginationVariant::Pages,
        }
    }
    pub fn variant(mut self, v: PaginationVariant) -> Self {
        self.variant = v;
        self
    }
}

impl<T: 'static, Msg: Clone + 'static> TablePlugin<T, Msg> for TablePagination<Msg> {
    fn rows(&self, rows: Vec<RowEntry>, _env: &TableEnv<T>) -> Vec<RowEntry> {
        let start = ((self.page - 1) * self.page_size) as usize;
        rows.into_iter()
            .skip(start)
            .take(self.page_size as usize)
            .collect()
    }
    fn footer(&self, _total: usize, env: &TableEnv<T>) -> Option<Element<Msg>> {
        let f = self.on_change.clone();
        Some(
            Pagination::new(self.page, move |p| f(p))
                .total_items(env.data.len() as u32)
                .page_size(self.page_size)
                .variant(self.variant)
                .into(),
        )
    }
}

/// A row number column (`useTableRowIndex`).
pub struct RowIndex {
    pub start: usize,
}

impl<T: 'static, Msg: 'static> TablePlugin<T, Msg> for RowIndex {
    fn columns(
        &self,
        columns: Vec<TableColumn<T, Msg>>,
        env: &TableEnv<T>,
    ) -> Vec<TableColumn<T, Msg>> {
        // Numbers follow data order; ids map rows to their numbers.
        let ids: Vec<String> = env.data.iter().map(|t| (env.id_of)(t)).collect();
        let start = self.start;
        let id_of_ptr: Rc<Vec<String>> = Rc::new(ids);
        let lookup = id_of_ptr.clone();
        let _ = lookup;
        let mut col: TableColumn<T, Msg> = TableColumn::new("__index", "#")
            .width(ColumnWidth::Pixel(48.0))
            .align(ColumnAlign::End);
        let index_of: Rc<dyn Fn(&T) -> usize> = {
            let ptr = env.data.as_ptr() as usize;
            let size = std::mem::size_of::<T>().max(1);
            Rc::new(move |t: &T| ((t as *const T as usize).saturating_sub(ptr)) / size)
        };
        col.value = Some(Rc::new(move |t| {
            CellValue::Number((index_of(t) + start) as f64)
        }));
        let mut out = vec![col];
        out.extend(columns);
        out
    }
}

/// A row's status marker (`useTableRowStatus`): a 28 point column with
/// a colored dot or an icon.
pub struct RowStatus<T> {
    pub status_of: StatusFn<T>,
}

/// What the status column shows for a row.
#[derive(Clone, Debug, PartialEq)]
pub enum RowStatusMark {
    Dot(IconColor),
    Icon(String, IconColor),
}

impl<T: 'static, Msg: 'static> TablePlugin<T, Msg> for RowStatus<T> {
    fn columns(
        &self,
        columns: Vec<TableColumn<T, Msg>>,
        _env: &TableEnv<T>,
    ) -> Vec<TableColumn<T, Msg>> {
        let f = self.status_of.clone();
        let mut col: TableColumn<T, Msg> = TableColumn::new("__rowStatus", "")
            .width(ColumnWidth::Pixel(28.0))
            .align(ColumnAlign::Center);
        col.render_cell = Some(Rc::new(move |t| match f(t) {
            Some(RowStatusMark::Icon(name, c)) => {
                Icon::new(name).size(IconSize::Sm).color(c).into()
            }
            Some(RowStatusMark::Dot(c)) => astryx_core::authoring::element(Dot(c)),
            None => astryx_core::authoring::element(Spacer(0.0)),
        }));
        let mut out = vec![col];
        out.extend(columns);
        out
    }
}

struct Dot(IconColor);

impl<Msg: 'static> Build<Msg> for Dot {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let ink = self.0.resolve(cx);
        cx.leaf(
            Node::new(Style {
                size: taffy::Size {
                    width: Dimension::length(8.0),
                    height: Dimension::length(8.0),
                },
                flex_shrink: 0.0,
                ..Style::default()
            })
            .draw(Draw::Box(BoxDraw::fill(ink, radii(4.0)))),
        );
    }
}

/// Rows grouped under collapsible headers (`useTableGroupedRows`).
pub struct GroupedRows<T, Msg> {
    pub group_by: Rc<dyn Fn(&T) -> String>,
    pub collapsed: HashSet<String>,
    pub on_toggle: Rc<dyn Fn(String) -> Msg>,
    pub group_order: Option<Vec<String>>,
}

impl<T: 'static, Msg: Clone + 'static> TablePlugin<T, Msg> for GroupedRows<T, Msg> {
    fn rows(&self, rows: Vec<RowEntry>, env: &TableEnv<T>) -> Vec<RowEntry> {
        let mut groups: Vec<(String, Vec<RowEntry>)> = Vec::new();
        for r in rows {
            let RowEntry::Item { index, .. } = r else {
                continue;
            };
            let k = (self.group_by)(&env.data[index]);
            match groups.iter_mut().find(|(g, _)| *g == k) {
                Some((_, v)) => v.push(r),
                None => groups.push((k, vec![r])),
            }
        }
        if let Some(order) = &self.group_order {
            groups.sort_by_key(|(k, _)| order.iter().position(|o| o == k).unwrap_or(usize::MAX));
        }
        let mut out = Vec::new();
        for (key, items) in groups {
            let collapsed = self.collapsed.contains(&key);
            out.push(RowEntry::Group {
                key,
                count: items.len(),
                collapsed,
            });
            if !collapsed {
                out.extend(items);
            }
        }
        out
    }
    fn group_row(&self, key: &str, count: usize, collapsed: bool) -> Option<Element<Msg>> {
        Some(
            Button::new(format!("{key} ({count})"))
                .variant(ButtonVariant::Ghost)
                .size(ComponentSize::Sm)
                .icon(Icon::new(if collapsed {
                    "chevronRight"
                } else {
                    "chevronDown"
                }))
                .on_click((self.on_toggle)(key.to_string()))
                .into(),
        )
    }
}

/// Tree rows (`useTableTreeData`): children under expandable parents.
pub struct TreeData<T, Msg> {
    pub children_of: ChildrenFn<T>,
    pub is_root: Rc<dyn Fn(&T) -> bool>,
    pub expanded: HashSet<String>,
    pub on_toggle: Rc<dyn Fn(String) -> Msg>,
}

impl<T: 'static, Msg: Clone + 'static> TablePlugin<T, Msg> for TreeData<T, Msg> {
    fn rows(&self, rows: Vec<RowEntry>, env: &TableEnv<T>) -> Vec<RowEntry> {
        fn walk<T, Msg>(
            p: &TreeData<T, Msg>,
            env: &TableEnv<T>,
            index: usize,
            depth: u32,
            out: &mut Vec<RowEntry>,
        ) {
            out.push(RowEntry::Item { index, depth });
            let item = &env.data[index];
            if p.expanded.contains(&(env.id_of)(item)) {
                for c in (p.children_of)(item) {
                    walk(p, env, c, depth + 1, out);
                }
            }
        }
        let mut out = Vec::new();
        for r in rows {
            if let RowEntry::Item { index, .. } = r {
                if (self.is_root)(&env.data[index]) {
                    walk(self, env, index, 0, &mut out);
                }
            }
        }
        out
    }
    fn body_row(&self, item: &T, _index: usize, parts: &mut BodyRowParts<Msg>, env: &TableEnv<T>) {
        let id = (env.id_of)(item);
        if (self.children_of)(item).is_empty() {
            parts
                .first_cell_before
                .push(astryx_core::authoring::element(Spacer(20.0)));
            return;
        }
        let open = self.expanded.contains(&id);
        parts.first_cell_before.push(
            Button::new(if open { "Collapse row" } else { "Expand row" })
                .variant(ButtonVariant::Ghost)
                .size(ComponentSize::Sm)
                .is_icon_only(true)
                .icon(Icon::new(if open { "chevronDown" } else { "chevronRight" }))
                .on_click((self.on_toggle)(id))
                .into(),
        );
    }
}

struct Spacer(f32);

impl<Msg: 'static> Build<Msg> for Spacer {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        cx.leaf(Node::new(Style {
            size: taffy::Size {
                width: Dimension::length(self.0),
                height: Dimension::length(1.0),
            },
            flex_shrink: 0.0,
            ..Style::default()
        }));
    }
}

/// Expandable detail under rows (`useTableRowExpansion`).
pub struct RowExpansion<T, Msg> {
    pub expanded: HashSet<String>,
    pub on_toggle: Rc<dyn Fn(String) -> Msg>,
    pub render: CellFn<T, Msg>,
}

impl<T: 'static, Msg: Clone + 'static> TablePlugin<T, Msg> for RowExpansion<T, Msg> {
    fn body_row(&self, item: &T, _index: usize, parts: &mut BodyRowParts<Msg>, env: &TableEnv<T>) {
        let id = (env.id_of)(item);
        let open = self.expanded.contains(&id);
        parts.first_cell_before.push(
            Button::new(if open { "Hide details" } else { "Show details" })
                .variant(ButtonVariant::Ghost)
                .size(ComponentSize::Sm)
                .is_icon_only(true)
                .icon(Icon::new(if open { "chevronDown" } else { "chevronRight" }))
                .on_click((self.on_toggle)(id))
                .into(),
        );
        if open {
            parts.after_row.push((self.render)(item));
        }
    }
}

/// Which columns show (`useTableColumnSettings`).
pub struct ColumnSettings {
    pub active: Vec<String>,
}

impl<T: 'static, Msg: 'static> TablePlugin<T, Msg> for ColumnSettings {
    fn columns(
        &self,
        columns: Vec<TableColumn<T, Msg>>,
        _env: &TableEnv<T>,
    ) -> Vec<TableColumn<T, Msg>> {
        columns
            .into_iter()
            .filter(|c| c.key.starts_with("__") || self.active.contains(&c.key))
            .collect()
    }
}

/// Column widths set by dragging header edges (`useTableColumnResize`).
pub struct ColumnResize {
    pub widths: BTreeMap<String, f32>,
    pub min_width: f32,
    pub max_width: f32,
}

impl<T: 'static, Msg: 'static> TablePlugin<T, Msg> for ColumnResize {
    fn width_of(&self, key: &str) -> Option<f32> {
        self.widths
            .get(key)
            .map(|w| w.clamp(self.min_width, self.max_width))
    }
}

/// Columns that stay in view while the table scrolls sideways
/// (`useTableStickyColumns`).
pub struct StickyColumns {
    pub count: usize,
}

impl<T: 'static, Msg: 'static> TablePlugin<T, Msg> for StickyColumns {
    fn columns(
        &self,
        mut columns: Vec<TableColumn<T, Msg>>,
        _env: &TableEnv<T>,
    ) -> Vec<TableColumn<T, Msg>> {
        for c in columns.iter_mut().take(self.count) {
            c.sticky = true;
        }
        columns
    }
}

/// Draws only the rows in view (`useTableRowsVirtualization`): fixed row
/// height, a scroll offset, and a few rows of overscan.
pub struct RowsVirtualization {
    pub row_height: f32,
    pub viewport_height: f32,
    pub scroll_top: f32,
    pub overscan: usize,
}

impl<T: 'static, Msg: 'static> TablePlugin<T, Msg> for RowsVirtualization {
    fn window(&self, count: usize) -> Option<RowWindow> {
        Some(row_window(
            count,
            self.row_height,
            self.viewport_height,
            self.scroll_top,
            self.overscan,
        ))
    }
}

/// The rows a viewport over fixed-height rows shows, with overscan.
pub fn row_window(
    count: usize,
    row_height: f32,
    viewport: f32,
    scroll_top: f32,
    overscan: usize,
) -> RowWindow {
    let h = row_height.max(1.0);
    let first = (scroll_top / h).floor().max(0.0) as usize;
    let visible = (viewport / h).ceil() as usize + 1;
    let start = first.saturating_sub(overscan).min(count);
    let end = (first + visible + overscan).min(count);
    RowWindow {
        start,
        end,
        before: start as f32 * h,
        after: (count - end) as f32 * h,
    }
}

#[allow(dead_code)]
fn _unused(_: CheckboxState) {}

/// A sortable header: the header text and its arrow, pressable as one
/// button, styled as the header text.
struct SortHeader<Msg> {
    label: String,
    marks: Vec<Element<Msg>>,
    msg: Msg,
}

impl<Msg: 'static> Build<Msg> for SortHeader<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let t = cx.tokens;
        let c = &t.color;
        let r = radii(t.radius.inner);
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::CENTER,
                gap: taffy::Size {
                    width: len(t.space.s1),
                    height: len(0.0),
                },
                ..Style::default()
            })
            .interact(Interact::new(Some(self.msg), false))
            .indicator(astryx_core::authoring::Indicator::ring(
                color(c.accent),
                r,
                None,
            ))
            .sem(Semantics::new(Role::Button, self.label.clone())),
        );
        let mut style = type_style(t.font.label, FontFamily::Body);
        style.weight = t.font.weight.semibold;
        let mut n = text_node(self.label, style, color(c.text_secondary));
        n.sem = None;
        cx.leaf(n);
        for m in self.marks {
            cx.child(m);
        }
        cx.end();
    }
}
