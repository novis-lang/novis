//! Tables across pages (`rule:core-classes/pdf-tables-across-pages`).
//!
//! A `table` or `inline-table` is a block box whose content is one [`Table`] block. Its
//! `table-row` children, directly or inside a `table-header-group`, `table-row-group` or
//! `table-footer-group`, are its rows, and each `table-cell` in a row is a cell, collected the
//! way a flex item is. A cell directly in a table or a group is in a row of its own, and text
//! directly in a table, a group or a row is left out. A `table-caption` is laid out at the
//! table's content width, above the rows or under them by its `caption-side`, and only once.
//! A `table` or `inline-table` directly in a table, a group or a row is in an anonymous cell of
//! its own, with no style, as CSS 2.1 says.
//!
//! **Columns** are computed once for the whole table, so every page has the same ones. A table
//! with a `width` is that wide. A table without one is as wide as its columns laid out
//! unbroken, up to its containing block, and is laid out as `table-layout: auto`. Each
//! column is as wide as the widest of its cells laid out unbroken, when the room allows, and
//! the room left over is shared out by those widths. With less room, each column narrows from
//! that width towards its narrowest cell broken at every chance. A cell over several columns
//! (`colspan`) spreads what its columns lack over them evenly. A `col` or `colgroup` with a
//! `width`, or a `width` attribute, sets the widest width of each column it covers, and its
//! narrowest cell can make it wider. Under `table-layout: fixed`, a `col` width sets its
//! column, a cell of the first row with a `width` sets the columns no `col` sets, and the
//! other columns share the rest evenly. `border-spacing` is the room between two cells and
//! between a cell and the table's content edge. Under `direction: rtl` the first column is
//! on the right, and a table narrower than its containing block keeps its right edge.
//! `colspan`, `rowspan` and `span` are read by
//! HTML's rules for non-negative integers, so `2x` is 2, and `rowspan="0"` runs to the last
//! row of its group.
//!
//! **Rows.** A row is as tall as its tallest cell, and a `rowspan` cell taller than its rows
//! makes the last of them taller. Each cell is drawn as tall as its rows, and its
//! `vertical-align` places its content in it: `top` (the default, and any other value),
//! `middle`, `bottom`, or `baseline`, which puts the first line of each `baseline` cell of a
//! row level with the lowest of them. The content moves down by more top padding, so the
//! cell's box keeps its place. A row's background is drawn under its cells, and a group's
//! under its rows, once on each page they are on. Their borders are drawn only under
//! `border-collapse: collapse`, as CSS 2.1 says, where they win an edge.
//! The rows are drawn in
//! this order: the header rows, the body rows in source order, then the footer rows. A body row
//! is placed as one piece, so a row that fits on a page is never split, and rows a `rowspan`
//! joins are one piece. A piece that does not fit under the last one, above the footer rows,
//! goes on the next page. A forced `break-before` or `break-after` on a body row or its group
//! starts a new page between two pieces, and is ignored inside a piece. A `left`, `right`,
//! `recto` or `verso` break goes to the next page, whatever its side.
//!
//! **A piece taller than a page**, one row or the rows a `rowspan` joins, starts where it is and
//! splits between lines. Each row starts under the one before it, on the page that row's own
//! cells end on. Each of its cells
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
//! fits on it. The header and footer rows are left off every page when the room they leave
//! is less than the shortest body piece, or less than a quarter of a page when every piece is
//! taller, so a page never holds them and no row.
//!
//! **Borders at a break.** The table's box and each cell of a split row are closed at every
//! page break, as `block`'s module doc says, unless `box-decoration-break` is `slice`.
//!
//! **Collapsed borders.** Under `border-collapse: collapse`, each edge of the grid is one
//! border, resolved by CSS 2.1 §17.6.2.1 from the borders of every box on it: the cells, the
//! rows, the row groups, the columns, the column groups and the table. A `hidden` border wins,
//! then the wider one, then the style `double`, `solid`, `dashed`, `dotted`, then the box
//! nearer the cell in that list, then the box further up and further toward the start side.
//! The cells, rows, groups and the table draw no border of their own: each width moves into
//! the box's padding, so its content stays where it is. Cells that meet overlap by the widest
//! resolved edge, and each edge is drawn centred in that strip, after the cells of its row,
//! so no background covers it. Where a repeated header row or a footer row meets a body row
//! it is not next to in the source, the stronger of their two edges is drawn. The edge at a
//! page break is resolved as the table's own top or bottom edge, so each page is closed by
//! the table's border where it wins, whatever `box-decoration-break` says. `border-spacing`
//! is not used.
//!
//! Cost: one grid slot per row and column, and the resolved edges of each row, while the
//! table is laid out.
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
//! Cost: each cell is laid out once at each of its two content widths in a layout pass, however
//! often the boxes around its table are measured, once at its column width and once more when
//! its row is taller, taller than a page, or its `vertical-align` is not `top`. A repeated row's
//! items are copied on each page. The heights and row counts of the body pieces are summed once
//! from the end, so paging reads each piece a number of times bounded by the rows a page holds
//! ([`super::Work::table_reads`]). Memory: two widths for each cell, per layout pass, and three
//! numbers for each body piece while the table is paged.

