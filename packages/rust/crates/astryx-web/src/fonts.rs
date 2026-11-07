//! Runtime font loading by script subset, the way CSS `unicode-range` works.
//!
//! `fonts/ranges.txt` lists the subsets. The first one loads for every
//! weight before the first frame; the others load when drawn text needs them,
//! only for the weights that text uses, or when the text engine reports a
//! script it has no face for.

use std::rc::Weak;

use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

const RANGES: &str = include_str!("../fonts/ranges.txt");

/// The CSS weights each subset comes in.
pub(crate) const FACES: [u16; 3] = [400, 500, 700];

/// The face that draws a CSS weight, by CSS font matching over 400, 500,
/// and 700, as the renderer picks faces.
pub(crate) fn face_for_weight(weight: u16) -> usize {
    match weight {
        0..=400 => 0,
        401..=500 => 1,
        _ => 2,
    }
}

pub(crate) struct Subset {
    pub name: String,
    ranges: Vec<(u32, u32)>,
}

impl Subset {
    fn covers(&self, c: u32) -> bool {
        self.ranges.iter().any(|(lo, hi)| (*lo..=*hi).contains(&c))
    }
}

/// Parses `<name> U+0000-00FF, U+0131, ...` lines. `#` starts a comment.
pub(crate) fn parse_ranges(text: &str) -> Vec<Subset> {
    let hex = |s: &str| u32::from_str_radix(s.trim().trim_start_matches("U+"), 16).ok();
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            // The source family only matters to the subset script.
            let (name, rest) = line.split_once(' ')?;
            let (_, list) = rest.split_once(' ')?;
            let ranges = list
                .split(',')
                .filter_map(|part| match part.split_once('-') {
                    Some((lo, hi)) => Some((hex(lo)?, hex(hi)?)),
                    None => hex(part).map(|c| (c, c)),
                })
                .collect();
            Some(Subset {
                name: name.to_string(),
                ranges,
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Idle,
    Requested,
    Done,
}

/// Which subset files are needed, requested, and done.
pub(crate) struct FontPlan {
    subsets: Vec<Subset>,
    state: Vec<[State; FACES.len()]>,
}

/// One subset file of one face: indices into the plan's subsets and [`FACES`].
pub(crate) type FontKey = (usize, usize);

impl FontPlan {
    pub(crate) fn new() -> Self {
        Self::from_ranges(RANGES)
    }

    fn from_ranges(text: &str) -> Self {
        let subsets = parse_ranges(text);
        let state = vec![[State::Idle; FACES.len()]; subsets.len()];
        Self { subsets, state }
    }

    fn take(&mut self, key: FontKey) -> Option<FontKey> {
        let state = &mut self.state[key.0][key.1];
        (*state == State::Idle).then(|| {
            *state = State::Requested;
            key
        })
    }

    /// The first subset for every face, for the first frame.
    pub(crate) fn request_initial(&mut self) -> Vec<FontKey> {
        if self.subsets.is_empty() {
            return Vec::new();
        }
        (0..FACES.len())
            .filter_map(|face| self.take((0, face)))
            .collect()
    }

    /// Files that `text` drawn at `weight` needs and nobody requested yet.
    /// Marks them requested.
    pub(crate) fn request_for(&mut self, text: &str, weight: u16) -> Vec<FontKey> {
        let face = face_for_weight(weight);
        let mut out = Vec::new();
        for c in text.chars() {
            let Some(subset) = self.subsets.iter().position(|s| s.covers(c as u32)) else {
                continue;
            };
            if let Some(key) = self.take((subset, face)) {
                out.push(key);
            }
        }
        out
    }

    /// The regular-weight file of the subset named after a script, such as
    /// "Arabic", for a script the text engine reported without a face.
    pub(crate) fn request_script(&mut self, script: &str) -> Option<FontKey> {
        let subset = self
            .subsets
            .iter()
            .position(|s| s.name.eq_ignore_ascii_case(script))?;
        self.take((subset, 0))
    }

    /// Marks a file done, loaded or failed. Failed files are not retried.
    pub(crate) fn finish(&mut self, key: FontKey) {
        self.state[key.0][key.1] = State::Done;
    }

    pub(crate) fn file_name(&self, key: FontKey) -> String {
        format!("{}-{}.ttf", self.subsets[key.0].name, FACES[key.1])
    }
}

/// Fetches one file and hands the result to `deliver` if its owner still
/// exists.
pub(crate) fn fetch<T: 'static>(
    url: String,
    owner: Weak<T>,
    deliver: impl FnOnce(&T, Result<Vec<u8>, JsValue>) + 'static,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_bytes(&url).await;
        if let Some(owner) = owner.upgrade() {
            deliver(&owner, result);
        }
    });
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let response: Response = JsFuture::from(window.fetch_with_str(url))
        .await?
        .dyn_into()?;
    if !response.ok() {
        return Err(format!("{url}: HTTP {}", response.status()).into());
    }
    let buffer = JsFuture::from(response.array_buffer()?).await?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RANGES: &str = "\
# comment
latin Roboto U+0000-00FF, U+2000-206F
cyrillic Roboto U+0400-045F, U+2116
greek Roboto U+0370-03FF
";

    #[test]
    fn parses_ranges() {
        let subsets = parse_ranges(RANGES);
        assert_eq!(subsets.len(), 3);
        assert_eq!(subsets[1].name, "cyrillic");
        assert!(subsets[1].covers(0x2116));
        assert!(subsets[0].covers(0x41));
        assert!(!subsets[0].covers(0x0416));
    }

    #[test]
    fn the_shipped_ranges_parse() {
        let plan = FontPlan::new();
        let names: Vec<&str> = plan.subsets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names[0], "latin");
        assert!(
            names.contains(&"cyrillic")
                && names.contains(&"greek")
                && names.contains(&"vietnamese")
        );
    }

    #[test]
    fn initial_request_is_the_first_subset_for_every_face() {
        let mut plan = FontPlan::from_ranges(RANGES);
        assert_eq!(plan.request_initial(), vec![(0, 0), (0, 1), (0, 2)]);
        assert_eq!(plan.request_initial(), vec![]);
        assert_eq!(plan.file_name((0, 2)), "latin-700.ttf");
    }

    #[test]
    fn text_requests_only_new_subsets_for_its_face() {
        let mut plan = FontPlan::from_ranges(RANGES);
        plan.request_initial();
        assert_eq!(plan.request_for("Hello", 400), vec![]);
        assert_eq!(plan.request_for("Привет и Γειά", 400), vec![(1, 0), (2, 0)]);
        assert_eq!(plan.request_for("Ещё", 400), vec![]);
        assert_eq!(plan.request_for("Ещё", 700), vec![(1, 2)]);
        assert_eq!(plan.request_for("\u{4e2d}", 400), vec![]);
        plan.finish((1, 0));
        assert_eq!(plan.request_for("Ж", 400), vec![]);
    }

    #[test]
    fn missing_scripts_map_to_subsets_by_name() {
        let mut plan = FontPlan::new();
        let arabic = plan.request_script("Arabic").expect("arabic subset");
        assert_eq!(plan.file_name(arabic), "arabic-400.ttf");
        assert_eq!(plan.request_script("Arabic"), None);
        assert!(plan.request_script("Hebrew").is_some());
        assert_eq!(plan.request_script("Han"), None);
        assert_eq!(
            plan.request_for("\u{5e9}\u{5dc}\u{5d5}\u{5dd}", 700).len(),
            1
        );
    }

    #[test]
    fn weights_map_to_faces() {
        assert_eq!(face_for_weight(300), 0);
        assert_eq!(face_for_weight(400), 0);
        assert_eq!(face_for_weight(450), 1);
        assert_eq!(face_for_weight(500), 1);
        assert_eq!(face_for_weight(600), 2);
        assert_eq!(face_for_weight(900), 2);
    }
}
