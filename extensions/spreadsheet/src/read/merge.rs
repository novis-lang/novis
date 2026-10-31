//! Merged ranges: [`Merge`], one range as a sheet declares it, and [`Fill`], which gives each cell
//! of a range its top-left value when a read asks for `merged: 'fill'`.
//!
//! **Where each format keeps them.** An xlsx sheet lists its ranges in `<mergeCells>`, after
//! `<sheetData>`, and an xlsb sheet in records after its data, so `super::sheet::merged` and
//! `super::binary::merged` read the sheet's part to its end. xls has `MERGEDCELLS` records and ods
//! a cell's `number-columns-spanned` and `number-rows-spanned`, both read with the rows at
//! `super::Reader::open`.
//!
//! **A fill reads the ranges before the first row**, at one more pass over the part of an xlsx or
//! an xlsb sheet. [`Fill`] keeps the ranges not reached yet, sorted by their first row, and the ones
//! the read is inside, each with its top-left value. A range is dropped once the read is past its
//! last row, so each row costs the ranges it is inside, not every range of the sheet. A row the
//! file does not write is given when it lies in a range below the range's top row, because its
//! cells have a value to fill.

use super::{MOST_COLUMNS, MOST_ROWS, Read, ReadCell, ReadRow, letters, sheet};
use crate::Error;

/// One merged range, its rows and columns from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Merge {
    pub first_row: u64,
    pub last_row: u64,
    pub first_column: u64,
    pub last_column: u64,
}

impl Merge {
    /// The range from its corners, in either order, or [`Error::Parse`] for a corner outside the
    /// grid.
    pub fn new(rows: (u64, u64), columns: (u64, u64)) -> Result<Self, Error> {
        let inside = |n: u64, most: u64| (1..=most).contains(&n);
        if !inside(rows.0, MOST_ROWS)
            || !inside(rows.1, MOST_ROWS)
            || !inside(columns.0, MOST_COLUMNS)
            || !inside(columns.1, MOST_COLUMNS)
        {
            return Err(Error::Parse(
                "The file is not a valid workbook: a merged range is past the last row or column of the sheet."
                    .to_owned(),
            ));
        }
        Ok(Self {
            first_row: rows.0.min(rows.1),
            last_row: rows.0.max(rows.1),
            first_column: columns.0.min(columns.1),
            last_column: columns.0.max(columns.1),
        })
    }

    /// The range an xlsx writes as `A1:C3`.
    pub fn parse(reference: &[u8]) -> Result<Self, Error> {
        let mut corners = reference.splitn(2, |&b| b == b':');
        let first = corners.next().unwrap_or_default();
        let last = corners.next().unwrap_or(first);
        let corner = |text: &[u8]| {
            sheet::column_of(text)
                .zip(sheet::row_of(text))
                .ok_or_else(|| {
                    Error::Parse(
                        "The file is not a valid workbook: a merged range is not a range of cells."
                            .to_owned(),
                    )
                })
        };
        let (first_column, first_row) = corner(first)?;
        let (last_column, last_row) = corner(last)?;
        Self::new((first_row, last_row), (first_column, last_column))
    }

    /// The range as `A1:C3` text.
    pub fn text(&self) -> String {
        format!(
            "{}{}:{}{}",
            letters(self.first_column),
            self.first_row,
            letters(self.last_column),
            self.last_row
        )
    }
}

/// The merged ranges of the sheet a `merged: 'fill'` read is reading.
#[derive(Debug, Default)]
pub struct Fill {
    /// The ranges the read has not reached, the one that starts first at the end.
    pending: Vec<Merge>,
    /// The ranges the read is inside, each with its top-left value and the text it displays.
    active: Vec<(Merge, Option<(Read, String)>)>,
}

impl Fill {
    pub fn new(mut ranges: Vec<Merge>) -> Self {
        ranges.sort_by_key(|range| std::cmp::Reverse(range.first_row));
        Self {
            pending: ranges,
            active: Vec::new(),
        }
    }

    /// Moves the read to the row `number`: drops each range that ends before it and starts each
    /// range that starts at or before it. Returns the ranges that start at `number`, as their
    /// place in `active` and their top-left column, for [`Fill::set`].
    pub fn reach(&mut self, number: u64) -> Vec<(usize, u64)> {
        self.active.retain(|(range, _)| range.last_row >= number);
        let mut starting = Vec::new();
        while let Some(range) = self.pending.pop_if(|range| range.first_row <= number) {
            if range.last_row < number {
                continue;
            }
            if range.first_row == number {
                starting.push((self.active.len(), range.first_column));
            }
            self.active.push((range, None));
        }
        starting
    }

    /// Gives the range at `at` in `active` its top-left value and the text it displays.
    pub fn set(&mut self, at: usize, value: Option<(Read, String)>) {
        if let Some((_, slot)) = self.active.get_mut(at) {
            *slot = value;
        }
    }

    /// The first row from `from` and before `before` that lies in a range below its top row.
    pub fn covered(&self, from: u64, before: u64) -> Option<u64> {
        let started = self.active.iter().map(|(range, _)| range);
        let next = self
            .pending
            .iter()
            .rev()
            .take_while(|range| range.first_row < before);
        started
            .chain(next)
            .filter_map(|range| {
                let row = from.max(range.first_row + 1);
                (row <= range.last_row && row < before).then_some(row)
            })
            .min()
    }

    /// Gives each cell of `row` that lies in a range the range's top-left value. A cell in a
    /// column of `shown` also gets the top-left cell's displayed text. A range whose top-left
    /// cell is empty changes nothing.
    pub fn fill(&self, row: &mut ReadRow, shown: &[(u64, u64)]) {
        let mut added = Vec::new();
        for (range, value) in &self.active {
            let Some((value, text)) = value else {
                continue;
            };
            if !(range.first_row..=range.last_row).contains(&row.number) {
                continue;
            }
            for column in range.first_column..=range.last_column {
                let display = shown
                    .iter()
                    .any(|&(first, last)| (first..=last).contains(&column))
                    .then(|| text.clone());
                match row.cells.binary_search_by_key(&column, |cell| cell.column) {
                    Ok(at) => {
                        row.cells[at].value = Some(value.clone());
                        row.cells[at].display = display;
                    }
                    Err(_) => added.push(ReadCell {
                        column,
                        value: Some(value.clone()),
                        formula: None,
                        display,
                    }),
                }
            }
        }
        if !added.is_empty() {
            row.cells.extend(added);
            row.cells.sort_by_key(|cell| cell.column);
            row.cells.dedup_by_key(|cell| cell.column);
        }
    }
}