use super::block::{BoxGeometry, FIT, Flow};
use super::boxes::{Line, edges};
use super::flex::fit;
use super::text::{Cx, Here, Placed, carry, shift};
use super::{BLACK, Block, Fact, Item, MARGIN, Role, keyword};
use crate::css::{Color, Style, Value, value};

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
    /// Whether its `direction` is `rtl`, so its first column is on the right.
    pub(super) rtl: bool,
    /// How many body rows a page break leaves before it and after it, when it can.
    pub(super) orphans: usize,
    pub(super) widows: usize,
    /// Each caption's index in the list of atoms, and whether it is under the rows.
    pub(super) captions: Vec<(usize, bool)>,
    /// The `width` of each column a `col` or `colgroup` sets, in order, `None` where it sets
    /// none.
    pub(super) cols: Vec<Option<f32>>,
    /// The border of each column, with its column group's where that group's side is the
    /// column's, in order. Read only under `border-collapse: collapse`.
    pub(super) col_edges: Vec<[Edge; 4]>,
    pub(super) rows: Vec<Row>,
    /// The box each group element draws under its rows, by the group's number less one.
    pub(super) groups: Vec<Option<BoxGeometry>>,
    /// The border of each group element, by the group's number less one.
    pub(super) group_edges: Vec<[Edge; 4]>,
    /// The table's own border. Under `border-collapse: collapse` its box draws none, and its
    /// cells draw it where it wins.
    pub(super) edges: [Edge; 4],
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
    /// The box it draws under its cells: its background, and its borders when they collapse.
    pub(super) decor: Option<BoxGeometry>,
    /// Its border, which is drawn only where it wins a collapsed edge.
    pub(super) edges: [Edge; 4],
    /// Whether a forced page break comes before it and after it, from its own `break-before`
    /// and `break-after` or its group's.
    pub(super) breaks: [bool; 2],
}

/// A cell: its index in the list of atoms, its `colspan`, its `rowspan` and its
/// `vertical-align`.
#[derive(Clone, Copy)]
pub(super) struct Cell {
    pub(super) atom: usize,
    pub(super) span: usize,
    pub(super) rows: usize,
    pub(super) align: Align,
}

/// Where a cell's content is placed in a cell taller than it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Align {
    Top,
    Middle,
    Bottom,
    /// The first line level with the first line of the row's other `baseline` cells.
    Baseline,
}

impl Align {
    /// The `vertical-align` of a cell with `style`, `top` when it has none or another value.
    pub(super) fn of(style: &Style) -> Align {
        match keyword(style, "vertical-align") {
            Some("middle") => Align::Middle,
            Some("bottom") => Align::Bottom,
            Some("baseline") => Align::Baseline,
            _ => Align::Top,
        }
    }
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

/// A `colspan`, `rowspan` or `span` attribute read by HTML's rules for non-negative integers:
/// leading spaces, an optional `+`, then the digits up to the first other character, so `2x`
/// is 2. `None` when there are no digits, and `usize::MAX` for more than it holds.
pub(super) fn span_number(text: &str) -> Option<usize> {
    let rest = text.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let rest = rest.strip_prefix('+').unwrap_or(rest);
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0).then(|| rest[..digits].parse().unwrap_or(usize::MAX))
}

/// Gives each `rowspan="0"` cell the rows from its own to the last of its row group: the run
/// of rows with the same group, or of rows directly in the table. Each group's end is found
/// once, from the last row up.
pub(super) fn span_to_group_end(rows: &mut [Row]) {
    let mut end = rows.len();
    for r in (0..rows.len()).rev() {
        if r + 1 < rows.len()
            && (rows[r].group != rows[r + 1].group || rows[r].part != rows[r + 1].part)
        {
            end = r + 1;
        }
        for cell in &mut rows[r].cells {
            if cell.rows == 0 {
                cell.rows = end - r;
            }
        }
    }
}

/// One side of a box in a collapsed table: its width, line, colour, whether it is `hidden`, and
/// its owner's rank in CSS 2.1's order, from 0 for a cell to 5 for the table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Edge {
    pub(super) width: f32,
    pub(super) line: Line,
    pub(super) color: Color,
    pub(super) hidden: bool,
    pub(super) owner: u8,
}

/// The owners of a collapsed edge, in CSS 2.1's order.
pub(super) const CELL: u8 = 0;
pub(super) const ROW: u8 = 1;
pub(super) const GROUP: u8 = 2;
pub(super) const COLUMN: u8 = 3;
pub(super) const COLUMN_GROUP: u8 = 4;
pub(super) const TABLE: u8 = 5;

impl Edge {
    /// No border.
    pub(super) const NONE: Edge = Edge {
        width: 0.0,
        line: Line::Solid,
        color: BLACK,
        hidden: false,
        owner: TABLE,
    };

    /// The four sides of the box `g`, owned by `owner`.
    pub(super) fn sides(g: &BoxGeometry, owner: u8) -> [Edge; 4] {
        [0, 1, 2, 3].map(|s| Edge {
            width: g.border[s],
            line: g.decor.line[s],
            color: g.decor.border_color[s],
            hidden: g.decor.hidden[s],
            owner,
        })
    }

    /// Whether this edge wins over `other` by CSS 2.1 §17.6.2.1: `hidden`, then the wider,
    /// then the style double, solid, dashed, dotted, then the owner nearer the cell. A tie is
    /// lost, so the edge met first wins it.
    fn beats(&self, other: &Edge) -> bool {
        if self.hidden || other.hidden {
            return self.hidden && !other.hidden;
        }
        let rank = |e: &Edge| match e.line {
            Line::Double => 4,
            Line::Solid => 3,
            Line::Dashed => 2,
            Line::Dotted => 1,
        };
        if (self.width - other.width).abs() > FIT {
            return self.width > other.width;
        }
        if self.width <= 0.0 {
            return false;
        }
        if rank(self) != rank(other) {
            return rank(self) > rank(other);
        }
        self.owner < other.owner
    }

    /// The stronger of this edge and `other`, this one on a tie.
    pub(super) fn max(self, other: Edge) -> Edge {
        if other.beats(&self) { other } else { self }
    }

    /// The edge `edges` resolve to, `None` when it draws nothing: the winner is `hidden` or
    /// has no width.
    fn resolve(edges: impl IntoIterator<Item = Edge>) -> Option<Edge> {
        let won = edges.into_iter().reduce(Edge::max)?;
        (!won.hidden && won.width > 0.0).then_some(won)
    }
}

/// The edges of a collapsed row, resolved: its top and bottom edge over each column, as the
/// column's left edge, its width and the edge, by [`TOP`], [`BOTTOM`], [`OUTER_TOP`] and
/// [`OUTER_BOTTOM`], and its sides, each a column line's left edge and its edge.
#[derive(Default, Clone)]
struct Rim {
    across: [Vec<Option<(f32, f32, Edge)>>; 4],
    sides: Vec<(f32, Edge)>,
}

