//! One `Core\Str::format` placeholder over an integer: its digits, its sign,
//! its truncation and its padding.
//!
//! This is the one home for that rendering. `nvs_stdlib::format` renders every
//! integer placeholder through [`IntPlaceholder::write`], and a literal
//! template's integer placeholder reaches [`crate::nvs_str_concat_parts`] as a
//! [`crate::StrPart::FORMAT`] part that runs the same code. So a placeholder
//! prints the same text whichever path renders it.
//!
//! The runtime cannot depend on `nvs_stdlib`, where the template grammar and
//! its parser live. So the parser builds an [`IntPlaceholder`], and a literal
//! template's placeholder is parsed while compiling and packed into the
//! part's kind word ([`IntPlaceholder::kind_word`]). The join unpacks it and
//! parses nothing.
//!
//! # What does not pack
//!
//! A width or a precision above [`PACKED_MAX`] does not fit the word, and the
//! template parser already stops at four digits. `%s` over a `string` with a
//! flag, a width or a precision stays a converted piece, rendered by
//! `nvs_core_str_format_piece` and joined as a string part, because its width
//! counts graphemes and `nvs_stdlib::granularity` owns that count. A float
//! placeholder stays there too. So a template prints the same text and throws
//! the same error whichever of the two paths renders each placeholder.
//!
//! # Memory
//!
//! Nothing on the heap. Measuring counts the digits and writes none. Writing
//! puts them into a buffer on the stack, by one loop for decimal and one for
//! the power-of-two bases, and never through Rust's formatter.

use crate::StrWriter;
use crate::helpers::{DECIMAL_MAX, decimal_len, write_decimal};

/// How an integer placeholder writes its value's digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntBase {
    /// `%d`, and `%s` over an `int`: signed decimal.
    Signed,
    /// `%u`, and `%s` over a `uint`: the value's bits as unsigned decimal.
    Unsigned,
    /// `%x`: the bits in lowercase hexadecimal.
    LowerHex,
    /// `%X`: the bits in uppercase hexadecimal.
    UpperHex,
    /// `%o`: the bits in octal.
    Octal,
    /// `%b`: the bits in binary.
    Binary,
}

impl IntBase {
    const ALL: [Self; 6] = [
        Self::Signed,
        Self::Unsigned,
        Self::LowerHex,
        Self::UpperHex,
        Self::Octal,
        Self::Binary,
    ];
}

/// One integer placeholder, already parsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntPlaceholder {
    /// How the digits are written.
    pub base: IntBase,
    /// The `+` flag. Only [`IntBase::Signed`] reads it.
    pub plus: bool,
    /// The `-` flag: pad on the right.
    pub left: bool,
    /// The padding character.
    pub pad: char,
    /// The minimum width, in characters.
    pub width: usize,
    /// A `%s` precision: the most characters of the digits that are kept.
    /// `None` for every integer conversion, which ignores its precision.
    pub precision: Option<usize>,
}

/// The widest rendering: 64 binary digits, or a sign and 20 decimal ones.
const DIGITS_MAX: usize = 72;

/// The bits of a packed placeholder that each field takes, above the kind's
/// own low byte.
const BASE_SHIFT: u32 = 8;
const PLUS_BIT: u64 = 1 << 11;
const LEFT_BIT: u64 = 1 << 12;
const PRECISION_BIT: u64 = 1 << 13;
const PAD_SHIFT: u32 = 14;
const WIDTH_SHIFT: u32 = 35;
const PRECISION_SHIFT: u32 = 49;
/// The widest width or precision a packed placeholder has room for.
pub const PACKED_MAX: usize = (1 << 14) - 1;

impl IntPlaceholder {
    /// This placeholder packed into a [`crate::StrPart`] kind word, with
    /// `kind` in the low byte. `None` when the width or the precision is
    /// above [`PACKED_MAX`].
    #[must_use]
    pub fn kind_word(&self, kind: u64) -> Option<u64> {
        if self.width > PACKED_MAX || self.precision.is_some_and(|p| p > PACKED_MAX) {
            return None;
        }
        let base = IntBase::ALL.iter().position(|b| *b == self.base)? as u64;
        let mut word = (kind & 0xff) | base << BASE_SHIFT;
        if self.plus {
            word |= PLUS_BIT;
        }
        if self.left {
            word |= LEFT_BIT;
        }
        if let Some(precision) = self.precision {
            word |= PRECISION_BIT | (precision as u64) << PRECISION_SHIFT;
        }
        word |= u64::from(u32::from(self.pad)) << PAD_SHIFT;
        word |= (self.width as u64) << WIDTH_SHIFT;
        Some(word)
    }

