//! The prefixes xlsx stores before a function newer than Excel 2007 (MS-XLSX § 2.2.2, "future
//! functions").
//!
//! A formula a program writes says `XLOOKUP(…)`, and Excel reads that name in a file as an
//! unknown function and shows `#NAME?`. The file has to say `_xlfn.XLOOKUP(…)`, and `FILTER` and
//! `SORT` take `_xlfn._xlws.`. [`prefixed`] adds the prefix to each name of [`FUTURE`] that is
//! followed by `(`, in any case, and leaves a name alone inside a string literal (`"…"`), a quoted
//! sheet name (`'…'`) and a structured reference (`[…]`), and when it has a prefix already. The
//! xlsx writer calls it for a cell's formula, a conditional format's and a validation's. The ods
//! writer and the template filler leave formulas as they are. Cost: one pass over the formula's
//! text, and a copy only when a prefix is added.

use std::borrow::Cow;

/// Every function MS-XLSX § 2.2.2 lists as a future function, and those Excel added since, in
/// byte order, with the prefix each is stored with.
pub const FUTURE: &[(&str, &str)] = &[
    ("ACOT", "_xlfn."),
    ("ACOTH", "_xlfn."),
    ("AGGREGATE", "_xlfn."),
    ("ANCHORARRAY", "_xlfn."),
    ("ARABIC", "_xlfn."),
    ("ARRAYTOTEXT", "_xlfn."),
    ("BASE", "_xlfn."),
    ("BETA.DIST", "_xlfn."),
    ("BETA.INV", "_xlfn."),
    ("BINOM.DIST", "_xlfn."),
    ("BINOM.DIST.RANGE", "_xlfn."),
    ("BINOM.INV", "_xlfn."),
    ("BITAND", "_xlfn."),
    ("BITLSHIFT", "_xlfn."),
    ("BITOR", "_xlfn."),
    ("BITRSHIFT", "_xlfn."),
    ("BITXOR", "_xlfn."),
    ("BYCOL", "_xlfn."),
    ("BYROW", "_xlfn."),
    ("CEILING.MATH", "_xlfn."),
    ("CEILING.PRECISE", "_xlfn."),
    ("CHISQ.DIST", "_xlfn."),
    ("CHISQ.DIST.RT", "_xlfn."),
    ("CHISQ.INV", "_xlfn."),
    ("CHISQ.INV.RT", "_xlfn."),
    ("CHISQ.TEST", "_xlfn."),
    ("CHOOSECOLS", "_xlfn."),
    ("CHOOSEROWS", "_xlfn."),
    ("COMBINA", "_xlfn."),
    ("CONCAT", "_xlfn."),
    ("CONFIDENCE.NORM", "_xlfn."),
    ("CONFIDENCE.T", "_xlfn."),
    ("COT", "_xlfn."),
    ("COTH", "_xlfn."),
    ("COVARIANCE.P", "_xlfn."),
    ("COVARIANCE.S", "_xlfn."),
    ("CSC", "_xlfn."),
    ("CSCH", "_xlfn."),
    ("DAYS", "_xlfn."),
    ("DECIMAL", "_xlfn."),
    ("DROP", "_xlfn."),
    ("ECMA.CEILING", "_xlfn."),
    ("ENCODEURL", "_xlfn."),
    ("ERF.PRECISE", "_xlfn."),
    ("ERFC.PRECISE", "_xlfn."),
    ("EXPAND", "_xlfn."),
    ("EXPON.DIST", "_xlfn."),
    ("F.DIST", "_xlfn."),
    ("F.DIST.RT", "_xlfn."),
    ("F.INV", "_xlfn."),
    ("F.INV.RT", "_xlfn."),
    ("F.TEST", "_xlfn."),
    ("FILTER", "_xlfn._xlws."),
    ("FILTERXML", "_xlfn."),
    ("FLOOR.MATH", "_xlfn."),
    ("FLOOR.PRECISE", "_xlfn."),
    ("FORECAST.ETS", "_xlfn."),
    ("FORECAST.ETS.CONFINT", "_xlfn."),
    ("FORECAST.ETS.SEASONALITY", "_xlfn."),
    ("FORECAST.ETS.STAT", "_xlfn."),
    ("FORECAST.LINEAR", "_xlfn."),
    ("FORMULATEXT", "_xlfn."),
    ("GAMMA", "_xlfn."),
    ("GAMMA.DIST", "_xlfn."),
    ("GAMMA.INV", "_xlfn."),
    ("GAMMALN.PRECISE", "_xlfn."),
    ("GAUSS", "_xlfn."),
    ("GROUPBY", "_xlfn."),
    ("HSTACK", "_xlfn."),
    ("HYPGEOM.DIST", "_xlfn."),
    ("IFNA", "_xlfn."),
    ("IFS", "_xlfn."),
    ("IMAGE", "_xlfn."),
    ("IMCOSH", "_xlfn."),
    ("IMCOT", "_xlfn."),
    ("IMCSC", "_xlfn."),
    ("IMCSCH", "_xlfn."),
    ("IMSEC", "_xlfn."),
    ("IMSECH", "_xlfn."),
    ("IMSINH", "_xlfn."),
    ("IMTAN", "_xlfn."),
    ("ISFORMULA", "_xlfn."),
    ("ISO.CEILING", "_xlfn."),
    ("ISOMITTED", "_xlfn."),
    ("ISOWEEKNUM", "_xlfn."),
    ("LAMBDA", "_xlfn."),
    ("LET", "_xlfn."),
    ("LOGNORM.DIST", "_xlfn."),
    ("LOGNORM.INV", "_xlfn."),
    ("MAKEARRAY", "_xlfn."),
    ("MAP", "_xlfn."),
    ("MAXIFS", "_xlfn."),
    ("MINIFS", "_xlfn."),
    ("MODE.MULT", "_xlfn."),
    ("MODE.SNGL", "_xlfn."),
    ("MUNIT", "_xlfn."),
    ("NEGBINOM.DIST", "_xlfn."),
    ("NETWORKDAYS.INTL", "_xlfn."),
    ("NORM.DIST", "_xlfn."),
    ("NORM.INV", "_xlfn."),
    ("NORM.S.DIST", "_xlfn."),
    ("NORM.S.INV", "_xlfn."),
    ("NUMBERVALUE", "_xlfn."),
    ("PDURATION", "_xlfn."),
    ("PERCENTILE.EXC", "_xlfn."),
    ("PERCENTILE.INC", "_xlfn."),
    ("PERCENTOF", "_xlfn."),
    ("PERCENTRANK.EXC", "_xlfn."),
    ("PERCENTRANK.INC", "_xlfn."),
    ("PERMUTATIONA", "_xlfn."),
    ("PHI", "_xlfn."),
    ("PIVOTBY", "_xlfn."),
    ("POISSON.DIST", "_xlfn."),
    ("QUARTILE.EXC", "_xlfn."),
    ("QUARTILE.INC", "_xlfn."),
    ("QUERYSTRING", "_xlfn."),
    ("RANDARRAY", "_xlfn."),
    ("RANK.AVG", "_xlfn."),
    ("RANK.EQ", "_xlfn."),
    ("REDUCE", "_xlfn."),
    ("REGEXEXTRACT", "_xlfn."),
    ("REGEXREPLACE", "_xlfn."),
    ("REGEXTEST", "_xlfn."),
    ("RRI", "_xlfn."),
    ("SCAN", "_xlfn."),
    ("SEC", "_xlfn."),
    ("SECH", "_xlfn."),
    ("SEQUENCE", "_xlfn."),
    ("SHEET", "_xlfn."),
    ("SHEETS", "_xlfn."),
    ("SINGLE", "_xlfn."),
    ("SKEW.P", "_xlfn."),
    ("SORT", "_xlfn._xlws."),
    ("SORTBY", "_xlfn."),
    ("STDEV.P", "_xlfn."),
    ("STDEV.S", "_xlfn."),
    ("STOCKHISTORY", "_xlfn."),
    ("SWITCH", "_xlfn."),
    ("T.DIST", "_xlfn."),
    ("T.DIST.2T", "_xlfn."),
    ("T.DIST.RT", "_xlfn."),
    ("T.INV", "_xlfn."),
    ("T.INV.2T", "_xlfn."),
    ("T.TEST", "_xlfn."),
    ("TAKE", "_xlfn."),
    ("TEXTAFTER", "_xlfn."),
    ("TEXTBEFORE", "_xlfn."),
    ("TEXTJOIN", "_xlfn."),
    ("TEXTSPLIT", "_xlfn."),
    ("TOCOL", "_xlfn."),
    ("TOROW", "_xlfn."),
    ("TRIMRANGE", "_xlfn."),
    ("UNICHAR", "_xlfn."),
    ("UNICODE", "_xlfn."),
    ("UNIQUE", "_xlfn."),
    ("VALUETOTEXT", "_xlfn."),
    ("VAR.P", "_xlfn."),
    ("VAR.S", "_xlfn."),
    ("VSTACK", "_xlfn."),
    ("WEBSERVICE", "_xlfn."),
    ("WEIBULL.DIST", "_xlfn."),
    ("WORKDAY.INTL", "_xlfn."),
    ("WRAPCOLS", "_xlfn."),
    ("WRAPROWS", "_xlfn."),
    ("XLOOKUP", "_xlfn."),
    ("XMATCH", "_xlfn."),
    ("XOR", "_xlfn."),
    ("Z.TEST", "_xlfn."),
];

