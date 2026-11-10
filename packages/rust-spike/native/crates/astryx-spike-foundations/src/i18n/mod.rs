//! Internationalization: locales, direction, Astryx's message catalogs, and
//! locale-aware number, date, list, and plural formatting (ICU4X).
//!
//! ```
//! use astryx_spike_foundations::{args, i18n::I18n};
//! let ar = I18n::new("ar-SA");
//! assert!(ar.dir().is_rtl());
//! let en = I18n::new("en");
//! assert_eq!(en.t("@astryx.pagination.nextBy", &args! {"step" => 2}), "Go forward 2 pages");
//! ```

mod catalogs;
mod icu;
pub mod message;

pub use message::{Args, Category, DateStyle, DateValue, Formats, Message, NumberStyle, Value};

use astryx_core::Direction;
use std::cell::RefCell;
use std::collections::HashMap;

/// A BCP 47 locale and its writing direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locale {
    /// The canonical tag, such as `pt-BR`.
    pub tag: String,
    pub dir: Direction,
}

impl Locale {
    /// Parses `tag`. A malformed tag falls back to `en`, as Astryx does.
    pub fn parse(tag: &str) -> Locale {
        let canonical = icu::canonical(tag).unwrap_or_else(|| "en".to_string());
        let dir = icu::direction(&canonical);
        Locale {
            tag: canonical,
            dir,
        }
    }

    /// The tag from most to least specific: `zh-Hans-CN`, `zh-Hans`, `zh`.
    pub fn chain(&self) -> Vec<String> {
        let parts: Vec<&str> = self.tag.split('-').collect();
        (1..=parts.len())
            .rev()
            .map(|n| parts[..n].join("-"))
            .collect()
    }

    /// The language subtag, such as `pt`.
    pub fn language(&self) -> &str {
        self.tag.split('-').next().unwrap_or("en")
    }
}

/// The locales Astryx ships catalogs for, by catalog name (`ar-SA`, `en`).
pub fn shipped_locales() -> impl Iterator<Item = &'static str> {
    catalogs::CATALOGS.iter().map(|(name, _)| *name)
}

/// One locale's messages: key to source text.
type Catalog = HashMap<&'static str, String>;

fn parse_catalog(text: &'static str) -> Catalog {
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(k, v)| (k, unescape(v)))
        .collect()
}

