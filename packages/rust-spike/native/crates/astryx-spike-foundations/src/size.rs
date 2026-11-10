//! The component size cascade: a group or container (ButtonGroup,
//! InputGroup, Toolbar) gives its size to the controls inside it, as
//! Astryx's `SizeContext` does. An explicit size prop wins, then the
//! nearest provider, then the component's own default; a provider of
//! `None` resets the cascade for its subtree.

use astryx_core::Element;
use std::rc::Rc;

/// Astryx's standard size axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ComponentSize {
    Sm,
    #[default]
    Md,
    Lg,
}

/// What a size provider gives its subtree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeScope(pub Option<ComponentSize>);

/// `element` and everything inside it inherit `size`; `None` resets the
/// cascade.
pub fn provide_size<Msg: 'static>(
    element: Element<Msg>,
    size: Option<ComponentSize>,
) -> Element<Msg> {
    element.provide(Rc::new(SizeScope(size)))
}

/// Resolves a participating component's size: `explicit` wins, then the
/// nearest provider's size, then `local_default`.
pub fn resolve(
    explicit: Option<ComponentSize>,
    inherited: Option<SizeScope>,
    local_default: ComponentSize,
) -> ComponentSize {
    explicit
        .or(inherited.and_then(|s| s.0))
        .unwrap_or(local_default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_then_nearest_then_default() {
        let sm = Some(SizeScope(Some(ComponentSize::Sm)));
        assert_eq!(
            resolve(Some(ComponentSize::Lg), sm, ComponentSize::Md),
            ComponentSize::Lg
        );
        assert_eq!(resolve(None, sm, ComponentSize::Md), ComponentSize::Sm);
        assert_eq!(resolve(None, None, ComponentSize::Lg), ComponentSize::Lg);
        assert_eq!(
            resolve(None, Some(SizeScope(None)), ComponentSize::Md),
            ComponentSize::Md,
            "reset"
        );
    }
}
