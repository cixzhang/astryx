//! ICU4X-backed locale data: canonical tags, direction, plurals, numbers,
//! dates, and lists.

use super::message::{Category, DateStyle, DateValue, Formats, NumberStyle};
use astryx_core::Direction;
use fixed_decimal::{Decimal, FloatPrecision};
use icu_calendar::Date;
use icu_datetime::fieldsets::{T, YMD, YMDE};
use icu_datetime::input::Time;
use icu_datetime::{DateTimeFormatter, NoCalendarFormatter};
use icu_decimal::DecimalFormatter;
use icu_list::options::{ListFormatterOptions, ListLength};
use icu_list::ListFormatter;
use icu_locale::LocaleDirectionality;
use icu_locale_core::Locale as IcuLocale;
use icu_plurals::{PluralCategory, PluralRules};
use writeable::Writeable;

/// The canonical form of `tag`, or `None` when it is not a BCP 47 tag.
pub(crate) fn canonical(tag: &str) -> Option<String> {
    let tag = tag.trim().replace('_', "-");
    IcuLocale::try_from_str(&tag).ok().map(|l| l.to_string())
}

/// The writing direction of `tag`'s script, as `Intl.Locale.getTextInfo`
/// reports it.
pub(crate) fn direction(tag: &str) -> Direction {
    let Ok(locale) = IcuLocale::try_from_str(tag) else {
        return Direction::Ltr;
    };
    match LocaleDirectionality::new_common().get(&locale.id) {
        Some(icu_locale::Direction::RightToLeft) => Direction::Rtl,
        _ => Direction::Ltr,
    }
}

/// Formatters for one locale, made once.
pub(crate) struct IcuFormats {
    language: String,
    cardinal: Option<PluralRules>,
    ordinal: Option<PluralRules>,
    decimal: Option<DecimalFormatter>,
    dates: [Option<DateTimeFormatter<YMD>>; 3],
    full_date: Option<DateTimeFormatter<YMDE>>,
    time_hm: Option<NoCalendarFormatter<T>>,
    time_hms: Option<NoCalendarFormatter<T>>,
    list: Option<ListFormatter>,
    /// The locale's ten digits, decimal separator, and grouping separator,
    /// read back from its own output, for parsing.
    digits: [char; 10],
    decimal_sep: String,
    group_sep: String,
}

impl IcuFormats {
    pub(crate) fn new(tag: &str) -> IcuFormats {
        let locale = IcuLocale::try_from_str(tag).unwrap_or(icu_locale_core::locale!("en"));
        let language = locale.id.language.to_string();
        let cardinal = PluralRules::try_new_cardinal((&locale).into()).ok();
        let ordinal = PluralRules::try_new_ordinal((&locale).into()).ok();
        let decimal = DecimalFormatter::try_new((&locale).into(), Default::default()).ok();
        let date = |fs: YMD| DateTimeFormatter::try_new((&locale).into(), fs).ok();
        let dates = [date(YMD::short()), date(YMD::medium()), date(YMD::long())];
        let full_date = DateTimeFormatter::try_new((&locale).into(), YMDE::long()).ok();
        let time_hm = NoCalendarFormatter::try_new((&locale).into(), T::hm()).ok();
        let time_hms = NoCalendarFormatter::try_new((&locale).into(), T::hms()).ok();
        let list = ListFormatter::try_new_and(
            (&locale).into(),
            ListFormatterOptions::default().with_length(ListLength::Wide),
        )
        .ok();
        let mut f = IcuFormats {
            language,
            cardinal,
            ordinal,
            decimal,
            dates,
            full_date,
            time_hm,
            time_hms,
            list,
            digits: ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
            decimal_sep: ".".into(),
            group_sep: ",".into(),
        };
        f.learn_symbols();
        f
    }

    fn learn_symbols(&mut self) {
        for d in 0..10 {
            if let Some(c) = self.raw(&Decimal::from(d as i64)).chars().next() {
                self.digits[d] = c;
            }
        }
        // 1234.5 prints as 1<group>234<decimal>5 in the locale's digits.
        let sample = {
            let mut d = Decimal::from(12345);
            d.multiply_pow10(-1);
            self.raw(&d)
        };
        let is_digit = |c: char| self.digits.contains(&c);
        let chars: Vec<char> = sample.chars().collect();
        let runs: Vec<String> = chars
            .split(|c| is_digit(*c))
            .filter(|s| !s.is_empty())
            .map(|s| s.iter().collect())
            .collect();
        match runs.as_slice() {
            [group, dec] => {
                self.group_sep = group.clone();
                self.decimal_sep = dec.clone();
            }
            [dec] => {
                self.group_sep = String::new();
                self.decimal_sep = dec.clone();
            }
            _ => {}
        }
    }

    fn raw(&self, d: &Decimal) -> String {
        match &self.decimal {
            Some(f) => f.format(d).write_to_string().into_owned(),
            None => d.to_string(),
        }
    }

    /// `d` in the locale's digits and decimal separator, without grouping.
    fn ungrouped(&self, d: &Decimal) -> String {
        d.to_string()
            .chars()
            .map(|c| match c {
                '0'..='9' => self.digits[(c as u8 - b'0') as usize].to_string(),
                '.' => self.decimal_sep.clone(),
                other => other.to_string(),
            })
            .collect()
    }

