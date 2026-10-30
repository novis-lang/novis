//! Tables across pages (`rule:core-classes/pdf-tables-across-pages`).
//!
//! A `table` or `inline-table` is a block box whose content is one [`Table`] block. Its
//! `table-row` children, directly or inside a `table-header-group`, `table-row-group` or
//! `table-footer-group`, are its rows, and each `table-cell` in a row is a cell, collected the
//! way a flex item is. A cell directly in a table or a group is in a row of its own, and text
//! directly in a table, a group or a row is left out. A `table-caption` is laid out at the
//! table's content width, above the rows or under them by its `caption-side`, and only once.
//! The table's box is as wide as a block box is, and a group's or a row's own box draws
//! nothing.
//!
//! **Columns** are computed once for the whole table, so every page has the same ones. Each
//! column is as wide as the widest of its cells laid out unbroken, when the room allows, and
//! the room left over is shared out by those widths. With less room, each column narrows from
//! that width towards its narrowest cell broken at every chance. A cell over several columns
//! (`colspan`) spreads what its columns lack over them evenly. Under `table-layout: fixed`, a
//! cell of the first row with a `width` sets its columns, and the other columns share the rest
//! evenly. `border-spacing` is the room between two cells and between a cell and the table's
//! content edge.
//!
//! **Rows.** A row is as tall as its tallest cell, and a `rowspan` cell taller than its rows
//! makes the last of them taller. Each cell is drawn as tall as its rows. The rows are drawn in
//! this order: the header rows, the body rows in source order, then the footer rows. A body row
//! is placed as one piece, so a row that fits on a page is never split, and rows a `rowspan`
//! joins are one piece. A piece that does not fit under the last one, above the footer rows,
//! goes on the next page.
//!
//! **A row taller than a page** starts where it is and splits between lines. Each of its cells
//! is laid out again down pages of its own, the first as tall as the room left above the footer
//! rows and each later one as tall as a page holds under the tallest header rows and above the
//! tallest footer rows, so a cell breaks the way a block box does: between lines, keeping its
//! paragraphs' `orphans` and `widows`. Each part of the row but the last fills its page, and
//! the last is as tall as the tallest cell that reaches it. A cell of a split row is not
//! stretched to the row's height.
//!
//! **Repeated rows.** The header rows are drawn at the top and the footer rows at the bottom of
//! each page the table is on. A row's `data-repeat` says on which: `every` (the default),
//! `first`, `last`, or `continued`, which is a header row on each page after the first and a
//! footer row on each page before the last. A page is the last one when the rest of the table
//! fits on it.
//!
//! **Borders at a break.** The table's box and each cell of a split row are closed at every
//! page break, as `block`'s module doc says, unless `box-decoration-break` is `slice`. Under
//! `border-collapse: collapse`, cells that meet overlap by the widest cell border and the
//! outer cells' borders lie on the table's, so two borders that meet are drawn as one. Each
//! cell keeps all four sides, so the last row before a break and the first after it are both
//! closed. `border-spacing` is not used, and the borders are not resolved by style or width.
//!
//! **Slots and marks.** A `<slot name>` in a cell draws nothing, and its content is left out.
//! Each page its row is drawn on records the cell's content box under the slot's name
//! ([`super::Page::slots`]), and the second pass draws the slot's fill there. A row's
//! `data-mark` marks are set on the page it is first drawn on.
//!
//! **Keeping rows together.** The table's `orphans` and `widows`, 2 when unset, count body rows.
//! A table does not start at the bottom of a page with fewer than `orphans` body rows under its
//! header rows: it starts on the next page. A break that would leave fewer than `widows` rows
//! after it is taken earlier, when that leaves `orphans` rows before it. The rows of a group
//! element (`tbody`, or `thead` or `tfoot` set to a body group's display) go on the next page
//! together when they fit on a page and not under the last row, and split between rows when
//! they do not fit on a page.
//!
//! **Not done yet.** A `rowspan` piece taller than a page is placed whole. A table without a
//! `width` fills its containing block. A cell's `vertical-align` is always `top`. A row's and a
//! group's own background and borders are not drawn. A `data-mark` or `string-set` inside a cell
//! counts as on the first page. A `display: table` element directly in a row is a cell whose
//! rows are block boxes (gap:nvs-pdf/pdf-table-leftovers).
//!
//! Cost: each cell is laid out once at each of its two content widths, once at its column
//! width and once more when its row is taller or taller than a page. A repeated row's items are
//! copied on each page.

