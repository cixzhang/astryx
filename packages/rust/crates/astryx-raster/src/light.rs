//! The light map of a lighting pass, computed on the CPU. The GPU renderer
//! computes the same values in a shader; both follow the functions here.

use astryx_core::{Light, Lighting};

/// Target pixels per light map pixel along each axis.
pub const MAP_DIVISOR: u32 = 4;

/// Side of the dapple noise lattice, in cells. The pattern repeats after it.
pub const LATTICE: usize = 16;

/// Flat levels of a light's falloff.
pub const LIGHT_BANDS: f32 = 4.0;

/// The noise lattice: one byte per cell, the same in every renderer.
pub fn lattice() -> [u8; LATTICE * LATTICE] {
    let mut out = [0u8; LATTICE * LATTICE];
    let mut state: u32 = 0x9e37_79b9;
    for v in out.iter_mut() {
        // xorshift32: fixed, so every renderer and run sees one pattern.
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        *v = (state >> 24) as u8;
    }
    out
}

/// A light ready to add, in target pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedLight {
    pub center: (f32, f32),
    pub radius: (f32, f32),
    /// Color times intensity times strength, per channel.
    pub rgb: [f32; 3],
}

impl PlacedLight {
    /// `light` at `time`, drawn at `scale` pixels per point, in a pass of
    /// `strength`.
    pub fn new(light: &Light, time: f32, scale: f32, strength: f32) -> PlacedLight {
        let k = light.intensity_at(time) * strength;
        PlacedLight {
            center: (light.center.x * scale, light.center.y * scale),
            radius: (light.radius_x * scale, light.radius_y * scale),
            rgb: [light.color.r * k, light.color.g * k, light.color.b * k],
        }
    }

    /// What it adds at target pixel `(x, y)`, in bands.
    pub fn falloff(&self, x: f32, y: f32) -> f32 {
        if self.radius.0 <= 0.0 || self.radius.1 <= 0.0 {
            return 0.0;
        }
        let dx = (x - self.center.0) / self.radius.0;
        let dy = (y - self.center.1) / self.radius.1;
        let f = (1.0 - (dx * dx + dy * dy).sqrt()).max(0.0);
        let f = f * f;
        if f > 0.0 {
            (f * LIGHT_BANDS).ceil() / LIGHT_BANDS
        } else {
            0.0
        }
    }
}

/// The map's starting color: white mixed toward ambient by strength.
pub fn base(lighting: &Lighting) -> [f32; 3] {
    let s = lighting.strength.clamp(0.0, 1.0);
    let a = lighting.ambient;
    [a.r, a.g, a.b].map(|c| 1.0 - s * (1.0 - c))
}

/// The dapple's multiplier at target pixel `(x, y)`, or 1 without dapple.
/// `lattice` comes from [`lattice`].
pub fn dapple(lighting: &Lighting, scale: f32, lattice: &[u8], x: f32, y: f32) -> f32 {
    let Some(d) = lighting.dapple else { return 1.0 };
    if d.cell <= 0.0 || d.bands < 2 {
        return 1.0;
    }
    let inv = 1.0 / (d.cell * scale);
    let u = x * inv + d.velocity.x * lighting.time / d.cell;
    let v = y * inv + d.velocity.y * lighting.time / d.cell;
    let n = value_noise(lattice, u, v);
    let bands = f32::from(d.bands);
    let band = (n * bands).floor().min(bands - 1.0) / (bands - 1.0);
    1.0 - d.depth.clamp(0.0, 1.0) * lighting.strength.clamp(0.0, 1.0) * (1.0 - band)
}

/// Smooth noise from 0 to 1 with one lattice value per whole coordinate.
fn value_noise(lattice: &[u8], u: f32, v: f32) -> f32 {
    let (iu, iv) = (u.floor(), v.floor());
    let (fu, fv) = (u - iu, v - iv);
    let (su, sv) = (fu * fu * (3.0 - 2.0 * fu), fv * fv * (3.0 - 2.0 * fv));
    let at = |i: f32, j: f32| {
        let n = LATTICE as i64;
        let (i, j) = ((i as i64).rem_euclid(n), (j as i64).rem_euclid(n));
        f32::from(lattice[(j * n + i) as usize]) / 255.0
    };
    let top = at(iu, iv) + (at(iu + 1.0, iv) - at(iu, iv)) * su;
    let bottom = at(iu, iv + 1.0) + (at(iu + 1.0, iv + 1.0) - at(iu, iv + 1.0)) * su;
    top + (bottom - top) * sv
}

