//! Small helpers shared by the components.

use astryx_core::contract::{Color, Radii};
use taffy::{Dimension, LengthPercentage, LengthPercentageAuto};

pub(crate) fn len(points: f32) -> LengthPercentage {
    LengthPercentage::length(points)
}

pub(crate) fn len_auto(points: f32) -> LengthPercentageAuto {
    LengthPercentageAuto::length(points)
}

pub(crate) fn dim(points: f32) -> Dimension {
    Dimension::length(points)
}

pub(crate) fn square(side: f32) -> taffy::Size<Dimension> {
    taffy::Size {
        width: dim(side),
        height: dim(side),
    }
}

pub(crate) fn radii(r: f32) -> Radii {
    Radii::all(r)
}

pub(crate) fn rect<T: Copy>(v: T) -> taffy::Rect<T> {
    taffy::Rect {
        left: v,
        right: v,
        top: v,
        bottom: v,
    }
}

pub(crate) fn color(rgba: [f32; 4]) -> Color {
    rgba.into()
}

/// `color-mix(in srgb, a, b p)`: `b` mixed into `a` by `p` (0..=1).
pub(crate) fn mix(a: Color, b: Color, p: f32) -> Color {
    let q = 1.0 - p;
    Color::rgba(
        a.r * q + b.r * p,
        a.g * q + b.g * p,
        a.b * q + b.b * p,
        a.a * q + b.a * p,
    )
}

/// Adds the Astryx base props (`id`, `is_hidden`) and the conversion into an
/// [`Element`](astryx_core::Element) to a component builder that has a
/// `base: Base` field.
macro_rules! component {
    (@methods) => {
        /// Astryx base `id`: a stable identity for focus and accessibility.
        pub fn id(mut self, id: impl Into<String>) -> Self {
            self.base.id = Some(id.into());
            self
        }
        /// Astryx base `isHidden`: removes the element.
        pub fn is_hidden(mut self, is_hidden: bool) -> Self {
            self.base.is_hidden = is_hidden;
            self
        }
    };
    ($ty:ident<Msg>) => {
        impl<Msg> $ty<Msg> {
            $crate::util::component!(@methods);
        }
        impl<Msg: Clone + 'static> From<$ty<Msg>> for astryx_core::Element<Msg> {
            fn from(component: $ty<Msg>) -> Self {
                astryx_core::authoring::element(component)
            }
        }
    };
    ($ty:ident) => {
        impl $ty {
            $crate::util::component!(@methods);
        }
        impl<Msg: 'static> From<$ty> for astryx_core::Element<Msg> {
            fn from(component: $ty) -> Self {
                astryx_core::authoring::element(component)
            }
        }
    };
}
pub(crate) use component;

/// Words shown to people, keyed by Astryx's locale catalog keys. English
/// until the foundations catalog lands.
pub(crate) mod strings {
    /// `@astryx.spinner.loading`
    pub const LOADING: &str = "Loading";
    /// `@astryx.link.newTab`
    pub const NEW_TAB: &str = "(opens in new tab)";
}
