//! The DOM accessibility mirror.
//!
//! Each frame's [`A11yNode`] tree becomes a tree of real, transparent HTML
//! elements placed over the canvas at the nodes' bounds. Screen readers,
//! switch access, and magnifiers read these elements; the canvas stays
//! hidden from them.

use std::collections::{HashMap, HashSet};

use astryx_core::{A11yNode, Live, Rect, Role};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, Element, FocusOptions, HtmlElement, HtmlInputElement};

use crate::focus::{focus_order, focused_id};

/// The data attribute that links a mirror element to its node id.
pub(crate) const ID_ATTR: &str = "data-astryx-id";
const ROOT_CLASS: &str = "astryx-a11y";
const STYLE_ID: &str = "astryx-a11y-style";

// The mirror stays transparent in forced-colors mode too, so the canvas
// remains the only visible layer. Pointer input goes through to the canvas.
const STYLE: &str = "\
.astryx-a11y{position:absolute;overflow:hidden;pointer-events:none;margin:0;padding:0;border:0;outline:none}\
.astryx-a11y,.astryx-a11y *{forced-color-adjust:none;color:transparent;background:transparent}\
.astryx-a11y *{position:absolute;box-sizing:border-box;margin:0;padding:0;border:0;\
font:14px/1 sans-serif;overflow:hidden;white-space:nowrap;outline:none;caret-color:transparent;\
appearance:none;-webkit-appearance:none;list-style:none}\
.astryx-a11y *::selection{background:transparent}";

/// The element shape and attributes one node maps to.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NodeDesc {
    pub tag: &'static str,
    pub attrs: Vec<(&'static str, String)>,
    /// Text content, for leaves whose name comes from their text.
    pub text: Option<String>,
    /// The value of an `<input>`.
    pub input_value: Option<String>,
    /// The `<input>` selection in UTF-16 offsets: start, end, backward.
    pub selection: Option<(u32, u32, bool)>,
}

/// Converts an `(anchor, focus)` pair of UTF-8 byte offsets into `value` to
/// the UTF-16 start, end, and direction that `setSelectionRange` takes.
/// Offsets past the end or inside a character move back to a boundary.
pub(crate) fn utf16_selection(value: &str, anchor: u32, focus: u32) -> (u32, u32, bool) {
    let to_utf16 = |byte: u32| {
        let mut byte = (byte as usize).min(value.len());
        while !value.is_char_boundary(byte) {
            byte -= 1;
        }
        value[..byte].encode_utf16().count() as u32
    };
    let (a, f) = (to_utf16(anchor), to_utf16(focus));
    (a.min(f), a.max(f), f < a)
}

fn heading_tag(level: u8) -> &'static str {
    match level {
        0 | 1 => "h1",
        2 => "h2",
        3 => "h3",
        4 => "h4",
        5 => "h5",
        _ => "h6",
    }
}

fn join(label: &Option<String>, value: &Option<String>) -> Option<String> {
    match (label, value) {
        (Some(l), Some(v)) => Some(format!("{l} {v}")),
        (Some(l), None) => Some(l.clone()),
        (None, Some(v)) => Some(v.clone()),
        (None, None) => None,
    }
}

/// Parses a progress value such as "40", "40%", or "0.4" into 0..=100.
fn percent(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    let (number, is_percent) = match trimmed.strip_suffix('%') {
        Some(n) => (n.trim(), true),
        None => (trimmed, false),
    };
    let n: f32 = number.parse().ok()?;
    let n = if !is_percent && (0.0..=1.0).contains(&n) && number.contains('.') {
        n * 100.0
    } else {
        n
    };
    n.is_finite().then_some(n.clamp(0.0, 100.0))
}

