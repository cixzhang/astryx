//! ICU MessageFormat: the syntax Astryx's catalogs use.
//!
//! Supported: literal text with ICU apostrophe quoting, `{name}`,
//! `{name, number[, integer|percent]}`, `{name, date[, short|medium|long|full]}`,
//! `{name, time[, short|medium]}`, `{name, plural, [offset:n] =N {...} one {...} other {...}}`
//! with `#`, `{name, selectordinal, ...}`, and `{name, select, a {...} other {...}}`.
//! Formatting is locale-aware through [`Formats`], so the parser has no
//! locale data of its own.

use std::collections::HashMap;
use std::fmt;

/// A value for one message argument.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Num(f64),
    Date(DateValue),
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
    }
}
macro_rules! num_from {
    ($($t:ty),*) => {$(
        impl From<$t> for Value {
            fn from(n: $t) -> Self {
                Value::Num(n as f64)
            }
        }
    )*};
}
num_from!(i32, i64, u32, u64, usize, f32, f64);
impl From<DateValue> for Value {
    fn from(d: DateValue) -> Self {
        Value::Date(d)
    }
}

/// A calendar date and wall-clock time, in the ISO calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct DateValue {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateValue {
    pub const fn ymd(year: i32, month: u8, day: u8) -> Self {
        DateValue {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
    pub const fn with_time(mut self, hour: u8, minute: u8) -> Self {
        self.hour = hour;
        self.minute = minute;
        self
    }
}

/// Message arguments by name.
pub type Args = HashMap<String, Value>;

/// Builds [`Args`]: `args!{"count" => 3, "name" => "Ada"}`.
#[macro_export]
macro_rules! args {
    () => { $crate::i18n::Args::new() };
    ($($k:expr => $v:expr),+ $(,)?) => {{
        let mut a = $crate::i18n::Args::new();
        $( a.insert(String::from($k), $crate::i18n::Value::from($v)); )+
        a
    }};
}

/// A length of a date or time pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateStyle {
    Short,
    Medium,
    Long,
    Full,
}

/// A style of number argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberStyle {
    Decimal,
    Integer,
    Percent,
    /// An ICU number skeleton, such as `::.0 group-off`.
    Skeleton(Skeleton),
}

/// The parts of an ICU number skeleton Astryx's catalogs use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Skeleton {
    pub min_fraction: i16,
    pub max_fraction: i16,
    pub grouping: bool,
    pub percent: bool,
}

impl Skeleton {
    /// Reads the tokens after `::`. Unknown tokens are ignored.
    pub fn parse(tokens: &str) -> Skeleton {
        let mut sk = Skeleton {
            min_fraction: 0,
            max_fraction: 3,
            grouping: true,
            percent: false,
        };
        for token in tokens.split_whitespace() {
            match token {
                "group-off" | ",_" => sk.grouping = false,
                "percent" | "%" => sk.percent = true,
                "precision-integer" => {
                    sk.min_fraction = 0;
                    sk.max_fraction = 0;
                }
                t if t.starts_with('.') && t[1..].chars().all(|c| c == '0' || c == '#') => {
                    let digits = &t[1..];
                    sk.min_fraction = digits.chars().filter(|&c| c == '0').count() as i16;
                    sk.max_fraction = digits.len() as i16;
                }
                _ => {}
            }
        }
        sk
    }
}

/// A plural category, as CLDR names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl Category {
    fn keyword(self) -> &'static str {
        match self {
            Category::Zero => "zero",
            Category::One => "one",
            Category::Two => "two",
            Category::Few => "few",
            Category::Many => "many",
            Category::Other => "other",
        }
    }
}

/// The locale data a message needs.
pub trait Formats {
    fn number(&self, n: f64, style: NumberStyle) -> String;
    fn plural(&self, n: f64, ordinal: bool) -> Category;
    fn date(&self, d: &DateValue, style: DateStyle) -> String;
    fn time(&self, d: &DateValue, style: DateStyle) -> String;
}

/// A syntax error, with the byte offset where parsing stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub reason: &'static str,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.reason, self.offset)
    }
}

