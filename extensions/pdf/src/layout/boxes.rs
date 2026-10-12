//! What a block box draws under its content: its background colour and image, its borders, its
//! outline and its rounded corners (`rule:core-classes/pdf-css-subset-table`).
//!
//! Every shape is a path [`Item`], filled with the even-odd rule or stroked. The background
//! colour and the background image fill the border box, rounded by `border-radius`. A corner is
//! one radius, and a percentage is of the box's width across and of its height down, so a
//! corner of a box that is not square is a quarter of an ellipse. Radii that do not fit their side
//! are all scaled down by the same factor, as CSS says.
//!
//! **Backgrounds.** `linear-gradient` takes an angle or a `to` side or corner, and `radial-gradient`
//! takes `circle` or `ellipse` and an extent keyword, centred in the padding box. Both take colour
//! stops with a length or percentage, and a stop with none is spaced evenly between its
//! neighbours. A stop past either end of the gradient line is drawn at that end. Any other image
//! function, and a gradient written another way, draws nothing. An image `url()` is laid in tiles
//! at its natural size from the padding box's top-left corner, repeated as `background-repeat`
//! says, where `space` and `round` repeat as `repeat` does. `background-position` and
//! `background-size` are not laid out yet.
//!
//! **Borders.** A solid side is a filled rectangle, a double side two rectangles a third of its
//! width each, and a dashed or dotted side a stroked line along its middle: a dash and the gap
//! after it are each twice the width, and a dot is the width across with its centre two widths
//! from the next. When all four sides are alike, a rounded or a dashed, dotted or double border is
//! drawn as one shape that follows the corners; sides that differ are drawn square. An outline is
//! drawn as a border would be, outside the border box by `outline-offset`, its corners following
//! the box's. A slice of a box that goes on past a page is open on that side: no border, no
//! outline edge and square corners.
//!
//! `opacity` is not drawn here: [`super::block::Flow`] wraps the box and its content in an
//! [`Item::Begin`] and an [`Item::End`].

use std::f32::consts::{FRAC_PI_2, PI};

use crate::css::value::{self, Comp};
use crate::css::{Color, Length, Style, Value};

use super::{BLACK, BoxGeometry, Item, color, keyword, length};

/// One step of a path, in points with `y` down the page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    /// A cubic Bézier curve: its two control points, then its end.
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// A gradient's colour at `offset`, 0 at the start of its line and 1 at the end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop {
    pub offset: f32,
    pub color: Color,
}

/// What a path is filled or stroked with.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    Color(Color),
    /// A linear gradient from `x1`, `y1` to `x2`, `y2`, its end colours going on past both ends.
    Linear {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        stops: Vec<Stop>,
    },
    /// A radial gradient: a circle of radius `r` at `cx`, `cy`, stretched down by `scale_y`
    /// about its centre.
    Radial {
        cx: f32,
        cy: f32,
        r: f32,
        scale_y: f32,
        stops: Vec<Stop>,
    },
    /// The image at `image` in [`super::Laid::images`] in tiles `width` by `height`, one of
    /// them with its top-left corner at `x`, `y`.
    Tiles {
        image: usize,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}

/// How a path is stroked: its width, its dash and gap lengths, empty for a solid line, and
/// whether its line ends are round.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub dash: Vec<f32>,
    pub round: bool,
}

/// A border side's or an outline's line style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Line {
    Solid,
    Dashed,
    Dotted,
    Double,
}

/// Which way a linear gradient runs: an angle, clockwise from up, or toward a corner, each
/// coordinate -1 or 1 with `y` down.
#[derive(Debug, Clone, Copy)]
pub(super) enum Direction {
    Angle(f32),
    Corner(f32, f32),
}

/// How far a radial gradient's ending shape reaches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Extent {
    ClosestSide,
    FarthestSide,
    ClosestCorner,
    FarthestCorner,
}

/// A colour stop as written: its colour and its position, if it has one.
type Written = (Color, Option<Length>);

/// The image behind a box's content.
#[derive(Debug, Clone)]
pub(super) enum Layer {
    Linear {
        to: Direction,
        stops: Vec<Written>,
    },
    Radial {
        circle: bool,
        extent: Extent,
        stops: Vec<Written>,
    },
    Image {
        image: usize,
        width: f32,
        height: f32,
        repeat_x: bool,
        repeat_y: bool,
    },
}

#[derive(Debug, Clone)]
pub(super) struct Outline {
    width: f32,
    line: Line,
    color: Color,
    offset: f32,
}

