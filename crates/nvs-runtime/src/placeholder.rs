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
//! Nothing on the heap. The digits are written into a buffer on the stack,
//! once to measure and once to write.

use std::fmt::Write as _;

use crate::StrWriter;

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
    /// to [`Self::precision`], before padding.
    #[expect(
        clippy::cast_sign_loss,
        reason = "every base but `Signed` reads the value's bits, as PHP's `printf` does"
    )]
    fn digits(&self, value: i64, buf: &mut Digits) {
        let bits = value as u64;
        // A write into `Digits` fails only past `DIGITS_MAX` bytes, which no
        // `i64` reaches in any base.
        let _ = match self.base {
            IntBase::Signed if self.plus && value >= 0 => write!(buf, "+{value}"),
            IntBase::Signed => write!(buf, "{value}"),
            IntBase::Unsigned => write!(buf, "{bits}"),
            IntBase::LowerHex => write!(buf, "{bits:x}"),
            IntBase::UpperHex => write!(buf, "{bits:X}"),
            IntBase::Octal => write!(buf, "{bits:o}"),
            IntBase::Binary => write!(buf, "{bits:b}"),
        };
        if let Some(precision) = self.precision {
            buf.len = buf.len.min(precision);
        }
    }

    /// How many padding characters go with `body`, which is ASCII.
    fn short(&self, body: &[u8]) -> usize {
        self.width.saturating_sub(body.len())
    }

    /// How many bytes [`Self::write`] writes for `value`.
    #[must_use]
    pub fn len(&self, value: i64) -> usize {
        let mut buf = Digits::default();
        self.digits(value, &mut buf);
        let body = buf.as_bytes();
        body.len() + self.short(body) * self.pad.len_utf8()
    }

    /// Writes `value` as this placeholder renders it.
    ///
    /// Zero padding goes after the sign: `%05d` of `-42` is `-0042`.
    pub fn write(&self, value: i64, out: &mut StrWriter<'_>) {
        let mut buf = Digits::default();
        self.digits(value, &mut buf);
        let body = buf.as_bytes();
        let short = self.short(body);
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

/// An integer's text in a buffer on the stack.
struct Digits {
    bytes: [u8; DIGITS_MAX],
    len: usize,
}

impl Default for Digits {
    fn default() -> Self {
        Self {
            bytes: [0; DIGITS_MAX],
            len: 0,
        }
    }
}

impl Digits {
    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl std::fmt::Write for Digits {
    fn write_str(&mut self, piece: &str) -> std::fmt::Result {
        let end = self.len + piece.len();
        let room = self.bytes.get_mut(self.len..end).ok_or(std::fmt::Error)?;
        room.copy_from_slice(piece.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
