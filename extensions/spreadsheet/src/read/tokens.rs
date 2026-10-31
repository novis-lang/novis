//! A formula stored as parsed tokens, turned into the A1 text an xlsx cell writes in its `<f>`
//! ([MS-XLS] 2.5.198 and [MS-XLSB] 2.5.97). xls (BIFF8) and xlsb (BIFF12) share their token ids
//! and differ in the width of a row, a string's count and a name's index, which [`Biff`] says.
//!
//! The tokens are postfix: each operand pushes its text, and each operator and function takes
//! its arguments from the top. Every operand and operator token is read, and so are the
//! functions of Excel's built-in table, [`FUNCTIONS`], the `Ftab` list of [MS-XLS] 2.5.198.17.
//! A function past that table, such as `XLOOKUP`, is stored as a call of the name
//! `_xlfn.XLOOKUP`, and its text is that name, as an xlsx file writes it. An array constant is
//! read from the formula's extra data, `{1,2;3,4}`. A space a person typed is not kept.
//!
//! A shared formula (`ptgExp` and `SHRFMLA` or `BrtShrFmla`) is decoded once for each cell it
//! covers, with its relative references moved by that cell's distance from the formula's first
//! cell. A relative reference counts rows and columns modulo the grid, as Excel does.
//!
//! A reference into another workbook is `[1]Shop!A1`, and a name in it `[1]!Rate`: [`Linked`]
//! numbers the other workbooks from 1 in the order the file lists them, as an xlsx file does.
//!
//! [`decode`] returns `None` for a token it does not know, a structured reference (`Table[x]`) in
//! an xlsb, a name it cannot find, and a stack that does not end with one value. The cell then reads as its stored result, as it does without
//! `formulas: true`.
//!
//! **The cost is linear in the tokens.** Each piece of text is written once into one buffer, and
//! the pieces are joined by links, so a deeply nested formula is not copied again at each level.
//! A formula whose text would pass 32 KiB, four bytes for each of the 8,192 characters Excel
//! allows, returns `None`.

use std::fmt::Write as _;

use super::binary::error_text;
use crate::cells::column_letters;
use crate::chart::quoted;

/// The record format the tokens come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Biff {
    /// xls, from Excel 97 to 2003.
    Eight,
    /// xlsb.
    Twelve,
}

impl Biff {
    fn rows(self) -> u32 {
        match self {
            Self::Eight => 65_536,
            Self::Twelve => 1_048_576,
        }
    }

    fn columns(self) -> u32 {
        match self {
            Self::Eight => 256,
            Self::Twelve => 16_384,
        }
    }
}

/// A workbook a sheet reference may name ([MS-XLS] 2.4.271 `SupBook`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Linked {
    /// This workbook.
    Own,
    /// An add-in, with the names of its functions in order.
    AddIn(Vec<String>),
    /// Another workbook: its number among the other workbooks, from 1, as an xlsx formula writes
    /// it in `[1]Shop!A1`, its sheet names and its defined names, each in order.
    Other {
        number: u32,
        sheets: Vec<String>,
        names: Vec<String>,
    },
}

impl Linked {
    /// Another workbook, numbered after the ones `linked` already lists.
    pub fn other(linked: &[Self]) -> Self {
        let before = linked
            .iter()
            .filter(|linked| matches!(linked, Self::Other { .. }))
            .count();
        Self::Other {
            number: u32::try_from(before).unwrap_or(u32::MAX).saturating_add(1),
            sheets: Vec::new(),
            names: Vec::new(),
        }
    }
}

/// What a workbook's formulas refer to by number: its sheets, the workbooks and sheet ranges its
/// 3-D references name, and its defined names.
#[derive(Debug, Clone, Default)]
pub struct Book {
    /// The sheet names, in tab order.
    pub sheets: Vec<String>,
    /// Each `SupBook`, in order.
    pub linked: Vec<Linked>,
    /// Each `XTI`: the index of its `SupBook`, and its first and last sheet.
    pub xti: Vec<(u32, i32, i32)>,
    /// The defined names, in order. A name's index in a token counts from 1.
    pub names: Vec<String>,
}

/// The shape of a formula's references: a cell's own formula, or a shared formula, a defined
/// name or a rule, whose relative references count from the cell `base`.
#[derive(Debug, Clone, Copy)]
pub struct Place {
    /// The cell the text is for: its row and column, each from 0.
    pub base: (u32, u32),
    /// Whether a relative reference in a 3-D token is a distance from `base`.
    pub shared: bool,
}

/// A fixed argument count that marks a function which takes a varying number. A `ptgFunc` of one
/// of these is not valid, and gives `None`.
const V: u8 = u8::MAX;