/// What a block box draws besides its content. The border's widths are its
/// [`BoxGeometry::border`]; the rest is here, each side top, right, bottom, left, and each corner
/// top-left first and clockwise.
#[derive(Debug, Clone)]
pub(super) struct Decor {
    pub background: Option<Color>,
    pub layer: Option<Layer>,
    pub line: [Line; 4],
    pub border_color: [Color; 4],
    pub radius: [Length; 4],
    pub outline: Option<Outline>,
    pub opacity: f32,
}

fn line_of(word: Option<&str>) -> Option<Line> {
    match word? {
        "solid" => Some(Line::Solid),
        "dashed" => Some(Line::Dashed),
        "dotted" => Some(Line::Dotted),
        "double" => Some(Line::Double),
        _ => None,
    }
}

/// `style`'s paint, all but its background image, which [`super::inline::Builder`] loads.
pub(super) fn decor(style: &Style) -> Decor {
    let text_color = color(style, "color").unwrap_or(BLACK);
    let sides = super::SIDES;
    let outline = line_of(keyword(style, "outline-style")).map(|line| Outline {
        width: match style.get("outline-width") {
            Some(Value::Length(l)) => l.pt,
            Some(Value::Keyword(w)) if w == "thin" => 0.75,
            Some(Value::Keyword(w)) if w == "thick" => 3.75,
            _ => 2.25,
        },
        line,
        color: color(style, "outline-color").unwrap_or(text_color),
        offset: length(style, "outline-offset", 0.0).unwrap_or(0.0),
    });
    Decor {
        background: color(style, "background-color"),
        layer: None,
        line: sides
            .map(|s| line_of(keyword(style, &format!("border-{s}-style"))).unwrap_or(Line::Solid)),
        border_color: sides
            .map(|s| color(style, &format!("border-{s}-color")).unwrap_or(text_color)),
        radius: ["top-left", "top-right", "bottom-right", "bottom-left"].map(|c| {
            match style.get(&format!("border-{c}-radius")) {
                Some(Value::Length(l)) => *l,
                _ => Length::default(),
            }
        }),
        outline: outline.filter(|o| o.width > 0.0),
        opacity: match style.get("opacity") {
            Some(Value::Number(n)) => n.clamp(0.0, 1.0),
            _ => 1.0,
        },
    }
}

/// The top-level components of `style`'s `background-image`, if it has one.
fn background_image(style: &Style) -> Option<Vec<Comp>> {
    match style.get("background-image") {
        Some(Value::Text(text)) => value::components(text),
        _ => None,
    }
}

/// The reference of `style`'s background image when it is a `url()`.
pub(super) fn image_reference(style: &Style) -> Option<String> {
    match background_image(style)?.as_slice() {
        [Comp::Url(reference)] => Some(reference.clone()),
        _ => None,
    }
}

/// Which way `style`'s background image repeats, across and down.
pub(super) fn repeat(style: &Style) -> (bool, bool) {
    match style.get("background-repeat") {
        Some(Value::Text(text)) => match text.trim() {
            "no-repeat" | "no-repeat no-repeat" => (false, false),
            "repeat-x" | "repeat no-repeat" => (true, false),
            "repeat-y" | "no-repeat repeat" => (false, true),
            _ => (true, true),
        },
        _ => (true, true),
    }
}

/// `style`'s background image when it is a gradient this layout draws, its stop lengths
/// against `style`'s font size and the root's, `rem`.
pub(super) fn gradient(style: &Style, rem: f32) -> Option<Layer> {
    let image = background_image(style)?;
    let [Comp::Function(name, args)] = image.as_slice() else {
        return None;
    };
    let groups: Vec<Vec<Comp>> = value::components(args)?
        .split(|c| *c == Comp::Comma)
        .map(<[Comp]>::to_vec)
        .collect();
    let em = style.font_size();
    let first = groups.first()?;
    match name.as_str() {
        "linear-gradient" => {
            let (to, rest) = match first.as_slice() {
                [Comp::Dimension(v, unit)] => (Direction::Angle(angle(*v, unit)?), &groups[1..]),
                [Comp::Ident(to), sides @ ..] if to == "to" => (direction(sides)?, &groups[1..]),
                _ => (Direction::Angle(PI), &groups[..]),
            };
            Some(Layer::Linear {
                to,
                stops: stops(rest, em, rem)?,
            })
        }
        "radial-gradient" => {
            let words: Option<Vec<&str>> = first
                .iter()
                .map(|c| match c {
                    Comp::Ident(w) => Some(w.as_str()),
                    _ => None,
                })
                .collect();
            let shape = words.filter(|w| {
                w.iter().all(|w| {
                    matches!(
                        *w,
                        "circle"
                            | "ellipse"
                            | "closest-side"
                            | "farthest-side"
                            | "closest-corner"
                            | "farthest-corner"
                    )
                })
            });
            let (words, rest) = match shape {
                Some(words) => (words, &groups[1..]),
                None => (Vec::new(), &groups[..]),
            };
            let extent = match words.iter().find(|w| **w != "circle" && **w != "ellipse") {
                Some(&"closest-side") => Extent::ClosestSide,
                Some(&"farthest-side") => Extent::FarthestSide,
                Some(&"closest-corner") => Extent::ClosestCorner,
                _ => Extent::FarthestCorner,
            };
            Some(Layer::Radial {
                circle: words.contains(&"circle"),
                extent,
                stops: stops(rest, em, rem)?,
            })
        }
        _ => None,
    }
}

