//! `Core\Validate` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 12's prose roster, "what survives of `filter`: the genuine validators
//! only" — all six of it. Four name a *format* a human wrote (`isEmail`,
//! `isDomain`, `isIp`, `isMac`) and two ask what a `string` is made of
//! (`isAscii`, `isPrintable`). Every one answers `bool`, takes
//! the subject first, and **launders nothing** — that is the whole reason the
//! sanitizing half of `filter_var` is not here, and
//! `rule:security/tainted-qualifier`
//! is why: a member that half-escapes produces exactly the false confidence
//! that ADR exists to prevent, so no member in this module ever returns its
//! subject.
//!
//! # No dependency, and that is the decision rather than the default
//!
//! `rule:packaging/a-c-dependency-answers-two-questions`'s first
//! question is emphatically **yes** here — a form field is the most
//! attacker-reachable string a web server holds, and it reaches these members
//! before anything else looks at it. That would normally argue for binding
//! somebody else's parser, as [`crate::csv`] does for RFC 4180. It argues the
//! other way here, for a reason specific to what a validator *is*:
//!
//! A parser's job is to agree with a specification, and RFC 4180's corners are
//! what a hand-written scanner gets wrong one release at a time. A validator's
//! job is to draw a **line**, and where the line falls is a policy question
//! the caller is entitled to read. RFC 5322 says `"a b"(comment)@[192.0.2.1]`
//! is a well-formed address; no MTA on the public internet will carry it, and
//! a program that accepted it from a form would be storing a string its own
//! downstream code has never been tested against. So an RFC-complete
//! `isEmail` is not the *safe* answer — it is a wider surface with a citation
//! attached. What this module owes instead is a line that is **narrow,
//! stated, and short enough to read**, which the next four sections do in
//! about forty lines of straight-line byte comparisons with no state machine,
//! no backtracking and no allocation.
//!
//! One dependency-shaped thing is used and is not a dependency:
//! `isIp` is [`std::net::IpAddr`]'s `FromStr`. Address syntax is the one place
//! here where an external specification genuinely bites — IPv6's `::`
//! compression, its embedded-IPv4 tail, and the leading-zero question that
//! made `inet_aton`'s octal reading an SSRF primitive — and the standard
//! library already owns all three, correctly and without a crate.
//!
//! # `isEmail`: one `@`, a dot-atom, and a domain
//!
//! The subject is at most 254 bytes (RFC 5321 § 4.5.3.1's reverse-path limit),
//! splits at its **first** `@`, and both halves must then hold:
//!
//! - **The local part** is a *dot-atom*: one or more atoms of at least one
//!   byte, joined by single `.`, at most 64 bytes in total. An atom's bytes
//!   are RFC 5322 § 3.2.3's `atext` — ASCII alphanumerics and the nineteen
//!   specials `!#$%&'*+-/=?^_`{|}~`. So no leading dot, no trailing dot and no
//!   `..`, which falls out of "an atom has at least one byte" rather than
//!   being three more rules.
//! - **The domain** is [`is_hostname`]'s, plus **at least two labels**. A bare
//!   `postmaster@localhost` is a real address on one host and is never what a
//!   form meant, so it is refused here and `isDomain("localhost")` still
//!   answers `true` — the two members ask different questions and this is the
//!   one place they part company.
//!
//! **Refused, deliberately, all legal under RFC 5322:** a quoted local part
//! (`"a b"@x.test`), a domain literal (`a@[192.0.2.1]`, `a@[IPv6:::1]`),
//! comments, folding whitespace, and any byte over `0x7F`. An
//! internationalized address is not rejected as unrepresentable — it is
//! rejected as *not yet encoded*: its domain punycodes to LDH and its local
//! part needs SMTPUTF8, which is a delivery question rather than a syntax one.
//!
//! # `isDomain`: LDH labels, and nothing about whether it resolves
//!
//! RFC 1123 § 2.1's hostname: one or more labels joined by single `.`, each 1
//! to 63 bytes of ASCII letters, digits and `-`, neither starting nor ending
//! with `-`, and 253 bytes in total. A trailing root dot is refused — it is a
//! DNS wire-form detail, not something a user types into a field.
//!
//! Digits are allowed in every label including the last, so `isDomain` and
//! therefore `isEmail` accept `1.2.3.4` where PHP's `FILTER_VALIDATE_EMAIL`
//! refuses it. That is a divergence and it is the right way round: an
//! all-numeric name is a syntactically valid hostname that can carry an MX
//! record, and "this looks like an IP address" is `isIp`'s question asked
//! deliberately rather than this member's guess.
//!
//! Nothing here consults DNS, and nothing ever will: a validator that made a
//! network call would put an attacker-controlled string on the request path's
//! latency budget, which is latency traded away for nothing.
//!
//! # `isIp(string $s, {version?: 4|6})`
//!
//! Omitting `version` accepts either family; `{version: 4}` and
//! `{version: 6}` accept exactly one. The option is
//! `rule:types/single-value-types`'s
//! set of allowed values rather than an `int`, so `{version: 5}` does not compile —
//! spec § 12 names this as the reason `isIpV4`/`isIpV6` are *gone* rather than
//! being two more member names. It is the registry's first union-typed option,
//! and [`crate::registry::CoreTy::Union`] records what that costs.
//!
//! Leading zeros are refused (`192.000.002.001` is not an address), matching
//! PHP and rejecting the octal reading that `inet_aton` made famous. A zone
//! identifier (`fe80::1%eth0`) is refused too, as PHP refuses it: a scope is
//! not part of the address.
//!
//! # `isMac`
//!
//! Six octets as `00:11:22:33:44:55` or `00-11-22-33-44-55`, or three groups
//! of four as Cisco's `0011.2233.4455`. Hex digits in either case, one
//! separator throughout — `00:11-22:33:44:55` is refused. This is exactly
//! `FILTER_VALIDATE_MAC`'s reading, with no divergence to state.
//!
//! # `isAscii` and `isPrintable`: two questions, not one asked twice
//!
//! These two replace PHP's `ctype_*` family rather than a `FILTER_*` constant,
//! and they part company from it in the one way an Novis `string` forces:
//! a PHP string is bytes, so `ctype_print` answers about ASCII `0x20`–`0x7E`
//! and says `false` for `café`. An Novis `string` is UTF-8
//! (`rule:types/bytes`), so a
//! byte-wise reading would leave no member that could ask about text at all.
//! The two are therefore split by what they actually ask:
//!
//! - **`isAscii`** is the encoding question: every byte is `0x00`–`0x7F`. That
//!   is what a caller means when the answer has to survive an ASCII-only
//!   protocol field, and it is a byte scan with no decoding at all.
//! - **`isPrintable`** is the *character* question: no `char` in Unicode
//!   general category `Cc`. That is exactly [`char::is_control`], and `Cc` is
//!   exactly the set with no printed form — the C0 range, `DEL`, and the C1
//!   range a UTF-8 decode can produce. Tab, carriage return and newline are
//!   `Cc` and so are refused, as `ctype_print` refuses them; a field that must
//!   hold a multi-line textarea is asking a different question and should not
//!   reach for this member.
//!
//! So `isPrintable("café")` is `true` where `ctype_print` said `false`, and
//! both members answer `true` for the empty string where every `ctype_*`
//! answers `false`. The second is the older divergence of the two: a string
//! with no character that fails a test has no character that fails it, and
//! PHP's answer there is a documented quirk of a function that also treats a
//! small `int` as a codepoint.
//!
//! **`isPrintable` is not a spoofing check, and must not be read as one.** It
//! accepts the bidirectional overrides (`U+202E`), zero-width joiners and
//! homoglyphs, because each of those *is* printable and refusing them is a
//! question about where the text will be rendered rather than about the text.
//! Answering it here, under this name, would be precisely the false
//! confidence `rule:security/tainted-qualifier` refuses — escaping at the sink is what makes a string
//! safe to display, and no member in this module launders anything.
//!
//! # What these members do with a qualifier
//!
//! That last sentence is `rule:security/unclassified-parameter-refuses-tainted`'s classification already, and the rows
//! say it: **all six subjects are [`Qual::Neutral`]**, because all six answer
//! a `bool` and a `bool` carries no byte of what it was asked about. That is
//! the `Qual` enum's own first bullet, reached without a judgement.
//!
//! What the mark does *not* say is the part worth keeping in view: a `tainted`
//! address that `isEmail` accepts is still tainted, because nothing here
//! returns it. A validator that answered its own subject back would be
//! [`Qual::Launder`] by construction and would be exactly the false confidence
//! this module's docs refuse above — passing a syntax check is not the same
//! fact as being safe at a sink.

