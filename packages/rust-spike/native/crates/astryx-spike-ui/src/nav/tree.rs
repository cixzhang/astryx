//! TreeList: a tree of rows with expandable branches and guide lines.

use crate::controls::ListDensity;
use crate::util::{color, len, len_auto, radii};
use astryx_core::authoring::{
    text_draw, text_node, type_style, Base, BoxDraw, Build, BuildCx, Draw, Indicator, Interact,
    KeyResponse, LocalHandle, Node, Semantics,
};
use astryx_core::contract::{FontFamily, Role};
use astryx_core::{Element, InputPhase, Key, KeyEvent};
use std::collections::HashSet;
use std::rc::Rc;
use taffy::{AlignItems, Dimension, FlexDirection, LengthPercentageAuto, Style};

/// One row of a TreeList (`TreeListItemData`).
pub struct TreeListItem<Msg> {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub start_content: Option<Element<Msg>>,
    pub end_content: Option<Element<Msg>>,
    pub children: Vec<TreeListItem<Msg>>,
    pub on_click: Option<Msg>,
    pub href: Option<String>,
    pub is_disabled: bool,
    pub is_selected: bool,
    /// Starts expanded.
    pub is_expanded: bool,
}

impl<Msg> TreeListItem<Msg> {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        TreeListItem {
            id: id.into(),
            label: label.into(),
            description: None,
            start_content: None,
            end_content: None,
            children: Vec::new(),
            on_click: None,
            href: None,
            is_disabled: false,
            is_selected: false,
            is_expanded: false,
        }
    }
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    pub fn start_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.start_content = Some(e.into());
        self
    }
    pub fn end_content(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.end_content = Some(e.into());
        self
    }
    pub fn child(mut self, c: TreeListItem<Msg>) -> Self {
        self.children.push(c);
        self
    }
    pub fn on_click(mut self, m: Msg) -> Self {
        self.on_click = Some(m);
        self
    }
    pub fn href(mut self, h: impl Into<String>) -> Self {
        self.href = Some(h.into());
        self
    }
    pub fn is_disabled(mut self, on: bool) -> Self {
        self.is_disabled = on;
        self
    }
    pub fn is_selected(mut self, on: bool) -> Self {
        self.is_selected = on;
        self
    }
    pub fn is_expanded(mut self, on: bool) -> Self {
        self.is_expanded = on;
        self
    }
}

/// TreeList `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TreeListVariant {
    #[default]
    LineGuides,
    NoGuides,
}

/// Which branches are open, after the person's toggles.
#[derive(Clone, Debug, Default, PartialEq)]
struct TreeState {
    opened: HashSet<String>,
    closed: HashSet<String>,
}

/// A tree of rows (`TreeList`). Right opens a branch or enters it, Left
/// closes it or goes to the parent; Up and Down move between rows.
pub struct TreeList<Msg> {
    base: Base,
    items: Vec<TreeListItem<Msg>>,
    density: ListDensity,
    variant: TreeListVariant,
    header: Option<Element<Msg>>,
    label: Option<String>,
}