/// A row's top edge against the row above it, and its bottom edge against the row under it.
const TOP: usize = 0;
const BOTTOM: usize = 1;
/// A row's top edge and its bottom edge against the table's, as at a page break.
const OUTER_TOP: usize = 2;
const OUTER_BOTTOM: usize = 3;

/// Each grid slot's cell, row by row, as its row and its index in that row: a `rowspan` cell
/// covers its rows below, and a `colspan` cell its columns.
fn cover(rows: &[Row], starts: &[Vec<usize>], columns: usize) -> Vec<Vec<Option<(usize, usize)>>> {
    let mut cover = vec![vec![None; columns]; rows.len()];
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.cells.iter().enumerate() {
            let from = starts[r][c].min(columns);
            let end = (from + cell.span).min(columns);
            for line in cover.iter_mut().skip(r).take(cell.rows.max(1)) {
                for slot in &mut line[from..end] {
                    slot.get_or_insert((r, c));
                }
            }
        }
    }
    cover
}

/// The collapsed edges of each row, in source order: its top and bottom edge over each column,
/// against the rows beside it and against the table's, and its side at each column line,
/// `None` where nothing is drawn or a cell goes on across it. A row's edge at a page break is
/// resolved as the table's edge, so the table is closed on every page. `seq` is the rows top
/// to bottom, and `owners` the columns', the groups' and the
/// table's borders. Each cell's border moves into its padding, so its content stays where it
/// is and the cell draws no border of its own.
///
/// Cost: one slot per row and column while the table is laid out.
fn resolve_edges(
    rows: &[Row],
    blocks: &mut [Vec<Vec<Block>>],
    starts: &[Vec<usize>],
    columns: usize,
    seq: &[usize],
    owners: (&[[Edge; 4]], &[[Edge; 4]], [Edge; 4]),
    rtl: bool,
) -> Vec<Resolved> {
    let (cols, groups, table) = owners;
    let cells: Vec<Vec<[Edge; 4]>> = blocks
        .iter_mut()
        .map(|row| {
            row.iter_mut()
                .map(|cell| match cell.first_mut() {
                    Some(Block::Open(g)) => {
                        let sides = Edge::sides(g, CELL);
                        for s in 0..4 {
                            g.padding[s] += g.border[s];
                            g.border[s] = 0.0;
                        }
                        sides
                    }
                    _ => [Edge::NONE; 4],
                })
                .collect()
        })
        .collect();
    let cover = cover(rows, starts, columns);
    let cell = |r: usize, col: usize, side: usize| {
        cover[r][col].map_or(Edge::NONE, |(r0, c)| cells[r0][c][side])
    };
    let group = |r: usize, side: usize| {
        rows[r]
            .group
            .and_then(|g| groups.get(g - 1))
            .map_or(Edge::NONE, |e| e[side])
    };
    let col = |c: usize, side: usize| cols.get(c).map_or(Edge::NONE, |e| e[side]);
    // The physical side a row starts on and the one it ends on.
    let (start, end) = if rtl { (1, 3) } else { (3, 1) };
    let mut out = vec![Resolved::default(); rows.len()];
    for (k, &r) in seq.iter().enumerate() {
        let near = [k.checked_sub(1).map(|p| seq[p]), seq.get(k + 1).copied()];
        // The top edge (0) and the bottom edge (2) over each column.
        let across = |side: usize, other: Option<usize>| -> Vec<Option<Edge>> {
            (0..columns)
                .map(|c| match other {
                    Some(o) if cover[r][c].is_some() && cover[o][c] == cover[r][c] => None,
                    Some(o) => {
                        // The upper box comes first, so it wins a tie.
                        let (up, down) = if side == 0 { (o, r) } else { (r, o) };
                        let mut all = vec![
                            cell(up, c, 2),
                            cell(down, c, 0),
                            rows[up].edges[2],
                            rows[down].edges[0],
                        ];
                        if rows[o].group != rows[r].group || rows[o].part != rows[r].part {
                            all.extend([group(up, 2), group(down, 0)]);
                        }
                        Edge::resolve(all)
                    }
                    None => Edge::resolve([
                        cell(r, c, side),
                        rows[r].edges[side],
                        group(r, side),
                        col(c, side),
                        table[side],
                    ]),
                })
                .collect()
        };
        let across = [
            across(0, near[0]),
            across(2, near[1]),
            across(0, None),
            across(2, None),
        ];
        let sides = (0..=columns)
            .map(|line| {
                let before = line.checked_sub(1);
                let after = (line < columns).then_some(line);
                match (before, after) {
                    (Some(b), Some(a)) if cover[r][b].is_some() && cover[r][b] == cover[r][a] => {
                        None
                    }
                    (Some(b), Some(a)) => Edge::resolve([
                        cell(r, b, end),
                        cell(r, a, start),
                        col(b, end),
                        col(a, start),
                    ]),
                    (None, Some(a)) => Edge::resolve([
                        cell(r, a, start),
                        rows[r].edges[start],
                        group(r, start),
                        col(a, start),
                        table[start],
                    ]),
                    (Some(b), None) => Edge::resolve([
                        cell(r, b, end),
                        rows[r].edges[end],
                        group(r, end),
                        col(b, end),
                        table[end],
                    ]),
                    (None, None) => None,
                }
            })
            .collect();
        out[r] = Resolved { across, sides };
    }
    out
}

/// A row's resolved edges: its top and bottom edges over each column, by [`TOP`],
/// [`BOTTOM`], [`OUTER_TOP`] and [`OUTER_BOTTOM`], and its side at each column line.
#[derive(Default, Clone)]
struct Resolved {
    across: [Vec<Option<Edge>>; 4],
    sides: Vec<Option<Edge>>,
}