/// Maps one node to its element. `tab_stop` is `Some(true)` for the single
/// element in the page tab order, `Some(false)` for other focusable nodes,
/// and `None` for nodes that cannot take focus.
pub(crate) fn describe(node: &A11yNode, tab_stop: Option<bool>) -> NodeDesc {
    let leaf = node.children.is_empty();
    let mut d = NodeDesc::default();
    let mut role = None;
    let mut name_from_text = false;
    match node.role {
        Role::Group => {
            d.tag = "div";
            role = Some("group");
        }
        Role::Separator => {
            d.tag = "div";
            role = Some("separator");
        }
        Role::ListBox => {
            d.tag = "div";
            role = Some("listbox");
        }
        Role::ComboBox | Role::ListBoxOption => {
            d.tag = "div";
            role = Some(if node.role == Role::ComboBox {
                "combobox"
            } else {
                "option"
            });
            if let Some(selected) = node.selected {
                d.attrs.push(("aria-selected", selected.to_string()));
            }
            d.text = join(&node.label, &node.value).filter(|_| leaf);
            name_from_text = true;
        }
        Role::Text => {
            d.tag = "div";
            d.text = join(&node.label, &node.value).filter(|_| leaf);
            name_from_text = true;
        }
        Role::Heading(level) => {
            d.tag = heading_tag(level);
            d.text = join(&node.label, &node.value).filter(|_| leaf);
            name_from_text = true;
        }
        Role::Button | Role::Switch | Role::Checkbox => {
            d.tag = "button";
            d.attrs.push(("type", "button".into()));
            match node.role {
                Role::Switch => role = Some("switch"),
                Role::Checkbox => role = Some("checkbox"),
                _ => {}
            }
            if let Some(checked) = node.checked {
                let attr = if node.role == Role::Button {
                    "aria-pressed"
                } else {
                    "aria-checked"
                };
                d.attrs.push((attr, checked.to_string()));
            } else if node.role != Role::Button {
                d.attrs.push(("aria-checked", "false".into()));
            }
            if let Some(value) = &node.value {
                d.attrs.push(("aria-description", value.clone()));
            }
            d.text = node.label.clone().filter(|_| leaf);
            name_from_text = true;
        }
        Role::ProgressBar => {
            d.tag = "div";
            role = Some("progressbar");
            d.attrs.push(("aria-valuemin", "0".into()));
            d.attrs.push(("aria-valuemax", "100".into()));
            if let Some(value) = &node.value {
                match percent(value) {
                    Some(n) => d.attrs.push(("aria-valuenow", format!("{n}"))),
                    None => d.attrs.push(("aria-valuetext", value.clone())),
                }
            }
        }
        Role::TextInput => {
            d.tag = "input";
            let kind = if node.secure { "password" } else { "text" };
            d.attrs.push(("type", kind.into()));
            // The app draws and owns the text; browser helpers would edit
            // the hidden field behind its back.
            d.attrs.push(("autocomplete", "off".into()));
            d.attrs.push(("autocapitalize", "off".into()));
            d.attrs.push(("spellcheck", "false".into()));
            if node.disabled {
                d.attrs.push(("readonly", String::new()));
            }
            let value = node.value.clone().unwrap_or_default();
            d.selection = node
                .text_selection
                .map(|(anchor, focus)| utf16_selection(&value, anchor, focus));
            d.input_value = Some(value);
        }
        Role::List => {
            d.tag = "div";
            role = Some("list");
        }
        Role::ListItem => {
            d.tag = "div";
            role = Some("listitem");
            d.text = join(&node.label, &node.value).filter(|_| leaf);
            name_from_text = true;
        }
        Role::Image => {
            d.tag = "div";
            role = Some("img");
        }
    }
    if let Some(role) = role {
        d.attrs.push(("role", role.into()));
    }
    if let Some(label) = &node.label {
        if !(name_from_text && leaf) {
            d.attrs.push(("aria-label", label.clone()));
        }
    }
    if let Some(description) = &node.description {
        match d
            .attrs
            .iter_mut()
            .find(|(name, _)| *name == "aria-description")
        {
            Some((_, existing)) => *existing = format!("{description} {existing}"),
            None => d.attrs.push(("aria-description", description.clone())),
        }
    }
    if node.disabled {
        d.attrs.push(("aria-disabled", "true".into()));
    }
    if let Some(live) = node.live {
        let politeness = match live {
            Live::Polite => "polite",
            Live::Assertive => "assertive",
        };
        d.attrs.push(("aria-live", politeness.into()));
        // Read the whole text on a change, not only the changed part.
        d.attrs.push(("aria-atomic", "true".into()));
    }
    match tab_stop {
        Some(true) => d.attrs.push(("tabindex", "0".into())),
        Some(false) => d.attrs.push(("tabindex", "-1".into())),
        None => {}
    }
    d.attrs.sort_by_key(|(name, _)| *name);
    d
}

