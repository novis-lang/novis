//! The value forms Active Directory sends that are not text: a GUID, a SID, a FILETIME, an interval, a GeneralizedTime, a flag field and an account type
//!
//! ADR 0278 § 8, `rule:core-classes/ldap-value-types`. Each reader takes the
//! bytes of one attribute value as the server sent them and returns the
//! numbers a typed `Core` value is built from, or `None`/an error where the
//! value is not in that form. Nothing here allocates a Novis value: the
//! standard library owns `Core\Uuid` and `Core\Time`, and builds them from
//! what these return.
//!
//! - A GUID is 16 bytes whose first three groups are little-endian, so
//!   [`uuid_from_guid`] swaps them into the order RFC 9562's text spells.
//! - A SID is [`Sid`], read from its binary form ([`Sid::from_bytes`]) or its
//!   `S-1-5-21-…` text ([`Sid::parse`]).
//! - A FILETIME is a count of 100-nanosecond ticks since 1601 written as a
//!   decimal integer. `0` and `i64::MAX` mean "never", and [`filetime`]
//!   returns `None` for both.
//! - An interval is the same tick count, negative, as AD stores `maxPwdAge`.
//!   `i64::MIN` means "never", and [`interval`] returns `None` for it.
//! - A GeneralizedTime is RFC 4517 § 3.3.13's text, read by
//!   [`generalized_time`] into civil fields and an offset.
//! - A flag field is a 32-bit integer, unsigned for `userAccountControl`
//!   and signed for `groupType`, read by [`flag_field`] as AD wrote it, so a
//!   bit [`ACCOUNT_FLAGS`] or [`GROUP_TYPE_FLAGS`] does not name is kept.
//!   `sAMAccountType` is one of the values [`account_type`] reads.
//! - An INTEGER is RFC 4517 § 3.3.16's decimal text, read by [`int`] where
//!   an `i64` holds it, and a Boolean is § 3.3.3's `TRUE` or `FALSE`, read
//!   by [`boolean`] with the case exactly as the RFC writes it.

use std::fmt;

/// A value that is not in the form a typed reader needs. The text says which
/// form it needed, and is completed by the caller with the attribute's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueError {
    /// Not 16 bytes, so not a GUID.
    NotAGuid,
    /// Not a SID's binary form.
    NotASid,
    /// Not a decimal integer, or an integer no FILETIME has.
    NotAFiletime,
    /// Not a decimal integer that is zero or negative.
    NotAnInterval,
    /// An interval longer than about 292 years, which no `Duration` holds.
    IntervalTooLong,
    /// Not RFC 4517's GeneralizedTime.
    NotAGeneralizedTime,
    /// Not a decimal integer a 32-bit flag field holds.
    NotAFlagField,
    /// Not one of the `sAMAccountType` values AD defines.
    NotAnAccountType,
    /// Not a decimal integer, or one past an `i64`.
    NotAnInt,
    /// Not `TRUE` or `FALSE`.
    NotABoolean,
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotAGuid => "is not a GUID, which is 16 bytes",
            Self::NotASid => "is not a SID",
            Self::NotAFiletime => "is not a time",
            Self::NotAnInterval => "is not a length of time",
            Self::IntervalTooLong => "is a length of time longer than a `Duration` can be",
            Self::NotAGeneralizedTime => "is not a time",
            Self::NotAFlagField => "is not a 32-bit number of flags",
            Self::NotAnAccountType => "is not an account type",
            Self::NotAnInt => "is not a whole number that fits in an `int`",
            Self::NotABoolean => "is not `TRUE` or `FALSE`",
        })
    }
}

/// The 16 octets of a `Core\Uuid` for AD's GUID bytes: the first group of
/// four, then the two groups of two, each reversed. The swap is its own
/// inverse, so the same function writes a `Uuid` back in AD's order.
///
/// # Errors
///
/// [`ValueError::NotAGuid`] for a value that is not 16 bytes.
pub fn uuid_from_guid(bytes: &[u8]) -> Result<[u8; 16], ValueError> {
    let mut octets: [u8; 16] = bytes.try_into().map_err(|_| ValueError::NotAGuid)?;
    octets[0..4].reverse();
    octets[4..6].reverse();
    octets[6..8].reverse();
    Ok(octets)
}

