//! Grid containers: the `grid-*` properties read into `taffy`'s grid style.
//!
//! A grid container is laid out as a flex container is (`flex`'s module doc): its items are
//! collected the same way, `taffy` places them, and they go down the page in bands, so a page
//! breaks only between rows. Only the container's and the items' `taffy` styles differ, and the
//! cascade keeps the `grid-*` properties as text, which this module reads.
//!
//! A track is a length, a percentage, `fr`, `auto`, `min-content`, `max-content`, `minmax()`,
//! `fit-content()`, or a `repeat()` of them by a count, `auto-fill` or `auto-fit`. A line name in
//! brackets names a line. A placement is `auto`, a line number, a name with an optional number,
//! or `span` with a number or a name; `grid-area` is a name from `grid-template-areas` or up to
//! four placements, and `grid-row` and `grid-column` win over it. Names are read in lower case,
//! in the template and in a placement alike. A value that does not read leaves the property at
//! its initial value. A track with both a length and a percentage in one `calc()` is not read.

use std::collections::BTreeMap;

use taffy::{
    GridAutoFlow, GridPlacement, GridTemplateArea, GridTemplateAreas, GridTemplateComponent,
    GridTemplateRepetition, LengthPercentage, Line, MaxTrackSizingFunction, MinMax,
    MinTrackSizingFunction, RepetitionCount, TrackSizingFunction, prelude::TaffyFitContent,
};

use crate::css::value::{self, Comp};
use crate::css::{Style, Value};

/// A template's tracks and the names of its lines, one set per line or none at all.
type Template = (Vec<GridTemplateComponent<String>>, Vec<Vec<String>>);

/// Sets the grid container properties of `style` on `out`, with `rem` for its lengths.
pub(super) fn container(style: &Style, rem: f32, out: &mut taffy::Style) {
    let em = style.font_size();
    if let Some((tracks, names)) =
        text(style, "grid-template-columns").and_then(|t| template(t, em, rem))
    {
        out.grid_template_columns = tracks;
        out.grid_template_column_names = names;
    }
    if let Some((tracks, names)) =
        text(style, "grid-template-rows").and_then(|t| template(t, em, rem))
    {
        out.grid_template_rows = tracks;
        out.grid_template_row_names = names;
    }
    if let Some(tracks) = text(style, "grid-auto-columns").and_then(|t| sizes(t, em, rem)) {
        out.grid_auto_columns = tracks;
    }
    if let Some(tracks) = text(style, "grid-auto-rows").and_then(|t| sizes(t, em, rem)) {
        out.grid_auto_rows = tracks;
    }
    if let Some(flow) = text(style, "grid-auto-flow").and_then(auto_flow) {
        out.grid_auto_flow = flow;
    }
    out.grid_template_areas = text(style, "grid-template-areas").and_then(areas);
}

/// Sets the placement of a grid item with `style` on `out`.
pub(super) fn item(style: &Style, out: &mut taffy::Style) {
    if let Some([row_start, column_start, row_end, column_end]) =
        text(style, "grid-area").and_then(|t| sides(t, 4))
    {
        let row_start = row_start.unwrap_or_default();
        let row_end = row_end.unwrap_or_else(|| implied(&row_start));
        let column_start = column_start.unwrap_or_else(|| implied(&row_start));
        let column_end = column_end.unwrap_or_else(|| implied(&column_start));
        out.grid_row = Line {
            start: row_start,
            end: row_end,
        };
        out.grid_column = Line {
            start: column_start,
            end: column_end,
        };
    }
    for (name, line) in [
        ("grid-row", &mut out.grid_row),
        ("grid-column", &mut out.grid_column),
    ] {
        if let Some([start, end, ..]) = text(style, name).and_then(|t| sides(t, 2)) {
            let start = start.unwrap_or_default();
            let end = end.unwrap_or_else(|| implied(&start));
            *line = Line { start, end };
        }
    }
}

fn text<'s>(style: &'s Style, name: &str) -> Option<&'s str> {
    match style.get(name) {
        Some(Value::Text(text)) => Some(text),
        _ => None,
    }
}

/// The tracks and line names of a `grid-template-columns` or `grid-template-rows` value, or
/// `None` for `none` and a value that does not read.
fn template(text: &str, em: f32, rem: f32) -> Option<Template> {
    if text.trim() == "none" {
        return None;
    }
    listing(text, em, rem, true)
}