use nvs_runtime::{Fault, Tag, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Validate`'s fully-qualified name, written once so the registry row
/// and every diagnostic naming the class cannot drift apart.
pub(crate) const NAME: &str = r"Core\Validate";

/// `Core\Validate`'s registry rows — spec § 12's prose roster, format half.
///
/// The section states these as sentences rather than as a table, so there is
/// no line on `tests/spec-members-outstanding.txt` to strike: the gates that
/// hold this class are `examples/collect.nvs` and `conformance_coverage.rs`.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "isEmail",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_email",
            doc: Some(&IS_EMAIL_DOC),
        },
        CoreMethod {
            name: "isDomain",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_domain",
            doc: Some(&IS_DOMAIN_DOC),
        },
        CoreMethod {
            name: "isIp",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Options(IP_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_ip",
            doc: Some(&IS_IP_DOC),
        },
        CoreMethod {
            name: "isMac",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_mac",
            doc: Some(&IS_MAC_DOC),
        },
        CoreMethod {
            name: "isAscii",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_ascii",
            doc: Some(&IS_ASCII_DOC),
        },
        CoreMethod {
            name: "isPrintable",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_printable",
            doc: Some(&IS_PRINTABLE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Validate`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Checks whether a text has a certain format. `isEmail`, `isDomain`, `isIp` and `isMac` \
            check for an email address, a domain name, an IP address and a MAC address. \
            `isAscii` and `isPrintable` check which characters a text contains. Each method \
            returns `true` or `false` and never changes the text.",
};