    /// The placeholder [`Self::kind_word`] packed into `word`. `None` for a
    /// word no placeholder packs to.
    #[must_use]
    pub fn of_kind_word(word: u64) -> Option<Self> {
        let field = |shift: u32, bits: u32| (word >> shift) & ((1 << bits) - 1);
        let base = *IntBase::ALL.get(usize::try_from(field(BASE_SHIFT, 3)).ok()?)?;
        let pad = char::from_u32(u32::try_from(field(PAD_SHIFT, 21)).ok()?)?;
        let width = usize::try_from(field(WIDTH_SHIFT, 14)).ok()?;
        let precision = match word & PRECISION_BIT {
            0 => None,
            _ => Some(usize::try_from(field(PRECISION_SHIFT, 14)).ok()?),
        };
        Some(Self {
            base,
            plus: word & PLUS_BIT != 0,
            left: word & LEFT_BIT != 0,
            pad,
            width,
            precision,
        })
    }

    /// `value`'s digits, with a `+` when [`Self::plus`] asks for one and cut
    /// to [`Self::precision`], before padding. They are written into the end
    /// of `buf`, and the result is the kept part.
    #[expect(
        clippy::cast_sign_loss,
        reason = "every base but `Signed` reads the value's bits, as PHP's `printf` does"
    )]
    fn digits<'b>(&self, value: i64, buf: &'b mut [u8; DIGITS_MAX]) -> &'b [u8] {
        let bits = value as u64;
        let start = match self.base {
            IntBase::Signed | IntBase::Unsigned => {
                let (negative, magnitude) = match self.base {
                    IntBase::Signed => (value < 0, value.unsigned_abs()),
                    _ => (false, bits),
                };
                let written = match <&mut [u8; DECIMAL_MAX]>::try_from(
                    &mut buf[DIGITS_MAX - DECIMAL_MAX..],
                ) {
                    Ok(tail) => write_decimal(negative, magnitude, tail).len(),
                    Err(_) => unreachable!("the tail is DECIMAL_MAX bytes long"),
                };
                let mut start = DIGITS_MAX - written;
                if self.plus && self.base == IntBase::Signed && !negative {
                    start -= 1;
                    buf[start] = b'+';
                }
                start
            }
            IntBase::LowerHex => write_radix(bits, 4, b"0123456789abcdef", buf),
            IntBase::UpperHex => write_radix(bits, 4, b"0123456789ABCDEF", buf),
            IntBase::Octal => write_radix(bits, 3, b"01234567", buf),
            IntBase::Binary => write_radix(bits, 1, b"01", buf),
        };
        let body = &buf[start..];
        match self.precision {
            Some(precision) => &body[..body.len().min(precision)],
            None => body,
        }
    }

    /// How many bytes [`Self::digits`] keeps for `value`, counted without
    /// writing a digit.
    #[expect(
        clippy::cast_sign_loss,
        reason = "every base but `Signed` reads the value's bits, as PHP's `printf` does"
    )]
    fn digits_len(&self, value: i64) -> usize {
        let bits = value as u64;
        let width = u64::BITS - bits.leading_zeros();
        let len = match self.base {
            IntBase::Signed => {
                let sign = value < 0 || self.plus;
                decimal_len(false, value.unsigned_abs()) + usize::from(sign)
            }
            IntBase::Unsigned => decimal_len(false, bits),
            IntBase::LowerHex | IntBase::UpperHex => radix_len(width, 4),
            IntBase::Octal => radix_len(width, 3),
            IntBase::Binary => radix_len(width, 1),
        };
        self.precision.map_or(len, |precision| len.min(precision))
    }

    /// How many padding characters go with `body_len` bytes of ASCII.
    fn short(&self, body_len: usize) -> usize {
        self.width.saturating_sub(body_len)
    }

    /// How many bytes [`Self::write`] writes for `value`.
    #[must_use]
    pub fn len(&self, value: i64) -> usize {
        let body = self.digits_len(value);
        body + self.short(body) * self.pad.len_utf8()
    }

    /// Writes `value` as this placeholder renders it.
    ///
    /// Zero padding goes after the sign: `%05d` of `-42` is `-0042`.
    pub fn write(&self, value: i64, out: &mut StrWriter<'_>) {
        let mut buf = [0u8; DIGITS_MAX];
        let body = self.digits(value, &mut buf);
        let short = self.short(body.len());
        if short == 0 {
            out.push(body);
            return;
        }
        if self.left {
            out.push(body);
            self.pad_run(short, out);
            return;
        }
        let sign = usize::from(self.pad == '0' && matches!(body.first(), Some(b'-' | b'+')));
        out.push(&body[..sign]);
        self.pad_run(short, out);
        out.push(&body[sign..]);
    }

    /// `count` padding characters.
    fn pad_run(&self, count: usize, out: &mut StrWriter<'_>) {
        let mut encoded = [0u8; 4];
        let pad = self.pad.encode_utf8(&mut encoded);
        for _ in 0..count {
            out.push_str(pad);
        }
    }
}