/// Excel's built-in functions by `iftab`, each with its fixed argument count, or [`V`] where it
/// takes a varying number or where its count is not certain. Index 255 is the call of a name.
#[rustfmt::skip]
pub const FUNCTIONS: [(&str, u8); 485] = [
    ("COUNT", V), ("IF", V), ("ISNA", 1), ("ISERROR", 1), ("SUM", V),
    ("AVERAGE", V), ("MIN", V), ("MAX", V), ("ROW", V), ("COLUMN", V),
    ("NA", 0), ("NPV", V), ("STDEV", V), ("DOLLAR", V), ("FIXED", V),
    ("SIN", 1), ("COS", 1), ("TAN", 1), ("ATAN", 1), ("PI", 0),
    ("SQRT", 1), ("EXP", 1), ("LN", 1), ("LOG10", 1), ("ABS", 1),
    ("INT", 1), ("SIGN", 1), ("ROUND", 2), ("LOOKUP", V), ("INDEX", V),
    ("REPT", 2), ("MID", 3), ("LEN", 1), ("VALUE", 1), ("TRUE", 0),
    ("FALSE", 0), ("AND", V), ("OR", V), ("NOT", 1), ("MOD", 2),
    ("DCOUNT", 3), ("DSUM", 3), ("DAVERAGE", 3), ("DMIN", 3), ("DMAX", 3),
    ("DSTDEV", 3), ("VAR", V), ("DVAR", 3), ("TEXT", 2), ("LINEST", V),
    ("TREND", V), ("LOGEST", V), ("GROWTH", V), ("GOTO", V), ("HALT", V),
    ("RETURN", V), ("PV", V), ("FV", V), ("NPER", V), ("PMT", V),
    ("RATE", V), ("MIRR", 3), ("IRR", V), ("RAND", 0), ("MATCH", V),
    ("DATE", 3), ("TIME", 3), ("DAY", 1), ("MONTH", 1), ("YEAR", 1),
    ("WEEKDAY", V), ("HOUR", 1), ("MINUTE", 1), ("SECOND", 1), ("NOW", 0),
    ("AREAS", 1), ("ROWS", 1), ("COLUMNS", 1), ("OFFSET", V), ("ABSREF", V),
    ("RELREF", V), ("ARGUMENT", V), ("SEARCH", V), ("TRANSPOSE", 1), ("ERROR", V),
    ("STEP", V), ("TYPE", 1), ("ECHO", V), ("SET.NAME", V), ("CALLER", V),
    ("DEREF", V), ("WINDOWS", V), ("SERIES", V), ("DOCUMENTS", V), ("ACTIVE.CELL", V),
    ("SELECTION", V), ("RESULT", V), ("ATAN2", 2), ("ASIN", 1), ("ACOS", 1),
    ("CHOOSE", V), ("HLOOKUP", V), ("VLOOKUP", V), ("LINKS", V), ("INPUT", V),
    ("ISREF", 1), ("GET.FORMULA", V), ("GET.NAME", V), ("SET.VALUE", V), ("LOG", V),
    ("EXEC", V), ("CHAR", 1), ("LOWER", 1), ("UPPER", 1), ("PROPER", 1),
    ("LEFT", V), ("RIGHT", V), ("EXACT", 2), ("TRIM", 1), ("REPLACE", 4),
    ("SUBSTITUTE", V), ("CODE", 1), ("NAMES", V), ("DIRECTORY", V), ("FIND", V),
    ("CELL", V), ("ISERR", 1), ("ISTEXT", 1), ("ISNUMBER", 1), ("ISBLANK", 1),
    ("T", 1), ("N", 1), ("FOPEN", V), ("FCLOSE", V), ("FSIZE", V),
    ("FREADLN", V), ("FREAD", V), ("FWRITELN", V), ("FWRITE", V), ("FPOS", V),
    ("DATEVALUE", 1), ("TIMEVALUE", 1), ("SLN", 3), ("SYD", 4), ("DDB", V),
    ("GET.DEF", V), ("REFTEXT", V), ("TEXTREF", V), ("INDIRECT", V), ("REGISTER", V),
    ("CALL", V), ("ADD.BAR", V), ("ADD.MENU", V), ("ADD.COMMAND", V), ("ENABLE.COMMAND", V),
    ("CHECK.COMMAND", V), ("RENAME.COMMAND", V), ("SHOW.BAR", V), ("DELETE.MENU", V), ("DELETE.COMMAND", V),
    ("GET.CHART.ITEM", V), ("DIALOG.BOX", V), ("CLEAN", 1), ("MDETERM", 1), ("MINVERSE", 1),
    ("MMULT", 2), ("FILES", V), ("IPMT", V), ("PPMT", V), ("COUNTA", V),
    ("CANCEL.KEY", V), ("FOR", V), ("WHILE", V), ("BREAK", V), ("NEXT", V),
    ("INITIATE", V), ("REQUEST", V), ("POKE", V), ("EXECUTE", V), ("TERMINATE", V),
    ("RESTART", V), ("HELP", V), ("GET.BAR", V), ("PRODUCT", V), ("FACT", 1),
    ("GET.CELL", V), ("GET.WORKSPACE", V), ("GET.WINDOW", V), ("GET.DOCUMENT", V), ("DPRODUCT", 3),
    ("ISNONTEXT", 1), ("GET.NOTE", V), ("NOTE", V), ("STDEVP", V), ("VARP", V),
    ("DSTDEVP", 3), ("DVARP", 3), ("TRUNC", V), ("ISLOGICAL", 1), ("DCOUNTA", 3),
    ("DELETE.BAR", V), ("UNREGISTER", V), ("", V), ("", V), ("USDOLLAR", V),
    ("FINDB", V), ("SEARCHB", V), ("REPLACEB", 4), ("LEFTB", V), ("RIGHTB", V),
    ("MIDB", 3), ("LENB", 1), ("ROUNDUP", 2), ("ROUNDDOWN", 2), ("ASC", 1),
    ("DBCS", 1), ("RANK", V), ("", V), ("", V), ("ADDRESS", V),
    ("DAYS360", V), ("TODAY", 0), ("VDB", V), ("ELSE", V), ("ELSE.IF", V),
    ("END.IF", V), ("FOR.CELL", V), ("MEDIAN", V), ("SUMPRODUCT", V), ("SINH", 1),
    ("COSH", 1), ("TANH", 1), ("ASINH", 1), ("ACOSH", 1), ("ATANH", 1),
    ("DGET", 3), ("CREATE.OBJECT", V), ("VOLATILE", V), ("LAST.ERROR", V), ("CUSTOM.UNDO", V),
    ("CUSTOM.REPEAT", V), ("FORMULA.CONVERT", V), ("GET.LINK.INFO", V), ("TEXT.BOX", V), ("INFO", 1),
    ("GROUP", V), ("GET.OBJECT", V), ("DB", V), ("PAUSE", V), ("", V),
    ("", V), ("RESUME", V), ("FREQUENCY", 2), ("ADD.TOOLBAR", V), ("DELETE.TOOLBAR", V),
    ("", V), ("RESET.TOOLBAR", V), ("EVALUATE", V), ("GET.TOOLBAR", V), ("GET.TOOL", V),
    ("SPELLING.CHECK", V), ("ERROR.TYPE", 1), ("APP.TITLE", V), ("WINDOW.TITLE", V), ("SAVE.TOOLBAR", V),
    ("ENABLE.TOOL", V), ("PRESS.TOOL", V), ("REGISTER.ID", V), ("GET.WORKBOOK", V), ("AVEDEV", V),
    ("BETADIST", V), ("GAMMALN", 1), ("BETAINV", V), ("BINOMDIST", 4), ("CHIDIST", 2),
    ("CHIINV", 2), ("COMBIN", 2), ("CONFIDENCE", 3), ("CRITBINOM", 3), ("EVEN", 1),
    ("EXPONDIST", 3), ("FDIST", 3), ("FINV", 3), ("FISHER", 1), ("FISHERINV", 1),
    ("FLOOR", 2), ("GAMMADIST", 4), ("GAMMAINV", 3), ("CEILING", 2), ("HYPGEOMDIST", 4),
    ("LOGNORMDIST", 3), ("LOGINV", 3), ("NEGBINOMDIST", 3), ("NORMDIST", 4), ("NORMSDIST", 1),
    ("NORMINV", 3), ("NORMSINV", 1), ("STANDARDIZE", 3), ("ODD", 1), ("PERMUT", 2),
    ("POISSON", 3), ("TDIST", 3), ("WEIBULL", 4), ("SUMXMY2", 2), ("SUMX2MY2", 2),
    ("SUMX2PY2", 2), ("CHITEST", 2), ("CORREL", 2), ("COVAR", 2), ("FORECAST", 3),
    ("FTEST", 2), ("INTERCEPT", 2), ("PEARSON", 2), ("RSQ", 2), ("STEYX", 2),
    ("SLOPE", 2), ("TTEST", 4), ("PROB", V), ("DEVSQ", V), ("GEOMEAN", V),
    ("HARMEAN", V), ("SUMSQ", V), ("KURT", V), ("SKEW", V), ("ZTEST", V),
    ("LARGE", 2), ("SMALL", 2), ("QUARTILE", 2), ("PERCENTILE", 2), ("PERCENTRANK", V),
    ("MODE", V), ("TRIMMEAN", 2), ("TINV", 2), ("", V), ("MOVIE.COMMAND", V),
    ("GET.MOVIE", V), ("CONCATENATE", V), ("POWER", 2), ("PIVOT.ADD.DATA", V), ("GET.PIVOT.TABLE", V),
    ("GET.PIVOT.FIELD", V), ("GET.PIVOT.ITEM", V), ("RADIANS", 1), ("DEGREES", 1), ("SUBTOTAL", V),
    ("SUMIF", V), ("COUNTIF", 2), ("COUNTBLANK", 1), ("SCENARIO.GET", V), ("OPTIONS.LISTS.GET", V),
    ("ISPMT", 4), ("DATEDIF", 3), ("DATESTRING", 1), ("NUMBERSTRING", 2), ("ROMAN", V),
    ("OPEN.DIALOG", V), ("SAVE.DIALOG", V), ("VIEW.GET", V), ("GETPIVOTDATA", V), ("HYPERLINK", V),
    ("PHONETIC", 1), ("AVERAGEA", V), ("MAXA", V), ("MINA", V), ("STDEVPA", V),
    ("VARPA", V), ("STDEVA", V), ("VARA", V), ("BAHTTEXT", 1), ("THAIDAYOFWEEK", 1),
    ("THAIDIGIT", 1), ("THAIMONTHOFYEAR", 1), ("THAINUMSOUND", 1), ("THAINUMSTRING", 1), ("THAISTRINGLENGTH", 1),
    ("ISTHAIDIGIT", 1), ("ROUNDBAHTDOWN", 1), ("ROUNDBAHTUP", 1), ("THAIYEAR", 1), ("RTD", V),
    ("CUBEVALUE", V), ("CUBEMEMBER", V), ("CUBEMEMBERPROPERTY", V), ("CUBERANKEDMEMBER", V), ("HEX2BIN", V),
    ("HEX2DEC", 1), ("HEX2OCT", V), ("DEC2BIN", V), ("DEC2HEX", V), ("DEC2OCT", V),
    ("OCT2BIN", V), ("OCT2HEX", V), ("OCT2DEC", 1), ("BIN2DEC", 1), ("BIN2OCT", V),
    ("BIN2HEX", V), ("IMSUB", 2), ("IMDIV", 2), ("IMPOWER", 2), ("IMABS", 1),
    ("IMSQRT", 1), ("IMLN", 1), ("IMLOG2", 1), ("IMLOG10", 1), ("IMSIN", 1),
    ("IMCOS", 1), ("IMEXP", 1), ("IMARGUMENT", 1), ("IMCONJUGATE", 1), ("IMAGINARY", 1),
    ("IMREAL", 1), ("COMPLEX", V), ("IMSUM", V), ("IMPRODUCT", V), ("SERIESSUM", V),
    ("FACTDOUBLE", 1), ("SQRTPI", 1), ("QUOTIENT", 2), ("DELTA", V), ("GESTEP", V),
    ("ISEVEN", 1), ("ISODD", 1), ("MROUND", 2), ("ERF", V), ("ERFC", 1),
    ("BESSELJ", 2), ("BESSELK", 2), ("BESSELY", 2), ("BESSELI", 2), ("XIRR", V),
    ("XNPV", V), ("PRICEMAT", V), ("YIELDMAT", V), ("INTRATE", V), ("RECEIVED", V),
    ("DISC", V), ("PRICEDISC", V), ("YIELDDISC", V), ("TBILLEQ", V), ("TBILLPRICE", V),
    ("TBILLYIELD", V), ("PRICE", V), ("YIELD", V), ("DOLLARDE", 2), ("DOLLARFR", 2),
    ("NOMINAL", 2), ("EFFECT", 2), ("CUMPRINC", 6), ("CUMIPMT", 6), ("EDATE", 2),
    ("EOMONTH", 2), ("YEARFRAC", V), ("COUPDAYBS", V), ("COUPDAYS", V), ("COUPDAYSNC", V),
    ("COUPNCD", V), ("COUPNUM", V), ("COUPPCD", V), ("DURATION", V), ("MDURATION", V),
    ("ODDLPRICE", V), ("ODDLYIELD", V), ("ODDFPRICE", V), ("ODDFYIELD", V), ("RANDBETWEEN", 2),
    ("WEEKNUM", V), ("AMORDEGRC", V), ("AMORLINC", V), ("CONVERT", 3), ("ACCRINT", V),
    ("ACCRINTM", V), ("WORKDAY", V), ("NETWORKDAYS", V), ("GCD", V), ("MULTINOMIAL", V),
    ("LCM", V), ("FVSCHEDULE", V), ("CUBEKPIMEMBER", V), ("CUBESET", V), ("CUBESETCOUNT", V),
    ("IFERROR", 2), ("COUNTIFS", V), ("SUMIFS", V), ("AVERAGEIF", V), ("AVERAGEIFS", V),
];