/// The prefix `name`, a function name in any case, is stored with, or `None` for a function
/// Excel 2007 already had.
pub fn prefix(name: &str) -> Option<&'static str> {
    let upper = name.to_ascii_uppercase();
    FUTURE
        .binary_search_by(|(known, _)| (*known).cmp(upper.as_str()))
        .ok()
        .map(|at| FUTURE[at].1)
}

/// `formula` with each future function's prefix added (the module doc).
pub fn prefixed(formula: &str) -> Cow<'_, str> {
    let bytes = formula.as_bytes();
    let mut out: Option<String> = None;
    let mut copied = 0;
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            quote @ (b'"' | b'\'') => {
                at += 1;
                while at < bytes.len() {
                    if bytes[at] == quote {
                        if bytes.get(at + 1) == Some(&quote) {
                            at += 2;
                            continue;
                        }
                        break;
                    }
                    at += 1;
                }
                at += 1;
            }
            b'[' => {
                let mut depth = 0usize;
                while at < bytes.len() {
                    match bytes[at] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    at += 1;
                }
                at += 1;
            }
            b if word_byte(b) => {
                let start = at;
                while at < bytes.len() && word_byte(bytes[at]) {
                    at += 1;
                }
                if bytes.get(at) == Some(&b'(')
                    && bytes[start].is_ascii_alphabetic()
                    && let Some(prefix) = prefix(&formula[start..at])
                {
                    let text = out.get_or_insert_with(|| String::with_capacity(formula.len() + 16));
                    text.push_str(&formula[copied..start]);
                    text.push_str(prefix);
                    copied = start;
                }
            }
            _ => at += 1,
        }
    }
    match out {
        Some(mut text) => {
            text.push_str(&formula[copied..]);
            Cow::Owned(text)
        }
        None => Cow::Borrowed(formula),
    }
}

/// A byte of a name: a letter, a digit, `_`, `.`, or part of a character outside ASCII.
fn word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b >= 0x80
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sorted_so_a_binary_search_finds_every_name() {
        assert!(FUTURE.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn a_name_that_already_has_its_prefix_or_ends_another_name_is_left_alone() {
        assert_eq!(
            prefixed("_xlfn.XLOOKUP(A1,B:B,C:C)"),
            "_xlfn.XLOOKUP(A1,B:B,C:C)"
        );
        assert_eq!(prefixed("MY.IFS(1)"), "MY.IFS(1)");
        assert_eq!(prefixed("'IFS(x'!A1+Shop[IFS(]"), "'IFS(x'!A1+Shop[IFS(]");
        assert_eq!(
            prefixed("sort(A1:A3)+ifs(1,2)"),
            "_xlfn._xlws.sort(A1:A3)+_xlfn.ifs(1,2)"
        );
    }
}