/// Position style for a node relative to its parent's origin.
fn geometry(bounds: &Rect, origin: (f32, f32)) -> String {
    format!(
        "left:{:.2}px;top:{:.2}px;width:{:.2}px;height:{:.2}px",
        bounds.x - origin.0,
        bounds.y - origin.1,
        bounds.width.max(0.0),
        bounds.height.max(0.0)
    )
}

struct Entry {
    el: HtmlElement,
    desc: NodeDesc,
    style: String,
}

pub(crate) struct Mirror {
    document: Document,
    root: HtmlElement,
    entries: HashMap<u64, Entry>,
    last: Option<A11yNode>,
    /// The element that holds the page tab stop.
    tab_stop: Option<u64>,
    label: Option<String>,
    /// True while an input method composes text in a mirror field.
    composing: bool,
}

impl Mirror {
    /// Creates the mirror root right after `canvas`, as its sibling.
    pub(crate) fn new(
        document: &Document,
        canvas: &HtmlElement,
        label: Option<String>,
    ) -> Result<Self, JsValue> {
        if document.get_element_by_id(STYLE_ID).is_none() {
            let style = document.create_element("style")?;
            style.set_id(STYLE_ID);
            style.set_text_content(Some(STYLE));
            if let Some(head) = document.head() {
                head.append_child(&style)?;
            }
        }
        let root: HtmlElement = document.create_element("div")?.unchecked_into();
        root.set_class_name(ROOT_CLASS);
        canvas.after_with_node_1(&root)?;
        Ok(Self {
            document: document.clone(),
            root,
            entries: HashMap::new(),
            last: None,
            tab_stop: None,
            label,
            composing: false,
        })
    }

    pub(crate) fn root(&self) -> &HtmlElement {
        &self.root
    }

    /// Places the mirror root over the canvas, in CSS pixels.
    pub(crate) fn place(&self, left: f64, top: f64, width: f64, height: f64) {
        let style =
            format!("left:{left:.2}px;top:{top:.2}px;width:{width:.2}px;height:{height:.2}px");
        let _ = self.root.set_attribute("style", &style);
    }

    /// True when keyboard focus is inside the app.
    pub(crate) fn has_dom_focus(&self) -> bool {
        match self.document.active_element() {
            Some(active) => self.root.contains(Some(&active)),
            None => false,
        }
    }

    /// Moves DOM focus to the element that holds the tab stop.
    pub(crate) fn focus_tab_stop(&self) {
        let target = match self.tab_stop.and_then(|id| self.entries.get(&id)) {
            Some(entry) => entry.el.clone(),
            None => self.root.clone(),
        };
        let target_el: &Element = target.as_ref();
        let already = self
            .document
            .active_element()
            .is_some_and(|a| a == *target_el);
        if !already {
            let options = FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = target.focus_with_options(&options);
        }
    }

    /// The node id of a mirror element or its nearest mirrored ancestor.
    pub(crate) fn node_id_of(&self, target: &JsValue) -> Option<u64> {
        let el: &Element = target.dyn_ref()?;
        if !self.root.contains(Some(el)) {
            return None;
        }
        let owner = el.closest(&format!("[{ID_ATTR}]")).ok()??;
        owner.get_attribute(ID_ATTR)?.parse().ok()
    }

