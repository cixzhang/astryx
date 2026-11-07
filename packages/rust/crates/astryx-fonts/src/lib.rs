//! Fonts embedded so desktop and headless shells draw text with no setup.
//! The web shell leaves this crate out and loads faces at runtime instead.
//!
//! - Roboto Regular, Medium, and Bold (Apache 2.0), full character sets.
//! - With the `noto-rtl` feature, Noto Sans Arabic and Hebrew (SIL OFL 1.1),
//!   handed to the text engine only when text in those scripts appears.

use astryx_raster::text::{FontData, FontSet, Script, TextEngine};

pub static ROBOTO_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");
pub static ROBOTO_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Medium.ttf");
pub static ROBOTO_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Bold.ttf");

/// The embedded Roboto faces as one family.
pub fn roboto() -> FontSet {
    let mut set = FontSet::new();
    for face in [ROBOTO_REGULAR, ROBOTO_MEDIUM, ROBOTO_BOLD] {
        set.add(face).expect("embedded Roboto parses");
    }
    set
}

#[cfg(feature = "noto-rtl")]
mod noto {
    pub static ARABIC: [&[u8]; 3] = [
        include_bytes!("../fonts/NotoSansArabic-Regular.ttf"),
        include_bytes!("../fonts/NotoSansArabic-Medium.ttf"),
        include_bytes!("../fonts/NotoSansArabic-Bold.ttf"),
    ];
    pub static HEBREW: [&[u8]; 3] = [
        include_bytes!("../fonts/NotoSansHebrew-Regular.ttf"),
        include_bytes!("../fonts/NotoSansHebrew-Medium.ttf"),
        include_bytes!("../fonts/NotoSansHebrew-Bold.ttf"),
    ];
}

/// The embedded faces for a script Roboto does not cover.
pub fn fallback(script: Script) -> Vec<FontData> {
    #[cfg(feature = "noto-rtl")]
    {
        let faces: &[&'static [u8]] = match script {
            Script::Arabic => &noto::ARABIC,
            Script::Hebrew => &noto::HEBREW,
            _ => &[],
        };
        faces.iter().map(|&f| FontData::Static(f)).collect()
    }
    #[cfg(not(feature = "noto-rtl"))]
    {
        let _ = script;
        Vec::new()
    }
}

/// A text engine with Roboto and the embedded fallbacks on demand.
pub fn engine() -> TextEngine {
    let engine = TextEngine::new(roboto());
    engine.set_fallback_provider(Box::new(fallback));
    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roboto_is_one_family() {
        assert_eq!(roboto().families().collect::<Vec<_>>(), vec!["Roboto"]);
    }

    #[cfg(feature = "noto-rtl")]
    #[test]
    fn hebrew_and_arabic_load_on_first_use() {
        use astryx_core::{FontFamily, TextAlign, TextMeasure, TextStyle};
        let engine = engine();
        let style = TextStyle {
            direction: None,
            family: FontFamily::Body,
            size: 16.0,
            weight: 400,
            leading: 1.4,
        };
        engine.measure("Hello", &style, None);
        let before = engine.generation();
        let layout = engine.layout("שלום مرحبا", &style, None, TextAlign::Start);
        assert!(engine.generation() > before, "faces were added");
        assert!(
            layout.lines[0].glyphs.iter().all(|g| g.id != 0),
            "no missing glyphs"
        );
        assert!(engine.take_missing_scripts().is_empty());
    }
}