/// The `iftab` of a call whose first argument is the function's name.
const CALL_BY_NAME: u16 = 255;

/// The most bytes of text one formula may give.
const MOST_TEXT: usize = 32 * 1024;

/// The A1 text of the parsed formula `tokens`, with its extra data `extra`, read for `place` in
/// `book`. `None` when the tokens are empty or have a part this decoder does not read.
pub fn decode(
    biff: Biff,
    tokens: &[u8],
    extra: &[u8],
    place: Place,
    book: &Book,
) -> Option<String> {
    if tokens.is_empty() {
        return None;
    }
    let mut decoder = Decoder {
        biff,
        place,
        book,
        tokens: Bytes {
            bytes: tokens,
            at: 0,
        },
        extra: Bytes {
            bytes: extra,
            at: 0,
        },
        out: Out::default(),
    };
    decoder.run()?;
    decoder.out.finish()
}

/// The `row` of a `ptgExp`: the first row of the shared formula a cell uses, and its first
/// column in an xls. `None` when `tokens` is not one `ptgExp`.
pub fn shared_at(biff: Biff, tokens: &[u8]) -> Option<(u32, Option<u32>)> {
    let mut bytes = Bytes {
        bytes: tokens,
        at: 0,
    };
    if bytes.u8()? != 0x01 {
        return None;
    }
    let at = match biff {
        Biff::Eight => (u32::from(bytes.u16()?), Some(u32::from(bytes.u16()?))),
        Biff::Twelve => (bytes.u32()?, None),
    };
    (bytes.at == tokens.len()).then_some(at)
}

