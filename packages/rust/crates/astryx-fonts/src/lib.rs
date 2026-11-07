//! Roboto Regular, Medium, and Bold (full character sets, Apache 2.0),
//! embedded so desktop and headless shells draw text with no setup. The web
//! shell leaves this out and loads faces at runtime instead.

use astryx_raster::text::FontSet;

pub static ROBOTO_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");
pub static ROBOTO_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Medium.ttf");
pub static ROBOTO_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Bold.ttf");

/// The embedded faces as one family.
pub fn roboto() -> FontSet {
    let mut set = FontSet::new();
    for face in [ROBOTO_REGULAR, ROBOTO_MEDIUM, ROBOTO_BOLD] {
        set.add(face).expect("embedded Roboto parses");
    }
    set
}

#[cfg(test)]
mod tests {
    #[test]
    fn roboto_is_one_family() {
        assert_eq!(
            super::roboto().families().collect::<Vec<_>>(),
            vec!["Roboto"]
        );
    }
}