/// `bits` in the power-of-two radix whose digits are `shift` bits wide,
/// written into the end of `buf` from `table`. The result is where the
/// digits start.
fn write_radix(mut bits: u64, shift: u32, table: &[u8], buf: &mut [u8; DIGITS_MAX]) -> usize {
    let mask = (1u64 << shift) - 1;
    let mut at = DIGITS_MAX;
    loop {
        at -= 1;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the mask keeps at most four bits"
        )]
        let digit = (bits & mask) as usize;
        buf[at] = table[digit];
        bits >>= shift;
        if bits == 0 {
            return at;
        }
    }
}

/// How many digits [`write_radix`] writes for a value `width` bits wide.
fn radix_len(width: u32, shift: u32) -> usize {
    width.div_ceil(shift).max(1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text `format!` gives for `value` under `placeholder`, which is
    /// what every placeholder printed before the digit writer replaced it.
    #[expect(clippy::cast_sign_loss, reason = "the bases read the value's bits")]
    fn formatted(placeholder: &IntPlaceholder, value: i64) -> String {
        let bits = value as u64;
        let mut body = match placeholder.base {
            IntBase::Signed if placeholder.plus && value >= 0 => format!("+{value}"),
            IntBase::Signed => format!("{value}"),
            IntBase::Unsigned => format!("{bits}"),
            IntBase::LowerHex => format!("{bits:x}"),
            IntBase::UpperHex => format!("{bits:X}"),
            IntBase::Octal => format!("{bits:o}"),
            IntBase::Binary => format!("{bits:b}"),
        };
        if let Some(precision) = placeholder.precision {
            body.truncate(precision);
        }
        let short = placeholder.width.saturating_sub(body.len());
        let pad: String = std::iter::repeat_n(placeholder.pad, short).collect();
        if placeholder.left {
            return body + &pad;
        }
        let sign = usize::from(placeholder.pad == '0' && body.starts_with(['-', '+']));
        format!("{}{pad}{}", &body[..sign], &body[sign..])
    }

    #[test]
    #[expect(clippy::cast_possible_wrap, reason = "`u64::MAX` is read as its bits")]
    fn every_integer_placeholder_writes_what_the_formatter_writes() {
        let values = [
            0,
            1,
            -1,
            7,
            9,
            10,
            -10,
            99,
            100,
            255,
            256,
            4096,
            -4096,
            999_999_999,
            1_000_000_000,
            i64::MIN,
            i64::MIN + 1,
            i64::MAX,
            u64::MAX as i64,
        ];
        // Miri takes past its 300 s limit over every case, so it runs a width
        // and a precision under and past each value's own length, zero, a
        // negative value, each end of the range, and one ASCII and one
        // multi-byte pad.
        let miri_values = [0, -4096, i64::MIN, u64::MAX as i64];
        let (values, pads): (&[i64], &[char]) = if cfg!(miri) {
            (&miri_values, &['0', 'é'])
        } else {
            (&values, &[' ', '0', 'é'])
        };
        let (widths, precisions): (&[usize], &[Option<usize>]) = if cfg!(miri) {
            (&[0, 25], &[None, Some(30)])
        } else {
            (
                &[0, 1, 5, 25, 80],
                &[None, Some(0), Some(1), Some(3), Some(30)],
            )
        };
        for base in IntBase::ALL {
            for plus in [false, true] {
                for left in [false, true] {
                    for &pad in pads {
                        for &width in widths {
                            for &precision in precisions {
                                let placeholder = IntPlaceholder {
                                    base,
                                    plus,
                                    left,
                                    pad,
                                    width,
                                    precision,
                                };
                                for &value in values {
                                    let want = formatted(&placeholder, value);
                                    let len = placeholder.len(value);
                                    let got = crate::NvsStr::build(len, |out| {
                                        placeholder.write(value, out);
                                    });
                                    assert_eq!(len, want.len(), "{placeholder:?} of {value}");
                                    assert_eq!(
                                        got.as_bytes(),
                                        want.as_bytes(),
                                        "{placeholder:?} of {value}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn sample() -> IntPlaceholder {
        IntPlaceholder {
            base: IntBase::UpperHex,
            plus: true,
            left: true,
            pad: 'é',
            width: 9999,
            precision: Some(3),
        }
    }

    #[test]
    fn a_placeholder_packs_into_a_kind_word_and_back() {
        let placeholder = sample();
        let word = placeholder.kind_word(3).expect("fits");
        assert_eq!(word & 0xff, 3);
        assert_eq!(IntPlaceholder::of_kind_word(word), Some(placeholder));
        let wide = IntPlaceholder {
            width: PACKED_MAX + 1,
            ..placeholder
        };
        assert_eq!(wide.kind_word(3), None);
    }
}