fn angle(v: f32, unit: &str) -> Option<f32> {
    Some(match unit {
        "deg" => v.to_radians(),
        "rad" => v,
        "grad" => v * PI / 200.0,
        "turn" => v * 2.0 * PI,
        _ => return None,
    })
}

fn direction(sides: &[Comp]) -> Option<Direction> {
    let (mut x, mut y) = (0.0_f32, 0.0_f32);
    for side in sides {
        let Comp::Ident(word) = side else {
            return None;
        };
        let (axis, by) = match word.as_str() {
            "left" => (&mut x, -1.0),
            "right" => (&mut x, 1.0),
            "top" => (&mut y, -1.0),
            "bottom" => (&mut y, 1.0),
            _ => return None,
        };
        if *axis != 0.0 {
            return None;
        }
        *axis = by;
    }
    match (x, y) {
        (0.0, 0.0) => None,
        (x, 0.0) => Some(Direction::Angle(if x > 0.0 {
            FRAC_PI_2
        } else {
            3.0 * FRAC_PI_2
        })),
        (0.0, y) => Some(Direction::Angle(if y > 0.0 { PI } else { 0.0 })),
        (x, y) => Some(Direction::Corner(x, y)),
    }
}

/// The colour stops of a gradient, at least two, each a colour and up to two positions.
fn stops(groups: &[Vec<Comp>], em: f32, rem: f32) -> Option<Vec<Written>> {
    let mut out = Vec::new();
    for group in groups {
        let (first, positions) = group.split_first()?;
        let color = value::color(first)?;
        let at = |c: &Comp| value::length(c).map(|sum| sum.compute(em, rem));
        match positions {
            [] => out.push((color, None)),
            [p] => out.push((color, Some(at(p)?))),
            [p, q] => {
                out.push((color, Some(at(p)?)));
                out.push((color, Some(at(q)?)));
            }
            _ => return None,
        }
    }
    (out.len() >= 2).then_some(out)
}

/// `stops` placed on a gradient line `line` long: the first at 0 and the last at 1 when they have
/// no position, each at least as far as the one before, and a run without positions spaced
/// evenly between its neighbours.
fn offsets(stops: &[Written], line: f32) -> Vec<Stop> {
    let mut at: Vec<Option<f32>> = stops
        .iter()
        .map(|(_, p)| p.map(|l| (l.pt + l.percent / 100.0 * line) / line.max(f32::EPSILON)))
        .collect();
    let last = at.len() - 1;
    at[0].get_or_insert(0.0);
    at[last].get_or_insert(1.0);
    let mut most = f32::MIN;
    for p in at.iter_mut().flatten() {
        *p = p.max(most);
        most = *p;
    }
    let mut i = 1;
    while i < last {
        if at[i].is_some() {
            i += 1;
            continue;
        }
        let end = (i..=last).find(|&k| at[k].is_some()).unwrap_or(last);
        let (a, b) = (at[i - 1].unwrap_or(0.0), at[end].unwrap_or(1.0));
        let span = (end - i + 1) as f32;
        for (n, slot) in at[i..end].iter_mut().enumerate() {
            *slot = Some(a + (b - a) * (n + 1) as f32 / span);
        }
        i = end;
    }
    stops
        .iter()
        .zip(at)
        .map(|((color, _), offset)| Stop {
            offset: offset.unwrap_or(0.0).clamp(0.0, 1.0),
            color: *color,
        })
        .collect()
}

