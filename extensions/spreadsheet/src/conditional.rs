//! Conditional formats: one `<conditionalFormatting>` per rule the program adds
//! (`rule:core-classes/spreadsheet-sheet-end-parts`).
//!
//! [`check`] checks a rule before anything is kept, and [`write`] writes it with its priority,
//! which counts the sheet's rules from 1 in the order they are added, so an earlier rule wins
//! where two apply. The tail writes the rules after the merged cells, as the xlsx schema orders
//! them.
//!
//! | Kind | What it writes |
//! |---|---|
//! | `Cell` | `cellIs` with the operator and its one or two values |
//! | `Formula` | `expression` with its one formula, true for the cells to format |
//! | `ColorScale` | two or three colours from the lowest value to the highest, red, yellow and green when none is given |
//! | `DataBar` | a bar of one colour, blue when none is given |
//! | `IconSet` | one of Excel's icon sets, split into equal percent bands |
//! | `Top`, `Bottom` | `top10` over `rank` cells or `rank` percent, 10 when not given |
//! | `Duplicates`, `Unique` | `duplicateValues` and `uniqueValues` |
//!
//! A value is formula text, so a text value is quoted: `"Done"`. A rule that formats cells needs
//! a `style`, which `styles.xml` keeps as a `dxf` ([`crate::styles::Styles::dxf`]); a colour scale,
//! a data bar and an icon set draw their own colours and take none.
//!
//! Memory: per rule, its written text until the sheet ends, and per distinct style one `dxf` until
//! the workbook ends.

use std::fmt::Write as _;

use crate::styles::argb;
use crate::tail::{Operator, area};
use crate::{Error, Style, xml};

/// The icon sets Excel draws.
const ICON_SETS: [&str; 17] = [
    "3Arrows",
    "3ArrowsGray",
    "3Flags",
    "3TrafficLights1",
    "3TrafficLights2",
    "3Signs",
    "3Symbols",
    "3Symbols2",
    "4Arrows",
    "4ArrowsGray",
    "4RedToBlack",
    "4Rating",
    "4TrafficLights",
    "5Arrows",
    "5ArrowsGray",
    "5Rating",
    "5Quarters",
];

/// The kinds of rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Cell,
    Formula,
    ColorScale,
    DataBar,
    IconSet,
    Top,
    Bottom,
    Duplicates,
    Unique,
}

impl Kind {
    /// Whether the rule formats its cells with a style.
    fn styled(self) -> bool {
        !matches!(self, Kind::ColorScale | Kind::DataBar | Kind::IconSet)
    }

    /// Its name in a message.
    fn name(self) -> &'static str {
        match self {
            Kind::Cell => "cell",
            Kind::Formula => "formula",
            Kind::ColorScale => "colorScale",
            Kind::DataBar => "dataBar",
            Kind::IconSet => "iconSet",
            Kind::Top => "top",
            Kind::Bottom => "bottom",
            Kind::Duplicates => "duplicates",
            Kind::Unique => "unique",
        }
    }
}

/// One conditional format over `formatted`, one range or several separated by spaces.
#[derive(Debug, Clone, PartialEq)]
pub struct Conditional {
    pub formatted: String,
    pub kind: Kind,
    pub operator: Option<Operator>,
    pub values: Vec<String>,
    pub colors: Vec<String>,
    pub icons: Option<String>,
    pub rank: Option<u64>,
    pub percent: bool,
    pub style: Option<Style>,
}

fn invalid(rule: &Conditional, what: &str) -> Error {
    Error::Invalid(format!(
        "The {} conditional format over `{}` {what}",
        rule.kind.name(),
        rule.formatted
    ))
}

/// Checks `rule`, all but its style, which [`crate::styles::Styles::dxf`] checks.
pub fn check(rule: &Conditional) -> Result<(), Error> {
    if rule.formatted.split(' ').any(|range| area(range).is_none()) {
        return Err(invalid(
            rule,
            "does not name a range of cells, such as `B2:B100`.",
        ));
    }
    match (rule.kind.styled(), &rule.style) {
        (true, None) => {
            return Err(invalid(rule, "needs a `style` for the cells it formats."));
        }
        (false, Some(_)) => {
            return Err(invalid(
                rule,
                "draws its own colors, so it takes `colors` and no `style`.",
            ));
        }
        _ => {}
    }
    for color in &rule.colors {
        argb(color)?;
    }
    match rule.kind {
        Kind::Cell => {
            let Some(operator) = rule.operator else {
                return Err(invalid(rule, "needs an operator, such as `greaterThan`."));
            };
            if rule.values.len() != operator.operands() {
                return Err(invalid(
                    rule,
                    &format!(
                        "has {} values. The operator `{}` needs {}.",
                        rule.values.len(),
                        operator.name(),
                        operator.operands()
                    ),
                ));
            }
        }
        Kind::Formula => {
            if rule.values.len() != 1 {
                return Err(invalid(rule, "needs one formula in `values`."));
            }
        }
        Kind::ColorScale => {
            if ![0, 2, 3].contains(&rule.colors.len()) {
                return Err(invalid(rule, "needs two or three colors, or none."));
            }
        }
        Kind::DataBar => {
            if rule.colors.len() > 1 {
                return Err(invalid(rule, "takes one color."));
            }
        }
        Kind::IconSet => {
            if let Some(icons) = &rule.icons
                && !ICON_SETS.contains(&icons.as_str())
            {
                return Err(invalid(
                    rule,
                    &format!(
                        "names the icon set `{icons}`. The icon sets are {}.",
                        ICON_SETS.join(", ")
                    ),
                ));
            }
        }
        Kind::Top | Kind::Bottom => {
            let most = if rule.percent { 100 } else { 1_000 };
            if let Some(rank) = rule.rank
                && !(1..=most).contains(&rank)
            {
                return Err(invalid(
                    rule,
                    &format!("has the rank {rank}. The rank is from 1 to {most}."),
                ));
            }
        }
        Kind::Duplicates | Kind::Unique => {}
    }
    if matches!(rule.kind, Kind::Cell | Kind::Formula) {
        for value in &rule.values {
            crate::cells::check_formula(value, || {
                format!(
                    "the {} conditional format over `{}`",
                    rule.kind.name(),
                    rule.formatted
                )
            })?;
        }
    }
    Ok(())
}

