//! The component catalog: every listed component builds from defaults, and
//! props outside the allowlist are refused.

use astryx_core::testing::{draw, tap};
use astryx_core::*;
use astryx_gallery::catalog::{self, CatalogError, DemoEvent, DemoState, PropKind};
use serde_json::{json, Map, Value};

fn props(v: Value) -> Map<String, Value> {
    v.as_object().cloned().expect("an object")
}

/// An app showing one catalog demo and keeping its state.
struct Demo {
    name: &'static str,
    variant: Option<&'static str>,
    props: Map<String, Value>,
    state: DemoState,
}

impl App for Demo {
    type Msg = DemoEvent;
    fn view(&self) -> Element<DemoEvent> {
        catalog::build(self.name, self.variant, &self.props, &self.state, |e| e)
            .expect("valid props")
    }
    fn update(&mut self, event: DemoEvent) -> Command {
        self.state.apply(&event);
        Command::None
    }
}

fn demo(name: &'static str, variant: Option<&'static str>, p: Value) -> Runtime<Demo> {
    Runtime::new(Demo {
        name,
        variant,
        props: props(p),
        state: DemoState::default(),
    })
}

#[test]
fn every_component_and_variant_builds_from_defaults() {
    for c in catalog::list() {
        for &variant in c.variants {
            let mut rt = demo(c.name, Some(variant), json!({}));
            let list = draw(&mut rt, 400, 300);
            assert!(
                !list.commands.is_empty(),
                "{} {variant} drew nothing",
                c.name
            );
        }
        assert!(
            catalog::validate(c.name, None, &Map::new()).is_ok(),
            "{}",
            c.name
        );
    }
}

#[test]
fn props_outside_the_allowlist_are_refused() {
    let check = |name, p: Value| catalog::validate(name, None, &props(p));
    assert_eq!(
        catalog::validate("Carousel", None, &Map::new()),
        Err(CatalogError::UnknownComponent("Carousel".into()))
    );
    assert!(matches!(
        catalog::validate("Button", Some("huge"), &Map::new()),
        Err(CatalogError::UnknownVariant { .. })
    ));
    assert!(matches!(
        check("Button", json!({"onClick": "alert(1)"})),
        Err(CatalogError::UnknownProp { .. })
    ));
    assert!(matches!(
        check("Button", json!({"label": "x".repeat(61)})),
        Err(CatalogError::BadValue { prop: "label", .. })
    ));
    assert!(matches!(
        check("Button", json!({"label": "see https://example.com"})),
        Err(CatalogError::BadValue { .. })
    ));
    assert!(matches!(
        check("Button", json!({"label": "www.example.com"})),
        Err(CatalogError::BadValue { .. })
    ));
    assert!(matches!(
        check("Button", json!({"label": "bell\u{7}"})),
        Err(CatalogError::BadValue { .. })
    ));
    assert!(matches!(
        check("Button", json!({"size": "xxl"})),
        Err(CatalogError::BadValue { prop: "size", .. })
    ));
    assert!(matches!(
        check("Button", json!({"is_loading": "yes"})),
        Err(CatalogError::BadValue { .. })
    ));
    assert!(matches!(
        check("ProgressBar", json!({"value": 101})),
        Err(CatalogError::BadValue { prop: "value", .. })
    ));
    assert!(matches!(
        check(
            "Selector",
            json!({"options": ["a", "b", "c", "d", "e", "f", "g"]})
        ),
        Err(CatalogError::BadValue {
            prop: "options",
            ..
        })
    ));
    assert!(matches!(
        catalog::validate_json("Badge", None, "[1, 2]"),
        Err(CatalogError::NotAnObject(_))
    ));
    assert!(matches!(
        catalog::validate_json("Badge", None, "{"),
        Err(CatalogError::NotAnObject(_))
    ));
    assert_eq!(
        catalog::validate_json("Badge", Some("success"), r#"{"label": "Shipped"}"#),
        Ok(())
    );
}

#[test]
fn props_shape_the_component() {
    let mut rt = demo(
        "Button",
        Some("destructive"),
        json!({"label": "Delete", "is_disabled": true}),
    );
    draw(&mut rt, 400, 300);
    let tree = rt.accessibility().unwrap();
    let button = tree
        .children
        .iter()
        .find(|n| n.role == Role::Button)
        .expect("a button");
    assert_eq!(
        (button.label.as_deref(), button.disabled),
        (Some("Delete"), true)
    );
}

#[test]
fn demos_keep_their_state() {
    let mut rt = demo("Switch", None, json!({"label": "Wi-Fi"}));
    draw(&mut rt, 400, 300);
    tap(&mut rt, Input::A);
    draw(&mut rt, 400, 300);
    assert_eq!(rt.app().state.toggled, Some(true));
    let tree = rt.accessibility().unwrap();
    assert!(tree
        .children
        .iter()
        .any(|n| n.role == Role::Switch && n.checked == Some(true)));

    let mut rt = demo("Selector", None, json!({"options": ["Tea", "Coffee"]}));
    draw(&mut rt, 400, 300);
    tap(&mut rt, Input::A);
    draw(&mut rt, 400, 300);
    tap(&mut rt, Input::Down);
    tap(&mut rt, Input::A);
    draw(&mut rt, 400, 300);
    assert_eq!(rt.app().state.selected.as_deref(), Some("Coffee"));

    let mut rt = demo("Button", None, json!({}));
    draw(&mut rt, 400, 300);
    tap(&mut rt, Input::A);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().state.presses, 2);
}

