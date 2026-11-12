//! The Lab section: experimental components Astryx does not define yet.

use crate::{group, images, row, Gallery, Msg};
use astryx_core::lab::{HoldButton, HoldCard, Image, SelectableCardGroup, StatusBar};
use astryx_core::*;

/// Sample device states for the status bar.
fn devices() -> [DeviceStatus; 3] {
    [
        DeviceStatus {
            wifi: Radio::Connected,
            wifi_name: Some("Garden".into()),
            bluetooth: Radio::Off,
            battery_percent: Some(82),
            charging: true,
            time: (19, 42),
            tether: Radio::Off,
        },
        DeviceStatus {
            wifi: Radio::On,
            bluetooth: Radio::Connected,
            battery_percent: Some(15),
            time: (9, 5),
            ..DeviceStatus::default()
        },
        DeviceStatus {
            time: (12, 0),
            ..DeviceStatus::default()
        },
    ]
}

/// A plan as a choice card: a title, a line about it, and a badge for
/// the recommended one.
fn plan(title: &'static str, body: &'static str, recommended: bool) -> SelectableCard<Msg> {
    let mut heading = HStack::new()
        .gap(Space::S2)
        .v_align(StackAlign::Center)
        .child(Heading::new(4, title));
    if recommended {
        heading = heading.child(Badge::new("Recommended").variant(BadgeVariant::Info));
    }
    SelectableCard::new(title, false).child(
        VStack::new()
            .gap(Space::S1)
            .child(heading)
            .child(Text::new(body).r#type(TextType::Supporting)),
    )
}

pub(crate) fn lab(gallery: &Gallery) -> Element<Msg> {
    let mut bars = VStack::new().gap(Space::S2);
    for device in devices() {
        bars = bars.child(
            Card::new()
                .padding(Space::S0)
                .child(StatusBar::new().device(device)),
        );
    }
    let plans = SelectableCardGroup::new("Plan")
        .value(gallery.plan.clone())
        .on_change(Msg::Plan)
        .is_horizontal(true)
        .option("free", plan("Free", "One garden, three plots", false))
        .option("pro", plan("Pro", "Every garden, unlimited plots", true))
        .option(
            "team",
            plan("Team", "Shared gardens for a household", false),
        );
    vstack![
        group("StatusBar", bars),
        group(
            "HoldButton: hold A to confirm",
            row()
                .child(
                    HoldButton::new("Hold to delete")
                        .variant(ButtonVariant::Destructive)
                        .on_confirm(Msg::Pressed("hold-delete")),
                )
                .child(
                    HoldButton::new("Hold to plant")
                        .duration_ms(1500)
                        .on_confirm(Msg::Pressed("hold-plant")),
                )
                .child(
                    HoldButton::new("Disabled")
                        .is_disabled(true)
                        .on_confirm(Msg::Pressed("hold-off"))
                ),
        ),
        group(
            "SelectableCardGroup: one choice, arrows move, A chooses",
            plans
        ),
        group(
            "HoldCard: hold A on the card to send",
            Card::new()
                .variant(CardVariant::Transparent)
                .padding(Space::S0)
                .width(260.0)
                .child(
                    HoldCard::new(
                        SelectableCard::new("Send the plan to Ada", gallery.cards[3])
                            .on_change(|on| Msg::Card(3, on))
                            .child(
                                vstack![
                                    Heading::new(4, "Send to Ada"),
                                    Text::new("Hold A to send").r#type(TextType::Supporting),
                                ]
                                .gap(Space::S1)
                            ),
                    )
                    .duration_ms(1000)
                )
        ),
        group(
            "Image: cover, contain, and pixel-perfect",
            row()
                .child(
                    Card::new()
                        .variant(CardVariant::Transparent)
                        .padding(Space::S0)
                        .width(240.0)
                        .child(
                            Image::new(ImageId::of(images::BEACH), "A beach at sunset")
                                .aspect_ratio(16.0 / 9.0)
                        ),
                )
                .child(
                    Card::new()
                        .variant(CardVariant::Transparent)
                        .padding(Space::S0)
                        .width(140.0)
                        .child(
                            Image::new(ImageId::of(images::STICKER), "A sticker")
                                .fit(ImageFit::Contain)
                                .aspect_ratio(1.0)
                        ),
                )
                .child(
                    Card::new()
                        .variant(CardVariant::Transparent)
                        .padding(Space::S0)
                        .width(140.0)
                        .child(
                            Image::new(ImageId::of("gallery/world/tree.png"), "A pixel tree")
                                .aspect_ratio(1.0)
                                .pixel_perfect(true)
                        ),
                ),
        ),
    ]
    .gap(Space::S4)
    .into()
}