/// The most sub-authorities a SID has, by MS-DTYP § 2.4.2.2.
const MOST_SUB_AUTHORITIES: usize = 15;

/// The identifier-authority values the text form writes in decimal; a larger
/// one is written in hex, by MS-DTYP § 2.4.2.1.
const DECIMAL_AUTHORITY_BELOW: u64 = 1 << 32;

/// A Windows security identifier, MS-DTYP § 2.4.2: revision 1, a 48-bit
/// identifier authority, and one to fifteen 32-bit sub-authorities.
///
/// A SID with no sub-authority has no `rid` and no domain, and none is ever
/// issued to an account, so both readers refuse one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sid {
    authority: u64,
    sub_authorities: Vec<u32>,
}

/// Why [`Sid::parse`] refused a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidTextError {
    /// It does not start with `S-1-`.
    Prefix,
    /// A part is not a number, or is out of its range.
    Part,
    /// It has no sub-authority, or more than fifteen.
    Count,
}

impl fmt::Display for SidTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Prefix => "a SID starts with `S-1-`",
            Self::Part => "every part of a SID after `S-1-` is a number",
            Self::Count => "a SID has 1 to 15 numbers after its authority",
        })
    }
}

impl Sid {
    /// The SID in its binary form: the revision, the count, six bytes of
    /// authority big-endian, and each sub-authority little-endian.
    ///
    /// # Errors
    ///
    /// [`ValueError::NotASid`] for any other bytes, a trailing byte included.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ValueError> {
        let [1, count, authority @ ..] = bytes else {
            return Err(ValueError::NotASid);
        };
        let count = usize::from(*count);
        if !(1..=MOST_SUB_AUTHORITIES).contains(&count) || authority.len() != 6 + 4 * count {
            return Err(ValueError::NotASid);
        }
        let (authority, rest) = authority.split_at(6);
        let authority = authority
            .iter()
            .fold(0_u64, |sum, &byte| (sum << 8) | u64::from(byte));
        let sub_authorities = rest
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        Ok(Self {
            authority,
            sub_authorities,
        })
    }

    /// The SID its text names, such as `S-1-5-21-1004336348-1177238915-682003330-512`.
    /// The authority may be written in hex with `0x`, as the text form writes
    /// one past 32 bits.
    ///
    /// # Errors
    ///
    /// A [`SidTextError`] saying which part is wrong.
    pub fn parse(text: &str) -> Result<Self, SidTextError> {
        let rest = text.strip_prefix("S-1-").ok_or(SidTextError::Prefix)?;
        let mut parts = rest.split('-');
        let authority = parts.next().unwrap_or_default();
        let authority = match authority.strip_prefix("0x") {
            Some(hex) => decimal_or_hex(hex, 16),
            None => decimal_or_hex(authority, 10),
        }
        .filter(|&value| value < 1 << 48)
        .ok_or(SidTextError::Part)?;
        let sub_authorities = parts
            .map(|part| {
                decimal_or_hex(part, 10)
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or(SidTextError::Part)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !(1..=MOST_SUB_AUTHORITIES).contains(&sub_authorities.len()) {
            return Err(SidTextError::Count);
        }
        Ok(Self {
            authority,
            sub_authorities,
        })
    }

    /// The text form, `S-1-` and then the authority and each sub-authority.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut text = if self.authority < DECIMAL_AUTHORITY_BELOW {
            format!("S-1-{}", self.authority)
        } else {
            format!("S-1-0x{:012X}", self.authority)
        };
        for sub in &self.sub_authorities {
            text.push('-');
            text.push_str(&sub.to_string());
        }
        text
    }

    /// The binary form [`Sid::from_bytes`] reads.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + 4 * self.sub_authorities.len());
        bytes.push(1);
        // At most fifteen, which `parse` and `from_bytes` both check.
        bytes.push(u8::try_from(self.sub_authorities.len()).unwrap_or(u8::MAX));
        bytes.extend_from_slice(&self.authority.to_be_bytes()[2..]);
        for sub in &self.sub_authorities {
            bytes.extend_from_slice(&sub.to_le_bytes());
        }
        bytes
    }

    /// The SID without its last sub-authority: for an account, the domain
    /// it belongs to. `None` for a SID with one sub-authority, since a SID
    /// has at least one.
    #[must_use]
    pub fn domain(&self) -> Option<Self> {
        let (_, rest) = self.sub_authorities.split_last()?;
        (!rest.is_empty()).then(|| Self {
            authority: self.authority,
            sub_authorities: rest.to_vec(),
        })
    }

    /// The last sub-authority, the relative identifier: `500` for a domain's
    /// built-in `Administrator`, `512` for `Domain Admins`.
    #[must_use]
    pub fn rid(&self) -> u32 {
        self.sub_authorities.last().copied().unwrap_or_default()
    }
}

