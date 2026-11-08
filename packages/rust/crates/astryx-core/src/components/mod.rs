//! Astryx components, named and shaped after the native component set.
//!
//! Each component is a builder with Astryx's prop names in snake_case
//! (`isDisabled` is `is_disabled`) and converts into an [`Element`].

pub(crate) mod button;
pub(crate) mod card;
pub(crate) mod display;
pub(crate) mod field;
pub(crate) mod icon_paths;
pub(crate) mod input;
pub(crate) mod item;
pub(crate) mod kbd;
pub(crate) mod media;
pub(crate) mod stack;
pub(crate) mod switch;
pub(crate) mod text;

pub use button::{Button, ButtonSize, ButtonVariant, ButtonWidth};
pub use card::{Card, CardVariant, SelectableCard};
pub use display::{
    Badge, BadgeVariant, Divider, DividerOrientation, DividerVariant, Icon, IconColor, IconSize,
    ProgressBar, ProgressBarMark, ProgressBarVariant, BUILT_IN_ICONS,
};
pub use field::{Status, StatusType};
pub use input::{
    FieldSize, Selector, SelectorIndicatorPosition, SelectorOption, SelectorVariant, StatusVariant,
    TextInput, TextInputType,
};
pub use item::{Item, ItemAlign, ItemDensity, ItemLayout, ListItem};
pub use kbd::Kbd;
pub use media::{
    Avatar, AvatarShape, AvatarSize, AvatarStatus, AvatarStatusDot, AvatarStatusDotVariant,
    Thumbnail, ThumbnailShowRemoveOn,
};
pub use stack::{HStack, StackAlign, StackJustify, StackWrap, VStack};
pub use switch::{Switch, SwitchSize};
pub use text::{
    Heading, HeadingType, Text, TextColor, TextJustify, TextSize, TextType, TextWeight,
};

use crate::contract::Radii;
use crate::element::Element;
use astryx_tokens::{ShadowLayer, Tokens};
use taffy::{Dimension, LengthPercentage, LengthPercentageAuto};

/// Shadow depth, read from the theme's `shadow` tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Elevation {
    #[default]
    None,
    Low,
    Med,
    High,
}

impl Elevation {
    pub(crate) fn layers(self, tokens: &'static Tokens) -> &'static [ShadowLayer] {
        match self {
            Elevation::None => &[],
            Elevation::Low => tokens.shadow.low,
            Elevation::Med => tokens.shadow.med,
            Elevation::High => tokens.shadow.high,
        }
    }
}

/// Words shown to people, in English until the core gains a catalog. They
/// match Astryx's locale catalog keys.
pub(crate) mod strings {
    /// `@astryx.button.loading`
    pub const LOADING: &str = "Loading";
    /// `@astryx.field.optional`
    pub const OPTIONAL: &str = "Optional";
    /// `@astryx.field.required`
    pub const REQUIRED: &str = "Required";
    /// How a hold-to-confirm control is used.
    pub const HOLD_TO_CONFIRM: &str = "Hold to confirm";
}

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

pub(crate) fn into_elements<Msg, E: Into<Element<Msg>>>(
    items: impl IntoIterator<Item = E>,
) -> Vec<Element<Msg>> {
    items.into_iter().map(Into::into).collect()
}
