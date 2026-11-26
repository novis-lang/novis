//! The message `DATA` carries: every header [`super`] writes, and the body.
//!
//! [`compose`] is the only function in `Core\Mail` that writes a `\r\n`. Every value that reaches it
//! has been checked by [`address_of`] or encoded by [`subject_line`], so no argument can add a
//! header line. The module doc of [`super`] is the reasoning.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use nvs_runtime::Fault;

/// The printable bytes [`address_of`] refuses, beside every control byte.
///
/// Each one changes how a mail reader or a relay reads an address list or a
/// path: `<` and `>` end the address, `,` and `;` start another one or a group,
/// `:` opens a group (`list: a@b.c;`), `(` and `)` open a comment a reader
/// skips, `"` opens a quoted local part that may hold any of these, `\` escapes
/// inside one, `%` is the source route a relay may forward on
/// (`victim%evil.example@relay.example`), and a space ends the path on an
/// `RCPT TO:` line. A real mailbox needs none of them.
const NOT_IN_AN_ADDRESS: &[u8] = b"<>,;:()\"\\% ";

/// The longest address [`address_of`] takes: RFC 5321 § 4.5.3.1.3's path is
/// 256 octets with its angle brackets. It is also what keeps every address a
/// header line can fold around under the 998-octet line limit.
const LONGEST_ADDRESS: usize = 254;

/// `text` as an addr-spec, or a refusal naming `what` it was.
///
/// The check is deliberately narrower than RFC 5321's grammar and deliberately
/// not a repair. What it establishes is the one property the rest of this module
/// relies on — that the value can be written inside `<...>` on a command line
/// and inside a header without changing either one's structure — so a byte that
/// would end a line, a byte [`NOT_IN_AN_ADDRESS`] names, an address longer than
/// [`LONGEST_ADDRESS`] and the absence of an `@` are each a refusal.
/// `rule:errors/ambiguous-input-refused` is why none of them is silently stripped:
/// an address a program did not mean is a message going somewhere it did not
/// mean, and the caller is the only one who can say which.
///
/// # Errors
///
/// A catchable `RuntimeError` naming the value and what was wrong with it.
pub(super) fn address_of(text: &str, what: &str, member: &str) -> Result<String, Fault> {
    let refuse = |why: &str| {
        Err(Fault::thrown(format!(
            "{member}: {what} is not an address — {why}"
        )))
    };
    if text.is_empty() {
        return refuse("it is empty");
    }
    if text.len() > LONGEST_ADDRESS {
        return refuse("it is longer than 254 bytes");
    }
    if text
        .bytes()
        .any(|byte| byte < 0x20 || byte == 0x7f || NOT_IN_AN_ADDRESS.contains(&byte))
    {
        return refuse(
            "it contains a character an address may not contain: a control character, a \
             space, or one of `< > , ; : ( ) \" \\ %`",
        );
    }
    if !text.is_ascii() {
        return refuse(
            "it is not ASCII, and internationalized addresses need the SMTPUTF8 extension this \
             transport does not negotiate",
        );
    }
    match text.split_once('@') {
        Some((local, domain)) if !local.is_empty() && domain.contains('.') => Ok(text.to_owned()),
        _ => refuse("it is not `local@domain`"),
    }
}

/// The column [`folded`] breaks a header line at: RFC 5322 § 2.1.1's
/// recommended 78 characters before the `\r\n`.
const LINE: usize = 78;

/// The column no header line may pass: RFC 5322 § 2.1.1's 998 characters
/// before the `\r\n`. A server refuses a message with a longer line.
const LONGEST_LINE: usize = 998;

/// The UTF-8 bytes one RFC 2047 encoded word carries. 42 bytes are 56 base64
/// characters, so a word with its `=?UTF-8?B?` and `?=` is 68, and it fits
/// after `Subject: ` under [`LINE`]. A word may not exceed 75 characters.
const WORD_BYTES: usize = 42;