#[test]
fn the_spec_json_lists_every_component_and_prop() {
    let spec: Value = serde_json::from_str(&catalog::spec_json()).unwrap();
    let components = spec["components"].as_array().unwrap();
    assert_eq!(components.len(), catalog::list().len());
    for (json, c) in components.iter().zip(catalog::list()) {
        assert_eq!(json["name"], c.name);
        assert_eq!(json["props"].as_array().unwrap().len(), c.props.len());
    }
    let label = &components[0]["props"][0];
    assert_eq!(
        (
            label["name"].as_str(),
            label["kind"].as_str(),
            label["max"].as_u64()
        ),
        (Some("label"), Some("text"), Some(60))
    );
    assert!(catalog::list()
        .iter()
        .flat_map(|c| c.props)
        .all(|p| !matches!(p.kind, PropKind::Text { max } if max > 60)));
}

#[test]
fn lab_demos_confirm_and_describe_themselves() {
    let mut rt = demo(
        "HoldButton",
        Some("destructive"),
        json!({"label": "Hold to delete", "duration_ms": 500}),
    );
    let frame = FrameInfo {
        width: 400,
        height: 300,
        scale: 1.0,
        dt: 0.0,
    };
    let m = astryx_core::testing::FixedMeasure::default();
    rt.frame(&frame, &m);
    rt.input(Input::A, InputPhase::Pressed);
    rt.frame(&frame, &m);
    rt.frame(&FrameInfo { dt: 0.6, ..frame }, &m);
    assert_eq!(rt.app().state.presses, 1, "held past 500 ms");
    rt.input(Input::A, InputPhase::Released);

    let mut rt = demo(
        "StatusBar",
        None,
        json!({"wifi": "on", "battery": 15, "hour": 9, "minute": 5}),
    );
    draw(&mut rt, 400, 100);
    let tree = rt.accessibility().unwrap();
    assert!(tree
        .children
        .iter()
        .any(|n| n.label.as_deref() == Some("Wi-Fi on, not connected, battery 15%, 9:05 AM")));

    let mut rt = demo(
        "ChoiceTile",
        None,
        json!({"title": "Pro", "is_recommended": true}),
    );
    draw(&mut rt, 400, 300);
    tap(&mut rt, Input::A);
    assert_eq!(rt.app().state.toggled, Some(true));
    assert!(
        catalog::validate(
            "Image",
            None,
            &props(json!({"image": "https://example.com/a.png"}))
        )
        .is_err(),
        "images are named samples, not links"
    );
}