impl<Msg> TreeList<Msg> {
    pub fn new(items: Vec<TreeListItem<Msg>>) -> Self {
        TreeList {
            base: Base::default(),
            items,
            density: ListDensity::Balanced,
            variant: TreeListVariant::LineGuides,
            header: None,
            label: None,
        }
    }
    pub fn density(mut self, d: ListDensity) -> Self {
        self.density = d;
        self
    }
    pub fn variant(mut self, v: TreeListVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn header(mut self, e: impl Into<Element<Msg>>) -> Self {
        self.header = Some(e.into());
        self
    }
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
}

crate::util::component!(TreeList<Msg>);

struct Ctx {
    tree_id: String,
    state: LocalHandle<TreeState>,
    row_height: f32,
    indent: f32,
    guides: bool,
    group: u64,
}

fn is_open<Msg>(item: &TreeListItem<Msg>, s: &TreeState) -> bool {
    if s.opened.contains(&item.id) {
        true
    } else if s.closed.contains(&item.id) {
        false
    } else {
        item.is_expanded
    }
}

fn row_id(tree: &str, item: &str) -> String {
    format!("{tree}:{item}")
}

fn rows<Msg: Clone + 'static>(
    cx: &mut BuildCx<Msg>,
    k: &Ctx,
    items: Vec<TreeListItem<Msg>>,
    level: u32,
    parent: Option<String>,
    first_stop: &mut bool,
) {
    let t = cx.tokens;
    let c = &t.color;
    let state = k.state.get();
    let count = items.len();
    for (i, item) in items.into_iter().enumerate() {
        let has_children = !item.children.is_empty();
        let open = has_children && is_open(&item, &state);
        let r = radii(t.radius.element);
        let mut sem = Semantics::new(Role::TreeItem, item.label.clone());
        sem.selected = item.is_selected.then_some(true);
        sem.disabled = item.is_disabled;
        sem.description = item.description.clone();
        sem.state.level = Some(level);
        sem.state.pos_in_set = Some(i as u32 + 1);
        sem.state.set_size = Some(count as u32);
        if has_children {
            sem.state.expanded = Some(open);
        }
        // Keys on the row: open, close, enter, or leave branches.
        let on_key = {
            let state = k.state.clone();
            let focus = cx.focus_requests();
            let id = item.id.clone();
            let tree = k.tree_id.clone();
            let first_child = item.children.first().map(|c| c.id.clone());
            let parent = parent.clone();
            Rc::new(move |e: &KeyEvent| -> KeyResponse<Msg> {
                if e.phase != InputPhase::Pressed {
                    return KeyResponse::Pass;
                }
                let (fwd, back) = (Key::Right, Key::Left);
                let mut s = state.get();
                if e.key == fwd && has_children {
                    if open {
                        if let Some(c) = &first_child {
                            focus.request(row_id(&tree, c));
                        }
                    } else {
                        s.closed.remove(&id);
                        s.opened.insert(id.clone());
                        state.set(s);
                    }
                    return KeyResponse::Handled;
                }
                if e.key == back {
                    if open {
                        s.opened.remove(&id);
                        s.closed.insert(id.clone());
                        state.set(s);
                        return KeyResponse::Handled;
                    }
                    if let Some(p) = &parent {
                        focus.request(row_id(&tree, p));
                        return KeyResponse::Handled;
                    }
                }
                KeyResponse::Pass
            })
        };
        let mut interact =
            Interact::new(item.on_click.clone(), item.is_disabled).href(item.href.as_deref());
        if has_children {
            let state = k.state.clone();
            let id = item.id.clone();
            interact = interact.effect(move || {
                let mut s = state.get();
                if open {
                    s.opened.remove(&id);
                    s.closed.insert(id.clone());
                } else {
                    s.closed.remove(&id);
                    s.opened.insert(id.clone());
                }
                state.set(s);
            });
        }
        let indent = level.saturating_sub(1) as f32 * k.indent;
        let mut node = Node::new(Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::CENTER,
            gap: taffy::Size {
                width: len(t.space.s2),
                height: len(0.0),
            },
            padding: taffy::Rect {
                left: len(t.space.s2 + indent),
                right: len(t.space.s2),
                top: len(0.0),
                bottom: len(0.0),
            },
            min_size: taffy::Size {
                width: LengthPercentageAuto::auto(),
                height: len_auto(k.row_height),
            },
            margin: taffy::Rect {
                left: len_auto(0.0),
                right: len_auto(0.0),
                top: len_auto(t.space.s0_5 / 2.0),
                bottom: len_auto(t.space.s0_5 / 2.0),
            },
            ..Style::default()
        })
        .draw(Draw::Box(BoxDraw::fill(
            if item.is_selected {
                color(c.accent_muted)
            } else {
                astryx_core::contract::Color::TRANSPARENT
            },
            r,
        )))
        .variant(
            astryx_core::layer::When::Hovered,
            Draw::Box(BoxDraw::fill(color(c.overlay_hover), r)),
        )
        .interact(interact)
        .indicator(Indicator::ring(
            color(c.accent),
            r,
            Some(color(c.overlay_pressed)),
        ))
        .sem(sem)
        .opacity(if item.is_disabled { 0.5 } else { 1.0 });
        node.on_key = Some(on_key);
        node.group = Some(k.group);
        if *first_stop || item.is_selected {
            node.group_entry = true;
            *first_stop = false;
        }
        cx.key(Some(&row_id(&k.tree_id, &item.id)));
        cx.begin(node);
        // Guide lines: one per ancestor level, through the chevron column.
        if k.guides {
            for l in 1..level {
                let x = t.space.s2 + (l - 1) as f32 * k.indent + 10.0;
                cx.leaf(
                    Node::new(Style {
                        position: taffy::Position::Absolute,
                        inset: taffy::Rect {
                            left: LengthPercentageAuto::length(x),
                            right: LengthPercentageAuto::auto(),
                            top: LengthPercentageAuto::length(-t.space.s0_5),
                            bottom: LengthPercentageAuto::length(-t.space.s0_5),
                        },
                        size: taffy::Size {
                            width: Dimension::length(t.size.border_width),
                            height: Dimension::auto(),
                        },
                        ..Style::default()
                    })
                    .draw(Draw::Box(BoxDraw::fill(color(c.border), radii(0.0)))),
                );
            }
        }
        // The chevron column, 20 points wide.
        cx.begin(Node::new(Style {
            size: taffy::Size {
                width: Dimension::length(20.0),
                height: Dimension::length(20.0),
            },
            align_items: AlignItems::CENTER,
            justify_content: taffy::JustifyContent::CENTER,
            flex_shrink: 0.0,
            ..Style::default()
        }));
        if has_children {
            let glyph = if open {
                "chevronDown"
            } else if cx.rtl() {
                "chevronLeft"
            } else {
                "chevronRight"
            };
            if let Some(n) =
                crate::primitives::icon_node::<Msg>(glyph, 16.0, color(c.icon_secondary))
            {
                cx.leaf(n);
            }
        }
        cx.end();
        if let Some(sc) = item.start_content {
            cx.child(sc);
        }
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_size: taffy::Size {
                width: len_auto(0.0),
                height: LengthPercentageAuto::auto(),
            },
            ..Style::default()
        }));
        let mut l = text_node(
            item.label,
            type_style(t.font.label, FontFamily::Body),
            color(c.text_primary),
        );
        text_draw(&mut l).max_lines = Some(1);
        l.sem = None;
        cx.leaf(l);
        if let Some(d) = item.description {
            let mut n = text_node(
                d,
                type_style(t.font.supporting, FontFamily::Body),
                color(c.text_secondary),
            );
            text_draw(&mut n).max_lines = Some(1);
            n.sem = None;
            cx.leaf(n);
        }
        cx.end();
        if let Some(e) = item.end_content {
            cx.child(e);
        }
        cx.end();
        if open {
            cx.begin(
                Node::new(Style {
                    flex_direction: FlexDirection::Column,
                    ..Style::default()
                })
                .sem(Semantics::role(Role::Group)),
            );
            rows(
                cx,
                k,
                item.children,
                level + 1,
                Some(item.id.clone()),
                first_stop,
            );
            cx.end();
        }
    }
}