/// `text` as an unsigned number in `radix`, with no sign and no empty text.
fn decimal_or_hex(text: &str, radix: u32) -> Option<u64> {
    if text.is_empty() || !text.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    u64::from_str_radix(text, radix).ok()
}

/// An attribute value's decimal integer, with an optional leading `-`.
fn integer(value: &[u8]) -> Option<i64> {
    let text = std::str::from_utf8(value).ok()?;
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Whether a value is a decimal integer, which is how a FILETIME is told
/// from a GeneralizedTime: the second always ends in `Z` or an offset.
#[must_use]
pub fn is_integer(value: &[u8]) -> bool {
    integer(value).is_some()
}

/// An INTEGER value as the `i64` it writes.
///
/// # Errors
///
/// [`ValueError::NotAnInt`] for a value that is not a decimal integer, or is
/// one no `i64` holds.
pub fn int(value: &[u8]) -> Result<i64, ValueError> {
    integer(value).ok_or(ValueError::NotAnInt)
}

/// A Boolean value as the `bool` it writes.
///
/// # Errors
///
/// [`ValueError::NotABoolean`] for anything but `TRUE` and `FALSE`.
pub fn boolean(value: &[u8]) -> Result<bool, ValueError> {
    match value {
        b"TRUE" => Ok(true),
        b"FALSE" => Ok(false),
        _ => Err(ValueError::NotABoolean),
    }
}

/// FILETIME ticks between 1601-01-01 and 1970-01-01.
const FILETIME_UNIX_EPOCH: i128 = 116_444_736_000_000_000;

/// Nanoseconds in a FILETIME tick.
const NANOS_PER_TICK: i128 = 100;

/// A FILETIME value as nanoseconds since 1970-01-01 UTC, negative before it,
/// or `None` for `0` and `i64::MAX`, which AD writes for "never".
///
/// # Errors
///
/// [`ValueError::NotAFiletime`] for a value that is not a decimal integer,
/// or is negative.
pub fn filetime(value: &[u8]) -> Result<Option<i128>, ValueError> {
    let ticks = integer(value).ok_or(ValueError::NotAFiletime)?;
    match ticks {
        0 | i64::MAX => Ok(None),
        ..0 => Err(ValueError::NotAFiletime),
        _ => Ok(Some(
            (i128::from(ticks) - FILETIME_UNIX_EPOCH) * NANOS_PER_TICK,
        )),
    }
}

/// An interval value as a length in nanoseconds, or `None` for `i64::MIN`,
/// which AD writes for "never". AD stores an interval as a negative tick
/// count, so `-36288000000000` is 42 days.
///
/// # Errors
///
/// [`ValueError::NotAnInterval`] for a value that is not a decimal integer,
/// or is positive, and [`ValueError::IntervalTooLong`] for one past
/// `i64::MAX` nanoseconds.
pub fn interval(value: &[u8]) -> Result<Option<i64>, ValueError> {
    let ticks = integer(value).ok_or(ValueError::NotAnInterval)?;
    if ticks == i64::MIN {
        return Ok(None);
    }
    if ticks > 0 {
        return Err(ValueError::NotAnInterval);
    }
    ticks
        .checked_neg()
        .and_then(|ticks| ticks.checked_mul(100))
        .map(Some)
        .ok_or(ValueError::IntervalTooLong)
}

/// One bit a flag field names: the name of the `bool` reader for it, and the
/// bit's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flag {
    /// The reader's name, such as `disabled`.
    pub name: &'static str,
    /// The bit, such as `0x2`.
    pub bit: u32,
}