fn unescape(v: &str) -> String {
    if !v.contains('\\') {
        return v.to_string();
    }
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The catalog for `name` (a shipped catalog name), parsed once per thread.
fn shipped(name: &str) -> Option<std::rc::Rc<Catalog>> {
    thread_local! {
        static CACHE: RefCell<HashMap<&'static str, std::rc::Rc<Catalog>>> = RefCell::new(HashMap::new());
    }
    let (key, text) = catalogs::CATALOGS.iter().find(|(n, _)| *n == name)?;
    Some(CACHE.with(|c| {
        c.borrow_mut()
            .entry(key)
            .or_insert_with(|| std::rc::Rc::new(parse_catalog(text)))
            .clone()
    }))
}

/// The shipped catalog that serves `tag`: an exact match, else the first
/// catalog of the same language (`ar` is served by `ar-SA`).
fn catalog_for(tag: &str) -> Option<std::rc::Rc<Catalog>> {
    if let Some(c) = shipped(tag) {
        return Some(c);
    }
    if tag.contains('-') {
        return None;
    }
    let prefix = format!("{tag}-");
    let name = catalogs::CATALOGS
        .iter()
        .map(|(n, _)| *n)
        .find(|n| n.starts_with(&prefix))?;
    shipped(name)
}

/// A locale with its catalogs and formatters: what components use to turn a
/// message key and arguments into text.
pub struct I18n {
    locale: Locale,
    formats: icu::IcuFormats,
    /// App overrides by locale tag, as Astryx's provider `messages` prop.
    overrides: HashMap<String, HashMap<String, String>>,
    parsed: RefCell<HashMap<String, Option<Message>>>,
}

impl I18n {
    pub fn new(tag: &str) -> I18n {
        let locale = Locale::parse(tag);
        let formats = icu::IcuFormats::new(&locale.tag);
        I18n {
            locale,
            formats,
            overrides: HashMap::new(),
            parsed: RefCell::new(HashMap::new()),
        }
    }

    /// Replaces the message for `key` in `tag` (and locales under it).
    pub fn with_override(mut self, tag: &str, key: &str, message: &str) -> Self {
        let tag = Locale::parse(tag).tag;
        self.overrides
            .entry(tag)
            .or_default()
            .insert(key.to_string(), message.to_string());
        self.parsed.borrow_mut().clear();
        self
    }

    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    pub fn dir(&self) -> Direction {
        self.locale.dir
    }

    /// The source text for `key`, by Astryx's lookup order: overrides for the
    /// locale chain, shipped catalogs for the chain, shipped English, else
    /// `None`.
    pub fn source(&self, key: &str) -> Option<String> {
        let chain = self.locale.chain();
        for tag in &chain {
            if let Some(m) = self.overrides.get(tag).and_then(|o| o.get(key)) {
                return Some(m.clone());
            }
        }
        for tag in &chain {
            if let Some(m) = catalog_for(tag).and_then(|c| c.get(key).cloned()) {
                return Some(m);
            }
        }
        shipped("en").and_then(|c| c.get(key).cloned())
    }

    /// The message for `key` with `args` filled in. An unknown key returns
    /// the key itself, so a gap is visible.
    pub fn t(&self, key: &str, args: &Args) -> String {
        let mut parsed = self.parsed.borrow_mut();
        let message = parsed
            .entry(key.to_string())
            .or_insert_with(|| self.source(key).and_then(|s| Message::parse(&s).ok()));
        match message {
            Some(m) => m.format(args, &self.formats),
            None => key.to_string(),
        }
    }

    /// Formats any ICU message source (not a catalog key).
    pub fn format_message(&self, source: &str, args: &Args) -> Result<String, message::ParseError> {
        Ok(Message::parse(source)?.format(args, &self.formats))
    }

    pub fn number(&self, n: f64) -> String {
        self.formats.number(n, NumberStyle::Decimal)
    }

    pub fn percent(&self, ratio: f64) -> String {
        self.formats.number(ratio, NumberStyle::Percent)
    }

    pub fn date(&self, d: &DateValue, style: DateStyle) -> String {
        self.formats.date(d, style)
    }

    pub fn time(&self, d: &DateValue, style: DateStyle) -> String {
        self.formats.time(d, style)
    }

    pub fn plural(&self, n: f64) -> Category {
        self.formats.plural(n, false)
    }

    /// "a, b, and c" in this locale.
    pub fn list(&self, items: &[&str]) -> String {
        self.formats.list(items)
    }

    /// Parses a number typed in this locale's digits and separators, such
    /// as `1 234,5` in French or Arabic-Indic digits.
    pub fn parse_number(&self, text: &str) -> Option<f64> {
        self.formats.parse_number(text)
    }
}

impl Default for I18n {
    fn default() -> Self {
        I18n::new("en")
    }
}

impl std::fmt::Debug for I18n {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I18n")
            .field("locale", &self.locale)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args;

    #[test]
    fn locale_chain_and_direction() {
        let l = Locale::parse("zh-hans-cn");
        assert_eq!(l.tag, "zh-Hans-CN");
        assert_eq!(l.chain(), vec!["zh-Hans-CN", "zh-Hans", "zh"]);
        assert_eq!(Locale::parse("ar").dir, Direction::Rtl);
        assert_eq!(Locale::parse("he-IL").dir, Direction::Rtl);
        assert_eq!(Locale::parse("fa").dir, Direction::Rtl);
        assert_eq!(Locale::parse("fr-FR").dir, Direction::Ltr);
        assert_eq!(Locale::parse("not a tag!").tag, "en");
    }

    #[test]
    fn every_shipped_catalog_parses() {
        let mut bad = Vec::new();
        for name in shipped_locales() {
            for (key, src) in shipped(name).unwrap().iter() {
                if let Err(e) = Message::parse(src) {
                    bad.push(format!("{name} {key}: {e}"));
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
        assert_eq!(shipped_locales().count(), 30);
        assert!(shipped("en").unwrap().len() >= 600);
    }

    #[test]
    fn english_plural_from_the_catalog() {
        let en = I18n::new("en-US");
        assert_eq!(
            en.t("@astryx.pagination.previousBy", &args! {"step" => 1}),
            "Go back 1 page"
        );
        assert_eq!(
            en.t("@astryx.pagination.previousBy", &args! {"step" => 1200}),
            "Go back 1,200 pages"
        );
    }

    #[test]
    fn arabic_plural_uses_all_six_categories() {
        let ar = I18n::new("ar");
        let page = |n: i64| ar.t("@astryx.pagination.previousBy", &args! {"step" => n});
        // zero, one, two, few (3-10), many (11-99), other (100-102).
        let forms: Vec<String> = [0, 1, 2, 3, 11, 100].iter().map(|&n| page(n)).collect();
        assert!(forms[0].ends_with("صفحات"), "{}", forms[0]);
        assert!(forms[1].ends_with("صفحة"), "{}", forms[1]);
        assert!(forms[2].ends_with("صفحتان"), "{}", forms[2]);
        assert!(forms[3].ends_with("صفحات"), "{}", forms[3]);
        assert!(forms[4].ends_with("صفحةً"), "{}", forms[4]);
        assert!(forms[5].ends_with("صفحة"), "{}", forms[5]);
        assert_eq!(ar.plural(0.0), Category::Zero);
        assert_eq!(ar.plural(2.0), Category::Two);
        assert_eq!(ar.plural(5.0), Category::Few);
        assert_eq!(ar.plural(11.0), Category::Many);
    }

    #[test]
    fn lookup_order_overrides_then_catalog_then_english_then_key() {
        let fr =
            I18n::new("fr-CA").with_override("fr", "@astryx.alertDialog.cancel", "Laisser tomber");
        assert_eq!(
            fr.t("@astryx.alertDialog.cancel", &Args::new()),
            "Laisser tomber"
        );
        let fr = I18n::new("fr-FR");
        assert_eq!(fr.t("@astryx.alertDialog.cancel", &Args::new()), "Annuler");
        // A key only English has falls back to English.
        let only_en = shipped("en")
            .unwrap()
            .keys()
            .find(|k| !shipped("fr-FR").unwrap().contains_key(*k) && !k.contains("plural"))
            .map(|k| k.to_string());
        if let Some(k) = only_en {
            assert_eq!(fr.source(&k), I18n::new("en").source(&k));
        }
        assert_eq!(fr.t("@astryx.not.a.key", &Args::new()), "@astryx.not.a.key");
    }

    #[test]
    fn numbers_dates_and_lists_follow_the_locale() {
        assert_eq!(I18n::new("en").number(1234.5), "1,234.5");
        assert_eq!(I18n::new("de-DE").number(1234.5), "1.234,5");
        let fr = I18n::new("fr-FR").number(1234.5);
        assert!(
            fr.starts_with('1') && fr.ends_with("234,5") && fr.len() > 6,
            "{fr}"
        );
        assert_eq!(I18n::new("ar-EG").number(12.0), "١٢");
        let d = DateValue::ymd(2026, 11, 9);
        assert_eq!(I18n::new("en").date(&d, DateStyle::Medium), "Nov 9, 2026");
        assert!(I18n::new("fr-FR")
            .date(&d, DateStyle::Long)
            .contains("novembre"));
        assert_eq!(I18n::new("en").list(&["A", "B", "C"]), "A, B, and C");
        assert_eq!(I18n::new("en").percent(0.25), "25%");
        let kb = I18n::new("de-DE").t(
            "@astryx.fileInput.fileSize.kilobytes",
            &args! {"size" => 1234.56},
        );
        assert!(kb.starts_with("1234,6"), "{kb}");
    }

    #[test]
    fn parse_number_reads_local_digits() {
        assert_eq!(I18n::new("en").parse_number("1,234.5"), Some(1234.5));
        assert_eq!(I18n::new("de-DE").parse_number("1.234,5"), Some(1234.5));
        assert_eq!(I18n::new("ar-EG").parse_number("١٢٣"), Some(123.0));
        assert_eq!(I18n::new("en").parse_number("abc"), None);
    }
}