impl std::error::Error for ParseError {}

#[derive(Clone, Debug, PartialEq)]
enum Selector {
    Exact(f64),
    Keyword(String),
}

#[derive(Clone, Debug, PartialEq)]
enum Part {
    Text(String),
    Pound,
    Arg(String),
    Number(String, NumberStyle),
    Date(String, DateStyle),
    Time(String, DateStyle),
    Plural {
        name: String,
        ordinal: bool,
        offset: f64,
        cases: Vec<(Selector, Vec<Part>)>,
    },
    Select {
        name: String,
        cases: Vec<(String, Vec<Part>)>,
    },
}

/// A parsed message, ready to format many times.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    parts: Vec<Part>,
}

impl Message {
    pub fn parse(source: &str) -> Result<Message, ParseError> {
        let mut p = Parser {
            s: source.as_bytes(),
            src: source,
            i: 0,
        };
        let parts = p.parts(false)?;
        if p.i < p.s.len() {
            return Err(p.err("unmatched '}'"));
        }
        Ok(Message { parts })
    }

    /// The message with `args` filled in. A missing argument prints its
    /// name in braces, so a gap shows instead of failing.
    pub fn format(&self, args: &Args, formats: &dyn Formats) -> String {
        let mut out = String::new();
        write_parts(&self.parts, args, formats, None, &mut out);
        out
    }
}

fn write_parts(parts: &[Part], args: &Args, f: &dyn Formats, pound: Option<f64>, out: &mut String) {
    for part in parts {
        match part {
            Part::Text(t) => out.push_str(t),
            Part::Pound => match pound {
                Some(n) => out.push_str(&f.number(n, NumberStyle::Decimal)),
                None => out.push('#'),
            },
            Part::Arg(name) => match args.get(name) {
                Some(Value::Str(s)) => out.push_str(s),
                Some(Value::Num(n)) => out.push_str(&f.number(*n, NumberStyle::Decimal)),
                Some(Value::Date(d)) => out.push_str(&f.date(d, DateStyle::Short)),
                None => missing(name, out),
            },
            Part::Number(name, style) => match args.get(name) {
                Some(Value::Num(n)) => out.push_str(&f.number(*n, *style)),
                Some(Value::Str(s)) => match s.parse::<f64>() {
                    Ok(n) => out.push_str(&f.number(n, *style)),
                    Err(_) => out.push_str(s),
                },
                _ => missing(name, out),
            },
            Part::Date(name, style) | Part::Time(name, style) => match args.get(name) {
                Some(Value::Date(d)) => {
                    let text = if matches!(part, Part::Date(..)) {
                        f.date(d, *style)
                    } else {
                        f.time(d, *style)
                    };
                    out.push_str(&text);
                }
                Some(Value::Str(s)) => out.push_str(s),
                _ => missing(name, out),
            },
            Part::Plural {
                name,
                ordinal,
                offset,
                cases,
            } => {
                let n = match args.get(name) {
                    Some(Value::Num(n)) => *n,
                    Some(Value::Str(s)) => s.parse().unwrap_or(0.0),
                    _ => {
                        missing(name, out);
                        continue;
                    }
                };
                let exact = cases
                    .iter()
                    .find(|(s, _)| matches!(s, Selector::Exact(v) if *v == n));
                let chosen = exact.or_else(|| {
                    let keyword = f.plural(n - offset, *ordinal).keyword();
                    cases
                        .iter()
                        .find(|(s, _)| matches!(s, Selector::Keyword(k) if k == keyword))
                });
                let chosen = chosen.or_else(|| {
                    cases
                        .iter()
                        .find(|(s, _)| matches!(s, Selector::Keyword(k) if k == "other"))
                });
                if let Some((_, body)) = chosen {
                    write_parts(body, args, f, Some(n - offset), out);
                }
            }
            Part::Select { name, cases } => {
                let key = match args.get(name) {
                    Some(Value::Str(s)) => s.clone(),
                    Some(Value::Num(n)) => n.to_string(),
                    _ => String::new(),
                };
                let chosen = cases
                    .iter()
                    .find(|(k, _)| *k == key)
                    .or_else(|| cases.iter().find(|(k, _)| k == "other"));
                if let Some((_, body)) = chosen {
                    write_parts(body, args, f, pound, out);
                }
            }
        }
    }
}