/// `Core\Validate::isEmail`'s reference card — `rule:core-api/reference-card`.
const IS_EMAIL_DOC: MethodDoc = MethodDoc {
    short: "Checks whether `$s` is an email address, such as `ada@example.com`. The part before \
            the `@` has at most 64 characters: letters, digits, single dots and some symbols such \
            as `+` and `_`. The part after the `@` is a domain name with at least one dot. The \
            whole address has at most 254 characters.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to test.",
        shape: &[],
    }],
    ret: "`true` for an email address. `false` for anything else, for example a space, two dots \
          in a row, quotes, an address such as `a@[192.0.2.1]`, a domain without a dot such as \
          `a@localhost`, or a character that is not ASCII. The method does not check whether \
          the address exists.",
    errors: &[],
};

/// `Core\Validate::isDomain`'s reference card — `rule:core-api/reference-card`.
const IS_DOMAIN_DOC: MethodDoc = MethodDoc {
    short: "Checks whether `$s` is a domain name, such as `example.com`. A domain name has one or \
            more parts joined by single dots. Each part has 1 to 63 letters, digits or `-`, and \
            does not start or end with `-`. The whole name has at most 253 characters.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to test.",
        shape: &[],
    }],
    ret: "`true` for a domain name. `localhost` and a name of digits such as `1.2.3.4` are \
          `true` too. `false` for an empty part, a dot at the end, a part that starts or ends \
          with `-`, or a character that is not ASCII. The method does not check whether the \
          name exists.",
    errors: &[],
};

/// `Core\Validate::isIp`'s reference card — `rule:core-api/reference-card`.
const IS_IP_DOC: MethodDoc = MethodDoc {
    short: "Checks whether `$s` is an IP address. An IPv4 address is four numbers from 0 to 255 \
            joined by dots, such as `192.0.2.1`. An IPv6 address is up to eight groups of hex \
            digits joined by `:`, such as `2001:db8::1`.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The text to test.",
            shape: &[],
        },
        ParamDoc {
            name: "version",
            desc: "`4` accepts only IPv4 addresses, and `6` accepts only IPv6 addresses. Without \
                   it, both are accepted.",
            shape: &[],
        },
    ],
    ret: "`true` for an IP address of the chosen version. `false` for anything else. A number \
          with a leading zero, such as `192.0.2.01`, gives `false`. An IPv6 address with a zone \
          at the end, such as `fe80::1%eth0`, also gives `false`.",
    errors: &[],
};

/// `Core\Validate::isMac`'s reference card — `rule:core-api/reference-card`.
const IS_MAC_DOC: MethodDoc = MethodDoc {
    short: "Checks whether `$s` is a MAC address, the hardware address of a network card. \
            It is six pairs of hex digits joined by `:` or by `-`, such as \
            `00:1a:2b:3c:4d:5e`. It can also be three groups of four hex digits joined by `.`, \
            such as `001a.2b3c.4d5e`.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to test.",
        shape: &[],
    }],
    ret: "`true` for one of the three ways to write it, in upper or lower case. `false` \
          otherwise. A text that mixes two separators, such as `00:1a-2b:3c:4d:5e`, gives \
          `false`.",
    errors: &[],
};