    /// Updates the DOM to match `tree`. Returns true when anything changed.
    pub(crate) fn sync(&mut self, tree: Option<&A11yNode>) -> bool {
        if tree == self.last.as_ref() {
            self.restore_inputs();
            return false;
        }
        let mut live = HashSet::new();
        match tree {
            Some(tree) => {
                let order = focus_order(tree);
                self.tab_stop = focused_id(tree).or_else(|| order.first().copied());
                let root = self.root.clone();
                self.place_children(&root, std::slice::from_ref(tree), (0.0, 0.0), &mut live);
                self.set_root_fallback(order.is_empty());
            }
            None => {
                self.tab_stop = None;
                let root = self.root.clone();
                self.place_children(&root, &[], (0.0, 0.0), &mut live);
                self.set_root_fallback(true);
            }
        }
        self.entries.retain(|id, entry| {
            let keep = live.contains(id);
            if !keep {
                entry.el.remove();
            }
            keep
        });
        self.last = tree.cloned();
        self.restore_inputs();
        true
    }

    /// Writes the app's values back into mirror fields. The app owns the
    /// text, so typing reaches it as input and comes back here. An input
    /// method's uncommitted text stays until it commits.
    fn restore_inputs(&self) {
        if self.composing {
            return;
        }
        let active = self.document.active_element();
        for entry in self.entries.values() {
            let (Some(value), Some(input)) = (
                &entry.desc.input_value,
                entry.el.dyn_ref::<HtmlInputElement>(),
            ) else {
                continue;
            };
            let changed = input.value() != *value;
            if changed {
                input.set_value(value);
            }
            // The caret matters only in the focused field: screen readers echo
            // from it and input methods place their window at it.
            if active.as_ref() != Some(input.as_ref()) {
                continue;
            }
            let end = value.encode_utf16().count() as u32;
            let (start, stop, backward) = match entry.desc.selection {
                Some(selection) => selection,
                None if changed => (end, end, false),
                None => continue,
            };
            let current = (
                input.selection_start().ok().flatten(),
                input.selection_end().ok().flatten(),
            );
            if changed || current != (Some(start), Some(stop)) {
                let direction = if backward { "backward" } else { "forward" };
                let _ = input.set_selection_range_with_direction(start, stop, direction);
            }
        }
    }

    pub(crate) fn set_composing(&mut self, composing: bool) {
        self.composing = composing;
        if !composing {
            self.restore_inputs();
        }
    }

    /// The mirror field `target` is, when it takes typing.
    pub(crate) fn is_text_field(&self, target: Option<&web_sys::EventTarget>) -> bool {
        target
            .and_then(|t| t.dyn_ref::<HtmlInputElement>())
            .is_some_and(|input| !input.read_only() && self.root.contains(Some(input)))
    }

    /// Without focusable nodes, the root takes the tab stop so keys still
    /// reach the app.
    fn set_root_fallback(&self, fallback: bool) {
        let root = &self.root;
        if fallback {
            let _ = root.set_attribute("tabindex", "0");
            let _ = root.set_attribute("role", "application");
            if let Some(label) = &self.label {
                let _ = root.set_attribute("aria-label", label);
            }
        } else {
            let _ = root.remove_attribute("tabindex");
            let _ = root.remove_attribute("role");
            let _ = root.remove_attribute("aria-label");
        }
    }

    fn place_children(
        &mut self,
        parent: &Element,
        nodes: &[A11yNode],
        origin: (f32, f32),
        live: &mut HashSet<u64>,
    ) {
        let mut wanted: Vec<HtmlElement> = Vec::with_capacity(nodes.len());
        for node in nodes {
            let el = self.place_node(node, origin, live);
            wanted.push(el);
        }
        let current = parent.children();
        for (index, el) in wanted.iter().enumerate() {
            let at = current.item(index as u32);
            if at.as_ref() != Some(el.as_ref()) {
                let _ = parent.insert_before(el, at.as_ref().map(|e| e.as_ref()));
            }
        }
        // Children past the wanted ones are stale. `retain` in `sync` removes
        // stale entries; this removes anything else left in the parent.
        while parent.children().length() > wanted.len() as u32 {
            match parent.last_element_child() {
                Some(extra)
                    if !wanted.iter().any(|w| {
                        let e: &Element = w.as_ref();
                        *e == extra
                    }) =>
                {
                    extra.remove()
                }
                _ => break,
            }
        }
    }