/// A run of bytes read from the front.
struct Bytes<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let out = self.bytes.get(self.at..self.at.checked_add(n)?)?;
        self.at += n;
        Some(out)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn f64(&mut self) -> Option<f64> {
        self.take(8)
            .map(|b| f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }

    fn done(&self) -> bool {
        self.at == self.bytes.len()
    }

    /// Text of `count` UTF-16 units.
    fn wide(&mut self, count: usize) -> Option<String> {
        let units = self.take(count.checked_mul(2)?)?;
        Some(
            char::decode_utf16(
                units
                    .chunks_exact(2)
                    .map(|u| u16::from_le_bytes([u[0], u[1]])),
            )
            .map(|c| c.unwrap_or('\u{FFFD}'))
            .collect(),
        )
    }

    /// BIFF8 characters after their flags byte: one byte each, or two when the low bit is set.
    fn characters(&mut self, count: usize) -> Option<String> {
        if self.u8()? & 1 == 0 {
            Some(self.take(count)?.iter().map(|&b| char::from(b)).collect())
        } else {
            self.wide(count)
        }
    }
}

/// One piece of the text: its bytes in [`Out::text`] and the piece after it.
#[derive(Clone, Copy)]
struct Segment {
    start: u32,
    end: u32,
    next: u32,
}

/// One value on the stack: the first and last of its linked segments.
#[derive(Clone, Copy)]
struct Piece {
    head: u32,
    tail: u32,
}

const END: u32 = u32::MAX;