/// The bits of `userAccountControl` that `Ad\AccountFlags` names, in bit
/// order. `PASSWD_CANT_CHANGE` (`0x40`) is not here: AD does not store it,
/// so a value that has it keeps it as a bit no reader names.
pub const ACCOUNT_FLAGS: &[Flag] = &[
    Flag {
        name: "script",
        bit: 0x1,
    },
    Flag {
        name: "disabled",
        bit: 0x2,
    },
    Flag {
        name: "homeDirectoryRequired",
        bit: 0x8,
    },
    Flag {
        name: "lockedOut",
        bit: 0x10,
    },
    Flag {
        name: "passwordNotRequired",
        bit: 0x20,
    },
    Flag {
        name: "reversibleEncryption",
        bit: 0x80,
    },
    Flag {
        name: "temporaryDuplicateAccount",
        bit: 0x100,
    },
    Flag {
        name: "normalAccount",
        bit: 0x200,
    },
    Flag {
        name: "interdomainTrustAccount",
        bit: 0x800,
    },
    Flag {
        name: "workstationTrustAccount",
        bit: 0x1000,
    },
    Flag {
        name: "serverTrustAccount",
        bit: 0x2000,
    },
    Flag {
        name: "passwordNeverExpires",
        bit: 0x1_0000,
    },
    Flag {
        name: "mnsLogonAccount",
        bit: 0x2_0000,
    },
    Flag {
        name: "smartcardRequired",
        bit: 0x4_0000,
    },
    Flag {
        name: "trustedForDelegation",
        bit: 0x8_0000,
    },
    Flag {
        name: "notDelegated",
        bit: 0x10_0000,
    },
    Flag {
        name: "useDesKeyOnly",
        bit: 0x20_0000,
    },
    Flag {
        name: "noPreauthRequired",
        bit: 0x40_0000,
    },
    Flag {
        name: "passwordExpired",
        bit: 0x80_0000,
    },
    Flag {
        name: "trustedToAuthForDelegation",
        bit: 0x100_0000,
    },
    Flag {
        name: "partialSecretsAccount",
        bit: 0x400_0000,
    },
];

/// The two [`ACCOUNT_FLAGS`] AD computes rather than stores, lockout and an
/// expired password. They are read from [`COMPUTED_ACCOUNT_CONTROL`].
pub const COMPUTED_ACCOUNT_FLAGS: u32 = 0x10 | 0x80_0000;

/// The constructed attribute that carries [`COMPUTED_ACCOUNT_FLAGS`].
pub const COMPUTED_ACCOUNT_CONTROL: &str = "msDS-User-Account-Control-Computed";

/// The bits of `groupType` that `Ad\GroupType` names, in bit order.
pub const GROUP_TYPE_FLAGS: &[Flag] = &[
    Flag {
        name: "system",
        bit: 0x1,
    },
    Flag {
        name: "global",
        bit: 0x2,
    },
    Flag {
        name: "domainLocal",
        bit: 0x4,
    },
    Flag {
        name: "universal",
        bit: 0x8,
    },
    Flag {
        name: "appBasic",
        bit: 0x10,
    },
    Flag {
        name: "appQuery",
        bit: 0x20,
    },
    Flag {
        name: "security",
        bit: 0x8000_0000,
    },
];

/// Every bit `flags` names, as one mask.
#[must_use]
pub fn named_bits(flags: &[Flag]) -> u32 {
    flags.iter().fold(0, |mask, flag| mask | flag.bit)
}