use super::block::Flow;
use super::flex::fit;
use super::text::{Cx, Placed, carry, shift};
use super::{Block, Item, MARGIN, Missing, Role, keyword, margin};
use crate::css::{Style, Value, value};

/// A table's content box, its `border-spacing`, its `table-layout`, and what it holds.
#[derive(Clone)]
pub(super) struct Table {
    /// The content box's left edge and width.
    pub(super) x: f32,
    pub(super) width: f32,
    /// The horizontal and the vertical `border-spacing`.
    pub(super) spacing: [f32; 2],
    /// Whether its `table-layout` is `fixed`.
    pub(super) fixed: bool,
    /// Whether its `border-collapse` is `collapse`.
    pub(super) collapse: bool,
    /// How many body rows a page break leaves before it and after it, when it can.
    pub(super) orphans: usize,
    pub(super) widows: usize,
    /// Each caption's index in the list of atoms, and whether it is under the rows.
    pub(super) captions: Vec<(usize, bool)>,
    pub(super) rows: Vec<Row>,
}

/// A row: the group it is in, when it shows, and its cells.
#[derive(Clone)]
pub(super) struct Row {
    pub(super) part: Part,
    /// The number of the group element it is in, `None` when it is directly in the table.
    pub(super) group: Option<usize>,
    pub(super) repeat: Repeat,
    /// The cells, in source order.
    pub(super) cells: Vec<Cell>,
    /// Each `<slot name>` in it: the index of its cell, and its name.
    pub(super) slots: Vec<(usize, String)>,
    /// Its `data-mark` marks, set on the page it is first drawn on.
    pub(super) marks: Vec<(String, String)>,
}

/// A cell: its index in the list of atoms, its `colspan` and its `rowspan`.
#[derive(Clone, Copy)]
pub(super) struct Cell {
    pub(super) atom: usize,
    pub(super) span: usize,
    pub(super) rows: usize,
}

/// Which group of a table a row is in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Part {
    Head,
    Body,
    Foot,
}

/// A row's `data-repeat`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Repeat {
    Every,
    First,
    Last,
    Continued,
}

impl Repeat {
    /// The `data-repeat` of a row, `every` when it has none or another value.
    pub(super) fn of(attr: Option<&str>) -> Repeat {
        match attr.map(str::trim) {
            Some("first") => Repeat::First,
            Some("last") => Repeat::Last,
            Some("continued") => Repeat::Continued,
            _ => Repeat::Every,
        }
    }

    /// Whether a row of `part` shows on a page that is the table's `first` and its `last`.
    fn shows(self, part: Part, first: bool, last: bool) -> bool {
        match self {
            Repeat::Every => true,
            Repeat::First => first,
            Repeat::Last => last,
            Repeat::Continued if part == Part::Head => !first,
            Repeat::Continued => !last,
        }
    }
}

/// A table's `border-spacing`: one length for both directions, or the horizontal one and then
/// the vertical one. `rem` is the root element's font size.
pub(super) fn spacing(style: &Style, rem: f32) -> [f32; 2] {
    let Some(Value::Text(text)) = style.get("border-spacing") else {
        return [0.0; 2];
    };
    let size = style.font_size();
    let lengths: Vec<f32> = value::components(text)
        .unwrap_or_default()
        .iter()
        .filter_map(value::length)
        .map(|l| (l.pt + l.em * size + l.rem * rem).max(0.0))
        .collect();
    match lengths[..] {
        [both] => [both; 2],
        [x, y] => [x, y],
        _ => [0.0; 2],
    }
}

/// Whether a table's `table-layout` is `fixed`.
pub(super) fn fixed(style: &Style) -> bool {
    keyword(style, "table-layout") == Some("fixed")
}

