//! A reader for the codes `image_barcode` draws, so each drawing is checked against the standard and
//! not against the writer's own tables. The 1D tables are each symbology's standard tables, typed
//! here and not taken from `barcoders`, which draws the codes. DataMatrix is sampled here into its
//! grid of modules, which `datamatrix`'s decoder reads: that is a different path through the crate
//! than the encoder the component calls, with its own Reed-Solomon correction.
//!
//! A 1D code is read along each row of pixels until one row reads, and a row reads only when its
//! guards, its check digit or checksum and every symbol match. A pixel is dark below 128. The reader
//! takes the clean images the component draws, a whole number of pixels per module, and makes no
//! attempt to read a photographed code.

/// The codes this reader reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Code128,
    Ean13,
    Ean8,
    UpcA,
    Code39,
    Itf,
    DataMatrix,
}

/// The text of the `format` code in the grey image `luma`, `width` by `height` pixels, or `None`
/// when no row reads. A GS1-128 code reads as `]C1` and its element strings, an FNC1 between two
/// of them read as the GS character.
pub(crate) fn read(width: u32, height: u32, luma: &[u8], format: Format) -> Option<String> {
    let (width, height) = (width as usize, height as usize);
    assert_eq!(luma.len(), width * height, "one byte a pixel");
    if format == Format::DataMatrix {
        return datamatrix(width, height, luma);
    }
    (0..height).find_map(|y| {
        let runs = runs(&luma[y * width..(y + 1) * width]);
        match format {
            Format::Code128 => code128(&runs),
            Format::Ean13 => upc_ean(&runs, 6),
            Format::Ean8 => upc_ean(&runs, 4),
            Format::UpcA => {
                upc_ean(&runs, 6).and_then(|digits| digits.strip_prefix('0').map(str::to_owned))
            }
            Format::Code39 => code39(&runs),
            Format::Itf => itf(&runs),
            Format::DataMatrix => unreachable!(),
        }
    })
}

fn dark(pixel: u8) -> bool {
    pixel < 128
}

/// The widths of the runs of one row, from its first dark pixel to its last: a bar, a space, a
/// bar and so on, ending on a bar.
fn runs(row: &[u8]) -> Vec<usize> {
    let (Some(first), Some(last)) = (
        row.iter().position(|&p| dark(p)),
        row.iter().rposition(|&p| dark(p)),
    ) else {
        return Vec::new();
    };
    let mut runs = Vec::new();
    let mut colour = true;
    let mut width = 0;
    for &pixel in &row[first..=last] {
        if dark(pixel) == colour {
            width += 1;
        } else {
            runs.push(width);
            colour = !colour;
            width = 1;
        }
    }
    runs.push(width);
    runs
}

/// `runs` as module counts, when they span `total` modules together.
fn modules(runs: &[usize], total: usize) -> Option<Vec<usize>> {
    let sum: usize = runs.iter().sum();
    let counts: Vec<usize> = runs
        .iter()
        .map(|&run| ((2 * run * total + sum) / (2 * sum)).max(1))
        .collect();
    (counts.iter().sum::<usize>() == total).then_some(counts)
}

/// Code 128's symbols 0 to 105, each as the widths of its bar, space, bar, space, bar and space.
const CODE128: [&str; 106] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312", "132212",
    "221213", "221312", "231212", "112232", "122132", "122231", "113222", "123122", "123221",
    "223211", "221132", "221231", "213212", "223112", "312131", "311222", "321122", "321221",
    "312212", "322112", "322211", "212123", "212321", "232121", "111323", "131123", "131321",
    "112313", "132113", "132311", "211313", "231113", "231311", "112133", "112331", "132131",
    "113123", "113321", "133121", "313121", "211331", "231131", "213113", "213311", "213131",
    "311123", "311321", "331121", "312113", "312311", "332111", "314111", "221411", "431111",
    "111224", "111422", "121124", "121421", "141122", "141221", "112214", "112412", "122114",
    "122411", "142112", "142211", "241211", "221114", "413111", "241112", "134111", "111242",
    "121142", "121241", "114212", "124112", "124211", "411212", "421112", "421211", "212141",
    "214121", "412121", "111143", "111341", "131141", "114113", "114311", "411113", "411311",
    "113141", "114131", "311141", "411131", "211412", "211214", "211232",
];

fn pattern(counts: &[usize]) -> String {
    counts.iter().map(|count| count.to_string()).collect()
}