    fn place_node(
        &mut self,
        node: &A11yNode,
        origin: (f32, f32),
        live: &mut HashSet<u64>,
    ) -> HtmlElement {
        live.insert(node.id);
        let tab_stop = node.focusable.then(|| self.tab_stop == Some(node.id));
        let desc = describe(node, tab_stop);
        let style = geometry(&node.bounds, origin);
        let reuse = self
            .entries
            .get(&node.id)
            .filter(|e| e.desc.tag == desc.tag)
            .map(|e| e.el.clone());
        let el = match reuse {
            Some(el) => el,
            None => {
                let el: HtmlElement = self
                    .document
                    .create_element(desc.tag)
                    .expect("create element")
                    .unchecked_into();
                let _ = el.set_attribute(ID_ATTR, &node.id.to_string());
                if let Some(old) = self.entries.remove(&node.id) {
                    old.el.remove();
                }
                self.entries.insert(
                    node.id,
                    Entry {
                        el: el.clone(),
                        desc: NodeDesc::default(),
                        style: String::new(),
                    },
                );
                el
            }
        };
        let entry = self.entries.get_mut(&node.id).expect("entry exists");
        if entry.style != style {
            let _ = el.set_attribute("style", &style);
            entry.style = style;
        }
        if entry.desc != desc {
            for (name, _) in &entry.desc.attrs {
                if !desc.attrs.iter().any(|(n, _)| n == name) {
                    let _ = el.remove_attribute(name);
                }
            }
            for (name, value) in &desc.attrs {
                if !entry
                    .desc
                    .attrs
                    .iter()
                    .any(|(n, v)| n == name && v == value)
                {
                    let _ = el.set_attribute(name, value);
                }
            }
            if entry.desc.text != desc.text {
                // Clearing text also drops child elements; `place_children`
                // re-inserts them right after.
                el.set_text_content(desc.text.as_deref());
            }
            entry.desc = desc;
        }
        let child_origin = (node.bounds.x, node.bounds.y);
        if !node.children.is_empty() {
            let parent: Element = el.clone().unchecked_into();
            self.place_children(&parent, &node.children, child_origin, live);
        }
        el
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(role: Role) -> A11yNode {
        A11yNode {
            text_selection: None,
            secure: false,
            description: None,
            selected: None,
            focusable: false,
            live: None,
            id: 7,
            role,
            label: Some("Wi-Fi".into()),
            value: None,
            bounds: Rect::new(10.0, 20.0, 100.0, 40.0),
            focused: false,
            disabled: false,
            checked: None,
            children: vec![],
        }
    }

    fn attr<'a>(d: &'a NodeDesc, name: &str) -> Option<&'a str> {
        d.attrs
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn switch_is_a_button_with_switch_role() {
        let mut n = node(Role::Switch);
        n.checked = Some(true);
        let d = describe(&n, Some(true));
        assert_eq!(d.tag, "button");
        assert_eq!(attr(&d, "role"), Some("switch"));
        assert_eq!(attr(&d, "aria-checked"), Some("true"));
        assert_eq!(attr(&d, "tabindex"), Some("0"));
        assert_eq!(d.text.as_deref(), Some("Wi-Fi"));
        assert_eq!(attr(&d, "aria-label"), None);
    }

    #[test]
    fn toggle_button_uses_pressed_and_disabled_stays_focusable() {
        let mut n = node(Role::Button);
        n.checked = Some(false);
        n.disabled = true;
        let d = describe(&n, Some(false));
        assert_eq!(attr(&d, "aria-pressed"), Some("false"));
        assert_eq!(attr(&d, "aria-disabled"), Some("true"));
        assert_eq!(attr(&d, "tabindex"), Some("-1"));
        assert_eq!(attr(&d, "disabled"), None);
    }

    #[test]
    fn headings_clamp_levels() {
        assert_eq!(describe(&node(Role::Heading(1)), None).tag, "h1");
        assert_eq!(describe(&node(Role::Heading(4)), None).tag, "h4");
        assert_eq!(describe(&node(Role::Heading(9)), None).tag, "h6");
    }