/// `Core\Validate::isAscii`'s reference card — `rule:core-api/reference-card`.
const IS_ASCII_DOC: MethodDoc = MethodDoc {
    short: "Checks whether every character of `$s` is an ASCII character. ASCII has 128 \
            characters: the English letters, the digits, common punctuation and the control \
            characters. Each of them is one byte, from `0x00` to `0x7F`.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to test.",
        shape: &[],
    }],
    ret: "`true` when every character is ASCII, and `true` for the empty string. `false` when \
          the text contains any other character, such as `é` or `日`.",
    errors: &[],
};

/// `Core\Validate::isPrintable`'s reference card — `rule:core-api/reference-card`.
const IS_PRINTABLE_DOC: MethodDoc = MethodDoc {
    short: "Checks whether `$s` contains no control characters. A control character has no \
            visible form, for example a tab, a new line or the null byte. Letters from any \
            language, such as `é` or `日`, are printable.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to test.",
        shape: &[],
    }],
    ret: "`true` when the text has no control character, and `true` for the empty string. \
          `false` for a tab, a new line, a carriage return, `DEL` or any other control \
          character. Some invisible characters are not control characters, such as the \
          zero-width joiner. They give `true`, so this method does not find text that hides \
          what it really says.",
    errors: &[],
};

/// `Core\Validate::isIp`'s one option, and the closed pair it admits.
///
/// [`Const::Null`] rather than a version of its own, because "either family"
/// is not `4` and is not `6`: the omitted case is a third answer, and
/// [`Const::Null`] is already the registry's spelling for an option whose
/// declared type has no "absent" value in it.
const IP_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "version",
    ty: CoreTy::Union(IP_VERSION),
    default: Const::Null,
}];

/// `4|6` — `rule:types/single-value-types`'s integer single-value type, twice.
const IP_VERSION: &[CoreTy] = &[CoreTy::SingleValueInt(4), CoreTy::SingleValueInt(6)];

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_validate_is_email" => (nvs_core_validate_is_email as *const ()).cast(),
        "nvs_core_validate_is_domain" => (nvs_core_validate_is_domain as *const ()).cast(),
        "nvs_core_validate_is_ip" => (nvs_core_validate_is_ip as *const ()).cast(),
        "nvs_core_validate_is_mac" => (nvs_core_validate_is_mac as *const ()).cast(),
        "nvs_core_validate_is_ascii" => (nvs_core_validate_is_ascii as *const ()).cast(),
        "nvs_core_validate_is_printable" => (nvs_core_validate_is_printable as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Arguments
// ============================================================================

/// The subject, as text.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is not a `string`, which the checker
/// has already refused — this is the ABI's own assertion, not a
/// program-visible outcome. It is the only failure: the tag
/// [`Value::as_text`] checks is itself `rule:types/bytes`'s UTF-8 guarantee, so there is
/// no encoding outcome left to report.
fn subject<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Validate::{member} expected {:?} for the subject, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

/// `isIp`'s `{version}`: the family it names, or `None` for either.
///
/// # Errors
///
/// A [`Fault::fatal`] for anything but `4`, `6` or the `Tag::Null` an omitting
/// call site passes. The declared type is `4|6`, so a fourth value here means
/// the checker let one through rather than that a program asked for one.
fn family(value: &Value) -> Result<Option<u8>, Fault> {
    match value.tag() {
        Some(Tag::Null) => Ok(None),
        Some(Tag::Int) => match value.as_int() {
            Some(4) => Ok(Some(4)),
            Some(6) => Ok(Some(6)),
            // Unreachable from source: `IP_VERSION` declares the option as ADR
            // 0047 § 1's set of allowed values `4|6`, so any other integer is
            // `E0401: expected `4|6`, found `int`` at the option's own value —
            // a literal and a binding alike, since the union is checked at the
            // argument rather than folded.
            other => Err(Fault::fatal(format!(
                "Core\\Validate::isIp received {other:?} for `version`, which its declared `4|6` \
                 cannot be"
            ))),
        },
        // The same declaration refuses every other tag one step earlier:
        // `{version: "4"}` is `E0401: expected `4|6`, found `string``, so this
        // arm is unreachable from source and covers only the `Tag::Null` an
        // omitting call site does not take.
        _ => Err(Fault::fatal(format!(
            "Core\\Validate::isIp expected `4`, `6` or nothing for `version`, got tag {}",
            value.tag_byte()
        ))),
    }
}

// ============================================================================
// The rules — each one readable on its own, per this module's docs
// ============================================================================

/// RFC 5322 § 3.2.3's `atext`: an ASCII alphanumeric, or one of the nineteen
/// specials an unquoted local part may hold.
const fn is_atext(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'/'
                | b'='
                | b'?'
                | b'^'
                | b'_'
                | b'`'
                | b'{'
                | b'|'
                | b'}'
                | b'~'
        )
}