/// The items of a collapsed row's edges, drawn in strips `strip` wide, between `top` and
/// `bottom` on the page: its sides when `sides`, the top edge `head` and the bottom edge
/// `foot`, each one of [`TOP`] to [`OUTER_BOTTOM`]. Each edge is centred in its strip.
fn rim_items(
    rim: &Rim,
    strip: f32,
    (top, bottom): (f32, f32),
    sides: bool,
    head: Option<usize>,
    foot: Option<usize>,
) -> Vec<Item> {
    let mut out = Vec::new();
    let flat = [(0.0, 0.0); 4];
    let inset = |e: &Edge| ((strip - e.width) / 2.0).max(0.0);
    for (x, e) in rim.sides.iter().filter(|_| sides) {
        let size = (x + inset(e), top, e.width, (bottom - top).max(0.0));
        edges(
            &mut out,
            size,
            [0.0, 0.0, 0.0, e.width],
            [e.line; 4],
            [e.color; 4],
            flat,
        );
    }
    let mut across = |run: &[Option<(f32, f32, Edge)>], y: f32| {
        for (x, w, e) in run.iter().flatten() {
            let size = (*x, y + inset(e), *w, e.width);
            edges(
                &mut out,
                size,
                [e.width, 0.0, 0.0, 0.0],
                [e.line; 4],
                [e.color; 4],
                flat,
            );
        }
    };
    if let Some(head) = head {
        across(&rim.across[head], top);
    }
    if let Some(foot) = foot {
        across(&rim.across[foot], bottom - strip);
    }
    out
}