    #[test]
    fn progress_parses_numbers_and_falls_back_to_text() {
        let mut n = node(Role::ProgressBar);
        n.value = Some("40%".into());
        let d = describe(&n, None);
        assert_eq!(attr(&d, "aria-valuenow"), Some("40"));
        assert_eq!(attr(&d, "aria-label"), Some("Wi-Fi"));
        n.value = Some("0.25".into());
        assert_eq!(attr(&describe(&n, None), "aria-valuenow"), Some("25"));
        n.value = Some("Step 2 of 5".into());
        let d = describe(&n, None);
        assert_eq!(attr(&d, "aria-valuetext"), Some("Step 2 of 5"));
        assert_eq!(attr(&d, "aria-valuenow"), None);
    }

    #[test]
    fn containers_name_with_aria_label() {
        let mut n = node(Role::ListItem);
        n.children.push(node(Role::Text));
        let d = describe(&n, None);
        assert_eq!(d.text, None);
        assert_eq!(attr(&d, "aria-label"), Some("Wi-Fi"));
        assert_eq!(
            attr(&describe(&node(Role::Image), None), "role"),
            Some("img")
        );
        assert_eq!(
            attr(&describe(&node(Role::List), None), "role"),
            Some("list")
        );
    }

    #[test]
    fn text_input_is_editable_unless_disabled() {
        let mut n = node(Role::TextInput);
        n.value = Some("hello".into());
        let d = describe(&n, Some(true));
        assert_eq!(d.tag, "input");
        assert_eq!(attr(&d, "readonly"), None);
        assert_eq!(attr(&d, "autocomplete"), Some("off"));
        assert_eq!(d.input_value.as_deref(), Some("hello"));
        n.disabled = true;
        assert_eq!(attr(&describe(&n, Some(false)), "readonly"), Some(""));
    }

    #[test]
    fn secure_fields_are_password_inputs_with_selection() {
        let mut n = node(Role::TextInput);
        n.value = Some("h\u{e9}llo".into());
        n.secure = true;
        n.text_selection = Some((6, 1));
        let d = describe(&n, Some(true));
        assert_eq!(attr(&d, "type"), Some("password"));
        assert_eq!(d.selection, Some((1, 5, true)));
    }

    #[test]
    fn selection_converts_utf8_bytes_to_utf16() {
        // "a" is 1 byte, "é" 2 bytes, the emoji 4 bytes and 2 UTF-16 units.
        let value = "a\u{e9}\u{1F600}b";
        assert_eq!(utf16_selection(value, 0, 0), (0, 0, false));
        assert_eq!(utf16_selection(value, 3, 7), (2, 4, false));
        assert_eq!(utf16_selection(value, 8, 1), (1, 5, true));
        // Inside a character and past the end.
        assert_eq!(utf16_selection(value, 2, 99), (1, 5, false));
    }

    #[test]
    fn description_joins_a_button_value() {
        let mut n = node(Role::Button);
        n.description = Some("Saves your settings".into());
        assert_eq!(
            attr(&describe(&n, None), "aria-description"),
            Some("Saves your settings")
        );
        n.value = Some("Unsaved".into());
        assert_eq!(
            attr(&describe(&n, None), "aria-description"),
            Some("Saves your settings Unsaved")
        );
    }

    #[test]
    fn live_maps_to_aria_live() {
        let mut n = node(Role::Text);
        n.live = Some(Live::Polite);
        let d = describe(&n, None);
        assert_eq!(attr(&d, "aria-live"), Some("polite"));
        assert_eq!(attr(&d, "aria-atomic"), Some("true"));
        n.live = Some(Live::Assertive);
        assert_eq!(attr(&describe(&n, None), "aria-live"), Some("assertive"));
        assert_eq!(attr(&describe(&node(Role::Text), None), "aria-live"), None);
    }

    #[test]
    fn geometry_is_relative_to_the_parent() {
        let g = geometry(&Rect::new(30.0, 50.0, 10.0, 5.0), (10.0, 20.0));
        assert_eq!(g, "left:20.00px;top:30.00px;width:10.00px;height:5.00px");
    }
}
