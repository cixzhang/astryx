//! Reads SVG path data (`d`) into path commands, for icons given as SVG.

use astryx_core::contract::PathCmd;
use astryx_core::Point;

fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}

struct Lexer<'a> {
    s: &'a [u8],
    i: usize,
}

impl Lexer<'_> {
    fn skip(&mut self) {
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_whitespace() || self.s[self.i] == b',')
        {
            self.i += 1;
        }
    }
    fn command(&mut self) -> Option<u8> {
        self.skip();
        let c = *self.s.get(self.i)?;
        if c.is_ascii_alphabetic() {
            self.i += 1;
            Some(c)
        } else {
            None
        }
    }
    fn at_number(&mut self) -> bool {
        self.skip();
        self.s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.'))
    }
    fn number(&mut self) -> Option<f32> {
        self.skip();
        let start = self.i;
        let mut seen_dot = false;
        let mut seen_exp = false;
        if matches!(self.s.get(self.i), Some(b'-' | b'+')) {
            self.i += 1;
        }
        while let Some(&c) = self.s.get(self.i) {
            match c {
                b'0'..=b'9' => self.i += 1,
                b'.' if !seen_dot && !seen_exp => {
                    seen_dot = true;
                    self.i += 1;
                }
                b'e' | b'E' if !seen_exp => {
                    seen_exp = true;
                    self.i += 1;
                    if matches!(self.s.get(self.i), Some(b'-' | b'+')) {
                        self.i += 1;
                    }
                }
                _ => break,
            }
        }
        std::str::from_utf8(&self.s[start..self.i])
            .ok()?
            .parse()
            .ok()
    }
    /// An arc flag: one digit, which may touch the next number.
    fn flag(&mut self) -> Option<bool> {
        self.skip();
        let c = *self.s.get(self.i)?;
        self.i += 1;
        Some(c == b'1')
    }
}

