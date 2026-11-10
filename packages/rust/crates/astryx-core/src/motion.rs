//! Component motion on astryx-motion: animated paint channels, exit copies,
//! and size animation that reflows the page.
//!
//! A component states targets while it builds; the runtime keeps one
//! [`Animated`](astryx_motion::Animated) per node and channel across
//! rebuilds, so a new target retargets from the current value and velocity
//! instead of jumping. Reduce Motion goes through the runtime's
//! [`MotionPolicy`](astryx_motion::MotionPolicy): travel lands at once,
//! opacity may cross-fade.

pub use astryx_motion::{Change, Motion};

/// What an animation moves. Paint channels change only paint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    /// Multiplies the node's opacity.
    Opacity,
    /// Moves the node and its subtree, in points.
    TranslateX,
    TranslateY,
    /// Scales the node and its subtree about its center.
    Scale,
}

/// One animated channel of a node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anim {
    pub channel: Channel,
    /// The value to move to.
    pub to: f32,
    /// The value to start from when the node first appears (an entrance).
    /// Without one, the first build lands at `to`.
    pub from: Option<f32>,
    pub motion: Motion,
    /// What kind of change this is, for Reduce Motion.
    pub change: Change,
}

impl Anim {
    pub fn new(channel: Channel, to: f32, motion: Motion) -> Self {
        Anim {
            channel,
            to,
            from: None,
            motion,
            change: Change::Transition,
        }
    }

    /// Enters from `from` when the node first appears.
    pub fn from(mut self, from: f32) -> Self {
        self.from = Some(from);
        self
    }

    pub fn change(mut self, change: Change) -> Self {
        self.change = change;
        self
    }
}

/// How a node leaves: when it goes away, the runtime keeps a copy of how it
/// was last drawn, plays this on the copy, and removes it. The copy is
/// paint only: inert, and outside the accessibility tree. One copy per
/// node: a second exit of the same node replaces the first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exit {
    pub opacity: f32,
    pub translate: (f32, f32),
    pub scale: f32,
    pub motion: Motion,
    pub change: Change,
}

impl Exit {
    /// Fades out with `motion`.
    pub fn fade(motion: Motion) -> Self {
        Exit {
            opacity: 0.0,
            translate: (0.0, 0.0),
            scale: 1.0,
            motion,
            change: Change::Transition,
        }
    }
}

/// Animates a node's laid-out height when it changes, as Astryx's animated
/// layout does: content below reflows every frame. Clip the node so its
/// content does not paint past the animating box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resize {
    pub motion: Motion,
    pub change: Change,
}

impl Resize {
    pub fn new(motion: Motion) -> Self {
        Resize {
            motion,
            change: Change::Transition,
        }
    }
}
