//! A box's 2D `transform` and `transform-origin` (`rule:core-classes/pdf-css-subset-table`).
//!
//! [`read`] takes the functions in the order they are written, each with its lengths in points
//! and its percentages kept, because those count against the border box, which is known only
//! when the box closes. [`Transform::matrix`] then gives the one matrix the box and its content
//! are drawn under: the functions composed left to right, about the origin. `transform-origin`
//! takes one or two keywords, lengths or percentages, and is the border box's centre when it is
//! missing or does not read. A `transform` with a function or an argument that does not read is
//! `none`, as a value the cascade did not take would be.

use super::boxes::angle;
use crate::css::value::{self, Comp};
use crate::css::{Style, Value};

/// An affine matrix `[a, b, c, d, e, f]`, in points with `y` down the page: a point `x, y` is
/// drawn at `a x + c y + e, b x + d y + f`.
pub type Matrix = [f32; 6];

pub const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// A length along one axis: points, and a percentage of the border box along it.
type Along = (f32, f32);

#[derive(Debug, Clone, PartialEq)]
enum Step {
    Matrix(Matrix),
    Translate(Along, Along),
    Scale(f32, f32),
    Rotate(f32),
    Skew(f32, f32),
}

/// A box's `transform`: its functions in order and its origin, across and down.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Transform {
    steps: Vec<Step>,
    origin: [Along; 2],
}

/// `m` after `n`: the matrix that draws a point under `n` and then under `m`.
pub fn compose(m: Matrix, n: Matrix) -> Matrix {
    [
        m[0] * n[0] + m[2] * n[1],
        m[1] * n[0] + m[3] * n[1],
        m[0] * n[2] + m[2] * n[3],
        m[1] * n[2] + m[3] * n[3],
        m[0] * n[4] + m[2] * n[5] + m[4],
        m[1] * n[4] + m[3] * n[5] + m[5],
    ]
}

fn translate(x: f32, y: f32) -> Matrix {
    [1.0, 0.0, 0.0, 1.0, x, y]
}

/// `m` for content moved right by `dx` and down by `dy`: the same matrix about the moved point.
pub fn shift(m: Matrix, dx: f32, dy: f32) -> Matrix {
    compose(translate(dx, dy), compose(m, translate(-dx, -dy)))
}

impl Transform {
    /// The matrix for a border box at `x`, `y`, `width` wide and `height` tall.
    pub(super) fn matrix(&self, x: f32, y: f32, width: f32, height: f32) -> Matrix {
        let at = |(pt, percent): Along, basis: f32| pt + percent / 100.0 * basis;
        let mut m = IDENTITY;
        for step in &self.steps {
            let n = match *step {
                Step::Matrix(n) => n,
                Step::Translate(tx, ty) => translate(at(tx, width), at(ty, height)),
                Step::Scale(sx, sy) => [sx, 0.0, 0.0, sy, 0.0, 0.0],
                Step::Rotate(a) => [a.cos(), a.sin(), -a.sin(), a.cos(), 0.0, 0.0],
                Step::Skew(ax, ay) => [1.0, ay.tan(), ax.tan(), 1.0, 0.0, 0.0],
            };
            m = compose(m, n);
        }
        let ox = x + at(self.origin[0], width);
        let oy = y + at(self.origin[1], height);
        compose(translate(ox, oy), compose(m, translate(-ox, -oy)))
    }
}

/// `style`'s `transform`, or `None` for `none` and a value that does not read.
pub(super) fn read(style: &Style, rem: f32) -> Option<Transform> {
    let Some(Value::Text(text)) = style.get("transform") else {
        return None;
    };
    let em = style.font_size();
    let steps = value::components(text)?
        .iter()
        .map(|c| match c {
            Comp::Function(name, args) => step(name, args, em, rem),
            _ => None,
        })
        .collect::<Option<Vec<Step>>>()?;
    Some(Transform {
        steps,
        origin: origin(style, em, rem).unwrap_or([(0.0, 50.0); 2]),
    })
}

/// One transform function from its name and argument text.
fn step(name: &str, args: &str, em: f32, rem: f32) -> Option<Step> {
    let args: Vec<Comp> = value::components(args)?
        .into_iter()
        .filter(|c| *c != Comp::Comma)
        .collect();
    let along = |c: &Comp| {
        value::length(c).map(|sum| {
            let l = sum.compute(em, rem);
            (l.pt, l.percent)
        })
    };
    let number = |c: &Comp| match c {
        Comp::Number(n, _) => Some(*n),
        _ => None,
    };
    let turn = |c: &Comp| match c {
        Comp::Dimension(v, unit) => angle(*v, unit),
        Comp::Number(n, _) if *n == 0.0 => Some(0.0),
        _ => None,
    };
    const NONE: Along = (0.0, 0.0);
    Some(match (name, args.as_slice()) {
        ("matrix", [a, b, c, d, e, f]) => Step::Matrix([
            number(a)?,
            number(b)?,
            number(c)?,
            number(d)?,
            number(e)?,
            number(f)?,
        ]),
        ("translate", [x]) => Step::Translate(along(x)?, NONE),
        ("translate", [x, y]) => Step::Translate(along(x)?, along(y)?),
        ("translatex", [x]) => Step::Translate(along(x)?, NONE),
        ("translatey", [y]) => Step::Translate(NONE, along(y)?),
        ("scale", [s]) => Step::Scale(number(s)?, number(s)?),
        ("scale", [x, y]) => Step::Scale(number(x)?, number(y)?),
        ("scalex", [x]) => Step::Scale(number(x)?, 1.0),
        ("scaley", [y]) => Step::Scale(1.0, number(y)?),
        ("rotate", [a]) => Step::Rotate(turn(a)?),
        ("skew", [x]) => Step::Skew(turn(x)?, 0.0),
        ("skew", [x, y]) => Step::Skew(turn(x)?, turn(y)?),
        ("skewx", [x]) => Step::Skew(turn(x)?, 0.0),
        ("skewy", [y]) => Step::Skew(0.0, turn(y)?),
        _ => return None,
    })
}

/// `style`'s `transform-origin` across and down. A keyword that names one axis sets that axis
/// whichever place it is in, and an axis the value does not give is centred.
fn origin(style: &Style, em: f32, rem: f32) -> Option<[Along; 2]> {
    let Some(Value::Text(text)) = style.get("transform-origin") else {
        return None;
    };
    let comps = value::components(text)?;
    const CENTRE: Along = (0.0, 50.0);
    let side = |c: &Comp| match c {
        Comp::Ident(word) => match word.as_str() {
            "left" | "top" => Some((0.0, 0.0)),
            "center" => Some(CENTRE),
            "right" | "bottom" => Some((0.0, 100.0)),
            _ => None,
        },
        c => value::length(c).map(|sum| {
            let l = sum.compute(em, rem);
            (l.pt, l.percent)
        }),
    };
    let down = |c: &Comp| matches!(c, Comp::Ident(w) if w == "top" || w == "bottom");
    let across = |c: &Comp| matches!(c, Comp::Ident(w) if w == "left" || w == "right");
    match comps.as_slice() {
        [one] if down(one) => Some([CENTRE, side(one)?]),
        [one] => Some([side(one)?, CENTRE]),
        // A third value is a depth, which a 2D transform does not have.
        [a, b, ..] if down(a) || across(b) => Some([side(b)?, side(a)?]),
        [a, b, ..] => Some([side(a)?, side(b)?]),
        [] => None,
    }
}