/// Converts an elliptical arc to cubic curves (SVG implementation notes,
/// F.6), pushing them onto `out`.
#[allow(clippy::too_many_arguments)]
fn arc(
    out: &mut Vec<PathCmd>,
    from: Point,
    rx: f32,
    ry: f32,
    rot: f32,
    large: bool,
    sweep: bool,
    to: Point,
) {
    if (from.x - to.x).abs() < 1e-6 && (from.y - to.y).abs() < 1e-6 {
        return;
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-6 || ry < 1e-6 {
        out.push(PathCmd::LineTo(to));
        return;
    }
    let phi = rot.to_radians();
    let (sin, cos) = phi.sin_cos();
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1 = cos * dx + sin * dy;
    let y1 = -sin * dx + cos * dy;
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let num = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coef = (num / den).sqrt();
    if large == sweep {
        coef = -coef;
    }
    let cx1 = coef * rx * y1 / ry;
    let cy1 = -coef * ry * x1 / rx;
    let cx = cos * cx1 - sin * cy1 + (from.x + to.x) / 2.0;
    let cy = sin * cx1 + cos * cy1 + (from.y + to.y) / 2.0;
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let t1 = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut dt = angle(
        (x1 - cx1) / rx,
        (y1 - cy1) / ry,
        (-x1 - cx1) / rx,
        (-y1 - cy1) / ry,
    );
    if !sweep && dt > 0.0 {
        dt -= std::f32::consts::TAU;
    } else if sweep && dt < 0.0 {
        dt += std::f32::consts::TAU;
    }
    let segments = (dt.abs() / (std::f32::consts::FRAC_PI_2)).ceil().max(1.0) as usize;
    let step = dt / segments as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let point = |t: f32| {
        let (s, c) = t.sin_cos();
        (rx * c, ry * s)
    };
    let map = |x: f32, y: f32| p(cos * x - sin * y + cx, sin * x + cos * y + cy);
    let mut t = t1;
    for _ in 0..segments {
        let (x0, y0) = point(t);
        let (x3, y3) = point(t + step);
        let (s0, c0) = t.sin_cos();
        let (s3, c3) = (t + step).sin_cos();
        let c1 = map(x0 - k * rx * s0, y0 + k * ry * c0);
        let c2 = map(x3 + k * rx * s3, y3 - k * ry * c3);
        out.push(PathCmd::CubicTo(c1, c2, map(x3, y3)));
        t += step;
    }
}

/// The commands for SVG path data; `None` when it does not parse.
pub fn parse_svg_path(d: &str) -> Option<Vec<PathCmd>> {
    let mut lx = Lexer {
        s: d.as_bytes(),
        i: 0,
    };
    let mut out = Vec::new();
    let mut cur = p(0.0, 0.0);
    let mut start = cur;
    let mut last_ctrl: Option<Point> = None;
    let mut last_quad: Option<Point> = None;
    let mut cmd = lx.command()?;
    loop {
        let rel = cmd.is_ascii_lowercase();
        let base = if rel { cur } else { p(0.0, 0.0) };
        let pt = |lx: &mut Lexer, b: Point| -> Option<Point> {
            Some(p(b.x + lx.number()?, b.y + lx.number()?))
        };
        let mut ctrl = None;
        let mut quad = None;
        match cmd.to_ascii_uppercase() {
            b'M' => {
                cur = pt(&mut lx, base)?;
                start = cur;
                out.push(PathCmd::MoveTo(cur));
                // Further pairs are line-tos.
                cmd = if rel { b'l' } else { b'L' };
                if !lx.at_number() {
                    cmd = lx.command().unwrap_or(0);
                    if cmd == 0 {
                        break;
                    }
                }
                last_ctrl = None;
                last_quad = None;
                continue;
            }
            b'L' => {
                cur = pt(&mut lx, base)?;
                out.push(PathCmd::LineTo(cur));
            }
            b'H' => {
                cur = p(if rel { cur.x } else { 0.0 } + lx.number()?, cur.y);
                out.push(PathCmd::LineTo(cur));
            }
            b'V' => {
                cur = p(cur.x, if rel { cur.y } else { 0.0 } + lx.number()?);
                out.push(PathCmd::LineTo(cur));
            }
            b'C' => {
                let c1 = pt(&mut lx, base)?;
                let c2 = pt(&mut lx, base)?;
                cur = pt(&mut lx, base)?;
                out.push(PathCmd::CubicTo(c1, c2, cur));
                ctrl = Some(c2);
            }
            b'S' => {
                let c1 = last_ctrl.map_or(cur, |c| p(2.0 * cur.x - c.x, 2.0 * cur.y - c.y));
                let c2 = pt(&mut lx, base)?;
                cur = pt(&mut lx, base)?;
                out.push(PathCmd::CubicTo(c1, c2, cur));
                ctrl = Some(c2);
            }
            b'Q' => {
                let c = pt(&mut lx, base)?;
                cur = pt(&mut lx, base)?;
                out.push(PathCmd::QuadTo(c, cur));
                quad = Some(c);
            }
            b'T' => {
                let c = last_quad.map_or(cur, |q| p(2.0 * cur.x - q.x, 2.0 * cur.y - q.y));
                cur = pt(&mut lx, base)?;
                out.push(PathCmd::QuadTo(c, cur));
                quad = Some(c);
            }
            b'A' => {
                let rx = lx.number()?;
                let ry = lx.number()?;
                let rot = lx.number()?;
                let large = lx.flag()?;
                let sweep = lx.flag()?;
                let to = pt(&mut lx, base)?;
                arc(&mut out, cur, rx, ry, rot, large, sweep, to);
                cur = to;
            }
            b'Z' => {
                out.push(PathCmd::Close);
                cur = start;
            }
            _ => return None,
        }
        last_ctrl = ctrl;
        last_quad = quad;
        if !cmd.eq_ignore_ascii_case(&b'Z') && lx.at_number() {
            continue;
        }
        match lx.command() {
            Some(c) => cmd = c,
            None => break,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_relative_commands_and_arcs() {
        let cmds = parse_svg_path(
            "M5.25 5.653c0-.856.917-1.398 1.667-.986l11.54 6.347a1.125 1.125 0 0 1 0 1.972Z",
        )
        .unwrap();
        assert!(matches!(cmds[0], PathCmd::MoveTo(Point { x, y }) if x == 5.25 && y == 5.653));
        assert!(matches!(cmds[1], PathCmd::CubicTo(..)));
        assert!(matches!(cmds.last(), Some(PathCmd::Close)));
        let h = parse_svg_path("M2 2h4v4H2z").unwrap();
        assert_eq!(h.len(), 5);
    }
}