    pub(crate) fn list(&self, items: &[&str]) -> String {
        match &self.list {
            Some(l) => l.format(items.iter()).write_to_string().into_owned(),
            None => items.join(", "),
        }
    }

    pub(crate) fn parse_number(&self, text: &str) -> Option<f64> {
        let mut ascii = String::new();
        let text = text.trim();
        let mut rest = text;
        while let Some(c) = rest.chars().next() {
            if let Some(d) = self.digits.iter().position(|&x| x == c) {
                ascii.push(char::from(b'0' + d as u8));
            } else if c.is_ascii_digit() {
                ascii.push(c);
            } else if !self.decimal_sep.is_empty() && rest.starts_with(&self.decimal_sep) {
                ascii.push('.');
                rest = &rest[self.decimal_sep.len()..];
                continue;
            } else if !self.group_sep.is_empty() && rest.starts_with(&self.group_sep) {
                rest = &rest[self.group_sep.len()..];
                continue;
            } else if c == '-' || c == '\u{2212}' {
                ascii.push('-');
            } else if c.is_whitespace() || c == '\u{202F}' || c == '\u{00A0}' || c == '\u{061C}' {
                // Space-like grouping and direction marks.
            } else {
                return None;
            }
            rest = &rest[c.len_utf8()..];
        }
        ascii.parse().ok()
    }

    fn percent_affix(&self, number: String) -> String {
        match self.language.as_str() {
            "tr" => format!("%{number}"),
            "fr" => format!("{number}\u{202F}%"),
            "de" | "es" | "sv" | "fi" | "nb" | "no" | "da" | "cs" | "pl" | "ru" | "uk" | "el"
            | "sk" | "hu" | "ro" | "sr" | "pt" | "ca" | "nl" | "af" => {
                if self.language == "nl" || self.language == "pt" && self.digits[0] == '0' {
                    format!("{number}%")
                } else {
                    format!("{number}\u{00A0}%")
                }
            }
            "ar" if self.digits[0] != '0' => format!("{number}\u{066A}\u{061C}"),
            "he" => format!("{number}%"),
            _ => format!("{number}%"),
        }
    }
}

fn decimal_of(n: f64, max_fraction: i16) -> Decimal {
    let mut d = Decimal::try_from_f64(n, FloatPrecision::RoundTrip).unwrap_or_default();
    d.round(-max_fraction);
    d.absolute.trim_end();
    d
}

fn date_of(d: &DateValue) -> Option<Date<icu_calendar::Iso>> {
    Date::try_new_iso(d.year, d.month, d.day).ok()
}

impl Formats for IcuFormats {
    fn number(&self, n: f64, style: NumberStyle) -> String {
        match style {
            // Intl.NumberFormat's default: at most three fraction digits.
            NumberStyle::Decimal => self.raw(&decimal_of(n, 3)),
            NumberStyle::Integer => self.raw(&decimal_of(n, 0)),
            NumberStyle::Percent => self.percent_affix(self.raw(&decimal_of(n * 100.0, 0))),
            NumberStyle::Skeleton(sk) => {
                let value = if sk.percent { n * 100.0 } else { n };
                let mut d = decimal_of(value, sk.max_fraction);
                d.absolute.pad_end(-sk.min_fraction);
                let text = match (&self.decimal, sk.grouping) {
                    (Some(_), true) => self.raw(&d),
                    _ => self.ungrouped(&d),
                };
                if sk.percent {
                    self.percent_affix(text)
                } else {
                    text
                }
            }
        }
    }

    fn plural(&self, n: f64, ordinal: bool) -> Category {
        let rules = if ordinal {
            &self.ordinal
        } else {
            &self.cardinal
        };
        let Some(rules) = rules else {
            return if n == 1.0 {
                Category::One
            } else {
                Category::Other
            };
        };
        let d = Decimal::try_from_f64(n, FloatPrecision::RoundTrip).unwrap_or_default();
        match rules.category_for(&d) {
            PluralCategory::Zero => Category::Zero,
            PluralCategory::One => Category::One,
            PluralCategory::Two => Category::Two,
            PluralCategory::Few => Category::Few,
            PluralCategory::Many => Category::Many,
            PluralCategory::Other => Category::Other,
        }
    }

    fn date(&self, d: &DateValue, style: DateStyle) -> String {
        let Some(date) = date_of(d) else {
            return format!("{}-{:02}-{:02}", d.year, d.month, d.day);
        };
        let out = match style {
            DateStyle::Full => self
                .full_date
                .as_ref()
                .map(|f| f.format(&date).write_to_string().into_owned()),
            _ => {
                let ix = match style {
                    DateStyle::Short => 0,
                    DateStyle::Medium => 1,
                    _ => 2,
                };
                self.dates[ix]
                    .as_ref()
                    .map(|f| f.format(&date).write_to_string().into_owned())
            }
        };
        out.unwrap_or_else(|| format!("{}-{:02}-{:02}", d.year, d.month, d.day))
    }

    fn time(&self, d: &DateValue, style: DateStyle) -> String {
        let Ok(time) = Time::try_new(d.hour, d.minute, d.second, 0) else {
            return format!("{}:{:02}", d.hour, d.minute);
        };
        let f = match style {
            DateStyle::Short => &self.time_hm,
            _ => &self.time_hms,
        };
        f.as_ref()
            .map(|f| f.format(&time).write_to_string().into_owned())
            .unwrap_or_else(|| format!("{}:{:02}", d.hour, d.minute))
    }
}