/// `layer`'s paint over the padding box `x`, `y`, `w`, `h`.
fn layer_paint(layer: &Layer, x: f32, y: f32, w: f32, h: f32) -> Paint {
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    match layer {
        Layer::Linear { to, stops } => {
            let (dx, dy) = match *to {
                Direction::Angle(a) => (a.sin(), -a.cos()),
                Direction::Corner(sx, sy) => {
                    let n = (h * h + w * w).sqrt().max(f32::EPSILON);
                    (sx * h / n, sy * w / n)
                }
            };
            let line = (w * dx).abs() + (h * dy).abs();
            Paint::Linear {
                x1: cx - dx * line / 2.0,
                y1: cy - dy * line / 2.0,
                x2: cx + dx * line / 2.0,
                y2: cy + dy * line / 2.0,
                stops: offsets(stops, line),
            }
        }
        Layer::Radial {
            circle,
            extent,
            stops,
        } => {
            let (hw, hh) = (w / 2.0, h / 2.0);
            let corner = matches!(extent, Extent::ClosestCorner | Extent::FarthestCorner);
            let (rx, ry) = if *circle {
                let r = match extent {
                    Extent::ClosestSide => hw.min(hh),
                    Extent::FarthestSide => hw.max(hh),
                    _ => hw.hypot(hh),
                };
                (r, r)
            } else if corner {
                (hw * 2.0_f32.sqrt(), hh * 2.0_f32.sqrt())
            } else {
                (hw, hh)
            };
            Paint::Radial {
                cx,
                cy,
                r: rx,
                scale_y: ry / rx.max(f32::EPSILON),
                stops: offsets(stops, rx),
            }
        }
        Layer::Image {
            image,
            width,
            height,
            ..
        } => Paint::Tiles {
            image: *image,
            x,
            y,
            width: *width,
            height: *height,
        },
    }
}

/// A corner's radius across and down.
type Radius = (f32, f32);

/// `radius` resolved against a box `w` by `h`, scaled down together until each side holds its
/// two corners.
fn radii(radius: &[Length; 4], w: f32, h: f32) -> [Radius; 4] {
    let r = radius.map(|l| {
        (
            (l.pt + l.percent / 100.0 * w).max(0.0),
            (l.pt + l.percent / 100.0 * h).max(0.0),
        )
    });
    let fit = |side: f32, a: f32, b: f32| if a + b > side { side / (a + b) } else { 1.0 };
    let scale = fit(w, r[0].0, r[1].0)
        .min(fit(h, r[1].1, r[2].1))
        .min(fit(w, r[2].0, r[3].0))
        .min(fit(h, r[3].1, r[0].1));
    r.map(|(a, b)| (a * scale, b * scale))
}

/// `r` for the box inset by `by`, top, right, bottom, left.
fn inset(r: [Radius; 4], by: [f32; 4]) -> [Radius; 4] {
    let cut = |(a, b): Radius, across: f32, down: f32| ((a - across).max(0.0), (b - down).max(0.0));
    [
        cut(r[0], by[3], by[0]),
        cut(r[1], by[1], by[0]),
        cut(r[2], by[1], by[2]),
        cut(r[3], by[3], by[2]),
    ]
}

/// How far a control point sits from a corner's end, as a share of its radius, for a quarter
/// circle drawn as one cubic curve.
const KAPPA: f32 = 0.552_284_8;

/// Adds the box `x`, `y`, `w`, `h` with corners `r` to `path`, as one closed contour.
fn rounded(path: &mut Vec<Seg>, x: f32, y: f32, w: f32, h: f32, r: [Radius; 4]) {
    let [tl, tr, br, bl] = r;
    let (right, bottom) = (x + w, y + h);
    let k = KAPPA;
    path.push(Seg::Move(x + tl.0, y));
    path.push(Seg::Line(right - tr.0, y));
    if tr.0 > 0.0 && tr.1 > 0.0 {
        path.push(Seg::Cubic(
            right - tr.0 * (1.0 - k),
            y,
            right,
            y + tr.1 * (1.0 - k),
            right,
            y + tr.1,
        ));
    }
    path.push(Seg::Line(right, bottom - br.1));
    if br.0 > 0.0 && br.1 > 0.0 {
        path.push(Seg::Cubic(
            right,
            bottom - br.1 * (1.0 - k),
            right - br.0 * (1.0 - k),
            bottom,
            right - br.0,
            bottom,
        ));
    }
    path.push(Seg::Line(x + bl.0, bottom));
    if bl.0 > 0.0 && bl.1 > 0.0 {
        path.push(Seg::Cubic(
            x + bl.0 * (1.0 - k),
            bottom,
            x,
            bottom - bl.1 * (1.0 - k),
            x,
            bottom - bl.1,
        ));
    }
    path.push(Seg::Line(x, y + tl.1));
    if tl.0 > 0.0 && tl.1 > 0.0 {
        path.push(Seg::Cubic(
            x,
            y + tl.1 * (1.0 - k),
            x + tl.0 * (1.0 - k),
            y,
            x + tl.0,
            y,
        ));
    }
    path.push(Seg::Close);
}