impl<Msg: Clone + 'static> Build<Msg> for TreeList<Msg> {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        if self.base.is_hidden {
            return;
        }
        let t = cx.tokens;
        let tree_id = self.base.id.clone().unwrap_or_else(|| "tree".into());
        cx.key(Some(&tree_id));
        let state = cx.local_handle::<TreeState>();
        let row_height = match self.density {
            ListDensity::Compact => t.size.element_sm,
            ListDensity::Balanced => t.size.element_md,
            ListDensity::Spacious => t.size.element_lg,
        };
        let group = tree_id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
        let k = Ctx {
            tree_id: tree_id.clone(),
            state,
            row_height,
            indent: t.space.s4,
            guides: self.variant == TreeListVariant::LineGuides,
            group,
        };
        cx.begin(Node::new(Style {
            flex_direction: FlexDirection::Column,
            ..Style::default()
        }));
        if let Some(h) = self.header {
            cx.begin(Node::new(Style {
                margin: taffy::Rect {
                    left: len_auto(0.0),
                    right: len_auto(0.0),
                    top: len_auto(0.0),
                    bottom: len_auto(t.space.s2),
                },
                ..Style::default()
            }));
            cx.child(h);
            cx.end();
        }
        let sem = match &self.label {
            Some(l) => Semantics::new(Role::Tree, l.clone()),
            None => Semantics::role(Role::Tree),
        };
        cx.begin(
            Node::new(Style {
                flex_direction: FlexDirection::Column,
                ..Style::default()
            })
            .sem(sem),
        );
        let mut first = true;
        rows(cx, &k, self.items, 1, None, &mut first);
        cx.end();
        cx.end();
    }
}
