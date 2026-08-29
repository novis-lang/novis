//! `Core\Validate` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 12's prose roster, "what survives of `filter`: the genuine validators
//! only" — all six of it. Four name a *format* a human wrote (`isEmail`,
//! `isDomain`, `isIp`, `isMac`) and two ask what a `string` is made of
//! (`isAscii`, `isPrintable`). Every one answers `bool`, takes
//! the subject first, and **launders nothing** — that is the whole reason the
//! sanitizing half of `filter_var` is not here, and
//! [ADR 0024](../../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)
//! is why: a member that half-escapes produces exactly the false confidence
//! that ADR exists to prevent, so no member in this module ever returns its
//! subject.
//!
//! # No dependency, and that is the decision rather than the default
//!
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) § 4's first
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
//! latency budget, which is priority 3 traded away for nothing.
//!
//! # `isIp(string $s, {version?: 4|6})`
//!
//! Omitting `version` accepts either family; `{version: 4}` and
//! `{version: 6}` accept exactly one. The option is
//! [ADR 0047](../../../../docs/adr/0047-literal-and-enum-case-types.md)'s
//! literal union rather than an `int`, so `{version: 5}` does not compile —
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
//! ([ADR 0009](../../../../docs/adr/0009-string-and-bytes.md) § 1), so a
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
//! confidence ADR 0024 refuses — escaping at the sink is what makes a string
//! safe to display, and no member in this module launders anything.

use nvs_runtime::{Fault, Tag, Value};

use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};

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
    methods: &[
        CoreMethod {
            name: "isEmail",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_email",
        },
        CoreMethod {
            name: "isDomain",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_domain",
        },
        CoreMethod {
            name: "isIp",
            params: &[CoreTy::Str, CoreTy::Options(IP_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_ip",
        },
        CoreMethod {
            name: "isMac",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_mac",
        },
        CoreMethod {
            name: "isAscii",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_ascii",
        },
        CoreMethod {
            name: "isPrintable",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_validate_is_printable",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
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

/// `4|6` — ADR 0047 § 1's integer literal type, twice.
const IP_VERSION: &[CoreTy] = &[CoreTy::IntLiteral(4), CoreTy::IntLiteral(6)];

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
/// [`Value::as_text`] checks is itself ADR 0009's UTF-8 guarantee, so there is
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
            // 0047 § 1's literal union `4|6`, so any other integer is
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
    /// ADR 0024 exists to prevent.
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

    /// The line `isEmail` draws, from both sides — the accepted shapes and
    /// each refusal the module docs name.
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
}