/// `Subject: ` and `text`, as header lines ending `\r\n`: itself, folded at
/// its spaces, when a header line can carry it, and RFC 2047 `=?UTF-8?B?...?=`
/// encoded words, one per continuation line, when it cannot.
///
/// This is the function the module doc's first section is about. A value that
/// reaches here holding a `\r\n` does not get it stripped and does not get it
/// through — it comes out base64, which is one header token by construction, so
/// the injected line arrives as characters a reader sees rather than as
/// structure a parser obeys. The only line breaks in what this returns are
/// followed by a space, which a reader unfolds into the value.
///
/// A long subject is folded rather than sent as one line, because a line over
/// [`LONGEST_LINE`] is a message the server refuses. A plain one with a word
/// too long to fold is encoded, since encoded words fold anywhere. What it
/// spends is the folded copy, about a third larger than the subject when it
/// is encoded, released with the message.
fn subject_line(text: &str) -> String {
    const NAME: &str = "Subject: ";
    let plain = text.is_ascii() && text.bytes().all(|byte| byte >= 0x20 && byte != 0x7f);
    if plain && let Some(folded) = folded(NAME.len(), text) {
        return format!("{NAME}{folded}\r\n");
    }
    let mut out = String::with_capacity(NAME.len() + text.len() / 3 * 5 + 16);
    out.push_str(NAME);
    let mut rest = text;
    loop {
        let mut end = rest.len().min(WORD_BYTES);
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        let (word, after) = rest.split_at(end);
        out.push_str("=?UTF-8?B?");
        out.push_str(&STANDARD.encode(word.as_bytes()));
        out.push_str("?=\r\n");
        rest = after;
        if rest.is_empty() {
            return out;
        }
        out.push(' ');
    }
}

/// `text`, already a valid header value, with `\r\n` put before a space
/// wherever the line would pass [`LINE`], starting at column `start`; `None`
/// when a run with no space in it would pass [`LONGEST_LINE`] anyway.
///
/// A line is never broken where it holds only spaces, since a folded line of
/// white space alone is one RFC 5322 § 3.2.2 forbids.
fn folded(start: usize, text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len() + text.len() / LINE * 2);
    let mut column = start;
    let mut bare = true;
    let mut rest = text;
    while !rest.is_empty() {
        let end = rest[1..].find(' ').map_or(rest.len(), |at| at + 1);
        let (piece, after) = rest.split_at(end);
        if !bare && piece.starts_with(' ') && column + piece.len() > LINE {
            out.push_str("\r\n");
            column = 0;
            bare = true;
        }
        if column + piece.len() > LONGEST_LINE {
            return None;
        }
        out.push_str(piece);
        column += piece.len();
        bare &= piece.bytes().all(|byte| byte == b' ');
        rest = after;
    }
    Some(out)
}

/// `name: ` and every address in angle brackets, comma-separated and folded,
/// as header lines ending `\r\n`.
///
/// Each address came through [`address_of`], so it has no space and is at most
/// [`LONGEST_ADDRESS`] long, and [`folded`] always finds a place to break.
fn address_line(name: &str, addresses: &[String]) -> String {
    let list = addresses
        .iter()
        .map(|address| format!("<{address}>"))
        .collect::<Vec<_>>()
        .join(", ");
    let folded = folded(name.len() + 2, &list).unwrap_or(list);
    format!("{name}: {folded}\r\n")
}

/// `bytes` as base64, wrapped at 76 columns with CRLF, which is the transfer
/// encoding both body parts use.
///
/// Base64 for every body rather than quoted-printable for some of them: it is
/// one path instead of two, it cannot produce a line that is a lone `.` — so
/// there is no dot-stuffing rule for a later reader to get wrong — and it holds
/// UTF-8 without a second question about which bytes needed escaping.
fn body_encoded(bytes: &[u8]) -> String {
    let raw = STANDARD.encode(bytes);
    let mut out = String::with_capacity(raw.len() + raw.len() / 76 * 2 + 2);
    for (index, chunk) in raw.as_bytes().chunks(76).enumerate() {
        if index > 0 {
            out.push_str("\r\n");
        }
        out.push_str(std::str::from_utf8(chunk).unwrap_or_default());
    }
    out.push_str("\r\n");
    out
}