/// The text being built. Each segment is written once into `text`, and a value is its segments
/// in link order.
#[derive(Default)]
struct Out {
    text: String,
    segments: Vec<Segment>,
    stack: Vec<Piece>,
}

impl Out {
    /// A new segment of the text written since `start`.
    fn segment_from(&mut self, start: usize) -> Option<u32> {
        if self.text.len() > MOST_TEXT {
            return None;
        }
        let id = u32::try_from(self.segments.len()).ok()?;
        self.segments.push(Segment {
            start: u32::try_from(start).ok()?,
            end: u32::try_from(self.text.len()).ok()?,
            next: END,
        });
        Some(id)
    }

    fn segment(&mut self, text: &str) -> Option<u32> {
        let start = self.text.len();
        self.text.push_str(text);
        self.segment_from(start)
    }

    fn push(&mut self, text: &str) -> Option<()> {
        let id = self.segment(text)?;
        self.stack.push(Piece { head: id, tail: id });
        Some(())
    }

    /// Pushes the text written since `start` as one value.
    fn push_from(&mut self, start: usize) -> Option<()> {
        let id = self.segment_from(start)?;
        self.stack.push(Piece { head: id, tail: id });
        Some(())
    }

    fn pop(&mut self) -> Option<Piece> {
        self.stack.pop()
    }

    fn link(&mut self, from: u32, to: u32) {
        self.segments[from as usize].next = to;
    }

    /// `piece` with `before` ahead of it and `after` behind it.
    fn around(&mut self, before: &str, piece: Piece, after: &str) -> Option<Piece> {
        let mut out = piece;
        if !before.is_empty() {
            let head = self.segment(before)?;
            self.link(head, piece.head);
            out.head = head;
        }
        if !after.is_empty() {
            let tail = self.segment(after)?;
            self.link(piece.tail, tail);
            out.tail = tail;
        }
        Some(out)
    }

    /// `left`, then `between`, then `right`.
    fn join(&mut self, left: Piece, between: &str, right: Piece) -> Option<Piece> {
        let left = self.around("", left, between)?;
        self.link(left.tail, right.head);
        Some(Piece {
            head: left.head,
            tail: right.tail,
        })
    }

    /// The text of the one value left, or `None` when the stack does not end with one.
    fn finish(self) -> Option<String> {
        let [piece] = self.stack[..] else {
            return None;
        };
        let mut text = String::with_capacity(self.text.len());
        let mut at = piece.head;
        while at != END {
            let segment = self.segments[at as usize];
            text.push_str(&self.text[segment.start as usize..segment.end as usize]);
            at = segment.next;
        }
        Some(text)
    }
}

/// A cell in a token: its row and column from 0, and whether each is relative.
#[derive(Clone, Copy)]
struct Loc {
    row: u32,
    column: u32,
    row_relative: bool,
    column_relative: bool,
}

struct Decoder<'a> {
    biff: Biff,
    place: Place,
    book: &'a Book,
    tokens: Bytes<'a>,
    extra: Bytes<'a>,
    out: Out,
}