fn missing(name: &str, out: &mut String) {
    out.push('{');
    out.push_str(name);
    out.push('}');
}

struct Parser<'a> {
    s: &'a [u8],
    src: &'a str,
    i: usize,
}

impl Parser<'_> {
    fn err(&self, reason: &'static str) -> ParseError {
        ParseError {
            offset: self.i,
            reason,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }

    /// Text and arguments up to an unquoted `}` or the end. `in_plural`
    /// makes `#` the plural number.
    fn parts(&mut self, in_plural: bool) -> Result<Vec<Part>, ParseError> {
        let mut parts = Vec::new();
        let mut text = String::new();
        while let Some(c) = self.peek() {
            match c {
                b'{' => {
                    if !text.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut text)));
                    }
                    self.i += 1;
                    parts.push(self.argument()?);
                }
                b'}' => break,
                b'#' if in_plural => {
                    if !text.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut text)));
                    }
                    self.i += 1;
                    parts.push(Part::Pound);
                }
                b'\'' => self.apostrophe(in_plural, &mut text),
                _ => {
                    let ch = self.src[self.i..].chars().next().expect("char boundary");
                    text.push(ch);
                    self.i += ch.len_utf8();
                }
            }
        }
        if !text.is_empty() {
            parts.push(Part::Text(text));
        }
        Ok(parts)
    }

    /// ICU quoting: `''` is one apostrophe; an apostrophe before a syntax
    /// character starts a quoted run up to the next lone apostrophe; any
    /// other apostrophe is itself.
    fn apostrophe(&mut self, in_plural: bool, text: &mut String) {
        self.i += 1;
        match self.peek() {
            Some(b'\'') => {
                text.push('\'');
                self.i += 1;
            }
            Some(b'{') | Some(b'}') | Some(b'|') => self.quoted(text),
            Some(b'#') if in_plural => self.quoted(text),
            _ => text.push('\''),
        }
    }

    fn quoted(&mut self, text: &mut String) {
        while let Some(c) = self.peek() {
            if c == b'\'' {
                if self.s.get(self.i + 1) == Some(&b'\'') {
                    text.push('\'');
                    self.i += 2;
                    continue;
                }
                self.i += 1;
                return;
            }
            let ch = self.src[self.i..].chars().next().expect("char boundary");
            text.push(ch);
            self.i += ch.len_utf8();
        }
    }

    fn word(&mut self) -> String {
        self.skip_ws();
        let start = self.i;
        while self
            .peek()
            .is_some_and(|c| !c.is_ascii_whitespace() && !b"{},".contains(&c))
        {
            self.i += 1;
        }
        self.src[start..self.i].to_string()
    }

    /// A format style: everything up to the argument's closing `}`, trimmed.
    fn style_text(&mut self) -> String {
        let start = self.i;
        while self.peek().is_some_and(|c| c != b'}') {
            self.i += 1;
        }
        self.src[start..self.i].trim().to_string()
    }

    fn expect(&mut self, c: u8, reason: &'static str) -> Result<(), ParseError> {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(self.err(reason))
        }
    }

    /// After `{`: an argument through its closing `}`.
    fn argument(&mut self) -> Result<Part, ParseError> {
        let name = self.word();
        if name.is_empty() {
            return Err(self.err("argument name expected"));
        }
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Part::Arg(name));
        }
        self.expect(b',', "',' or '}' expected")?;
        let kind = self.word();
        self.skip_ws();
        let part = match kind.as_str() {
            "number" | "date" | "time" => {
                let style = if self.peek() == Some(b',') {
                    self.i += 1;
                    self.style_text()
                } else {
                    String::new()
                };
                match kind.as_str() {
                    "number" => Part::Number(
                        name,
                        match style.as_str() {
                            "integer" => NumberStyle::Integer,
                            "percent" => NumberStyle::Percent,
                            s if s.starts_with("::") => {
                                NumberStyle::Skeleton(Skeleton::parse(&s[2..]))
                            }
                            _ => NumberStyle::Decimal,
                        },
                    ),
                    "date" => Part::Date(name, date_style(&style)),
                    _ => Part::Time(name, date_style(&style)),
                }
            }
            "plural" | "selectordinal" => {
                self.expect(b',', "',' expected after plural")?;
                let mut offset = 0.0;
                self.skip_ws();
                if self.src[self.i..].starts_with("offset:") {
                    self.i += "offset:".len();
                    offset = self
                        .word()
                        .parse()
                        .map_err(|_| self.err("number expected after offset:"))?;
                }
                let mut cases = Vec::new();
                loop {
                    self.skip_ws();
                    if self.peek() == Some(b'}') || self.peek().is_none() {
                        break;
                    }
                    let sel = self.word();
                    let selector = match sel.strip_prefix('=') {
                        Some(v) => Selector::Exact(
                            v.parse()
                                .map_err(|_| self.err("number expected after '='"))?,
                        ),
                        None => Selector::Keyword(sel),
                    };
                    self.expect(b'{', "'{' expected for a plural case")?;
                    let body = self.parts(true)?;
                    self.expect(b'}', "'}' expected after a plural case")?;
                    cases.push((selector, body));
                }
                if !cases
                    .iter()
                    .any(|(s, _)| matches!(s, Selector::Keyword(k) if k == "other"))
                {
                    return Err(self.err("plural needs an 'other' case"));
                }
                Part::Plural {
                    name,
                    ordinal: kind == "selectordinal",
                    offset,
                    cases,
                }
            }
            "select" => {
                self.expect(b',', "',' expected after select")?;
                let mut cases = Vec::new();
                loop {
                    self.skip_ws();
                    if self.peek() == Some(b'}') || self.peek().is_none() {
                        break;
                    }
                    let key = self.word();
                    self.expect(b'{', "'{' expected for a select case")?;
                    let body = self.parts(false)?;
                    self.expect(b'}', "'}' expected after a select case")?;
                    cases.push((key, body));
                }
                if !cases.iter().any(|(k, _)| k == "other") {
                    return Err(self.err("select needs an 'other' case"));
                }
                Part::Select { name, cases }
            }
            _ => return Err(self.err("unknown argument type")),
        };
        self.expect(b'}', "'}' expected")?;
        Ok(part)
    }
}