/// The first column of each cell, row by row, and the number of columns. A cell starts at the
/// first column no `rowspan` from a row above still covers.
fn grid(rows: &[Row]) -> (Vec<Vec<usize>>, usize) {
    // The rows each column is still covered for, after the row being read.
    let mut covered: Vec<usize> = Vec::new();
    let mut starts = Vec::with_capacity(rows.len());
    for row in rows {
        let mut column = 0;
        let mut here = Vec::with_capacity(row.cells.len());
        for cell in &row.cells {
            while covered.get(column).is_some_and(|c| *c > 0) {
                column += 1;
            }
            here.push(column);
            let end = column + cell.span;
            if covered.len() < end {
                covered.resize(end, 0);
            }
            for c in &mut covered[column..end] {
                *c = cell.rows;
            }
            column = end;
        }
        for c in &mut covered {
            *c = c.saturating_sub(1);
        }
        starts.push(here);
    }
    let columns = covered.len().max(1);
    (starts, columns)
}

/// The widths of columns whose narrowest and widest widths are `min` and `max`, in `room`.
fn distribute(min: &[f32], max: &[f32], room: f32) -> Vec<f32> {
    let least: f32 = min.iter().sum();
    let most: f32 = max.iter().sum();
    if most <= room {
        let extra = room - most;
        if most > 0.0 {
            max.iter().map(|m| m + extra * m / most).collect()
        } else {
            vec![room / max.len() as f32; max.len()]
        }
    } else if least >= room {
        min.to_vec()
    } else {
        let t = (room - least) / (most - least);
        min.iter()
            .zip(max)
            .map(|(lo, hi)| lo + t * (hi - lo))
            .collect()
    }
}

/// Spreads what the columns `from..from + span` lack of `need`, with `gap` between two of
/// them, over those columns evenly.
fn spread(widths: &mut [f32], from: usize, span: usize, gap: f32, need: f32) {
    let columns = &mut widths[from..from + span];
    let have: f32 = columns.iter().sum::<f32>() + gap * (span - 1) as f32;
    if need > have {
        let each = (need - have) / span as f32;
        for c in columns {
            *c += each;
        }
    }
}

/// A row laid out: each cell's left edge and its items, the row's height, each slot's name
/// and its cell's content box, its top from the row's top, and the row's marks.
struct Laid {
    cells: Vec<(f32, Placed)>,
    height: f32,
    slots: Vec<(String, [f32; 4])>,
    marks: Vec<(String, String)>,
}

