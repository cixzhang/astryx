//! The `shell-nav` template: an editor frame. A menu bar of dropdown menus,
//! a search field that opens the command palette (also Ctrl/Cmd+K), a run
//! button and Share; a resizable side navigation with the file tree; and a
//! placeholder editor with tabs and code lines.

use astryx_core::authoring::{Build, BuildCx, KeyResponse, Node};
use astryx_core::{
    App, Command, Element, InputPhase, Key, KeyEvent, Mode, Space, Theme, ThemeChoice,
};
use astryx_spike_foundations::ComponentSize;
use astryx_spike_ui::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Palette(bool),
    Ran(Entry),
    Noop,
}

pub struct ShellNav {
    pub palette_open: bool,
    pub mode: Mode,
}

impl Default for ShellNav {
    fn default() -> Self {
        ShellNav {
            palette_open: false,
            mode: Mode::Light,
        }
    }
}

/// A menu: its label and groups of (item, shortcut).
type Menu = (&'static str, &'static [&'static [(&'static str, &'static str)]]);

const MENUS: &[Menu] = &[
    (
        "File",
        &[
            &[("New File", "mod+n"), ("New Window", "shift+mod+n")],
            &[
                ("Open...", "mod+o"),
                ("Save", "mod+s"),
                ("Save As...", "shift+mod+s"),
            ],
            &[("Close Editor", "mod+w")],
        ],
    ),
    (
        "Edit",
        &[
            &[("Undo", "mod+z"), ("Redo", "shift+mod+z")],
            &[("Cut", "mod+x"), ("Copy", "mod+c"), ("Paste", "mod+v")],
            &[("Find", "mod+f")],
        ],
    ),
    (
        "View",
        &[
            &[("Command Palette", "shift+mod+p")],
            &[("Explorer", "shift+mod+e"), ("Search", "shift+mod+f")],
            &[("Toggle Terminal", "ctrl+`"), ("Zen Mode", "mod+k")],
        ],
    ),
    (
        "Window",
        &[
            &[("Minimize", "mod+m"), ("Zoom", "")],
            &[("Next Tab", "ctrl+tab"), ("Previous Tab", "ctrl+shift+tab")],
            &[("Bring All to Front", "")],
        ],
    ),
    (
        "Help",
        &[&[
            ("Documentation", ""),
            ("Release Notes", ""),
            ("Report Issue", ""),
            ("About", ""),
        ]],
    ),
];

const CODE_LINES: [f32; 15] = [
    0.38, 0.62, 0.54, 0.0, 0.46, 0.70, 0.58, 0.34, 0.0, 0.50, 0.66, 0.42, 0.60, 0.28, 0.0,
];

const COMMANDS: [(&str, &str); 11] = [
    ("new-file", "New File"),
    ("open-file", "Open File…"),
    ("save-all", "Save All"),
    ("find-in-files", "Find in Files"),
    ("toggle-terminal", "Toggle Terminal"),
    ("go-to-symbol", "Go to Symbol…"),
    ("appshell", "AppShell.tsx"),
    ("topnav", "TopNav.tsx"),
    ("sidenav", "SideNav.tsx"),
    ("use-theme", "useTheme.ts"),
    ("theme", "theme.ts"),
];

fn file_tree() -> Vec<TreeListItem<Msg>> {
    let folder = |id: &str, kids: Vec<TreeListItem<Msg>>, open: bool| {
        let mut f = TreeListItem::new(id, id)
            .start_content(Icon::new("folder").size(IconSize::Xsm))
            .is_expanded(open);
        for k in kids {
            f = f.child(k);
        }
        f
    };
    let file = |id: &str, selected: bool| {
        TreeListItem::new(id, id)
            .start_content(Icon::new("document").size(IconSize::Xsm))
            .is_selected(selected)
    };
    vec![
        folder(
            "src",
            vec![
                folder(
                    "components",
                    vec![
                        file("AppShell.tsx", true),
                        file("TopNav.tsx", false),
                        file("SideNav.tsx", false),
                    ],
                    true,
                ),
                folder(
                    "hooks",
                    vec![file("useTheme.ts", false), file("useResizable.ts", false)],
                    true,
                ),
                file("index.tsx", false),
                file("App.tsx", false),
            ],
            true,
        ),
        folder(
            "public",
            vec![file("favicon.ico", false), file("robots.txt", false)],
            false,
        ),
        file("package.json", false),
        file("tsconfig.json", false),
        file("README.md", false),
    ]
}

fn menus() -> Vec<Element<Msg>> {
    MENUS
        .iter()
        .map(|(label, groups)| {
            let mut items = Vec::new();
            for (gi, group) in groups.iter().enumerate() {
                if gi > 0 {
                    items.push(MenuOption::Divider);
                }
                for (name, keys) in group.iter() {
                    let mut it = MenuOption::item(*name).on_click(Msg::Noop);
                    if !keys.is_empty() {
                        it = it.end_content(Kbd::new(keys));
                    }
                    items.push(it);
                }
            }
            DropdownMenu::new(format!("menu-{label}"), items)
                .label(*label)
                .variant(ButtonVariant::Ghost)
                .size(ComponentSize::Sm)
                .has_chevron(false)
                .menu_width(280.0)
                .into()
        })
        .collect()
}

fn muted_block(w: SizeValue, h: f32) -> Element<Msg> {
    Card::new()
        .variant(CardVariant::Muted)
        .padding(Space::S0)
        .width(w)
        .height(SizeValue::Points(h))
        .into()
}

fn editor() -> Element<Msg> {
    let mut tabs = HStack::new().gap(Space::S2);
    for _ in 0..3 {
        tabs = tabs.child(muted_block(SizeValue::Points(132.0), 36.0));
    }
    let mut lines = VStack::new().gap(Space::S2);
    for w in CODE_LINES {
        if w == 0.0 {
            lines = lines.child(muted_block(SizeValue::Points(1.0), 14.0));
        } else {
            lines = lines.child(
                HStack::new()
                    .gap(Space::S3)
                    .v_align(StackAxisAlign::Center)
                    .child(muted_block(SizeValue::Points(20.0), 14.0))
                    .child(muted_block(SizeValue::Percent(w * 100.0), 14.0)),
            );
        }
    }
    Layout::new()
        .height(LayoutHeight::Fill)
        .header(
            LayoutHeader::new()
                .has_divider(true)
                .padding(Space::S6)
                .child(tabs),
        )
        .content(LayoutContent::new().padding(Space::S6).child(lines))
        .into()
}

/// Opens the palette on Ctrl/Cmd+K from anywhere in the page.
struct Shortcuts {
    child: Element<Msg>,
}

impl Build<Msg> for Shortcuts {
    fn build(self: Box<Self>, cx: &mut BuildCx<Msg>) {
        let mut n = Node::new(taffy::Style {
            flex_direction: taffy::FlexDirection::Column,
            flex_grow: 1.0,
            ..taffy::Style::default()
        });
        n.on_key = Some(Rc::new(|e: &KeyEvent| {
            let k = matches!(&e.key, Key::Character(c) if c.eq_ignore_ascii_case("k"));
            if e.phase == InputPhase::Pressed && k && (e.modifiers.ctrl || e.modifiers.meta) {
                KeyResponse::Send(Msg::Palette(true))
            } else {
                KeyResponse::Pass
            }
        }));
        cx.begin(n);
        cx.child(self.child);
        cx.end();
    }
}

impl App for ShellNav {
    type Msg = Msg;

    fn view(&self) -> Element<Msg> {
        let mut top = TopNav::new().label("Astryx Studio menu bar");
        for m in menus() {
            top = top.start_content(m);
        }
        top = top
            .end_content(
                Button::new("Search files and commands…")
                    .variant(ButtonVariant::Secondary)
                    .size(ComponentSize::Sm)
                    .icon(Icon::new("search"))
                    .width(SizeValue::Points(240.0))
                    .on_click(Msg::Palette(true)),
            )
            .end_content(
                Button::new("Run project")
                    .variant(ButtonVariant::Ghost)
                    .icon(Icon::new("play"))
                    .is_icon_only(true)
                    .tooltip("Run"),
            )
            .end_content(Button::new("Share").variant(ButtonVariant::Secondary));
        let shell = AppShell::new()
            .top_nav(top)
            .side_nav(|| {
                SideNav::new()
                    .width(240.0)
                    .resizable(180.0, 400.0)
                    .child(
                        TreeList::new(file_tree())
                            .density(ListDensity::Compact)
                            .label("Files"),
                    )
                    .into()
            })
            .child(editor());
        let source = static_source(COMMANDS.iter().map(|(id, l)| Entry::new(*id, *l)).collect());
        let palette = CommandPalette::new("palette", self.palette_open, Msg::Palette, source)
            .label("Search files and commands")
            .on_value_change(Msg::Ran);
        astryx_core::authoring::element(Shortcuts {
            child: VStack::new().child(shell).child(palette).into(),
        })
    }

    fn update(&mut self, msg: Msg) -> Command {
        match msg {
            Msg::Palette(open) => self.palette_open = open,
            Msg::Ran(_) => self.palette_open = false,
            Msg::Noop => {}
        }
        Command::None
    }

    fn theme(&self) -> ThemeChoice {
        ThemeChoice {
            theme: Theme::NEUTRAL,
            mode: Some(self.mode),
        }
    }
}
