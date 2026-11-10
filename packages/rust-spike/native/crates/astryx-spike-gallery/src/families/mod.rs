//! The component families the gallery shows, with their cases.

mod wave1;
mod wave2;
mod wave5;

use crate::props::Props;
use crate::Msg;
use astryx_core::Element;
use serde_json::{json, Map, Value};

include!(concat!(env!("OUT_DIR"), "/qa_cases.rs"));

/// Renders one case's props.
pub type Render = fn(Props) -> Element<Msg>;

/// One family: how to render a case's props, and the default sweep when the
/// QA lane has no case file for it.
pub struct Family {
    /// The Astryx family name, also the QA case file stem (`Button`).
    pub name: &'static str,
    /// Props every generated case starts from.
    pub base: fn() -> Value,
    /// Single-prop sweeps: each value of each prop over the base.
    pub sweeps: fn() -> Vec<(&'static str, Vec<Value>)>,
    pub render: Render,
}

/// One case: a stable id, its family, and its props.
#[derive(Clone, Debug)]
pub struct Case {
    pub id: String,
    pub family: &'static str,
    pub props: Value,
}

pub(crate) fn v(values: &[&str]) -> Vec<Value> {
    values.iter().map(|s| json!(s)).collect()
}

/// Every family, in wave order.
pub fn all() -> Vec<Family> {
    let mut out = wave1::families();
    out.extend(wave2::families());
    out.extend(wave5::families());
    out
}

fn prefix(name: &str) -> String {
    let mut s = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            s.push('-');
        }
        s.push(ch.to_ascii_lowercase());
    }
    s
}

/// The cases for a family: the QA lane's when it has a file, else the base
/// plus every single-prop sweep.
pub fn cases(f: &Family) -> Vec<Case> {
    if let Some((_, text)) = QA_CASES.iter().find(|(name, _)| *name == f.name) {
        if let Ok(doc) = serde_json::from_str::<Value>(text) {
            let qa = qa_cases(f.name, &doc);
            if !qa.is_empty() {
                return qa;
            }
        }
    }
    let base = (f.base)();
    let mut out = vec![Case {
        id: format!("{}-0001", prefix(f.name)),
        family: f.name,
        props: base.clone(),
    }];
    for (prop, values) in (f.sweeps)() {
        for value in values {
            let mut props: Map<String, Value> = base.as_object().cloned().unwrap_or_default();
            props.insert(prop.to_string(), value);
            out.push(Case {
                id: format!("{}-{:04}", prefix(f.name), out.len() + 1),
                family: f.name,
                props: Value::Object(props),
            });
        }
    }
    out
}

/// Reads a QA case file: `{components: [{component, cases: [{id, props}]}]}`,
/// or a bare `[{id, props}]` list. Each case's props gain `$component`, the
/// part the case is for.
fn qa_cases(family: &'static str, doc: &Value) -> Vec<Case> {
    let mut out = Vec::new();
    let mut push = |component: &str, items: &Value| {
        for item in items.as_array().into_iter().flatten() {
            let Some(id) = item.get("id").and_then(Value::as_str) else {
                continue;
            };
            let mut props = item
                .get("props")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            props.insert("$component".into(), json!(component));
            out.push(Case {
                id: id.to_string(),
                family,
                props: Value::Object(props),
            });
        }
    };
    match doc {
        Value::Array(_) => push(family, doc),
        Value::Object(o) => {
            for c in o
                .get("components")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let name = c.get("component").and_then(Value::as_str).unwrap_or(family);
                if let Some(cases) = c.get("cases") {
                    push(name, cases);
                }
            }
        }
        _ => {}
    }
    out
}

/// Finds a case by id across every family.
pub fn find_case(id: &str) -> Option<(Case, Render)> {
    for f in all() {
        if let Some(c) = cases(&f).into_iter().find(|c| c.id == id) {
            return Some((c, f.render));
        }
    }
    None
}