impl Decoder<'_> {
    fn run(&mut self) -> Option<()> {
        while !self.tokens.done() {
            let token = self.tokens.u8()?;
            // A token from 0x20 on comes in three classes, 0x20, 0x40 and 0x60, which read alike.
            let base = if token < 0x20 {
                token
            } else {
                (token & 0x1F) | 0x20
            };
            match base {
                0x03..=0x11 => self.binary(base)?,
                0x12 => self.unary("+", "")?,
                0x13 => self.unary("-", "")?,
                0x14 => self.unary("", "%")?,
                0x15 => self.unary("(", ")")?,
                0x16 => self.out.push("")?,
                0x17 => self.string()?,
                0x19 => self.attribute()?,
                0x1C => {
                    let text = error_text(self.tokens.u8()?)?;
                    self.out.push(text)?;
                }
                0x1D => {
                    let text = if self.tokens.u8()? == 0 {
                        "FALSE"
                    } else {
                        "TRUE"
                    };
                    self.out.push(text)?;
                }
                0x1E => {
                    let n = self.tokens.u16()?;
                    let start = self.out.text.len();
                    let _ = write!(self.out.text, "{n}");
                    self.out.push_from(start)?;
                }
                0x1F => {
                    let n = self.tokens.f64()?;
                    let start = self.out.text.len();
                    number(&mut self.out.text, n);
                    self.out.push_from(start)?;
                }
                0x20 => self.array()?,
                0x21 => {
                    let index = self.tokens.u16()?;
                    let (_, count) = FUNCTIONS.get(usize::from(index))?;
                    if *count == V {
                        return None;
                    }
                    self.call(index, usize::from(*count))?;
                }
                0x22 => {
                    let count = self.tokens.u8()? & 0x7F;
                    let index = self.tokens.u16()? & 0x7FFF;
                    self.call(index, usize::from(count))?;
                }
                0x23 => {
                    let index = match self.biff {
                        Biff::Eight => {
                            let index = u32::from(self.tokens.u16()?);
                            self.tokens.take(2)?;
                            index
                        }
                        Biff::Twelve => self.tokens.u32()?,
                    };
                    let name = self.book.names.get(index.checked_sub(1)? as usize)?;
                    self.out.push(name)?;
                }
                0x24 | 0x2C => {
                    let loc = self.loc()?;
                    let relative = base == 0x2C;
                    let start = self.out.text.len();
                    self.cell(loc, relative);
                    self.out.push_from(start)?;
                }
                0x25 | 0x2D => {
                    let (first, last) = self.area()?;
                    let relative = base == 0x2D;
                    let start = self.out.text.len();
                    self.range(first, last, relative);
                    self.out.push_from(start)?;
                }
                // `ptgMemArea`, `ptgMemErr` and `ptgMemNoMem` say how much of what follows is one
                // reference. What follows is read as it is.
                0x26..=0x28 => {
                    self.tokens.take(6)?;
                    if base == 0x26 {
                        self.skip_mem_extra()?;
                    }
                }
                0x29 | 0x2E | 0x2F => {
                    self.tokens.take(2)?;
                }
                0x2A => {
                    self.tokens.take(self.loc_size())?;
                    self.out.push("#REF!")?;
                }
                0x2B => {
                    self.tokens.take(2 * self.loc_size())?;
                    self.out.push("#REF!")?;
                }
                0x39 => self.external_name()?,
                0x3A => {
                    let sheet = self.sheet()?;
                    let loc = self.loc()?;
                    let start = self.out.text.len();
                    self.out.text.push_str(&sheet);
                    self.cell(loc, self.place.shared);
                    self.out.push_from(start)?;
                }
                0x3B => {
                    let sheet = self.sheet()?;
                    let (first, last) = self.area()?;
                    let start = self.out.text.len();
                    self.out.text.push_str(&sheet);
                    self.range(first, last, self.place.shared);
                    self.out.push_from(start)?;
                }
                0x3C | 0x3D => {
                    let sheet = self.sheet()?;
                    let size = if base == 0x3C { 1 } else { 2 };
                    self.tokens.take(size * self.loc_size())?;
                    let start = self.out.text.len();
                    self.out.text.push_str(&sheet);
                    self.out.text.push_str("#REF!");
                    self.out.push_from(start)?;
                }
                // `ptgExp` and `ptgTbl` name another record, a structured reference or a chart
                // token: none of them has text here.
                _ => return None,
            }
        }
        Some(())
    }

    fn binary(&mut self, token: u8) -> Option<()> {
        let op = match token {
            0x03 => "+",
            0x04 => "-",
            0x05 => "*",
            0x06 => "/",
            0x07 => "^",
            0x08 => "&",
            0x09 => "<",
            0x0A => "<=",
            0x0B => "=",
            0x0C => ">=",
            0x0D => ">",
            0x0E => "<>",
            0x0F => " ",
            0x10 => ",",
            _ => ":",
        };
        let right = self.out.pop()?;
        let left = self.out.pop()?;
        let joined = self.out.join(left, op, right)?;
        self.out.stack.push(joined);
        Some(())
    }

    fn unary(&mut self, before: &str, after: &str) -> Option<()> {
        let piece = self.out.pop()?;
        let piece = self.out.around(before, piece, after)?;
        self.out.stack.push(piece);
        Some(())
    }

    fn string(&mut self) -> Option<()> {
        let text = match self.biff {
            Biff::Eight => {
                let count = usize::from(self.tokens.u8()?);
                self.tokens.characters(count)?
            }
            Biff::Twelve => {
                let count = usize::from(self.tokens.u16()?);
                self.tokens.wide(count)?
            }
        };
        let start = self.out.text.len();
        quote_text(&mut self.out.text, &text);
        self.out.push_from(start)
    }

    /// `ptgAttr`: a hint for Excel's own evaluator, except `SUM` of one argument.
    fn attribute(&mut self) -> Option<()> {
        let kind = self.tokens.u8()?;
        let data = self.tokens.u16()?;
        match kind {
            // `tAttrChoose`: one offset for each choice, and one past them.
            0x04 => {
                self.tokens.take((usize::from(data) + 1) * 2)?;
            }
            0x10 => self.unary("SUM(", ")")?,
            _ => {}
        }
        Some(())
    }

    /// A `ptgMemArea`'s rectangles in the extra data, which no text shows.
    fn skip_mem_extra(&mut self) -> Option<()> {
        match self.biff {
            Biff::Eight => {
                let count = usize::from(self.extra.u16()?);
                self.extra.take(count.checked_mul(8)?)?;
            }
            Biff::Twelve => {
                let count = self.extra.u32()? as usize;
                self.extra.take(count.checked_mul(16)?)?;
            }
        }
        Some(())
    }

    /// An array constant: its token's unused bytes, and its values in the extra data.
    fn array(&mut self) -> Option<()> {
        let (rows, columns) = match self.biff {
            Biff::Eight => {
                self.tokens.take(7)?;
                let columns = usize::from(self.extra.u8()?) + 1;
                let rows = usize::from(self.extra.u16()?) + 1;
                (rows, columns)
            }
            Biff::Twelve => {
                self.tokens.take(14)?;
                let rows = self.extra.u32()? as usize;
                let columns = self.extra.u32()? as usize;
                (rows, columns)
            }
        };
        let start = self.out.text.len();
        self.out.text.push('{');
        for row in 0..rows {
            if row > 0 {
                self.out.text.push(';');
            }
            for column in 0..columns {
                if column > 0 {
                    self.out.text.push(',');
                }
                self.array_value()?;
                if self.out.text.len() > MOST_TEXT {
                    return None;
                }
            }
        }
        self.out.text.push('}');
        self.out.push_from(start)
    }

    /// One value of an array constant, written into the text.
    fn array_value(&mut self) -> Option<()> {
        let kind = self.extra.u8()?;
        match (self.biff, kind) {
            (Biff::Eight, 0x00) => {
                self.extra.take(8)?;
            }
            (Biff::Eight, 0x01) | (Biff::Twelve, 0x00) => {
                let n = self.extra.f64()?;
                number(&mut self.out.text, n);
            }
            (Biff::Eight, 0x02) => {
                let count = usize::from(self.extra.u16()?);
                let text = self.extra.characters(count)?;
                quote_text(&mut self.out.text, &text);
            }
            (Biff::Twelve, 0x01) => {
                let count = self.extra.u32()? as usize;
                let text = self.extra.wide(count)?;
                quote_text(&mut self.out.text, &text);
            }
            (Biff::Eight, 0x04) | (Biff::Twelve, 0x02) => {
                let value = self.extra.u8()?;
                if self.biff == Biff::Eight {
                    self.extra.take(7)?;
                }
                self.out
                    .text
                    .push_str(if value == 0 { "FALSE" } else { "TRUE" });
            }
            (Biff::Eight, 0x10) | (Biff::Twelve, 0x04) => {
                let code = self.extra.u8()?;
                if self.biff == Biff::Eight {
                    self.extra.take(7)?;
                }
                self.out.text.push_str(error_text(code)?);
            }
            _ => return None,
        }
        Some(())
    }

    /// The function `index` of `count` arguments from the stack.
    fn call(&mut self, index: u16, count: usize) -> Option<()> {
        let at = self.out.stack.len().checked_sub(count)?;
        let args: Vec<Piece> = self.out.stack.split_off(at);
        let (head, rest) = if index == CALL_BY_NAME {
            let (&name, rest) = args.split_first()?;
            (name, rest)
        } else {
            let name = FUNCTIONS.get(usize::from(index))?.0;
            if name.is_empty() {
                return None;
            }
            let id = self.out.segment(name)?;
            (Piece { head: id, tail: id }, &args[..])
        };
        let Some((&first, others)) = rest.split_first() else {
            let piece = self.out.around("", head, "()")?;
            self.out.stack.push(piece);
            return Some(());
        };
        let mut joined = first;
        for &arg in others {
            joined = self.out.join(joined, ",", arg)?;
        }
        let joined = self.out.around("(", joined, ")")?;
        self.out.link(head.tail, joined.head);
        self.out.stack.push(Piece {
            head: head.head,
            tail: joined.tail,
        });
        Some(())
    }

    /// `ptgNameX`: a name in another workbook, `[1]!Rate`, or an add-in. A function past Excel's
    /// built-in table is an add-in name such as `_xlfn.XLOOKUP`.
    fn external_name(&mut self) -> Option<()> {
        let xti = usize::from(self.tokens.u16()?);
        let index = match self.biff {
            Biff::Eight => {
                let index = u32::from(self.tokens.u16()?);
                self.tokens.take(2)?;
                index
            }
            Biff::Twelve => self.tokens.u32()?,
        };
        let &(linked, ..) = self.book.xti.get(xti)?;
        let name = match self.book.linked.get(linked as usize)? {
            Linked::AddIn(names) => names.get(index.checked_sub(1)? as usize)?,
            Linked::Own => self.book.names.get(index.checked_sub(1)? as usize)?,
            Linked::Other { number, names, .. } => {
                let name = names.get(index.checked_sub(1)? as usize)?;
                let text = format!("[{number}]!{name}");
                return self.out.push(&text);
            }
        };
        let name = name.clone();
        self.out.push(&name)
    }

    fn loc_size(&self) -> usize {
        match self.biff {
            Biff::Eight => 4,
            Biff::Twelve => 6,
        }
    }

    fn column_of(bits: u16) -> (u32, bool, bool) {
        (
            u32::from(bits & 0x3FFF),
            bits & 0x4000 != 0,
            bits & 0x8000 != 0,
        )
    }

    fn loc(&mut self) -> Option<Loc> {
        let row = match self.biff {
            Biff::Eight => u32::from(self.tokens.u16()?),
            Biff::Twelve => self.tokens.u32()?,
        };
        let (column, column_relative, row_relative) = Self::column_of(self.tokens.u16()?);
        Some(Loc {
            row,
            column,
            row_relative,
            column_relative,
        })
    }

    /// An area's first and last cell. Its rows come first, then its columns.
    fn area(&mut self) -> Option<(Loc, Loc)> {
        let (first_row, last_row) = match self.biff {
            Biff::Eight => (u32::from(self.tokens.u16()?), u32::from(self.tokens.u16()?)),
            Biff::Twelve => (self.tokens.u32()?, self.tokens.u32()?),
        };
        let first = Self::column_of(self.tokens.u16()?);
        let last = Self::column_of(self.tokens.u16()?);
        Some((
            Loc {
                row: first_row,
                column: first.0,
                column_relative: first.1,
                row_relative: first.2,
            },
            Loc {
                row: last_row,
                column: last.0,
                column_relative: last.1,
                row_relative: last.2,
            },
        ))
    }

    /// `loc` placed in the grid: a relative part of a `relative` token is a distance from the
    /// base cell, counted modulo the grid.
    fn placed(&self, loc: Loc, relative: bool) -> (u32, u32) {
        let (rows, columns) = (self.biff.rows(), self.biff.columns());
        let (base_row, base_column) = self.place.base;
        let row = if relative && loc.row_relative {
            (base_row % rows + loc.row % rows) % rows
        } else {
            loc.row % rows
        };
        let column = if relative && loc.column_relative {
            // An xls keeps a column distance in the low eight bits.
            let distance = match self.biff {
                Biff::Eight => loc.column & 0xFF,
                Biff::Twelve => loc.column,
            };
            (base_column % columns + distance % columns) % columns
        } else {
            loc.column % columns
        };
        (row, column)
    }

    fn cell(&mut self, loc: Loc, relative: bool) {
        let (row, column) = self.placed(loc, relative);
        self.cell_text(loc, row, column);
    }

    /// An area as A1 text, a whole column as `A:B` and a whole row as `1:2`.
    fn range(&mut self, first: Loc, last: Loc, relative: bool) {
        let (top, left) = self.placed(first, relative);
        let (bottom, right) = self.placed(last, relative);
        let text = &mut self.out.text;
        let dollar = |text: &mut String, absolute: bool| {
            if absolute {
                text.push('$');
            }
        };
        if top == 0 && bottom == self.biff.rows() - 1 {
            dollar(text, !first.column_relative);
            column_letters(text, left);
            text.push(':');
            dollar(text, !last.column_relative);
            column_letters(text, right);
            return;
        }
        if left == 0 && right == self.biff.columns() - 1 {
            dollar(text, !first.row_relative);
            let _ = write!(text, "{}:", top + 1);
            dollar(text, !last.row_relative);
            let _ = write!(text, "{}", bottom + 1);
            return;
        }
        self.cell_text(first, top, left);
        self.out.text.push(':');
        self.cell_text(last, bottom, right);
    }

    fn cell_text(&mut self, loc: Loc, row: u32, column: u32) {
        let text = &mut self.out.text;
        if !loc.column_relative {
            text.push('$');
        }
        column_letters(text, column);
        if !loc.row_relative {
            text.push('$');
        }
        let _ = write!(text, "{}", row + 1);
    }

    /// The sheet prefix of a 3-D token: `Shop!` or `'Shop 1:Shop 3'!`, and `[1]Shop!` or
    /// `'[1]My Shop'!` for a sheet of another workbook. `None` for an add-in.
    fn sheet(&mut self) -> Option<String> {
        let xti = usize::from(self.tokens.u16()?);
        let &(linked, first, last) = self.book.xti.get(xti)?;
        let (book, sheets) = match self.book.linked.get(linked as usize) {
            // A workbook that lists no `SupBook` names only its own sheets.
            None if self.book.linked.is_empty() => (String::new(), &self.book.sheets),
            Some(Linked::Own) => (String::new(), &self.book.sheets),
            Some(Linked::Other { number, sheets, .. }) => (format!("[{number}]"), sheets),
            _ => return None,
        };
        let name = |index: i32| {
            usize::try_from(index)
                .ok()
                .and_then(|index| sheets.get(index))
        };
        let (Some(first), Some(last)) = (name(first), name(last)) else {
            return Some("#REF!".to_owned());
        };
        let both = if first == last {
            format!("{book}{first}")
        } else {
            format!("{book}{first}:{last}")
        };
        let mut out = if quoted(first) == *first && quoted(last) == *last {
            both
        } else {
            format!("'{}'", both.replace('\'', "''"))
        };
        out.push('!');
        Some(out)
    }
}