/// The tracks of `text` and the names in brackets between them. A `repeat()` is read only when
/// `repeat` is true, and is one track of the result.
fn listing(text: &str, em: f32, rem: f32, repeat: bool) -> Option<Template> {
    let mut tracks = Vec::new();
    let mut names: Vec<Vec<String>> = Vec::new();
    let mut depth = 0usize;
    let mut segment = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '[' if depth == 0 => {
                tracks.extend(segment_tracks(&segment, em, rem, repeat)?);
                segment.clear();
                let inside: String = chars.by_ref().take_while(|c| *c != ']').collect();
                if names.len() <= tracks.len() {
                    names.resize(tracks.len() + 1, Vec::new());
                }
                names[tracks.len()].extend(inside.split_whitespace().map(str::to_ascii_lowercase));
                continue;
            }
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        segment.push(c);
    }
    tracks.extend(segment_tracks(&segment, em, rem, repeat)?);
    if tracks.is_empty() {
        return None;
    }
    if names.iter().all(Vec::is_empty) {
        names.clear();
    } else {
        names.resize(tracks.len() + 1, Vec::new());
    }
    Some((tracks, names))
}

/// The tracks of a run of `text` with no line names in it.
fn segment_tracks(
    text: &str,
    em: f32,
    rem: f32,
    repeat: bool,
) -> Option<Vec<GridTemplateComponent<String>>> {
    value::components(text)?
        .iter()
        .map(|c| match c {
            Comp::Function(name, args) if name == "repeat" && repeat => repetition(args, em, rem),
            c => track(c, em, rem).map(GridTemplateComponent::Single),
        })
        .collect()
}

