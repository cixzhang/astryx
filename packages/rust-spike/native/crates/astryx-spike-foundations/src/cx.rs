//! What components read while they build: locale and messages, size, and
//! extra tokens, through context on [`BuildCx`].

use crate::i18n::{Args, I18n, Locale};
use crate::size::{self, ComponentSize, SizeScope};
use crate::theme::ExtraTokens;
use astryx_core::authoring::BuildCx;
use astryx_core::{Direction, Element};
use std::rc::Rc;

/// Gives `element` a locale: its messages and formats, and the locale's
/// writing direction for layout, as Astryx's `InternationalizationProvider`
/// with `dir` on its root does.
pub fn provide_i18n<Msg: 'static>(element: Element<Msg>, i18n: I18n) -> Element<Msg> {
    let dir = i18n.dir();
    element.provide(Rc::new(i18n)).direction(dir)
}

/// Foundations reads on the build context.
pub trait CxExt {
    /// The nearest locale, or English when none is provided.
    fn i18n(&self) -> Rc<I18n>;
    fn locale(&self) -> Locale {
        self.i18n().locale().clone()
    }
    /// The writing direction of the region being built.
    fn dir(&self) -> Direction;
    /// The message for `key` in the nearest locale.
    fn t(&self, key: &str, args: &Args) -> String {
        self.i18n().t(key, args)
    }
    /// A participating component's size: `explicit`, else the nearest
    /// provider's, else `local_default`.
    fn size(&self, explicit: Option<ComponentSize>, local_default: ComponentSize) -> ComponentSize;
    /// Tokens beyond the generated set, for the current theme and mode.
    fn extra_tokens(&self) -> ExtraTokens;
}

thread_local! {
    static ENGLISH: Rc<I18n> = Rc::new(I18n::new("en"));
}

impl<Msg> CxExt for BuildCx<Msg> {
    fn i18n(&self) -> Rc<I18n> {
        self.context::<I18n>()
            .unwrap_or_else(|| ENGLISH.with(Rc::clone))
    }

    fn dir(&self) -> Direction {
        self.direction
    }

    fn size(&self, explicit: Option<ComponentSize>, local_default: ComponentSize) -> ComponentSize {
        size::resolve(
            explicit,
            self.context::<SizeScope>().map(|s| *s),
            local_default,
        )
    }

    fn extra_tokens(&self) -> ExtraTokens {
        ExtraTokens::of(self.tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args;
    use crate::size::provide_size;
    use astryx_core::authoring::{element, Build, Node};
    use std::cell::RefCell;

    /// A leaf that records what it read from the build context.
    struct Probe(Rc<RefCell<Vec<String>>>, Option<ComponentSize>);
    impl Build<()> for Probe {
        fn build(self: Box<Self>, cx: &mut BuildCx<()>) {
            let size = cx.size(self.1, ComponentSize::Md);
            let text = cx.t("@astryx.pagination.nextBy", &args! {"step" => 3});
            self.0
                .borrow_mut()
                .push(format!("{size:?} {:?} {}", cx.dir(), text));
            cx.leaf(Node::new(Default::default()));
        }
    }

    fn build(el: Element<()>) {
        let mut cx = BuildCx::new(&astryx_tokens::Theme::ASTRYX.light);
        cx.child(el);
    }

    #[test]
    fn context_reaches_nested_children_and_resets() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let probe = |s| element(Probe(log.clone(), s));
        let tree = astryx_core::VStack::new()
            .child(probe(None))
            .child(provide_size(
                astryx_core::VStack::new()
                    .child(probe(None))
                    .child(probe(Some(ComponentSize::Lg)))
                    .child(provide_size(probe(None), None))
                    .into(),
                Some(ComponentSize::Sm),
            ))
            .into();
        build(provide_i18n(tree, I18n::new("ar")));
        let log = log.borrow();
        assert!(log[0].starts_with("Md Rtl "), "{}", log[0]);
        assert!(log[1].starts_with("Sm Rtl"));
        assert!(log[2].starts_with("Lg Rtl"));
        assert!(log[3].starts_with("Md Rtl"), "None resets");
        assert!(!log[0].contains("Go forward"), "Arabic text: {}", log[0]);
    }

    #[test]
    fn english_without_a_provider() {
        let log = Rc::new(RefCell::new(Vec::new()));
        build(element(Probe(log.clone(), None)));
        assert_eq!(log.borrow()[0], "Md Ltr Go forward 3 pages");
    }
}