/// A dot-atom of at most `limit` bytes: atoms of `atext`, joined by single
/// dots, none of them empty.
fn is_dot_atom(text: &str, limit: usize) -> bool {
    !text.is_empty()
        && text.len() <= limit
        && text
            .split('.')
            .all(|atom| !atom.is_empty() && atom.bytes().all(is_atext))
}

/// RFC 1123 § 2.1's hostname — this module's docs own the rule and its one
/// divergence from PHP.
fn is_hostname(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 253
        && text.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

/// Whether `text` is an address: one `@`, a dot-atom local part, and a
/// hostname of at least two labels. This module's docs own the line.
fn is_email_address(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    text.len() <= 254 && is_dot_atom(local, 64) && is_hostname(domain) && domain.contains('.')
}

/// Whether `text` is one of the three MAC spellings — colon-, hyphen- or
/// dot-separated, with one separator throughout.
fn is_mac_address(text: &str) -> bool {
    for (separator, groups, width) in [(':', 6, 2), ('-', 6, 2), ('.', 3, 4)] {
        if !text.contains(separator) {
            continue;
        }
        let mut seen = 0_usize;
        let mut well_formed = true;
        for group in text.split(separator) {
            seen += 1;
            if group.len() != width || !group.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                well_formed = false;
            }
        }
        return well_formed && seen == groups;
    }
    false
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Validate::isEmail(string $s): bool` — replacing
    /// `filter_var($s, FILTER_VALIDATE_EMAIL)`.
    ///
    /// One `@`, a dot-atom local part, and a domain of at least two LDH
    /// labels. This module's own docs own the line, what falls outside it and
    /// why an RFC-complete reading would be the wider surface rather than the
    /// safer one.
    ///
    /// Never throws: a validator's answer to a malformed subject is `false`,
    /// which is the whole point of asking.
    fn nvs_core_validate_is_email(_ctx, args: [1]) {
        Ok(Value::bool(is_email_address(subject(&args[0], "isEmail")?)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Validate::isDomain(string $s): bool` — replacing
    /// `filter_var($s, FILTER_VALIDATE_DOMAIN, FILTER_FLAG_HOSTNAME)`.
    ///
    /// Syntax only, and never DNS. This module's docs own the label rule, the
    /// refusal of a trailing root dot, and why an all-numeric name is accepted
    /// here where PHP's email filter refuses it.
    ///
    /// Never throws.
    fn nvs_core_validate_is_domain(_ctx, args: [1]) {
        Ok(Value::bool(is_hostname(subject(&args[0], "isDomain")?)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Validate::isIp(string $s, {version?: 4|6}): bool` — replacing
    /// `filter_var($s, FILTER_VALIDATE_IP)` and its two family flags, which
    /// spec § 12 folds into this one option.
    ///
    /// [`std::net::IpAddr`]'s parser, so `::` compression, an embedded-IPv4
    /// tail and the leading-zero refusal are the standard library's rather
    /// than this module's.
    ///
    /// # Errors
    ///
    /// [`family`]'s, for a `version` the checker should already have refused.
    /// The subject itself never throws.
    fn nvs_core_validate_is_ip(_ctx, args: [2]) {
        let text = subject(&args[0], "isIp")?;
        Ok(Value::bool(match family(&args[1])? {
            None => text.parse::<std::net::IpAddr>().is_ok(),
            Some(4) => text.parse::<std::net::Ipv4Addr>().is_ok(),
            _ => text.parse::<std::net::Ipv6Addr>().is_ok(),
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Validate::isMac(string $s): bool` — replacing
    /// `filter_var($s, FILTER_VALIDATE_MAC)`, whose three accepted spellings
    /// this member keeps exactly.
    ///
    /// Never throws.
    fn nvs_core_validate_is_mac(_ctx, args: [1]) {
        Ok(Value::bool(is_mac_address(subject(&args[0], "isMac")?)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Validate::isAscii(string $s): bool` — the encoding question, and
    /// a byte scan with no decoding at all.
    ///
    /// This module's docs own why this and `isPrintable` are two questions
    /// rather than one asked twice, and why the empty string answers `true`
    /// here where every `ctype_*` answers `false`.
    ///
    /// Never throws.
    fn nvs_core_validate_is_ascii(_ctx, args: [1]) {
        Ok(Value::bool(subject(&args[0], "isAscii")?.is_ascii()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Validate::isPrintable(string $s): bool` — no `char` in Unicode
    /// general category `Cc`, which is [`char::is_control`] and is exactly the
    /// set with no printed form.
    ///
    /// **Not a spoofing check** — this module's docs own why a bidi override
    /// is printable and why refusing one here would be the false confidence
    /// `rule:security/tainted-qualifier` exists to prevent.
    ///
    /// Never throws.
    fn nvs_core_validate_is_printable(_ctx, args: [1]) {
        Ok(Value::bool(
            !subject(&args[0], "isPrintable")?.chars().any(char::is_control),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::CLASSES;

    /// `rule:security/launderers-are-sink-named`, asked of the rows rather than of the module doc above
    /// that argues it: **`Core\Validate` launders nothing**, because nothing
    /// in it answers its own subject back.
    ///
    /// The class is the one `rule:core-api/tier-roster` calls "*the* launderer" — the member
    /// list PHP's `filter` shrank to once its sanitizing half was dropped —
    /// so a blanket [`Qual::Launder`] over it is the plausible reading, and it
    /// is the false confidence `rule:security/tainted-qualifier` exists to prevent: a syntactically
    /// valid address is still a `tainted` one at every sink. What holds the
    /// rows to that is structural rather than a promise. All six answer a
    /// [`CoreTy::Bool`], and a verdict carries no byte of what it was asked
    /// about, so there is nothing here a qualifier could be removed *from*.
    ///
    /// Which *type* a launderer hands its subject back as is a separate
    /// question, and `rule:security/launderer-answers-a-carrier`'s — `Core\Html::escape` answers a carrier
    /// and `Core\Regex::quote` a plain `string`. `crate::html`'s
    /// `every_launderer_for_an_auto_escaping_sink_answers_a_carrier` is that
    /// claim's home; what matters here is only that the answer is the subject.
    ///
    /// The last assertion is the same claim asked of the whole registry, and
    /// is what makes this more than these six rows read back: **no laundering
    /// member anywhere answers a `bool`**. Every one of them hands its subject
    /// back — `Core\Regex::quote` as an unqualified `string`,
    /// `Core\Html::escape` as a carrier — because laundering is a
    /// transformation and validating is a question about the input. A
    /// validator that grew a `Qual::Launder` fails here on the day it is
    /// added, in whichever class it is added to.
    #[test]
    fn validate_launders_only_what_it_actually_validated() {
        // Counted, not read off six lines: a seventh member added without a
        // thought for its qualifier fails here rather than passing by absence.
        let members: Vec<&'static CoreMethod> = CLASS.members().collect();
        assert_eq!(
            members.len(),
            6,
            "spec § 12's prose roster is six validators"
        );

        for method in &members {
            assert!(
                matches!(method.return_ty, CoreTy::Bool),
                "`Core\\Validate::{}` answers a verdict, which is what makes \
                 [`Qual::Neutral`] the honest mark on its subject",
                method.name
            );
            for param in method.params {
                assert!(
                    !matches!(param, CoreTy::Text(Qual::Launder | Qual::Reveal)),
                    "`Core\\Validate::{}` removes no qualifier on either axis",
                    method.name
                );
                if let CoreTy::Text(qual) = param {
                    assert!(
                        matches!(qual, Qual::Neutral),
                        "`Core\\Validate::{}`'s text parameter is [`Qual::Neutral`], \
                         not {qual:?}",
                        method.name
                    );
                }
            }
        }

        let laundering_validators: Vec<String> = CLASSES
            .iter()
            .flat_map(|class| class.members().map(move |method| (class.name, method)))
            .filter(|(_, method)| {
                matches!(method.return_ty, CoreTy::Bool)
                    && method
                        .params
                        .iter()
                        .any(|param| matches!(param, CoreTy::Text(Qual::Launder)))
            })
            .map(|(class, method)| format!("{class}::{}", method.name))
            .collect();
        assert!(
            laundering_validators.is_empty(),
            "a launderer answers its subject back, not a verdict: {laundering_validators:?}"
        );
    }

    /// The line `isEmail` draws, from both sides — the accepted shapes and
    /// each refusal the module docs name.
    // covers: Core\Validate::isEmail
    #[test]
    fn an_email_is_a_dot_atom_at_a_multi_label_hostname() {
        for accepted in [
            "ada@example.test",
            "ada+list@mail.example.test",
            "a.b.c@x.y",
            "!#$%&'*+-/=?^_`{|}~@example.test",
            "postmaster@1.2.3.4",
        ] {
            assert!(
                is_email_address(accepted),
                "{accepted} should be an address"
            );
        }
        for refused in [
            "no-at-sign",
            "a..b@example.test",
            ".a@example.test",
            "a.@example.test",
            "a b@example.test",
            "\"a b\"@example.test",
            "a@[192.0.2.1]",
            "a@localhost",
            "a@-x.test",
            "a@x-.test",
            "a@exam_ple.test",
            "a@x..y",
            "a@example.test.",
            "a@b@example.test",
            "汉@例え.jp",
            "",
        ] {
            assert!(
                !is_email_address(refused),
                "{refused} should not be an address"
            );
        }
    }

    /// A 65-byte local part and a 255-byte subject are each one byte past
    /// their limit — the two bounds RFC 5321 sets and nothing in the byte
    /// grammar would catch.
    #[test]
    fn an_email_is_bounded_at_both_lengths() {
        let long_local = format!("{}@example.test", "a".repeat(64));
        assert!(is_email_address(&long_local));
        let over_local = format!("{}@example.test", "a".repeat(65));
        assert!(!is_email_address(&over_local));

        // 254 bytes exactly: a 64-byte local, `@`, and a 189-byte domain made
        // of labels no longer than 63.
        let domain = format!(
            "{}.{}.{}.te",
            "b".repeat(62),
            "c".repeat(62),
            "d".repeat(60)
        );
        let whole = format!("{}@{domain}", "a".repeat(64));
        assert_eq!(whole.len(), 254);
        assert!(is_email_address(&whole));
        assert!(!is_email_address(&format!("a{whole}")));
    }

    /// `isDomain` answers the hostname question, which is not `isEmail`'s.
    // covers: Core\Validate::isDomain
    #[test]
    fn a_domain_is_one_or_more_ldh_labels() {
        for accepted in [
            "localhost",
            "example.test",
            "x",
            "1.2.3.4",
            "a-b.example.test",
        ] {
            assert!(is_hostname(accepted), "{accepted} should be a domain");
        }
        for refused in [
            "",
            ".",
            "-x.test",
            "x-.test",
            "x..y",
            "example.test.",
            "a_b.test",
        ] {
            assert!(!is_hostname(refused), "{refused} should not be a domain");
        }
        assert!(is_hostname(&"a".repeat(63)));
        assert!(!is_hostname(&"a".repeat(64)));
    }

    /// The three spellings PHP accepts, and the mixed separator it does not.
    // covers: Core\Validate::isMac
    #[test]
    fn a_mac_takes_one_separator_throughout() {
        for accepted in [
            "00:11:22:33:44:55",
            "00-11-22-33-44-55",
            "0011.2233.4455",
            "AA:bb:CC:dd:EE:ff",
        ] {
            assert!(is_mac_address(accepted), "{accepted} should be a MAC");
        }
        for refused in [
            "00:11-22:33:44:55",
            "00:11:22:33:44",
            "0:1:2:3:4:5",
            "001122334455",
            "",
        ] {
            assert!(!is_mac_address(refused), "{refused} should not be a MAC");
        }
    }

    /// `isAscii` through the boundary compiled code reaches it at: `true`
    /// exactly where the subject's byte count equals its character count, at
    /// both edges of the one-byte range and wherever the other character sits.
    /// A subject that is not a `string` is the ABI's own fatal, not a verdict.
    // covers: Core\Validate::isAscii
    #[test]
    fn is_ascii_is_true_exactly_when_every_character_is_one_byte() {
        use nvs_runtime::{Ctx, NvsStr, OutputSink, call};

        let ask = |subject: Value| {
            let mut ctx = Ctx::new(OutputSink::Sink);
            call(nvs_core_validate_is_ascii, &mut ctx, &[subject])
        };
        for text in [
            "",
            "a",
            "\0",
            "\x7f",
            "~",
            "Order 1042",
            "\u{80}",
            "é",
            "aé",
            "éa",
            "a日b",
            "🙂",
        ] {
            let subject = Value::str(NvsStr::new(text.as_bytes()));
            let answer = ask(subject).expect("`isAscii` answers a `string` without throwing");
            assert_eq!(
                answer.as_bool(),
                Some(text.len() == text.chars().count()),
                "`isAscii({text:?})`"
            );
            #[expect(
                unsafe_code,
                reason = "this test owns the string it built; the helper borrowed its argument \
                          and answered a scalar"
            )]
            unsafe {
                subject.release();
            }
        }
        assert!(
            ask(Value::int(7)).is_err(),
            "a non-`string` subject is refused"
        );
    }

    /// `isIp` through the boundary compiled code reaches it at, under each of
    /// the three `version` answers: omitted takes either family, `4` and `6`
    /// take exactly theirs. The leading-zero octet, the zone identifier and an
    /// octet one past 255 are refused under all three, and a `version` the
    /// checker cannot let through is the ABI's own fatal.
    // covers: Core\Validate::isIp
    #[test]
    fn is_ip_takes_the_family_its_version_names_and_nothing_malformed() {
        use nvs_runtime::{Ctx, NvsStr, OutputSink, call};

        let ask = |text: &str, version: Value| {
            let subject = Value::str(NvsStr::new(text.as_bytes()));
            let mut ctx = Ctx::new(OutputSink::Sink);
            let answer = call(nvs_core_validate_is_ip, &mut ctx, &[subject, version]);
            #[expect(
                unsafe_code,
                reason = "this Rust closure owns the string it built; the helper borrowed its \
                          argument and answered a scalar"
            )]
            unsafe {
                subject.release();
            }
            answer
        };
        // (subject, either, only 4, only 6)
        for (text, either, v4, v6) in [
            ("192.0.2.1", true, true, false),
            ("0.0.0.0", true, true, false),
            ("255.255.255.255", true, true, false),
            ("2001:db8::1", true, false, true),
            ("::", true, false, true),
            ("::ffff:192.0.2.1", true, false, true),
            ("256.0.0.1", false, false, false),
            ("192.0.2.01", false, false, false),
            ("192.0.2", false, false, false),
            (" 192.0.2.1", false, false, false),
            ("fe80::1%eth0", false, false, false),
            ("2001:db8::1::2", false, false, false),
            ("", false, false, false),
        ] {
            for (version, want) in [
                (Value::null(), either),
                (Value::int(4), v4),
                (Value::int(6), v6),
            ] {
                let answer = ask(text, version)
                    .expect("`isIp` answers a `string` and a declared version without throwing");
                assert_eq!(
                    answer.as_bool(),
                    Some(want),
                    "`isIp({text:?})` with version {:?}",
                    version.as_int()
                );
            }
        }
        assert!(
            ask("192.0.2.1", Value::int(5)).is_err(),
            "a version outside `4|6` is refused"
        );
    }

    /// `isPrintable` through the boundary compiled code reaches it at: every
    /// C0 control, `DEL` and the C1 range are refused wherever they sit, and
    /// a character outside category `Cc` is printable even when it has no
    /// width — the zero-width joiner and a bidi override are the two the
    /// module docs name, because this is not a spoofing check.
    // covers: Core\Validate::isPrintable
    #[test]
    fn is_printable_is_false_exactly_where_a_control_character_sits() {
        use nvs_runtime::{Ctx, NvsStr, OutputSink, call};

        let ask = |subject: Value| {
            let mut ctx = Ctx::new(OutputSink::Sink);
            call(nvs_core_validate_is_printable, &mut ctx, &[subject])
        };
        for (text, want) in [
            ("", true),
            ("Order 1042", true),
            ("café", true),
            ("日本語", true),
            ("🙂", true),
            ("~", true),
            ("\u{a0}", true),
            ("\u{200d}", true),
            ("\u{202e}", true),
            ("\0", false),
            ("\t", false),
            ("\n", false),
            ("\r", false),
            ("\x1f", false),
            ("\x7f", false),
            ("\u{80}", false),
            ("\u{9f}", false),
            ("a\nb", false),
            ("abc\t", false),
            ("\x1bcafé", false),
        ] {
            let subject = Value::str(NvsStr::new(text.as_bytes()));
            let answer = ask(subject).expect("`isPrintable` answers a `string` without throwing");
            assert_eq!(answer.as_bool(), Some(want), "`isPrintable({text:?})`");
            #[expect(
                unsafe_code,
                reason = "this test owns the string it built; the helper borrowed its argument \
                          and answered a scalar"
            )]
            unsafe {
                subject.release();
            }
        }
        assert!(
            ask(Value::int(7)).is_err(),
            "a non-`string` subject is refused"
        );
    }
}