/// A light map: RGB bytes, `width` by `height`, each pixel covering
/// [`MAP_DIVISOR`] target pixels square.
pub struct LightMap {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

/// The light map for a target `width` by `height` pixels. Each map pixel
/// takes the value at its center.
pub fn light_map(
    lighting: &Lighting,
    lights: &[PlacedLight],
    scale: f32,
    width: u32,
    height: u32,
) -> LightMap {
    let (mw, mh) = (width.div_ceil(MAP_DIVISOR), height.div_ceil(MAP_DIVISOR));
    let lattice = lattice();
    let base = base(lighting);
    let d = MAP_DIVISOR as f32;
    let mut rgb = Vec::with_capacity((mw * mh * 3) as usize);
    for j in 0..mh {
        for i in 0..mw {
            let (x, y) = ((i as f32 + 0.5) * d, (j as f32 + 0.5) * d);
            let shade = dapple(lighting, scale, &lattice, x, y);
            let mut c = base.map(|b| b * shade);
            for light in lights {
                let f = light.falloff(x, y);
                if f > 0.0 {
                    for (c, l) in c.iter_mut().zip(light.rgb) {
                        *c += l * f;
                    }
                }
            }
            rgb.extend(c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8));
        }
    }
    LightMap {
        width: mw,
        height: mh,
        rgb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astryx_core::{Color, Dapple, Flicker, Point, Pulse};

    fn lighting(strength: f32) -> Lighting {
        Lighting {
            ambient: Color::rgba(0.07, 0.094, 0.22, 1.0),
            strength,
            time: 0.0,
            dapple: None,
        }
    }

    fn lamp() -> Light {
        Light {
            center: Point { x: 16.0, y: 16.0 },
            radius_x: 12.0,
            radius_y: 12.0,
            color: Color::rgba(1.0, 0.8, 0.5, 1.0),
            intensity: 1.0,
            flicker: Flicker::default(),
            pulse: Pulse::default(),
        }
    }

    #[test]
    fn strength_zero_is_white_and_full_strength_is_ambient() {
        assert_eq!(base(&lighting(0.0)), [1.0, 1.0, 1.0]);
        let b = base(&lighting(1.0));
        assert!((b[2] - 0.22).abs() < 1e-6);
    }

    #[test]
    fn lights_fall_off_in_four_bands() {
        let l = PlacedLight::new(&lamp(), 0.0, 1.0, 1.0);
        assert_eq!(l.falloff(16.0, 16.0), 1.0);
        assert_eq!(l.falloff(40.0, 16.0), 0.0);
        let mut levels: Vec<u32> = (0..130)
            .map(|i| (l.falloff(16.0 + i as f32 * 0.1, 16.0) * 4.0) as u32)
            .collect();
        levels.dedup();
        assert_eq!(levels, [4, 3, 2, 1, 0]);
    }

    #[test]
    fn a_light_brightens_the_map_around_it() {
        let lights = [PlacedLight::new(&lamp(), 0.0, 1.0, 0.84)];
        let map = light_map(&lighting(0.84), &lights, 1.0, 64, 48);
        assert_eq!((map.width, map.height), (16, 12));
        let px = |i: u32, j: u32| map.rgb[((j * map.width + i) * 3) as usize];
        assert!(px(3, 3) > 200, "center {}", px(3, 3));
        assert!(px(15, 11) < 60, "corner {}", px(15, 11));
    }

    #[test]
    fn flicker_and_pulse_change_intensity_over_time() {
        let mut l = lamp();
        l.pulse = Pulse {
            period: 2.0,
            min: 0.5,
        };
        assert!((l.intensity_at(0.0) - 1.0).abs() < 1e-5);
        assert!((l.intensity_at(1.0) - 0.5).abs() < 1e-5);
        l.pulse = Pulse::default();
        l.flicker = Flicker {
            amplitude: 0.1,
            hz: 2.0,
        };
        let values: Vec<f32> = (0..20).map(|i| l.intensity_at(i as f32 * 0.05)).collect();
        assert!(values.iter().all(|v| (0.89..=1.11).contains(v)));
        assert!(values.iter().any(|v| (v - 1.0).abs() > 0.02));
    }

    #[test]
    fn dapple_darkens_in_flat_bands_and_moves() {
        let mut lit = lighting(0.45);
        lit.dapple = Some(Dapple {
            cell: 8.0,
            velocity: Point { x: 4.0, y: 0.0 },
            bands: 3,
            depth: 1.0,
        });
        let lat = lattice();
        let mut seen: Vec<u32> = (0..400)
            .map(|i| (dapple(&lit, 1.0, &lat, i as f32, 3.0) * 1000.0).round() as u32)
            .collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen, [550, 775, 1000]);
        let still = dapple(&lit, 1.0, &lat, 37.0, 5.0);
        lit.time = 1.0;
        let moved: Vec<f32> = (0..64)
            .map(|x| dapple(&lit, 1.0, &lat, x as f32, 5.0))
            .collect();
        assert!(moved.iter().any(|&v| v != still));
    }
}
