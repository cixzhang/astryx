//! Wave 1: the leaf components everything else draws with.
//!
//! `Text`, `Heading`, `Badge`, `Divider`, `Avatar`,
//! `AvatarStatusDot` and `Kbd` come from `astryx-core`, which ships them at
//! Astryx parity. `Icon` (all 20 colors), `Spinner`, `Link`, `VisuallyHidden`, `AvatarGroup` and the
//! choice indicators are built here.

mod avatar_group;
mod icon;
mod indicators;
mod link;
mod spinner;
mod visually_hidden;

pub use astryx_core::{
    Avatar, AvatarShape, AvatarSize, AvatarStatus, AvatarStatusDot, AvatarStatusDotVariant, Badge,
    BadgeVariant, Divider, DividerOrientation, DividerVariant, Heading, HeadingType, Kbd, Text,
    TextColor, TextJustify, TextSize, TextType, TextWeight, BUILT_IN_ICONS,
};
pub use avatar_group::{AvatarGroup, AvatarGroupOverflow};
pub(crate) use icon::icon_node;
pub use icon::{glyph, Icon, IconColor, IconSize};
pub use indicators::{
    CheckIndicator, CheckboxIndicator, CheckboxState, IndicatorSize, RadioIndicator,
};
pub use link::{Link, LinkDisplay};
pub(crate) use spinner::spinner_painter;
pub use spinner::{Spinner, SpinnerShade, SpinnerSize};
pub use visually_hidden::VisuallyHidden;
