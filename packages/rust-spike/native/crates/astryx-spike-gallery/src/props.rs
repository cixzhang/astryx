//! Reading Astryx-named props from a case's JSON.

use astryx_core::Space;
use astryx_spike_ui::SizeValue;
use serde_json::Value;

/// A case's props, keyed by Astryx prop names (`isDisabled`, `variant`).
#[derive(Clone, Copy)]
pub struct Props<'a>(pub &'a Value);

impl<'a> Props<'a> {
    pub fn get(&self, key: &str) -> Option<&'a Value> {
        self.0.get(key)
    }
    pub fn str(&self, key: &str) -> Option<&'a str> {
        self.get(key).and_then(Value::as_str)
    }
    pub fn str_or(&self, key: &str, default: &'a str) -> &'a str {
        self.str(key).unwrap_or(default)
    }
    pub fn bool(&self, key: &str) -> bool {
        self.get(key).and_then(Value::as_bool).unwrap_or(false)
    }
    pub fn num(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(Value::as_f64)
    }
    pub fn space(&self, key: &str) -> Option<Space> {
        self.num(key).and_then(space)
    }
    pub fn size(&self, key: &str) -> Option<SizeValue> {
        match self.get(key)? {
            Value::Number(n) => n.as_f64().map(|v| SizeValue::Points(v as f32)),
            Value::String(s) => s
                .strip_suffix('%')
                .and_then(|v| v.parse::<f32>().ok())
                .map(SizeValue::Percent)
                .or_else(|| {
                    s.strip_suffix("px")
                        .and_then(|v| v.parse::<f32>().ok())
                        .map(SizeValue::Points)
                }),
            _ => None,
        }
    }
    /// The words a case shows: `children`, `label` or `title`, else `fallback`.
    pub fn words(&self, fallback: &'a str) -> &'a str {
        self.str("children")
            .or(self.str("label"))
            .or(self.str("title"))
            .unwrap_or(fallback)
    }
}

/// A spacing step from its Astryx number (0, 0.5, 1 … 10).
pub fn space(v: f64) -> Option<Space> {
    Some(match (v * 2.0).round() as i64 {
        0 => Space::S0,
        1 => Space::S0_5,
        2 => Space::S1,
        3 => Space::S1_5,
        4 => Space::S2,
        6 => Space::S3,
        8 => Space::S4,
        10 => Space::S5,
        12 => Space::S6,
        16 => Space::S8,
        20 => Space::S10,
        _ => return None,
    })
}

/// Every step of the spacing scale, as Astryx numbers.
pub const SPACE_STEPS: [f64; 11] = [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0];