/// A `repeat()` from its argument text: a count, `auto-fill` or `auto-fit`, a comma, and tracks.
fn repetition(args: &str, em: f32, rem: f32) -> Option<GridTemplateComponent<String>> {
    let (count, rest) = args.split_once(',')?;
    let count = match count.trim() {
        "auto-fill" => RepetitionCount::AutoFill,
        "auto-fit" => RepetitionCount::AutoFit,
        n => RepetitionCount::Count(n.parse().ok().filter(|n| *n > 0)?),
    };
    let (components, line_names) = listing(rest, em, rem, false)?;
    let tracks = components
        .into_iter()
        .map(|c| match c {
            GridTemplateComponent::Single(t) => Some(t),
            GridTemplateComponent::Repeat(_) => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(GridTemplateComponent::Repeat(GridTemplateRepetition {
        count,
        tracks,
        line_names,
    }))
}

/// The tracks of a `grid-auto-columns` or `grid-auto-rows` value.
fn sizes(text: &str, em: f32, rem: f32) -> Option<Vec<TrackSizingFunction>> {
    let tracks = value::components(text)?
        .iter()
        .map(|c| track(c, em, rem))
        .collect::<Option<Vec<_>>>()?;
    (!tracks.is_empty()).then_some(tracks)
}

/// One track's sizing.
fn track(c: &Comp, em: f32, rem: f32) -> Option<TrackSizingFunction> {
    match c {
        Comp::Function(name, args) if name == "minmax" => {
            let (min, max) = args.split_once(',')?;
            let [min] = &value::components(min)?[..] else {
                return None;
            };
            let [max] = &value::components(max)?[..] else {
                return None;
            };
            Some(MinMax {
                min: least(min, em, rem)?,
                max: most(max, em, rem)?,
            })
        }
        Comp::Function(name, args) if name == "fit-content" => {
            let [limit] = &value::components(args)?[..] else {
                return None;
            };
            Some(MinMax {
                min: MinTrackSizingFunction::auto(),
                max: MaxTrackSizingFunction::fit_content(span(limit, em, rem)?),
            })
        }
        Comp::Dimension(_, unit) if unit == "fr" => Some(MinMax {
            min: MinTrackSizingFunction::auto(),
            max: most(c, em, rem)?,
        }),
        c => Some(MinMax {
            min: least(c, em, rem)?,
            max: most(c, em, rem)?,
        }),
    }
}

/// A track's smallest size: a length, a percentage, `auto`, `min-content` or `max-content`.
fn least(c: &Comp, em: f32, rem: f32) -> Option<MinTrackSizingFunction> {
    Some(match c {
        Comp::Ident(word) => match word.as_str() {
            "auto" => MinTrackSizingFunction::auto(),
            "min-content" => MinTrackSizingFunction::min_content(),
            "max-content" => MinTrackSizingFunction::max_content(),
            _ => return None,
        },
        c => span(c, em, rem)?.into(),
    })
}

/// A track's largest size: what [`least`] reads, and a share of the free space in `fr`.
fn most(c: &Comp, em: f32, rem: f32) -> Option<MaxTrackSizingFunction> {
    match c {
        Comp::Dimension(v, unit) if unit == "fr" => {
            (*v >= 0.0).then(|| MaxTrackSizingFunction::fr(*v))
        }
        c => least(c, em, rem).map(MaxTrackSizingFunction::from),
    }
}

/// A length or a percentage that is not negative.
fn span(c: &Comp, em: f32, rem: f32) -> Option<LengthPercentage> {
    let l = value::length(c)?.compute(em, rem);
    match (l.pt, l.percent) {
        (pt, 0.0) if pt >= 0.0 => Some(LengthPercentage::length(pt)),
        (0.0, percent) if percent >= 0.0 => Some(LengthPercentage::percent(percent / 100.0)),
        _ => None,
    }
}

fn auto_flow(text: &str) -> Option<GridAutoFlow> {
    let words: Vec<String> = value::components(text)?
        .into_iter()
        .map(|c| match c {
            Comp::Ident(word) => Some(word),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let column = words.iter().any(|w| w == "column");
    let dense = words.iter().any(|w| w == "dense");
    if words
        .iter()
        .any(|w| !matches!(w.as_str(), "row" | "column" | "dense"))
    {
        return None;
    }
    Some(match (column, dense) {
        (false, false) => GridAutoFlow::Row,
        (true, false) => GridAutoFlow::Column,
        (false, true) => GridAutoFlow::RowDense,
        (true, true) => GridAutoFlow::ColumnDense,
    })
}

/// The named areas of a `grid-template-areas` value: one string per row, each the same number
/// of cells, and each name a rectangle. A cell of dots names nothing.
fn areas(text: &str) -> Option<GridTemplateAreas<String>> {
    let rows: Vec<Vec<String>> = value::components(text)?
        .into_iter()
        .map(|c| match c {
            Comp::Str(row) => Some(
                row.split_whitespace()
                    .map(str::to_ascii_lowercase)
                    .collect(),
            ),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let columns = rows.first()?.len();
    if columns == 0 || rows.iter().any(|r| r.len() != columns) {
        return None;
    }
    // Each name's first and last row and column, and how many cells it has.
    let mut found: BTreeMap<&str, ([usize; 4], usize)> = BTreeMap::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, name) in row.iter().enumerate() {
            if name.chars().all(|ch| ch == '.') {
                continue;
            }
            let (bounds, cells) = found.entry(name).or_insert(([r, c, r, c], 0));
            *bounds = [
                bounds[0].min(r),
                bounds[1].min(c),
                bounds[2].max(r),
                bounds[3].max(c),
            ];
            *cells += 1;
        }
    }
    let mut out = Vec::with_capacity(found.len());
    for (name, ([top, left, bottom, right], cells)) in found {
        if cells != (bottom - top + 1) * (right - left + 1) {
            return None;
        }
        out.push(GridTemplateArea {
            name: name.to_string(),
            row_start: u16::try_from(top + 1).ok()?,
            row_end: u16::try_from(bottom + 2).ok()?,
            column_start: u16::try_from(left + 1).ok()?,
            column_end: u16::try_from(right + 2).ok()?,
        });
    }
    Some(GridTemplateAreas {
        areas: out,
        row_count: u16::try_from(rows.len()).ok()?,
        column_count: u16::try_from(columns).ok()?,
    })
}

/// Up to `most` placements separated by `/`, the first always there, or `None` when one does
/// not read.
fn sides(text: &str, most: usize) -> Option<[Option<GridPlacement>; 4]> {
    let comps = value::components(text)?;
    let parts: Vec<&[Comp]> = comps.split(|c| matches!(c, Comp::Slash)).collect();
    if parts.len() > most {
        return None;
    }
    let mut out: [Option<GridPlacement>; 4] = Default::default();
    for (slot, part) in out.iter_mut().zip(&parts) {
        *slot = Some(placement(part)?);
    }
    out[0].is_some().then_some(out)
}

/// The end a placement that is only `start` implies: the same name, or `auto`.
fn implied(start: &GridPlacement) -> GridPlacement {
    match start {
        GridPlacement::NamedLine(name, 0) => GridPlacement::NamedLine(name.clone(), 0),
        _ => GridPlacement::Auto,
    }
}

/// One placement: `auto`, a line number, a name with an optional number, or `span` with either.
fn placement(part: &[Comp]) -> Option<GridPlacement> {
    let mut spans = false;
    let mut number: Option<i16> = None;
    let mut name: Option<String> = None;
    for c in part {
        match c {
            Comp::Ident(word) if word == "span" && !spans => spans = true,
            Comp::Ident(word) if word == "auto" && part.len() == 1 => {
                return Some(GridPlacement::Auto);
            }
            Comp::Ident(word) if name.is_none() => name = Some(word.clone()),
            Comp::Number(n, true) if number.is_none() && *n != 0.0 => {
                number = Some(i16::try_from(*n as i64).ok()?);
            }
            _ => return None,
        }
    }
    Some(match (spans, number, name) {
        (true, Some(n), None) if n > 0 => GridPlacement::Span(n as u16),
        (true, n, Some(name)) if n.is_none_or(|n| n > 0) => {
            GridPlacement::NamedSpan(name, n.unwrap_or(1) as u16)
        }
        (false, Some(n), None) => GridPlacement::Line(n.into()),
        (false, n, Some(name)) => GridPlacement::NamedLine(name, n.unwrap_or(0)),
        _ => return None,
    })
}