/// `text` as a formula's string literal, with each `"` doubled.
fn quote_text(out: &mut String, text: &str) {
    out.push('"');
    out.push_str(&text.replace('"', "\"\""));
    out.push('"');
}

/// A number as formula text: a whole number below 10^15 in digits, and any other number in the
/// shortest form that reads back the same, its exponent written `E+20` as Excel writes it.
fn number(out: &mut String, n: f64) {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the number is whole and below 10^15"
        )]
        let _ = write!(out, "{}", n as i64);
        return;
    }
    let mut buffer = ryu::Buffer::new();
    let text = buffer.format(n);
    match text.split_once('e') {
        Some((mantissa, exponent)) => {
            let (sign, digits) = match exponent.strip_prefix('-') {
                Some(digits) => ('-', digits),
                None => ('+', exponent),
            };
            let mantissa = mantissa.strip_suffix(".0").unwrap_or(mantissa);
            let _ = write!(out, "{mantissa}E{sign}{digits:0>2}");
        }
        None => out.push_str(text.strip_suffix(".0").unwrap_or(text)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> Book {
        Book {
            sheets: vec!["Shop".to_owned(), "Sales 2024".to_owned()],
            linked: vec![Linked::Own, Linked::AddIn(vec!["_xlfn.XLOOKUP".to_owned()])],
            xti: vec![(0, 0, 0), (0, 1, 1), (0, 0, 1), (1, 0, 0)],
            names: vec!["Rate".to_owned()],
        }
    }

    fn own() -> Place {
        Place {
            base: (0, 0),
            shared: false,
        }
    }

    #[test]
    fn nested_text_is_linked_once() {
        // `(-(-(...-1)))`, nested deeply, decodes in linear time and stays whole.
        let mut tokens = vec![0x1E, 1, 0];
        for _ in 0..5_000 {
            tokens.extend([0x13, 0x15]);
        }
        let text = decode(Biff::Eight, &tokens, &[], own(), &book()).unwrap();
        assert_eq!(text.len(), 1 + 3 * 5_000);
        assert!(text.starts_with("(-(-("));
        assert!(text.ends_with(&format!("(-1{}", ")".repeat(5_000))));
    }

    #[test]
    fn a_number_reads_as_excel_writes_it() {
        let mut out = String::new();
        number(&mut out, 1.5);
        out.push(' ');
        number(&mut out, 1e20);
        out.push(' ');
        number(&mut out, 2.5e-7);
        out.push(' ');
        number(&mut out, 70_000.0);
        assert_eq!(out, "1.5 1E+20 2.5E-07 70000");
    }
}