fn code128(runs: &[usize]) -> Option<String> {
    if runs.len() < 6 * 3 + 7 || !(runs.len() - 7).is_multiple_of(6) {
        return None;
    }
    let (symbols, stop) = runs.split_at(runs.len() - 7);
    if pattern(&modules(stop, 13)?) != "2331112" {
        return None;
    }
    let values = symbols
        .chunks(6)
        .map(|chunk| {
            let pattern = pattern(&modules(chunk, 11)?);
            CODE128.iter().position(|&p| p == pattern)
        })
        .collect::<Option<Vec<usize>>>()?;
    let (&start, rest) = values.split_first()?;
    let (&checksum, data) = rest.split_last()?;
    let sum = data
        .iter()
        .enumerate()
        .fold(start, |sum, (at, value)| sum + (at + 1) * value);
    if !(103..=105).contains(&start) || sum % 103 != checksum {
        return None;
    }
    #[derive(Clone, Copy, PartialEq)]
    enum Set {
        A,
        B,
        C,
    }
    let mut set = [Set::A, Set::B, Set::C][start - 103];
    let mut shifted = None;
    let mut text = String::new();
    for (at, &value) in data.iter().enumerate() {
        let current = shifted.take().unwrap_or(set);
        if value == 102 {
            text.push_str(if at == 0 { "]C1" } else { "\u{1d}" });
            continue;
        }
        match (current, value) {
            (Set::C, 0..=99) => text.push_str(&format!("{value:02}")),
            (Set::C, 100) | (Set::A, 100) => set = Set::B,
            (Set::C, 101) | (Set::B, 101) => set = Set::A,
            (Set::A | Set::B, 99) => set = Set::C,
            (Set::A, 98) => shifted = Some(Set::B),
            (Set::B, 98) => shifted = Some(Set::A),
            (Set::A, 0..=63) | (Set::B, 0..=95) => {
                text.push(char::from(u8::try_from(value + 32).ok()?))
            }
            (Set::A, 64..=95) => text.push(char::from(u8::try_from(value - 64).ok()?)),
            _ => return None,
        }
    }
    Some(text)
}

/// The UPC and EAN L patterns of the digits 0 to 9: space, bar, space, bar. A G pattern is an L
/// pattern reversed, and an R pattern has the L widths starting with a bar.
const UPC_EAN_L: [&str; 10] = [
    "3211", "2221", "2122", "1411", "1132", "1231", "1114", "1312", "1213", "3112",
];

/// EAN-13's first digit, from which of the six left digits are G patterns: bit 5 is the first.
const EAN13_FIRST: [usize; 10] = [0x00, 0x0B, 0x0D, 0x0E, 0x13, 0x19, 0x1C, 0x15, 0x16, 0x1A];

/// An EAN-13 or EAN-8 code with `half` digits each side of the middle guard, its check digit checked.
fn upc_ean(runs: &[usize], half: usize) -> Option<String> {
    if runs.len() != 3 + 4 * half + 5 + 4 * half + 3 {
        return None;
    }
    let guard = |at: usize, len: usize| {
        modules(&runs[at..at + len], len).is_some_and(|counts| counts.iter().all(|&c| c == 1))
    };
    let middle = 3 + 4 * half;
    if !guard(0, 3) || !guard(middle, 5) || !guard(runs.len() - 3, 3) {
        return None;
    }
    let digit = |at: usize| -> Option<(usize, bool)> {
        let pattern = pattern(&modules(&runs[at..at + 4], 7)?);
        let reversed: String = pattern.chars().rev().collect();
        let l = UPC_EAN_L.iter().position(|&p| p == pattern);
        let g = UPC_EAN_L.iter().position(|&p| p == reversed);
        l.map(|d| (d, false)).or(g.map(|d| (d, true)))
    };
    let mut digits = Vec::new();
    let mut parity = 0;
    for i in 0..half {
        let (d, even) = digit(3 + 4 * i)?;
        digits.push(d);
        parity = parity << 1 | usize::from(even);
    }
    for i in 0..half {
        let pattern = pattern(&modules(&runs[middle + 5 + 4 * i..middle + 9 + 4 * i], 7)?);
        digits.push(UPC_EAN_L.iter().position(|&p| p == pattern)?);
    }
    if half == 6 {
        let first = EAN13_FIRST.iter().position(|&p| p == parity)?;
        digits.insert(0, first);
    } else if parity != 0 {
        return None;
    }
    check_digit(&digits).then(|| digits.iter().map(|d| d.to_string()).collect())
}

/// Whether the last of `digits` is the GS1 check digit of the ones before it.
fn check_digit(digits: &[usize]) -> bool {
    let (&check, rest) = digits.split_last().expect("a code has digits");
    let sum: usize = rest
        .iter()
        .rev()
        .enumerate()
        .map(|(at, d)| if at % 2 == 0 { 3 * d } else { *d })
        .sum();
    (10 - sum % 10) % 10 == check
}

/// Which of a run of widths are wide: more than one and a half times the narrowest.
fn wide(runs: &[usize]) -> Vec<bool> {
    let narrow = *runs.iter().min().expect("a symbol has runs");
    runs.iter().map(|&run| 2 * run > 3 * narrow).collect()
}

