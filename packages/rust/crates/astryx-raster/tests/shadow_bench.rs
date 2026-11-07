//! `cargo test --release -p astryx-raster --test shadow_bench -- --ignored --nocapture`

use astryx_core::{Color, ShadowLayer};
use astryx_raster::geom::PxRect;
use astryx_raster::shadow::{shadow_mask, ShadowKey};

#[test]
#[ignore]
fn big_card_shadow() {
    let rect = PxRect {
        x0: 0.0,
        y0: 0.0,
        x1: 926.0,
        y1: 420.0,
    };
    for blur in [2.0, 8.0, 24.0] {
        let layer = ShadowLayer {
            x: 0.0,
            y: 4.0,
            blur,
            spread: 0.0,
            inset: false,
            color: Color::rgba(0.0, 0.0, 0.0, 0.2),
        };
        let key = ShadowKey::new(&rect, [24.0; 4], &layer, 2.0);
        let t = std::time::Instant::now();
        let m = shadow_mask(&key).unwrap();
        eprintln!("blur {blur}: {}x{} in {:?}", m.width, m.height, t.elapsed());
    }
}