impl Cx<'_> {
    /// Lays the rows of `table` out with the same columns throughout, and places them down
    /// `flow` with its header and footer rows on each page it is on.
    pub(super) fn table(&mut self, table: Table, flow: &mut Flow) {
        let Table {
            x,
            width,
            spacing,
            fixed,
            collapse,
            orphans,
            widows,
            captions,
            rows,
        } = table;
        let blocks: Vec<Vec<Vec<Block>>> = rows
            .iter()
            .map(|row| {
                row.cells
                    .iter()
                    .map(|c| self.waiting[c.atom].take().unwrap_or_default())
                    .collect()
            })
            .collect();
        // Under `border-collapse: collapse`, cells that meet overlap by the widest cell border,
        // so two borders that meet are drawn as one, and the outer cells' borders lie on the
        // table's. Each cell keeps all four, so a row is closed on both sides of a page break.
        let [gx, gy] = if collapse {
            let widest = blocks
                .iter()
                .flatten()
                .filter_map(|cell| match cell.first() {
                    Some(Block::Open(g)) => g.border.into_iter().reduce(f32::max),
                    _ => None,
                })
                .fold(0.0, f32::max);
            [-widest; 2]
        } else {
            spacing
        };
        let (starts, columns) = grid(&rows);
        let room = (width - gx * (columns as f32 + 1.0)).max(0.0);
        let widths = if fixed {
            self.fixed_columns(&rows, &blocks, &starts, columns, room, gx)
        } else {
            self.auto_columns(&rows, &blocks, &starts, columns, room, gx)
        };
        let mut lefts = Vec::with_capacity(columns);
        let mut at = x + gx;
        for w in &widths {
            lefts.push(at);
            at += w + gx;
        }
        let cell_width = |from: usize, span: usize| {
            let end = (from + span).min(columns);
            widths[from..end].iter().sum::<f32>() + gx * (end - from).saturating_sub(1) as f32
        };

        // Each cell at its width, and each row's height from the cells that span only it.
        let mut measured: Vec<Vec<Placed>> = Vec::with_capacity(rows.len());
        let mut heights = vec![0.0f32; rows.len()];
        for (r, row) in rows.iter().enumerate() {
            let mut here = Vec::with_capacity(row.cells.len());
            for (c, cell) in row.cells.iter().enumerate() {
                let laid = self.item_at(&blocks[r][c], cell_width(starts[r][c], cell.span), None);
                if cell.rows == 1 {
                    heights[r] = heights[r].max(laid.height);
                }
                here.push(laid);
            }
            measured.push(here);
        }
        // A `rowspan` cell taller than its rows makes the last of them taller.
        let span_end = |r: usize, cell: &Cell| (r + cell.rows).min(rows.len());
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.cells.iter().enumerate() {
                if cell.rows > 1 {
                    let end = span_end(r, cell);
                    let have = heights[r..end].iter().sum::<f32>() + gy * (end - r - 1) as f32;
                    let need = measured[r][c].height;
                    if need > have {
                        heights[end - 1] += need - have;
                    }
                }
            }
        }
        // Each cell again at the height of its rows, where that is taller.
        let mut laid: Vec<Laid> = Vec::with_capacity(rows.len());
        for (r, row) in rows.iter().enumerate() {
            let mut cells = Vec::with_capacity(row.cells.len());
            for (c, cell) in row.cells.iter().enumerate() {
                let end = span_end(r, cell);
                let tall = heights[r..end].iter().sum::<f32>() + gy * (end - r - 1) as f32;
                let first = std::mem::replace(&mut measured[r][c], Placed::empty());
                let placed = if tall > first.height + 0.01 {
                    self.item_at(
                        &blocks[r][c],
                        cell_width(starts[r][c], cell.span),
                        Some(tall),
                    )
                } else {
                    first
                };
                cells.push((lefts[starts[r][c].min(columns - 1)], placed));
            }
            // A slot fills its cell's content box, as tall as the cell's rows.
            let slots = row
                .slots
                .iter()
                .filter(|(c, _)| *c < row.cells.len())
                .map(|(c, name)| {
                    let end = span_end(r, &row.cells[*c]);
                    let tall = heights[r..end].iter().sum::<f32>() + gy * (end - r - 1) as f32;
                    let width = cell_width(starts[r][*c], row.cells[*c].span);
                    let [top, right, bottom, left] = match blocks[r][*c].first() {
                        Some(Block::Open(g)) => [0, 1, 2, 3].map(|s| g.border[s] + g.padding[s]),
                        _ => [0.0; 4],
                    };
                    let area = [
                        cells[*c].0 + left,
                        top,
                        (width - left - right).max(0.0),
                        (tall - top - bottom).max(0.0),
                    ];
                    (name.clone(), area)
                })
                .collect();
            laid.push(Laid {
                cells,
                height: heights[r],
                slots,
                marks: row.marks.clone(),
            });
        }

        let order = |part: Part| -> Vec<usize> {
            (0..rows.len()).filter(|r| rows[*r].part == part).collect()
        };
        let (head, body, foot) = (order(Part::Head), order(Part::Body), order(Part::Foot));
        // The body rows in pieces: each piece runs on to the last row a `rowspan` in it reaches.
        let mut pieces: Vec<Vec<usize>> = Vec::new();
        let mut reach = 0;
        for (i, &r) in body.iter().enumerate() {
            match pieces.last_mut() {
                Some(piece) if i < reach => piece.push(r),
                _ => pieces.push(vec![r]),
            }
            let longest = rows[r].cells.iter().map(|c| c.rows).max().unwrap_or(1);
            reach = reach.max(i + longest);
        }
        let piece_height =
            |piece: &[usize]| piece.iter().map(|r| gy + laid[*r].height).sum::<f32>();
        let shown = |rows_of: &[usize], first: bool, last: bool| -> Vec<usize> {
            rows_of
                .iter()
                .copied()
                .filter(|r| rows[*r].repeat.shows(rows[*r].part, first, last))
                .collect()
        };

        let mut drawn = vec![false; rows.len()];
        for (atom, bottom) in &captions {
            if !bottom {
                self.caption(*atom, x, width, flow);
            }
        }
        // A table does not start at the bottom of a page with fewer than `orphans` body rows
        // under its header rows.
        let rows_from = |from: usize| pieces[from..].iter().map(Vec::len).sum::<usize>();
        if let Some((top, limit, empty)) = flow.room() {
            let lead = (1..=pieces.len())
                .find(|n| rows_from(0) - rows_from(*n) >= orphans)
                .unwrap_or(pieces.len());
            let start = piece_height(&shown(&head, true, false))
                + pieces[..lead].iter().map(|p| piece_height(p)).sum::<f32>()
                + piece_height(&shown(&foot, true, lead == pieces.len()));
            if !empty && top + start > limit {
                flow.break_page();
            }
        }
        let mut first = true;
        let rest = |from: usize| pieces[from..].iter().map(|p| piece_height(p)).sum::<f32>();
        // Whether the rest of the table from the piece `from`, and `extra` more, fits on the
        // last page.
        let fits_rest = |flow: &mut Flow, from: usize, first: bool, extra: f32| match flow.room() {
            Some((top, limit, _)) => {
                top + piece_height(&shown(&head, first, true))
                    + extra
                    + rest(from)
                    + piece_height(&shown(&foot, first, true))
                    + gy
                    <= limit
            }
            None => true,
        };
        // Ends the last page with its footer rows and starts the next with its header rows.
        let turn = |cx: &mut Self,
                    flow: &mut Flow,
                    drawn: &mut [bool],
                    first: &mut bool,
                    from: usize,
                    extra: f32| {
            cx.rows(&shown(&foot, *first, false), &laid, gy, flow, drawn);
            // Collapsed borders end on the table's bottom edge, as they do on the last page.
            if gy < 0.0 {
                flow.place(gy);
            }
            flow.break_page();
            *first = false;
            let last = fits_rest(flow, from, false, extra);
            cx.rows(&shown(&head, false, last), &laid, gy, flow, drawn);
        };
        let most = |rows_of: &[usize]| {
            piece_height(&shown(rows_of, false, false))
                .max(piece_height(&shown(rows_of, false, true)))
        };
        // The room a row has on a page after the first, under the header rows and above the
        // footer rows.
        let later = flow.next_limit() - most(&head) - most(&foot) - 2.0 * gy - 0.01;
        let group_of = |p: usize| rows[pieces[p][0]].group;
        // Whether to break before the piece `i`, which fits at `top` on a page `limit` tall
        // with `here` body rows above it: the group it starts fits on a page and not on this
        // one, or a later break would leave fewer than `widows` rows after it.
        let keep = |i: usize, top: f32, limit: f32, first: bool, here: usize| {
            let count = pieces.len();
            let below = |end: bool| {
                piece_height(&shown(&foot, first, end)) + if end { gy } else { 0.0 }
            };
            if let Some(group) = group_of(i)
                && (i == 0 || group_of(i - 1) != Some(group))
            {
                let end = (i..count)
                    .find(|p| group_of(*p) != Some(group))
                    .unwrap_or(count);
                let height: f32 = pieces[i..end].iter().map(|p| piece_height(p)).sum();
                if height <= later && top + height + below(end == count) > limit {
                    return true;
                }
            }
            let (mut y, mut k) = (top, i);
            while k < count && y + piece_height(&pieces[k]) + below(k + 1 == count) <= limit {
                y += piece_height(&pieces[k]);
                k += 1;
            }
            if k == count || rows_from(k) >= widows || piece_height(&pieces[k]) > later {
                return false;
            }
            let target = (i..=k).rev().find(|j| rows_from(*j) >= widows);
            target == Some(i) && here >= orphans
        };
        let last = fits_rest(flow, 0, first, 0.0);
        self.rows(&shown(&head, first, last), &laid, gy, flow, &mut drawn);
        let mut here = 0;
        for (i, piece) in pieces.iter().enumerate() {
            let end = i + 1 == pieces.len();
            let below = piece_height(&shown(&foot, first, end)) + if end { gy } else { 0.0 };
            // A row taller than a page starts where it is and splits between lines.
            if let [r] = piece[..]
                && flow.paginates()
                && laid[r].height > later
            {
                let room = |flow: &mut Flow, first: bool| match flow.room() {
                    Some((top, limit, _)) => {
                        limit - top - gy - piece_height(&shown(&foot, first, false)) - 0.01
                    }
                    None => f32::INFINITY,
                };
                if room(flow, first) <= 0.0 {
                    turn(self, flow, &mut drawn, &mut first, i, 0.0);
                }
                let now = room(flow, first);
                let cells: Vec<(f32, Vec<Block>)> = rows[r]
                    .cells
                    .iter()
                    .enumerate()
                    .map(|(c, cell)| {
                        let width = cell_width(starts[r][c], cell.span);
                        (laid[r].cells[c].0, fit(&blocks[r][c], width, None))
                    })
                    .collect();
                let mut split = self.split(cells, [now, later]);
                let count = split.iter().map(|(_, f, _)| f.pages.len()).max().unwrap_or(1);
                // The last part is as tall as the tallest cell that reaches it.
                let tail = split
                    .iter()
                    .filter(|(_, f, _)| f.pages.len() == count)
                    .map(|(_, f, _)| f.cursor + f.pending)
                    .fold(0.0, f32::max);
                let page = flow.pages.len() as u64;
                for part in 0..count {
                    let height = match part {
                        _ if part + 1 == count => tail,
                        0 => now,
                        _ => later,
                    };
                    if part > 0 {
                        let extra = if part + 1 == count {
                            gy + tail
                        } else {
                            f32::INFINITY
                        };
                        turn(self, flow, &mut drawn, &mut first, i + 1, extra);
                    }
                    let y = flow.place(gy + height) + gy;
                    if part == 0 {
                        self.mark(&laid[r], flow);
                    }
                    // A cell's tags do not close on the page it starts on, so each piece of
                    // it is a cell of its own.
                    flow.page().items.push(Item::Tag(Role::Row));
                    for (left, cell, _) in &mut split {
                        if let Some(on) = cell.pages.get_mut(part) {
                            let items = std::mem::take(&mut on.items)
                                .into_iter()
                                .filter(|item| !matches!(item, Item::Tag(_) | Item::Untag))
                                .map(|item| shift(item, *left, y));
                            let page = flow.page();
                            page.items.push(Item::Tag(Role::Cell));
                            page.items.extend(items);
                            page.items.push(Item::Untag);
                        }
                    }
                    flow.page().items.push(Item::Untag);
                }
                for (_, _, missing) in split {
                    carry(&mut self.missing, missing, page);
                }
                drawn[r] = true;
                here = 1;
                continue;
            }
            if here > 0
                && let Some((top, limit, _)) = flow.room()
                && (top + piece_height(piece) + below > limit
                    || keep(i, top, limit, first, here))
            {
                turn(self, flow, &mut drawn, &mut first, i, 0.0);
                here = 0;
            }
            self.rows(piece, &laid, gy, flow, &mut drawn);
            here += piece.len();
        }
        self.rows(&shown(&foot, first, true), &laid, gy, flow, &mut drawn);
        if !rows.is_empty() {
            flow.place(gy);
        }
        for (atom, bottom) in &captions {
            if *bottom {
                self.caption(*atom, x, width, flow);
            }
        }
    }

    /// Draws the rows `piece` as one piece under the last block, each with the vertical
    /// spacing above it. A row drawn before is drawn again from a copy of its items.
    fn rows(
        &mut self,
        piece: &[usize],
        laid: &[Laid],
        gy: f32,
        flow: &mut Flow,
        drawn: &mut [bool],
    ) {
        if piece.is_empty() {
            return;
        }
        let height: f32 = piece.iter().map(|r| gy + laid[*r].height).sum();
        let mut y = flow.place(height);
        let page = flow.pages.len() as u64;
        for &r in piece {
            y += gy;
            if !drawn[r] {
                self.mark(&laid[r], flow);
            }
            // A row drawn again is not read again.
            let role = if drawn[r] { Role::Artifact } else { Role::Row };
            flow.page().items.push(Item::Tag(role));
            for (left, cell) in &laid[r].cells {
                let left = *left;
                let items = cell
                    .items
                    .iter()
                    .cloned()
                    .map(|item| shift(item, left, MARGIN + y));
                flow.page().items.extend(items);
                if !drawn[r] {
                    carry(&mut self.missing, cell.missing.clone(), page);
                }
            }
            flow.page().items.push(Item::Untag);
            for (name, [x, top, width, height]) in &laid[r].slots {
                let area = [*x, MARGIN + y + top, *width, *height];
                flow.page().slots.push((name.clone(), area));
            }
            drawn[r] = true;
            y += laid[r].height;
        }
    }

    /// Sets the marks of the row `row` on the last page of `flow`.
    fn mark(&mut self, row: &Laid, flow: &Flow) {
        for (name, value) in &row.marks {
            self.strings.push(margin::Set {
                name: name.clone(),
                value: value.clone(),
                page: flow.pages.len() - 1,
                top: false,
                mark: true,
            });
        }
    }

    /// Lays each cell of a row taller than a page out down pages of its own: the first is
    /// `rooms[0]` tall and every later one `rooms[1]`. A cell breaks between lines as a block
    /// box does, and its items stay on the page they are laid on. Each cell keeps its left edge.
    fn split(
        &mut self,
        cells: Vec<(f32, Vec<Block>)>,
        rooms: [f32; 2],
    ) -> Vec<(f32, Flow, Vec<Missing>)> {
        cells
            .into_iter()
            .map(|(left, blocks)| {
                let [now, later] = rooms;
                let pages = Flow::paged(Box::new(move |page, _| if page == 0 { now } else { later }));
                let (flow, missing) = self.down(blocks, pages);
                (left, flow, missing)
            })
            .collect()
    }

    /// Lays the caption at `atom` out at the table's content width and places it.
    fn caption(&mut self, atom: usize, x: f32, width: f32, flow: &mut Flow) {
        let blocks = self.waiting[atom].take().unwrap_or_default();
        let mut laid = self.item_at(&blocks, width, None);
        let top = flow.place(laid.height);
        let page = flow.pages.len() as u64;
        let items: Vec<_> = std::mem::take(&mut laid.items)
            .into_iter()
            .map(|item| shift(item, x, MARGIN + top))
            .collect();
        flow.page().items.extend(items);
        carry(&mut self.missing, laid.missing, page);
    }

    /// The columns' widths in `room` under `table-layout: auto`.
    fn auto_columns(
        &mut self,
        rows: &[Row],
        blocks: &[Vec<Vec<Block>>],
        starts: &[Vec<usize>],
        columns: usize,
        room: f32,
        gap: f32,
    ) -> Vec<f32> {
        let mut min = vec![0.0f32; columns];
        let mut max = vec![0.0f32; columns];
        let mut spanning = Vec::new();
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.cells.iter().enumerate() {
                let from = starts[r][c];
                let lo = self.content_width(&blocks[r][c], false);
                let hi = self.content_width(&blocks[r][c], true).max(lo);
                if cell.span == 1 {
                    min[from] = min[from].max(lo);
                    max[from] = max[from].max(hi);
                } else {
                    spanning.push((from, cell.span.min(columns - from), lo, hi));
                }
            }
        }
        for (from, span, lo, hi) in spanning {
            spread(&mut min, from, span, gap, lo);
            spread(&mut max, from, span, gap, hi);
        }
        distribute(&min, &max, room)
    }

    /// The columns' widths in `room` under `table-layout: fixed`: from the first row's cells
    /// with a `width`, and the rest shared evenly.
    fn fixed_columns(
        &mut self,
        rows: &[Row],
        blocks: &[Vec<Vec<Block>>],
        starts: &[Vec<usize>],
        columns: usize,
        room: f32,
        gap: f32,
    ) -> Vec<f32> {
        let mut widths: Vec<Option<f32>> = vec![None; columns];
        if let Some(row) = rows.first() {
            for (c, cell) in row.cells.iter().enumerate() {
                if let Some(Block::Open(g)) = blocks[0][c].first()
                    && !g.fill
                {
                    let from = starts[0][c];
                    let span = cell.span.min(columns - from);
                    let each = (g.width + g.margin[1] + g.margin[3] - gap * (span - 1) as f32)
                        / span as f32;
                    for w in &mut widths[from..from + span] {
                        *w = Some(each.max(0.0));
                    }
                }
            }
        }
        let set: f32 = widths.iter().flatten().sum();
        let free = widths.iter().filter(|w| w.is_none()).count();
        let share = if free > 0 {
            (room - set).max(0.0) / free as f32
        } else {
            0.0
        };
        widths.into_iter().map(|w| w.unwrap_or(share)).collect()
    }
}