pub(super) fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<Seg> {
    let mut path = Vec::with_capacity(6);
    rounded(&mut path, x, y, w, h, [(0.0, 0.0); 4]);
    path
}

pub(super) fn fill(path: Vec<Seg>, color: Color) -> Item {
    Item::Path {
        path,
        paint: Paint::Color(color),
        stroke: None,
    }
}

/// The dash of a dashed or dotted line `width` wide, and whether its ends are round.
fn dash(line: Line, width: f32) -> (Vec<f32>, bool) {
    match line {
        Line::Dotted => (vec![0.0, 2.0 * width], true),
        _ => (vec![2.0 * width, 2.0 * width], false),
    }
}

/// The edges of the box `x`, `y`, `w`, `h` with corners `r`: each side `width` wide, in its line
/// style and colour.
fn edges(
    out: &mut Vec<Item>,
    (x, y, w, h): (f32, f32, f32, f32),
    width: [f32; 4],
    line: [Line; 4],
    color: [Color; 4],
    r: [Radius; 4],
) {
    let alike = width.iter().all(|v| *v == width[0])
        && line.iter().all(|l| *l == line[0])
        && color.iter().all(|c| *c == color[0]);
    let curved = r.iter().any(|(a, b)| *a > 0.0 && *b > 0.0);
    if alike && width[0] > 0.0 && (curved || line[0] != Line::Solid) {
        let t = width[0];
        let ring = |from: f32, to: f32| {
            let mut path = Vec::new();
            rounded(
                &mut path,
                x + from,
                y + from,
                w - 2.0 * from,
                h - 2.0 * from,
                inset(r, [from; 4]),
            );
            rounded(
                &mut path,
                x + to,
                y + to,
                w - 2.0 * to,
                h - 2.0 * to,
                inset(r, [to; 4]),
            );
            fill(path, color[0])
        };
        match line[0] {
            Line::Solid => out.push(ring(0.0, t)),
            Line::Double => {
                out.push(ring(0.0, t / 3.0));
                out.push(ring(2.0 * t / 3.0, t));
            }
            Line::Dashed | Line::Dotted => {
                let half = t / 2.0;
                let mut path = Vec::new();
                rounded(
                    &mut path,
                    x + half,
                    y + half,
                    w - t,
                    h - t,
                    inset(r, [half; 4]),
                );
                let (dash, round) = dash(line[0], t);
                out.push(Item::Path {
                    path,
                    paint: Paint::Color(color[0]),
                    stroke: Some(Stroke {
                        width: t,
                        dash,
                        round,
                    }),
                });
            }
        }
        return;
    }
    for side in 0..4 {
        let t = width[side];
        if t <= 0.0 {
            continue;
        }
        let across = side % 2 == 0;
        let (sx, sy, sw, sh) = match side {
            0 => (x, y, w, t),
            1 => (x + w - t, y, t, h),
            2 => (x, y + h - t, w, t),
            _ => (x, y, t, h),
        };
        match line[side] {
            Line::Solid => out.push(fill(rect(sx, sy, sw, sh), color[side])),
            Line::Double => {
                let third = t / 3.0;
                let (inner, outer) = if across {
                    (
                        rect(sx, sy, sw, third),
                        rect(sx, sy + 2.0 * third, sw, third),
                    )
                } else {
                    (
                        rect(sx, sy, third, sh),
                        rect(sx + 2.0 * third, sy, third, sh),
                    )
                };
                out.push(fill(inner, color[side]));
                out.push(fill(outer, color[side]));
            }
            Line::Dashed | Line::Dotted => {
                let (dash, round) = dash(line[side], t);
                let trim = if round { t / 2.0 } else { 0.0 };
                let path = if across {
                    vec![
                        Seg::Move(sx + trim, sy + t / 2.0),
                        Seg::Line(sx + sw - trim, sy + t / 2.0),
                    ]
                } else {
                    vec![
                        Seg::Move(sx + t / 2.0, sy + trim),
                        Seg::Line(sx + t / 2.0, sy + sh - trim),
                    ]
                };
                out.push(Item::Path {
                    path,
                    paint: Paint::Color(color[side]),
                    stroke: Some(Stroke {
                        width: t,
                        dash,
                        round,
                    }),
                });
            }
        }
    }
}