/// Appends `<formula>` with `value`, its leading `=` dropped.
fn formula(out: &mut String, value: &str) {
    out.push_str("<formula>");
    xml::text(out, value.strip_prefix('=').unwrap_or(value));
    out.push_str("</formula>");
}

/// `rule`, checked by [`check`], as its `<conditionalFormatting>` element, `dxf` being the id of
/// its style and `priority` its place among the sheet's rules.
pub fn write(rule: &Conditional, dxf: Option<u32>, priority: usize) -> String {
    let mut out = String::from("<conditionalFormatting sqref=\"");
    xml::attr(&mut out, &rule.formatted);
    out.push_str("\"><cfRule type=\"");
    out.push_str(match rule.kind {
        Kind::Cell => "cellIs",
        Kind::Formula => "expression",
        Kind::ColorScale => "colorScale",
        Kind::DataBar => "dataBar",
        Kind::IconSet => "iconSet",
        Kind::Top | Kind::Bottom => "top10",
        Kind::Duplicates => "duplicateValues",
        Kind::Unique => "uniqueValues",
    });
    out.push('"');
    if let Some(dxf) = dxf {
        let _ = write!(out, " dxfId=\"{dxf}\"");
    }
    let _ = write!(out, " priority=\"{priority}\"");
    let colors: Vec<String> = rule
        .colors
        .iter()
        .map(|color| argb(color).unwrap_or_default())
        .collect();
    match rule.kind {
        Kind::Cell => {
            let operator = rule.operator.unwrap_or(Operator::Equal);
            let _ = write!(out, " operator=\"{}\">", operator.name());
            for value in &rule.values {
                formula(&mut out, value);
            }
        }
        Kind::Formula => {
            out.push('>');
            formula(&mut out, &rule.values[0]);
        }
        Kind::ColorScale => {
            let colors = if colors.is_empty() {
                vec![
                    "FFF8696B".to_owned(),
                    "FFFFEB84".to_owned(),
                    "FF63BE7B".to_owned(),
                ]
            } else {
                colors
            };
            out.push_str("><colorScale><cfvo type=\"min\"/>");
            if colors.len() == 3 {
                out.push_str("<cfvo type=\"percentile\" val=\"50\"/>");
            }
            out.push_str("<cfvo type=\"max\"/>");
            for color in &colors {
                let _ = write!(out, "<color rgb=\"{color}\"/>");
            }
            out.push_str("</colorScale>");
        }
        Kind::DataBar => {
            let color = colors.first().map_or("FF638EC6", String::as_str);
            let _ = write!(
                out,
                "><dataBar><cfvo type=\"min\"/><cfvo type=\"max\"/><color rgb=\"{color}\"/></dataBar>"
            );
        }
        Kind::IconSet => {
            let icons = rule.icons.as_deref().unwrap_or("3TrafficLights1");
            let count: u32 = icons[..1].parse().unwrap_or(3);
            let _ = write!(out, "><iconSet iconSet=\"{icons}\">");
            for band in 0..count {
                let _ = write!(
                    out,
                    "<cfvo type=\"percent\" val=\"{}\"/>",
                    (band * 100 + count / 2) / count
                );
            }
            out.push_str("</iconSet>");
        }
        Kind::Top | Kind::Bottom => {
            let _ = write!(out, " rank=\"{}\"", rule.rank.unwrap_or(10));
            if rule.percent {
                out.push_str(" percent=\"1\"");
            }
            if rule.kind == Kind::Bottom {
                out.push_str(" bottom=\"1\"");
            }
            out.push_str("/></conditionalFormatting>");
            return out;
        }
        Kind::Duplicates | Kind::Unique => {
            out.push_str("/></conditionalFormatting>");
            return out;
        }
    }
    out.push_str("</cfRule></conditionalFormatting>");
    out
}