fn date_style(s: &str) -> DateStyle {
    match s {
        "medium" => DateStyle::Medium,
        "long" => DateStyle::Long,
        "full" => DateStyle::Full,
        _ => DateStyle::Short,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// English-like rules with plain number output, so tests read clearly.
    struct Plain;
    impl Formats for Plain {
        fn number(&self, n: f64, style: NumberStyle) -> String {
            match style {
                NumberStyle::Percent => format!("{}%", n * 100.0),
                NumberStyle::Integer => format!("{}", n.round()),
                NumberStyle::Decimal => format!("{n}"),
                NumberStyle::Skeleton(sk) => format!("{n:.*}", sk.min_fraction as usize),
            }
        }
        fn plural(&self, n: f64, ordinal: bool) -> Category {
            if ordinal {
                return match (n as i64) % 10 {
                    1 => Category::One,
                    2 => Category::Two,
                    3 => Category::Few,
                    _ => Category::Other,
                };
            }
            if n == 1.0 {
                Category::One
            } else {
                Category::Other
            }
        }
        fn date(&self, d: &DateValue, _: DateStyle) -> String {
            format!("{}-{}-{}", d.year, d.month, d.day)
        }
        fn time(&self, d: &DateValue, _: DateStyle) -> String {
            format!("{}:{:02}", d.hour, d.minute)
        }
    }

    fn fmt(src: &str, args: Args) -> String {
        Message::parse(src).unwrap().format(&args, &Plain)
    }

    #[test]
    fn simple_arguments_and_text() {
        assert_eq!(
            fmt("Hello, {name}!", crate::args! {"name" => "Ada"}),
            "Hello, Ada!"
        );
        assert_eq!(
            fmt("Page {page, number}", crate::args! {"page" => 3}),
            "Page 3"
        );
        assert_eq!(fmt("No args", Args::new()), "No args");
        assert_eq!(fmt("Missing {who}", Args::new()), "Missing {who}");
    }

    #[test]
    fn plural_with_exact_match_offset_and_pound() {
        let src = "{n, plural, =0 {no pages} one {# page} other {# pages}}";
        assert_eq!(fmt(src, crate::args! {"n" => 0}), "no pages");
        assert_eq!(fmt(src, crate::args! {"n" => 1}), "1 page");
        assert_eq!(fmt(src, crate::args! {"n" => 7}), "7 pages");
        let off = "{n, plural, offset:1 =0 {nobody} =1 {you} one {you and # other} other {you and # others}}";
        assert_eq!(fmt(off, crate::args! {"n" => 2}), "you and 1 other");
        assert_eq!(fmt(off, crate::args! {"n" => 5}), "you and 4 others");
        assert_eq!(fmt(off, crate::args! {"n" => 1}), "you");
    }

    #[test]
    fn astryx_pagination_message() {
        let src = "Go back {step, number} {step, plural, one {page} other {pages}}";
        assert_eq!(fmt(src, crate::args! {"step" => 1}), "Go back 1 page");
        assert_eq!(fmt(src, crate::args! {"step" => 5}), "Go back 5 pages");
    }

    #[test]
    fn select_and_ordinal() {
        let src = "{g, select, female {She} male {He} other {They}} replied";
        assert_eq!(fmt(src, crate::args! {"g" => "female"}), "She replied");
        assert_eq!(fmt(src, crate::args! {"g" => "x"}), "They replied");
        let ord = "{n, selectordinal, one {#st} two {#nd} few {#rd} other {#th}}";
        assert_eq!(fmt(ord, crate::args! {"n" => 2}), "2nd");
        assert_eq!(fmt(ord, crate::args! {"n" => 4}), "4th");
    }

    #[test]
    fn nested_plural_inside_select() {
        let src = "{who, select, me {{n, plural, one {I have # item} other {I have # items}}} other {{n} things}}";
        assert_eq!(
            fmt(src, crate::args! {"who" => "me", "n" => 1}),
            "I have 1 item"
        );
        assert_eq!(
            fmt(src, crate::args! {"who" => "you", "n" => 3}),
            "3 things"
        );
    }

    #[test]
    fn apostrophe_quoting() {
        assert_eq!(fmt("It''s {x}", crate::args! {"x" => "ok"}), "It's ok");
        assert_eq!(fmt("Use '{braces}' here", Args::new()), "Use {braces} here");
        assert_eq!(fmt("l'école", Args::new()), "l'école");
        assert_eq!(
            fmt("{n, plural, other {'#' is # }}", crate::args! {"n" => 3}),
            "# is 3 "
        );
        assert_eq!(fmt("a # b", Args::new()), "a # b");
    }

    #[test]
    fn dates_times_and_percent() {
        let d = DateValue::ymd(2026, 11, 9).with_time(18, 5);
        assert_eq!(
            fmt(
                "{d, date, medium} at {d, time, short}",
                crate::args! {"d" => d}
            ),
            "2026-11-9 at 18:05"
        );
        assert_eq!(
            fmt("{p, number, percent}", crate::args! {"p" => 0.5}),
            "50%"
        );
    }

    #[test]
    fn number_skeletons() {
        let sk = Skeleton::parse(" .0 group-off");
        assert_eq!(
            (sk.min_fraction, sk.max_fraction, sk.grouping),
            (1, 1, false)
        );
        assert_eq!(
            fmt(
                "{size, number, :: .0 group-off} KB",
                crate::args! {"size" => 2.5}
            ),
            "2.5 KB"
        );
    }

    #[test]
    fn syntax_errors_report_where() {
        assert!(
            Message::parse("{n, plural, one {x}}").is_err(),
            "needs other"
        );
        assert!(Message::parse("{n, wat}").is_err());
        assert!(Message::parse("open {").is_err());
        assert!(Message::parse("close }").is_err());
    }
}