/// The items that paint the slice of the box `g` from `y` to `y + h`, under its content. `first`
/// and `last` say whether the slice has the box's top edge and its bottom edge.
pub(super) fn paint(g: &BoxGeometry, y: f32, h: f32, first: bool, last: bool) -> Vec<Item> {
    let (x, w, d) = (g.x, g.width, &g.decor);
    let mut out = Vec::new();
    if w <= 0.0 || h <= 0.0 {
        return out;
    }
    let mut r = radii(&d.radius, w, h);
    if !first {
        r[0] = (0.0, 0.0);
        r[1] = (0.0, 0.0);
    }
    if !last {
        r[2] = (0.0, 0.0);
        r[3] = (0.0, 0.0);
    }
    let width = [
        if first { g.border[0] } else { 0.0 },
        g.border[1],
        if last { g.border[2] } else { 0.0 },
        g.border[3],
    ];
    let mut outer = Vec::new();
    rounded(&mut outer, x, y, w, h, r);
    if let Some(background) = d.background {
        out.push(fill(outer.clone(), background));
    }
    if let Some(layer) = &d.layer {
        let (px, py) = (x + width[3], y + width[0]);
        let (pw, ph) = (w - width[1] - width[3], h - width[0] - width[2]);
        let paint = layer_paint(layer, px, py, pw, ph);
        match *layer {
            Layer::Image {
                image,
                width: iw,
                height: ih,
                repeat_x,
                repeat_y,
            } if !(repeat_x && repeat_y) => {
                out.push(Item::Begin {
                    opacity: 1.0,
                    clip: Some(outer.clone()),
                });
                out.push(match (repeat_x, repeat_y) {
                    (true, _) => Item::Path {
                        path: rect(x, py, w, ih),
                        paint,
                        stroke: None,
                    },
                    (_, true) => Item::Path {
                        path: rect(px, y, iw, h),
                        paint,
                        stroke: None,
                    },
                    _ => Item::Image {
                        image,
                        x: px,
                        y: py,
                        width: iw,
                        height: ih,
                    },
                });
                out.push(Item::End);
            }
            _ => out.push(Item::Path {
                path: outer.clone(),
                paint,
                stroke: None,
            }),
        }
    }
    edges(&mut out, (x, y, w, h), width, d.line, d.border_color, r);
    if let Some(o) = &d.outline {
        let grow = o.offset + o.width;
        let (top, bottom) = (
            if first { grow } else { 0.0 },
            if last { grow } else { 0.0 },
        );
        let r = r.map(|(a, b)| {
            if a > 0.0 && b > 0.0 {
                ((a + grow).max(0.0), (b + grow).max(0.0))
            } else {
                (0.0, 0.0)
            }
        });
        let width = [
            if first { o.width } else { 0.0 },
            o.width,
            if last { o.width } else { 0.0 },
            o.width,
        ];
        edges(
            &mut out,
            (x - grow, y - top, w + 2.0 * grow, h + top + bottom),
            width,
            [o.line; 4],
            [o.color; 4],
            r,
        );
    }
    out
}

/// The box around every point of `path`, control points included: its left, top, right and
/// bottom.
pub fn bounds(path: &[Seg]) -> (f32, f32, f32, f32) {
    let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let mut add = |x: f32, y: f32| {
        b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    };
    for seg in path {
        match *seg {
            Seg::Move(x, y) | Seg::Line(x, y) => add(x, y),
            Seg::Cubic(x1, y1, x2, y2, x, y) => {
                add(x1, y1);
                add(x2, y2);
                add(x, y);
            }
            Seg::Close => {}
        }
    }
    b
}

/// `path` moved down by `by`.
pub(super) fn lower(path: &mut [Seg], by: f32) {
    for seg in path {
        match seg {
            Seg::Move(_, y) | Seg::Line(_, y) => *y += by,
            Seg::Cubic(_, y1, _, y2, _, y) => {
                *y1 += by;
                *y2 += by;
                *y += by;
            }
            Seg::Close => {}
        }
    }
}