/// Code 39's characters, and the wide elements of each as 9 bits, the first element highest.
const CODE39_CHARS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";
const CODE39: [u32; 43] = [
    0x034, 0x121, 0x061, 0x160, 0x031, 0x130, 0x070, 0x025, 0x124, 0x064, 0x109, 0x049, 0x148,
    0x019, 0x118, 0x058, 0x00D, 0x10C, 0x04C, 0x01C, 0x103, 0x043, 0x142, 0x013, 0x112, 0x052,
    0x007, 0x106, 0x046, 0x016, 0x181, 0x0C1, 0x1C0, 0x091, 0x190, 0x0D0, 0x085, 0x184, 0x0C4,
    0x0A8, 0x0A2, 0x08A, 0x02A,
];
const CODE39_START_STOP: u32 = 0x094;

fn code39(runs: &[usize]) -> Option<String> {
    if runs.len() < 2 * 10 - 1 || !(runs.len() + 1).is_multiple_of(10) {
        return None;
    }
    let chars = runs
        .chunks(10)
        .map(|chunk| {
            let wide = wide(&chunk[..9]);
            (wide.iter().filter(|&&w| w).count() == 3)
                .then(|| wide.iter().fold(0, |bits, &w| bits << 1 | u32::from(w)))
        })
        .collect::<Option<Vec<u32>>>()?;
    let (&first, rest) = chars.split_first()?;
    let (&last, data) = rest.split_last()?;
    if first != CODE39_START_STOP || last != CODE39_START_STOP {
        return None;
    }
    data.iter()
        .map(|bits| {
            let at = CODE39.iter().position(|p| p == bits)?;
            CODE39_CHARS.chars().nth(at)
        })
        .collect()
}

/// The interleaved 2 of 5 digits 0 to 9, each as its five elements, `true` where wide.
const ITF: [[bool; 5]; 10] = {
    const N: bool = false;
    const W: bool = true;
    [
        [N, N, W, W, N],
        [W, N, N, N, W],
        [N, W, N, N, W],
        [W, W, N, N, N],
        [N, N, W, N, W],
        [W, N, W, N, N],
        [N, W, W, N, N],
        [N, N, N, W, W],
        [W, N, N, W, N],
        [N, W, N, W, N],
    ]
};

fn itf(runs: &[usize]) -> Option<String> {
    if runs.len() < 4 + 10 + 3 || !(runs.len() - 7).is_multiple_of(10) {
        return None;
    }
    let wide = wide(runs);
    let stop = &wide[runs.len() - 3..];
    if wide[..4].iter().any(|&w| w) || stop != [true, false, false] {
        return None;
    }
    let mut digits = Vec::new();
    for pair in wide[4..runs.len() - 3].chunks(10) {
        let bars: Vec<bool> = pair.iter().step_by(2).copied().collect();
        let spaces: Vec<bool> = pair.iter().skip(1).step_by(2).copied().collect();
        for elements in [bars, spaces] {
            digits.push(ITF.iter().position(|p| p[..] == elements[..])?);
        }
    }
    check_digit(&digits).then(|| digits.iter().map(|d| d.to_string()).collect())
}

/// The DataMatrix in the image, sampled into its modules at each module's centre. The top edge
/// alternates dark and light from a dark module at the left, and the right edge from a dark
/// module at the bottom, so half their modules are dark runs.
fn datamatrix(width: usize, height: usize, luma: &[u8]) -> Option<String> {
    let at = |x: usize, y: usize| dark(luma[y * width + x]);
    let xs = (0..width).filter(|&x| (0..height).any(|y| at(x, y)));
    let ys = (0..height).filter(|&y| (0..width).any(|x| at(x, y)));
    let (left, right) = (xs.clone().min()?, xs.max()?);
    let (top, bottom) = (ys.clone().min()?, ys.max()?);
    let dark_runs = |cells: Vec<bool>| {
        cells.windows(2).filter(|w| w[1] && !w[0]).count() + usize::from(cells[0])
    };
    let columns = 2 * dark_runs((left..=right).map(|x| at(x, top)).collect());
    let rows = 2 * dark_runs((top..=bottom).map(|y| at(right, y)).collect());
    let pixels: Vec<bool> = (0..rows)
        .flat_map(|row| (0..columns).map(move |column| (row, column)))
        .map(|(row, column)| {
            let x = left + (2 * column + 1) * (right - left + 1) / (2 * columns);
            let y = top + (2 * row + 1) * (bottom - top + 1) / (2 * rows);
            at(x, y)
        })
        .collect();
    let data = ::datamatrix::DataMatrix::decode(&pixels, columns).ok()?;
    String::from_utf8(data).ok()
}