/// A flag field's value as the integer AD wrote. `userAccountControl` is
/// unsigned and `groupType` is signed, so `-2147483646` is a security group,
/// and every value in either range is read unchanged.
///
/// # Errors
///
/// [`ValueError::NotAFlagField`] for a value that is not a decimal integer in
/// `i32::MIN..=u32::MAX`.
pub fn flag_field(value: &[u8]) -> Result<i64, ValueError> {
    integer(value)
        .filter(|bits| (i64::from(i32::MIN)..=i64::from(u32::MAX)).contains(bits))
        .ok_or(ValueError::NotAFlagField)
}

/// Whether `bits`, as [`flag_field`] read it, has `flag` set.
#[must_use]
pub fn has_flag(bits: i64, flag: u32) -> bool {
    bits & i64::from(flag) != 0
}

/// `bits`, as [`flag_field`] read it, with each `(flag, on)` of `changes` set
/// or cleared and every other bit kept. The result is the 32 bits AD stores:
/// `userAccountControl` reads it unsigned and `groupType` signed, so the
/// sign of a `groupType` is its security bit and nothing else.
#[must_use]
pub fn with_flags(bits: i64, changes: impl IntoIterator<Item = (u32, bool)>) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "`flag_field` reads a value in `i32::MIN..=u32::MAX`, whose low 32 bits are the field"
    )]
    let stored = bits as u32;
    changes.into_iter().fold(
        stored,
        |out, (flag, on)| if on { out | flag } else { out & !flag },
    )
}

/// `sAMAccountType`'s values, as `Ad\AccountType` names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountType {
    /// `SAM_DOMAIN_OBJECT`, `0x0`.
    Domain,
    /// `SAM_GROUP_OBJECT`, `0x10000000`.
    Group,
    /// `SAM_NON_SECURITY_GROUP_OBJECT`, `0x10000001`.
    NonSecurityGroup,
    /// `SAM_ALIAS_OBJECT`, `0x20000000`, a domain-local security group.
    Alias,
    /// `SAM_NON_SECURITY_ALIAS_OBJECT`, `0x20000001`.
    NonSecurityAlias,
    /// `SAM_USER_OBJECT`, `0x30000000`.
    User,
    /// `SAM_MACHINE_ACCOUNT`, `0x30000001`.
    Machine,
    /// `SAM_TRUST_ACCOUNT`, `0x30000002`.
    Trust,
    /// `SAM_APP_BASIC_GROUP`, `0x40000000`.
    AppBasicGroup,
    /// `SAM_APP_QUERY_GROUP`, `0x40000001`.
    AppQueryGroup,
}

impl AccountType {
    /// Every type with the value AD writes for it.
    pub const ALL: &[(Self, i64)] = &[
        (Self::Domain, 0x0),
        (Self::Group, 0x1000_0000),
        (Self::NonSecurityGroup, 0x1000_0001),
        (Self::Alias, 0x2000_0000),
        (Self::NonSecurityAlias, 0x2000_0001),
        (Self::User, 0x3000_0000),
        (Self::Machine, 0x3000_0001),
        (Self::Trust, 0x3000_0002),
        (Self::AppBasicGroup, 0x4000_0000),
        (Self::AppQueryGroup, 0x4000_0001),
    ];

    /// The case's name, as the registry's enum spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Domain => "Domain",
            Self::Group => "Group",
            Self::NonSecurityGroup => "NonSecurityGroup",
            Self::Alias => "Alias",
            Self::NonSecurityAlias => "NonSecurityAlias",
            Self::User => "User",
            Self::Machine => "Machine",
            Self::Trust => "Trust",
            Self::AppBasicGroup => "AppBasicGroup",
            Self::AppQueryGroup => "AppQueryGroup",
        }
    }
}

/// A `sAMAccountType` value as the type it names.
///
/// # Errors
///
/// [`ValueError::NotAnAccountType`] for a value that is not a decimal
/// integer AD defines.
pub fn account_type(value: &[u8]) -> Result<AccountType, ValueError> {
    let read = integer(value).ok_or(ValueError::NotAnAccountType)?;
    AccountType::ALL
        .iter()
        .find(|(_, written)| *written == read)
        .map(|(kind, _)| *kind)
        .ok_or(ValueError::NotAnAccountType)
}