/// Draws the bottom edge of the collapsed row drawn last, on its page, if it is still owed:
/// against the table's when `outer`, at a page break.
fn close_rim(laid: &[Laid], flow: &mut Flow, drawing: &mut Drawing, outer: bool) {
    if let Some((r, page)) = drawing.pending.take() {
        let bottom = MARGIN + drawing.bottom;
        let ends = (bottom - drawing.strip, bottom);
        let which = if outer { OUTER_BOTTOM } else { BOTTOM };
        let items = rim_items(&laid[r].rim, drawing.strip, ends, false, None, Some(which));
        flow.pages[page].items.extend(items);
    }
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
/// and its cell's content box, its top from the row's top, the row's marks, the box it draws
/// under its cells and the number of its group.
struct Laid {
    cells: Vec<(f32, Placed)>,
    height: f32,
    slots: Vec<(String, [f32; 4])>,
    marks: Vec<(String, String)>,
    under: Option<BoxGeometry>,
    group: Option<usize>,
    /// Its collapsed edges, empty when the borders are separate.
    rim: Rim,
}

/// What a table has drawn so far: whether each row is drawn, the left edge and the width of
/// its rows, each run of one group's rows on one page, the width of a collapsed edge's
/// strip, and the collapsed row whose bottom edge is still owed, with its page and its bottom.
struct Drawing {
    drawn: Vec<bool>,
    left: f32,
    width: f32,
    runs: Vec<Run>,
    strip: f32,
    pending: Option<(usize, usize)>,
    bottom: f32,
    /// The last page a row was drawn on.
    page: Option<usize>,
}

/// Rows of one group drawn on one page: the group's number, the page's index, the index of
/// the page's first item drawn for them, and their top and bottom.
struct Run {
    group: usize,
    page: usize,
    at: usize,
    top: f32,
    bottom: f32,
}

/// A place in a piece split over pages: the part it is in, and its top from that part's top.
type At = (usize, f32);

/// The later of two places in a split piece.
fn latest(a: At, b: At) -> At {
    if (b.0, b.1) > (a.0, a.1) { b } else { a }
}

impl Cx<'_> {
    /// Lays the rows of `table` out with the same columns throughout, and places them down
    /// `flow` with its header and footer rows on each page it is on.
    pub(super) fn table(&mut self, table: Table, flow: &mut Flow) {
        let Table {
            mut x,
            mut width,
            spacing,
            fixed,
            collapse,
            rtl,
            orphans,
            widows,
            captions,
            cols,
            col_edges,
            rows,
            groups,
            group_edges,
            edges,
        } = table;
        let mut blocks: Vec<Vec<Vec<Block>>> = rows
            .iter()
            .map(|row| {
                row.cells
                    .iter()
                    .map(|c| self.waiting[c.atom].take().unwrap_or_default())
                    .collect()
            })
            .collect();
        let (starts, columns) = grid(&rows);
        // The rows top to bottom: the header rows, the body rows, then the footer rows.
        let seq: Vec<usize> = {
            let rows = &rows;
            [Part::Head, Part::Body, Part::Foot]
                .into_iter()
                .flat_map(|part| (0..rows.len()).filter(move |r| rows[*r].part == part))
                .collect()
        };
        // Under `border-collapse: collapse`, each edge is resolved once, and cells that meet
        // overlap by the widest resolved edge: that strip is where the edge is drawn.
        let resolved = if collapse {
            resolve_edges(
                &rows,
                &mut blocks,
                &starts,
                columns,
                &seq,
                (&col_edges, &group_edges, edges),
                rtl,
            )
        } else {
            Vec::new()
        };
        let strip = resolved
            .iter()
            .flat_map(|r| r.across.iter().flatten().chain(&r.sides))
            .flatten()
            .map(|e| e.width)
            .fold(0.0, f32::max);
        let [gx, gy] = if collapse { [-strip; 2] } else { spacing };
        let gaps = gx * (columns as f32 + 1.0);
        // A table with no `width` is as wide as its columns laid out unbroken, up to its
        // containing block, and its layout is `auto`, as CSS 2.1 says.
        let auto = flow.fills();
        let widths = if fixed && !auto {
            let room = (width - gaps).max(0.0);
            self.fixed_columns(&rows, &blocks, &starts, &cols, columns, room, gx)
        } else {
            let (min, max) = self.column_ranges(&rows, &blocks, &starts, &cols, columns, gx);
            let need = max.iter().sum::<f32>() + gaps;
            // A right-to-left table keeps its right edge.
            if auto && need < width {
                flow.narrow(width - need, rtl);
                if rtl {
                    x += width - need;
                }
                width = need;
            }
            distribute(&min, &max, (width - gaps).max(0.0))
        };
        let mut lefts = Vec::with_capacity(columns);
        let mut at = x + gx;
        for w in &widths {
            lefts.push(at);
            at += w + gx;
        }
        let mut drawing = Drawing {
            drawn: vec![false; rows.len()],
            left: lefts[0],
            width: at - gx - lefts[0],
            runs: Vec::new(),
            strip,
            pending: None,
            bottom: 0.0,
            page: None,
        };
        // A right-to-left table has its columns mirrored in its content box.
        if rtl {
            for (left, w) in lefts.iter_mut().zip(&widths) {
                *left = 2.0 * x + width - *left - w;
            }
            drawing.left = 2.0 * x + width - (drawing.left + drawing.width);
        }
        // A cell's left edge: its first column's, or its last one's when the table is right
        // to left.
        let cell_left = |from: usize, span: usize| {
            let column = if rtl { from + span - 1 } else { from };
            lefts[column.min(columns - 1)]
        };
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
                here.push(laid);
            }
            measured.push(here);
        }
        // Each row's baseline: the lowest first line of its `baseline` cells. A `baseline`
        // cell moves down by the difference, and its row grows to hold it.
        let baselines: Vec<Option<f32>> = rows
            .iter()
            .zip(&measured)
            .map(|(row, here)| {
                row.cells
                    .iter()
                    .zip(here)
                    .filter(|(cell, _)| cell.align == Align::Baseline)
                    .filter_map(|(_, placed)| placed.first)
                    .reduce(f32::max)
            })
            .collect();
        let lowered = |r: usize, cell: &Cell, placed: &Placed| match (cell.align, baselines[r]) {
            (Align::Baseline, Some(base)) => placed.first.map_or(0.0, |first| base - first),
            _ => 0.0,
        };
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.cells.iter().enumerate() {
                if cell.rows == 1 {
                    let placed = &measured[r][c];
                    heights[r] = heights[r].max(placed.height + lowered(r, cell, placed));
                }
            }
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
                let free = (tall - first.height).max(0.0);
                // The content moves down by more top padding, so the cell keeps its height.
                let down = match cell.align {
                    Align::Top => 0.0,
                    Align::Middle => free / 2.0,
                    Align::Bottom => free,
                    Align::Baseline => lowered(r, cell, &first),
                }
                .clamp(0.0, free);
                let width = cell_width(starts[r][c], cell.span);
                let placed = if down > 0.01 {
                    let mut moved = blocks[r][c].clone();
                    if let Some(Block::Open(g)) = moved.first_mut() {
                        g.padding[0] += down;
                    }
                    self.item_at(&moved, width, Some(tall))
                } else if free > 0.01 {
                    self.item_at(&blocks[r][c], width, Some(tall))
                } else {
                    first
                };
                cells.push((cell_left(starts[r][c], cell.span), placed));
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
            // The collapsed edges at their places: each column's left edge and width, and
            // each column line's strip.
            let rim = resolved.get(r).map_or_else(Rim::default, |resolved| {
                let run = |edges: &[Option<Edge>]| {
                    edges
                        .iter()
                        .enumerate()
                        .map(|(c, e)| e.map(|e| (lefts[c], widths[c], e)))
                        .collect()
                };
                let line_x = |line: usize| match (rtl, line < columns) {
                    (false, true) => lefts[line],
                    (false, false) => lefts[columns - 1] + widths[columns - 1] - strip,
                    (true, true) => lefts[line] + widths[line] - strip,
                    (true, false) => lefts[columns - 1],
                };
                Rim {
                    across: [0, 1, 2, 3].map(|i| run(&resolved.across[i])),
                    sides: resolved
                        .sides
                        .iter()
                        .enumerate()
                        .filter_map(|(line, e)| e.map(|e| (line_x(line), e)))
                        .collect(),
                }
            });
            laid.push(Laid {
                cells,
                height: heights[r],
                slots,
                marks: row.marks.clone(),
                under: row.decor.clone(),
                group: row.group,
                rim,
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
        // The header and footer rows are left off when the room they leave on a page is less
        // than the shortest body piece, or than a quarter of the page when every piece is
        // taller. A row then never shares a page with nothing but them.
        let (mut head, mut foot) = (head, foot);
        if flow.paginates() && !pieces.is_empty() {
            let tallest = |rows_of: &[usize]| {
                [true, false]
                    .into_iter()
                    .flat_map(|first| [true, false].map(|last| (first, last)))
                    .map(|(first, last)| piece_height(&shown(rows_of, first, last)))
                    .fold(0.0, f32::max)
            };
            let page = flow.next_limit();
            let room = page - tallest(&head) - tallest(&foot) - 2.0 * gy;
            let shortest = pieces
                .iter()
                .map(|p| piece_height(p))
                .fold(f32::INFINITY, f32::min);
            if room + FIT < shortest.min(page / 4.0) {
                head.clear();
                foot.clear();
            }
        }

        for (atom, bottom) in &captions {
            if !bottom {
                self.caption(*atom, x, width, flow);
            }
        }
        // Each piece's height, and the height and the row count of the pieces from each one to
        // the end, summed once, so paging reads each in one step.
        let reads = std::cell::Cell::new(pieces.len());
        let heights_of: Vec<f32> = pieces.iter().map(|p| piece_height(p)).collect();
        let mut after = vec![(0.0_f32, 0_usize); pieces.len() + 1];
        for p in (0..pieces.len()).rev() {
            after[p] = (
                after[p + 1].0 + heights_of[p],
                after[p + 1].1 + pieces[p].len(),
            );
        }
        let height = |p: usize| {
            reads.set(reads.get() + 1);
            heights_of[p]
        };
        let rows_from = |from: usize| {
            reads.set(reads.get() + 1);
            after[from].1
        };
        let rest = |from: usize| {
            reads.set(reads.get() + 1);
            after[from].0
        };
        // A table does not start at the bottom of a page with fewer than `orphans` body rows
        // under its header rows.
        if let Some((top, limit, empty)) = flow.room() {
            let lead = (1..=pieces.len())
                .find(|n| rows_from(0) - rows_from(*n) >= orphans)
                .unwrap_or(pieces.len());
            let start = piece_height(&shown(&head, true, false))
                + (rest(0) - rest(lead))
                + piece_height(&shown(&foot, true, lead == pieces.len()));
            if !empty && top + start > limit {
                flow.break_page();
            }
        }
        let mut first = true;
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
                    drawing: &mut Drawing,
                    first: &mut bool,
                    from: usize,
                    extra: f32| {
            cx.rows(&shown(&foot, *first, false), &laid, gy, flow, drawing);
            close_rim(&laid, flow, drawing, true);
            // Collapsed borders end on the table's bottom edge, as they do on the last page.
            if gy < 0.0 {
                flow.place(gy);
            }
            flow.break_page();
            *first = false;
            let last = fits_rest(flow, from, false, extra);
            cx.rows(&shown(&head, false, last), &laid, gy, flow, drawing);
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
            let below =
                |end: bool| piece_height(&shown(&foot, first, end)) + if end { gy } else { 0.0 };
            if let Some(group) = group_of(i)
                && (i == 0 || group_of(i - 1) != Some(group))
            {
                let end = (i..count)
                    .find(|p| group_of(*p) != Some(group))
                    .unwrap_or(count);
                let tall = rest(i) - rest(end);
                if tall <= later && top + tall + below(end == count) > limit {
                    return true;
                }
            }
            let (mut y, mut k) = (top, i);
            while k < count && y + height(k) + below(k + 1 == count) <= limit {
                y += height(k);
                k += 1;
            }
            if k == count || rows_from(k) >= widows || height(k) > later {
                return false;
            }
            let target = (i..=k).rev().find(|j| rows_from(*j) >= widows);
            target == Some(i) && here >= orphans
        };
        let last = fits_rest(flow, 0, first, 0.0);
        self.rows(&shown(&head, first, last), &laid, gy, flow, &mut drawing);
        let mut here = 0;
        for (i, piece) in pieces.iter().enumerate() {
            // A `break-before` on the piece's first row or a `break-after` on the last row of
            // the piece before starts a new page. Any forced value breaks to the next page.
            let forced = rows[piece[0]].breaks[0]
                || i.checked_sub(1)
                    .and_then(|p| pieces[p].last())
                    .is_some_and(|r| rows[*r].breaks[1]);
            if forced && here > 0 {
                turn(self, flow, &mut drawing, &mut first, i, 0.0);
                here = 0;
            }
            let end = i + 1 == pieces.len();
            let below = piece_height(&shown(&foot, first, end)) + if end { gy } else { 0.0 };
            // A piece taller than a page starts where it is and splits between lines.
            if flow.paginates() && height(i) - gy > later {
                let room = |flow: &mut Flow, first: bool| match flow.room() {
                    Some((top, limit, _)) => {
                        limit - top - gy - piece_height(&shown(&foot, first, false)) - 0.01
                    }
                    None => f32::INFINITY,
                };
                if room(flow, first) <= 0.0 {
                    turn(self, flow, &mut drawing, &mut first, i, 0.0);
                }
                close_rim(&laid, flow, &mut drawing, false);
                let now = room(flow, first);
                let part_room = |part: usize| if part == 0 { now } else { later };
                // Each row starts under the row before it, in the part that row ends in, and
                // each of its cells is laid out down pages of its own from there.
                let mut cursor: At = (0, 0.0);
                let mut end: At = cursor;
                // Each row's start, the end of its own cells and the end of all its cells.
                let mut spans: Vec<(At, At, At)> = Vec::with_capacity(piece.len());
                // Each cell's row in the piece, its start, its left edge and its pages.
                let mut parts: Vec<(usize, At, f32, Flow)> = Vec::new();
                let mut lost = Vec::new();
                for (k, &r) in piece.iter().enumerate() {
                    if cursor.1 >= part_room(cursor.0) - 0.01 {
                        cursor = (cursor.0 + 1, 0.0);
                    }
                    let start = cursor;
                    let (mut own, mut reach) = (start, start);
                    for (c, cell) in rows[r].cells.iter().enumerate() {
                        let width = cell_width(starts[r][c], cell.span);
                        let rooms = [part_room(start.0) - start.1, later];
                        let pages = Flow::paged(
                            Box::new(move |page, _| rooms[usize::from(page > 0)]),
                            None,
                        );
                        let (pages, missing) = self.down(fit(&blocks[r][c], width, None), pages);
                        let count = pages.pages.len();
                        let bottom = pages.cursor + pages.pending;
                        let done = if count == 1 {
                            (start.0, start.1 + bottom)
                        } else {
                            (start.0 + count - 1, bottom)
                        };
                        if cell.rows == 1 {
                            own = latest(own, done);
                        }
                        reach = latest(reach, done);
                        parts.push((k, start, laid[r].cells[c].0, pages));
                        lost.push(missing);
                    }
                    spans.push((start, own, reach));
                    end = latest(end, reach);
                    cursor = (own.0, own.1 + gy);
                }
                // The last part is as tall as the lowest cell that reaches it.
                let (count, tail) = (end.0 + 1, end.1);
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
                        turn(self, flow, &mut drawing, &mut first, i + 1, extra);
                    }
                    let y = flow.place(gy + height) + gy;
                    for (k, &r) in piece.iter().enumerate() {
                        let (start, own, reach) = spans[k];
                        if part < start.0 || part > reach.0 {
                            continue;
                        }
                        if part == start.0 {
                            self.mark(&laid[r], flow);
                        }
                        if part <= own.0 {
                            let top = if part == start.0 { start.1 } else { 0.0 };
                            let bottom = if part == own.0 { own.1 } else { height };
                            self.under(&laid[r], y + top, y + bottom, flow, &mut drawing);
                        }
                        // Each part of a collapsed row is closed by its top and bottom edges.
                        let ends = if part <= own.0 {
                            let top = if part == start.0 { start.1 } else { 0.0 };
                            let bottom = if part == own.0 { own.1 } else { height };
                            let head = if part == start.0 { TOP } else { OUTER_TOP };
                            let foot = if part == own.0 { BOTTOM } else { OUTER_BOTTOM };
                            Some(((MARGIN + y + top, MARGIN + y + bottom), head, foot))
                        } else {
                            None
                        };
                        // A cell's tags do not close on the page it starts on, so each piece
                        // of it is a cell of its own.
                        flow.page().items.push(Item::Tag(Role::Row));
                        for (_, start, left, cell) in parts.iter_mut().filter(|p| p.0 == k) {
                            let Some(on) = part
                                .checked_sub(start.0)
                                .and_then(|n| cell.pages.get_mut(n))
                            else {
                                continue;
                            };
                            let dy = y + if part == start.0 { start.1 } else { 0.0 };
                            let items = std::mem::take(&mut on.items)
                                .into_iter()
                                .filter(|item| !matches!(item, Item::Tag(_) | Item::Untag))
                                .map(|item| shift(item, *left, dy));
                            let page = flow.page();
                            page.items.push(Item::Tag(Role::Cell));
                            page.items.extend(items);
                            page.items.push(Item::Untag);
                        }
                        flow.page().items.push(Item::Untag);
                        if let Some((ends, head, foot)) = ends {
                            let rim = &laid[r].rim;
                            let items = rim_items(rim, strip, ends, true, Some(head), Some(foot));
                            flow.page().items.extend(items);
                            drawing.page = Some(flow.pages.len() - 1);
                        }
                    }
                }
                for missing in lost {
                    carry(&mut self.missing, missing, page);
                }
                for &r in piece {
                    drawing.drawn[r] = true;
                }
                here = piece.len();
                continue;
            }
            if here > 0
                && let Some((top, limit, _)) = flow.room()
                && (top + height(i) + below > limit || keep(i, top, limit, first, here))
            {
                turn(self, flow, &mut drawing, &mut first, i, 0.0);
                here = 0;
            }
            self.rows(piece, &laid, gy, flow, &mut drawing);
            here += piece.len();
        }
        self.rows(&shown(&foot, first, true), &laid, gy, flow, &mut drawing);
        close_rim(&laid, flow, &mut drawing, false);
        self.work.table_reads += reads.get();
        if !rows.is_empty() {
            flow.place(gy);
        }
        // Each group's box is drawn under its rows, once for each page they are on. The last
        // run is put in first, so the place of each earlier one stays where it was.
        for run in std::mem::take(&mut drawing.runs).into_iter().rev() {
            let Some(Some(g)) = groups.get(run.group - 1) else {
                continue;
            };
            let items: Vec<Item> = self
                .decoration(g, drawing.width, run.bottom - run.top)
                .into_iter()
                .map(|item| shift(item, drawing.left, MARGIN + run.top))
                .collect();
            flow.pages[run.page].items.splice(run.at..run.at, items);
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
        drawing: &mut Drawing,
    ) {
        if piece.is_empty() {
            return;
        }
        let height: f32 = piece.iter().map(|r| gy + laid[*r].height).sum();
        let mut y = flow.place(height);
        let page = flow.pages.len() as u64;
        for &r in piece {
            y += gy;
            let drawn = drawing.drawn[r];
            if !drawn {
                self.mark(&laid[r], flow);
            }
            self.under(&laid[r], y, y + laid[r].height, flow, drawing);
            // A row drawn again is not read again.
            let role = if drawn { Role::Artifact } else { Role::Row };
            flow.page().items.push(Item::Tag(role));
            for (left, cell) in &laid[r].cells {
                let left = *left;
                let items = cell
                    .items
                    .iter()
                    .cloned()
                    .map(|item| shift(item, left, MARGIN + y));
                flow.page().items.extend(items);
                if !drawn {
                    carry(&mut self.missing, cell.missing.clone(), page);
                }
            }
            flow.page().items.push(Item::Untag);
            // The edge under the row drawn last on this page, when it meets this row, is one
            // edge with this row's top: the stronger of the two over each column. It is the
            // same edge unless a header row repeats above this row or a footer row follows it.
            // An edge still owed elsewhere is drawn first. Each is drawn after this row's
            // cells, so no background covers it.
            // The first row on a page has its top edge against the table's.
            let page = flow.pages.len() - 1;
            let head = if drawing.page == Some(page) {
                TOP
            } else {
                OUTER_TOP
            };
            drawing.page = Some(page);
            let mut rim = laid[r].rim.clone();
            if let Some((above, on)) = drawing.pending
                && on == page
                && (drawing.bottom - drawing.strip - y).abs() < FIT
            {
                drawing.pending = None;
                let under = &laid[above].rim.across[BOTTOM];
                for (top, under) in rim.across[head].iter_mut().zip(under) {
                    *top = match (*top, *under) {
                        (Some((x, w, a)), Some((_, _, b))) => Some((x, w, a.max(b))),
                        (a, b) => a.or(b),
                    };
                }
            }
            close_rim(laid, flow, drawing, false);
            let ends = (MARGIN + y, MARGIN + y + laid[r].height);
            let items = rim_items(&rim, drawing.strip, ends, true, Some(head), None);
            flow.page().items.extend(items);
            if laid[r].rim.across[BOTTOM].iter().any(Option::is_some)
                || laid[r].rim.across[OUTER_BOTTOM].iter().any(Option::is_some)
            {
                drawing.pending = Some((r, flow.pages.len() - 1));
                drawing.bottom = y + laid[r].height;
            }
            for (name, [x, top, width, height]) in &laid[r].slots {
                let area = [*x, MARGIN + y + top, *width, *height];
                flow.page().slots.push((name.clone(), area));
            }
            drawing.drawn[r] = true;
            y += laid[r].height;
        }
    }

    /// Draws the box of the row `row` under its cells, from `top` to `bottom` on the last
    /// page of `flow`, and adds the row to its group's run on that page.
    fn under(&mut self, row: &Laid, top: f32, bottom: f32, flow: &mut Flow, drawing: &mut Drawing) {
        let page = flow.pages.len() - 1;
        let at = flow.page().items.len();
        if let Some(group) = row.group {
            match drawing.runs.last_mut() {
                Some(run) if run.group == group && run.page == page => run.bottom = bottom,
                _ => drawing.runs.push(Run {
                    group,
                    page,
                    at,
                    top,
                    bottom,
                }),
            }
        }
        if let Some(g) = &row.under {
            let items: Vec<Item> = self
                .decoration(g, drawing.width, bottom - top)
                .into_iter()
                .map(|item| shift(item, drawing.left, MARGIN + top))
                .collect();
            flow.page().items.extend(items);
        }
    }

    /// The items of the box `g`, `width` wide and `height` tall, with its top-left corner at
    /// 0, 0.
    fn decoration(&mut self, g: &BoxGeometry, width: f32, height: f32) -> Vec<Item> {
        let mut g = g.clone();
        g.x = 0.0;
        g.width = width;
        g.min_content = (height - g.border[0] - g.border[2]).max(0.0);
        self.lay(
            vec![Block::Open(g), Block::Close],
            parley::VerticalAlign::BASELINE,
        )
        .items
    }

    /// Sets the marks of the row `row` on the page of `flow` it is drawn on.
    fn mark(&mut self, row: &Laid, flow: &mut Flow) {
        for (name, value) in &row.marks {
            let mark = Here::String {
                name: name.clone(),
                value: value.clone(),
                fact: Fact::Mark,
            };
            self.note(flow, mark);
        }
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

    /// The width `table`'s rows need from its content box's left edge: with each cell
    /// unbroken when `max` is true, and broken at every chance when it is false. A table with
    /// a `width` of its own needs no more than it has.
    pub(super) fn table_need(&mut self, table: &Table, max: bool) -> f32 {
        let blocks: Vec<Vec<Vec<Block>>> = table
            .rows
            .iter()
            .map(|row| {
                row.cells
                    .iter()
                    .map(|c| self.waiting[c.atom].take().unwrap_or_default())
                    .collect()
            })
            .collect();
        let (starts, columns) = grid(&table.rows);
        let gx = if table.collapse {
            0.0
        } else {
            table.spacing[0]
        };
        let (min, most) =
            self.column_ranges(&table.rows, &blocks, &starts, &table.cols, columns, gx);
        for (row, cells) in table.rows.iter().zip(blocks) {
            for (cell, blocks) in row.cells.iter().zip(cells) {
                self.waiting[cell.atom] = Some(blocks);
            }
        }
        let widths = if max { most } else { min };
        (widths.iter().sum::<f32>() + gx * (columns as f32 + 1.0)).min(table.width)
    }

    /// Each column's narrowest and widest width under `table-layout: auto`. A column with a
    /// `col` width is that wide, or as wide as its narrowest cell when that is wider.
    #[allow(clippy::too_many_arguments)]
    fn column_ranges(
        &mut self,
        rows: &[Row],
        blocks: &[Vec<Vec<Block>>],
        starts: &[Vec<usize>],
        cols: &[Option<f32>],
        columns: usize,
        gap: f32,
    ) -> (Vec<f32>, Vec<f32>) {
        let mut min = vec![0.0f32; columns];
        let mut max = vec![0.0f32; columns];
        let mut spanning = Vec::new();
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.cells.iter().enumerate() {
                let from = starts[r][c];
                // A cell inside a nested table is measured once, however many tables around
                // it measure theirs, so nesting costs no more than the cells it has.
                let (lo, hi) = match self.ranges.get(&cell.atom) {
                    Some(&range) => range,
                    None => {
                        let lo = self.content_width(&blocks[r][c], false);
                        let hi = self.content_width(&blocks[r][c], true).max(lo);
                        self.ranges.insert(cell.atom, (lo, hi));
                        (lo, hi)
                    }
                };
                if cell.span == 1 {
                    min[from] = min[from].max(lo);
                    max[from] = max[from].max(hi);
                } else {
                    spanning.push((from, cell.span.min(columns - from), lo, hi));
                }
            }
        }
        for (c, width) in cols.iter().enumerate().take(columns) {
            if let Some(w) = width {
                max[c] = w.max(min[c]);
            }
        }
        for (from, span, lo, hi) in spanning {
            spread(&mut min, from, span, gap, lo);
            spread(&mut max, from, span, gap, hi);
        }
        (min, max)
    }

    /// The columns' widths in `room` under `table-layout: fixed`: from the `col` widths, then
    /// from the first row's cells with a `width` for the columns no `col` sets, and the rest
    /// shared evenly.
    #[allow(clippy::too_many_arguments)]
    fn fixed_columns(
        &mut self,
        rows: &[Row],
        blocks: &[Vec<Vec<Block>>],
        starts: &[Vec<usize>],
        cols: &[Option<f32>],
        columns: usize,
        room: f32,
        gap: f32,
    ) -> Vec<f32> {
        let mut widths: Vec<Option<f32>> = (0..columns)
            .map(|c| cols.get(c).copied().flatten())
            .collect();
        let set_by_col: Vec<bool> = widths.iter().map(Option::is_some).collect();
        if let Some(row) = rows.first() {
            for (c, cell) in row.cells.iter().enumerate() {
                if let Some(Block::Open(g)) = blocks[0][c].first()
                    && !g.fill
                {
                    let from = starts[0][c];
                    let span = cell.span.min(columns - from);
                    let each = (g.width + g.margin[1] + g.margin[3] - gap * (span - 1) as f32)
                        / span as f32;
                    for (w, by_col) in widths[from..from + span]
                        .iter_mut()
                        .zip(&set_by_col[from..from + span])
                    {
                        if !by_col {
                            *w = Some(each.max(0.0));
                        }
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