/// `now` as RFC 5322 writes a date, in UTC.
///
/// Written here rather than reached for through [`crate::time`]: what a header
/// needs is one fixed English spelling with a `+0000` offset, which is not the
/// locale-shaped, zone-shaped question that module answers, and a `DateTime`
/// threaded through this module for one line would be the larger coupling.
fn rfc5322_date(now: SystemTime) -> String {
    let secs =
        i64::try_from(now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()).unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    let (hour, minute, second) = (rest / 3_600, (rest % 3_600) / 60, rest % 60);

    // Howard Hinnant's civil_from_days, which is the shortest correct spelling
    // of a proleptic Gregorian conversion and has no table to get wrong.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let doe = shifted.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };

    const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let weekday = WEEKDAYS[usize::try_from(days.rem_euclid(7)).unwrap_or(0)];
    let month_name = MONTHS[usize::try_from(month - 1).unwrap_or(0)];

    format!("{weekday}, {day:02} {month_name} {year} {hour:02}:{minute:02}:{second:02} +0000")
}

/// The message `DATA` carries — every header this class writes, then the body.
///
/// The **only** place a `\r\n` is produced. Every value reaching it has been
/// through [`subject_line`] or [`address_of`], so there is no path from an
/// argument to a line break here, which is the property the module doc's first
/// section names.
pub(super) fn compose(
    from: &str,
    to: &[String],
    cc: &[String],
    subject: &str,
    text: &str,
    html: Option<&str>,
    now: SystemTime,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("Date: {}\r\n", rfc5322_date(now)));
    out.push_str(&format!("From: <{from}>\r\n"));
    out.push_str(&address_line("To", to));
    if !cc.is_empty() {
        out.push_str(&address_line("Cc", cc));
    }
    out.push_str(&subject_line(subject));
    out.push_str("MIME-Version: 1.0\r\n");

    match html {
        None => {
            out.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
            out.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
            out.push_str(&body_encoded(text.as_bytes()));
        }
        Some(html) => {
            // A fixed boundary is safe because neither part is ever transmitted
            // raw: both are base64, whose alphabet cannot contain it.
            const BOUNDARY: &str = "nvs-alternative-boundary";
            out.push_str(&format!(
                "Content-Type: multipart/alternative; boundary=\"{BOUNDARY}\"\r\n\r\n"
            ));
            out.push_str(&format!("--{BOUNDARY}\r\n"));
            out.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
            out.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
            out.push_str(&body_encoded(text.as_bytes()));
            out.push_str(&format!("\r\n--{BOUNDARY}\r\n"));
            out.push_str("Content-Type: text/html; charset=UTF-8\r\n");
            out.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
            out.push_str(&body_encoded(html.as_bytes()));
            out.push_str(&format!("\r\n--{BOUNDARY}--\r\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// The module doc's first section, as the property it claims: no value of
    /// an argument can add a header, because the encoder answers a header line
    /// with one token whenever the value could not be one.
    // covers: Core\Mail::send
    #[test]
    fn a_header_value_cannot_carry_a_line_break_through() {
        assert_eq!(subject_line("Your receipt"), "Subject: Your receipt\r\n");
        let injected = subject_line("Hi\r\nBcc: attacker@example.com");
        assert!(
            injected.starts_with("Subject: =?UTF-8?B?") && only_folded(&injected),
            "an injected header survived encoding: {injected}"
        );
    }

    /// Every `\r\n` in `lines` but the last is followed by a space, so a reader
    /// unfolds it into the value and no new header starts.
    fn only_folded(lines: &str) -> bool {
        let body = lines.strip_suffix("\r\n").unwrap_or(lines);
        let mut lines = body.split("\r\n");
        lines.all(|line| !line.contains(['\r', '\n']))
            && body.split("\r\n").skip(1).all(|line| line.starts_with(' '))
    }

    /// A long subject is folded under 78 columns, plain at its spaces and
    /// encoded in words of whole characters, and each unfolds to the subject.
    /// On main it was one line, and past 998 bytes the server refused it.
    // covers: Core\Mail::send
    #[test]
    fn a_long_subject_is_folded_and_unfolds_to_itself() {
        // The two subjects with spaces are each many times `LINE`, so each
        // folds many times. The unbroken one is 1,200 bytes, past the 998 a
        // server takes on one line.
        let plain = "word ".repeat(100);
        let encoded = "é\r\nBcc: x@example.com ".repeat(75);
        let unbroken = "x".repeat(1200);
        for subject in [plain.as_str(), encoded.as_str(), unbroken.as_str()] {
            let line = subject_line(subject);
            assert!(only_folded(&line), "a line break was not a fold: {line:?}");
            for each in line.split("\r\n") {
                assert!(
                    each.len() <= LINE,
                    "a line of {} columns: {each:?}",
                    each.len()
                );
            }
            let unfolded = line
                .strip_prefix("Subject: ")
                .and_then(|rest| rest.strip_suffix("\r\n"))
                .unwrap_or_default()
                .replace("\r\n", "");
            let read = if unfolded.starts_with("=?UTF-8?B?") {
                let mut bytes = Vec::new();
                for word in unfolded.split(' ') {
                    let inner = word
                        .strip_prefix("=?UTF-8?B?")
                        .and_then(|w| w.strip_suffix("?="));
                    let decoded = STANDARD
                        .decode(inner.unwrap_or_default())
                        .unwrap_or_default();
                    assert!(
                        std::str::from_utf8(&decoded).is_ok(),
                        "a word split a character"
                    );
                    bytes.extend(decoded);
                }
                String::from_utf8(bytes).unwrap_or_default()
            } else {
                unfolded
            };
            assert_eq!(read, subject);
        }
    }

    /// Every recipient is written in angle brackets, and a long list is folded
    /// between addresses. On main `To:` was a bare comma-separated line.
    // covers: Core\Mail::send
    #[test]
    fn recipients_are_in_angle_brackets_and_a_long_list_is_folded() {
        // Unfolded, this many addresses make one line past `LONGEST_LINE`.
        let count = LONGEST_LINE / "<user0@example.com>, ".len() + 1;
        let many: Vec<String> = (0..count).map(|n| format!("user{n}@example.com")).collect();
        let unfolded = many
            .iter()
            .map(|address| format!("<{address}>"))
            .collect::<Vec<_>>()
            .join(", ")
            .len();
        assert!(unfolded > LONGEST_LINE, "the list is {unfolded} bytes");
        let line = address_line("To", &many);
        assert!(line.starts_with("To: <user0@example.com>, <user1@example.com>,"));
        assert!(only_folded(&line), "a line break was not a fold: {line:?}");
        assert!(line.split("\r\n").all(|each| each.len() <= LINE));
        assert_eq!(line.matches('<').count(), count);
    }

    /// `rule:errors/ambiguous-input-refused`, on the parameter where repair is most tempting: an address
    /// with a line break in it is refused rather than trimmed.
    #[test]
    fn an_address_is_refused_rather_than_repaired() {
        let member = "Core\\Mail::send";
        assert!(address_of("user@example.com", "to", member).is_ok());
        assert!(address_of("first.last+tag@mail.example.com", "to", member).is_ok());
        let too_long = format!("{}@example.com", "a".repeat(250));
        for bad in [
            "user@example.com\r\nRCPT TO:<victim@example.com>",
            "a@b.c, d@e.f",
            "<user@example.com>",
            "userexample.com",
            "",
            "(comment)user@example.com",
            "\"a@b.c\"@example.com",
            "list:user@example.com",
            "a@b.c;d@example.com",
            "victim%evil.example@relay.example.com",
            "user@example.com NOTIFY=NEVER",
            too_long.as_str(),
        ] {
            assert!(
                address_of(bad, "to", member).is_err(),
                "`{bad}` was accepted as an address"
            );
        }
    }

    /// The one date this module writes, against a value with a known answer.
    #[test]
    fn the_date_header_is_rfc_5322() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_756_684_800);
        assert_eq!(rfc5322_date(epoch), "Mon, 01 Sep 2025 00:00:00 +0000");
        assert_eq!(rfc5322_date(UNIX_EPOCH), "Thu, 01 Jan 1970 00:00:00 +0000");
    }

    /// A `bcc` recipient reaches the envelope and no header — asserted over the
    /// composed message, because that is the only place the difference shows.
    // covers: Core\Mail::send
    #[test]
    fn a_bcc_recipient_is_in_no_header() {
        let message = compose(
            "noreply@example.com",
            &["to@example.com".to_owned()],
            &["cc@example.com".to_owned()],
            "Hello",
            "body",
            None,
            UNIX_EPOCH,
        );
        assert!(message.contains("To: <to@example.com>\r\n"));
        assert!(message.contains("Cc: <cc@example.com>\r\n"));
        assert!(!message.contains("bcc@example.com"));
    }
}