/// A GeneralizedTime's civil fields as the text wrote them, and its offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneralizedTime {
    /// The year, four digits.
    pub year: i32,
    /// The month, 1 to 12, not yet checked against the day.
    pub month: u8,
    /// The day of the month, not yet checked against the month.
    pub day: u8,
    /// The hour, 0 to 23.
    pub hour: u8,
    /// The minute, 0 to 59.
    pub minute: u8,
    /// The second, 0 to 60, where 60 is a leap second.
    pub second: u8,
    /// The fraction, in nanoseconds of the second.
    pub nanosecond: u32,
    /// Seconds east of UTC: `0` for `Z`.
    pub offset: i32,
}

/// RFC 4517 § 3.3.13's GeneralizedTime: `YYYYMMDDHH`, then optional minutes
/// and seconds, an optional fraction of the last of those, and `Z` or an
/// offset of `±HH` or `±HHMM`. AD writes `20240101120000.0Z`.
///
/// # Errors
///
/// [`ValueError::NotAGeneralizedTime`] for any other text, a field out of
/// its range included. A day the month does not have is the caller's to
/// refuse, with the calendar it builds the value in.
pub fn generalized_time(value: &[u8]) -> Result<GeneralizedTime, ValueError> {
    read_generalized_time(value).ok_or(ValueError::NotAGeneralizedTime)
}

/// Two ASCII digits at `at` as a number.
fn two_digits(value: &[u8], at: usize) -> Option<u8> {
    match value.get(at..at + 2)? {
        &[a, b] if a.is_ascii_digit() && b.is_ascii_digit() => Some((a - b'0') * 10 + (b - b'0')),
        _ => None,
    }
}

fn read_generalized_time(value: &[u8]) -> Option<GeneralizedTime> {
    const NANOS_PER_SECOND: u64 = 1_000_000_000;
    let year = i32::from(two_digits(value, 0)?) * 100 + i32::from(two_digits(value, 2)?);
    let month = two_digits(value, 4)?;
    let day = two_digits(value, 6)?;
    let hour = two_digits(value, 8)?;
    let mut at = 10;
    // The seconds the fraction is a part of: an hour's, a minute's or one.
    let mut unit = 3600_u64;
    let mut minute = 0;
    let mut second = 0;
    if let Some(read) = two_digits(value, at) {
        minute = read;
        at += 2;
        unit = 60;
        if let Some(read) = two_digits(value, at) {
            second = read;
            at += 2;
            unit = 1;
        }
    }
    // The fraction of `unit`, in nanoseconds.
    let mut fraction = 0_u64;
    if matches!(value.get(at), Some(b'.' | b',')) {
        at += 1;
        let digits = value[at..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if digits == 0 {
            return None;
        }
        let mut scale = NANOS_PER_SECOND;
        for &digit in &value[at..at + digits] {
            scale /= 10;
            fraction += u64::from(digit - b'0') * scale;
        }
        at += digits;
    }
    let offset = match value.get(at..)? {
        b"Z" => 0,
        [sign @ (b'+' | b'-'), rest @ ..] => {
            let hours = i32::from(two_digits(rest, 0)?);
            let minutes = match rest.len() {
                2 => 0,
                4 => i32::from(two_digits(rest, 2)?),
                _ => return None,
            };
            if hours > 23 || minutes > 59 {
                return None;
            }
            let seconds = hours * 3600 + minutes * 60;
            if *sign == b'-' { -seconds } else { seconds }
        }
        _ => return None,
    };
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    if second > 60 {
        return None;
    }
    // A fraction of an hour or a minute adds whole minutes and seconds.
    let extra = fraction * unit;
    let extra_seconds = extra / NANOS_PER_SECOND;
    let nanosecond = u32::try_from(extra % NANOS_PER_SECOND).ok()?;
    let minute = minute + u8::try_from(extra_seconds / 60).ok()?;
    let second = second + u8::try_from(extra_seconds % 60).ok()?;
    Some(GeneralizedTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        nanosecond,
        offset,
    })
}
