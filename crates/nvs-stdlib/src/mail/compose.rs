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

/// `text` as the addr-spec this module writes, or a refusal naming `what` it was.
///
/// The check is deliberately narrower than RFC 5321's grammar and deliberately
/// not a repair. What it establishes is the one property the rest of this module
/// relies on — that the value can be written inside `<...>` on a command line
/// and inside a header without changing either one's structure — so a character
/// that would end a line, a byte [`NOT_IN_AN_ADDRESS`] names, an address longer
/// than [`LONGEST_ADDRESS`] and the absence of an `@` are each a refusal.
/// `rule:errors/ambiguous-input-refused` is why none of them is silently stripped:
/// an address a program did not mean is a message going somewhere it did not
/// mean, and the caller is the only one who can say which.
///
/// A domain that is not ASCII is written as its punycode (`xn--`) form, by UTS 46
/// with the STD3 rules a host name follows, and one that has no such form is a
/// refusal. Every reader resolves the two spellings to one host, so this is the
/// one conversion that is not a repair. A local part that is not ASCII is kept
/// as written: only the server can carry it, with `SMTPUTF8`, and
/// [`super::smtp`] refuses the send when it is not advertised.
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
    if text
        .chars()
        .any(|c| c.is_control() || u8::try_from(c).is_ok_and(|b| NOT_IN_AN_ADDRESS.contains(&b)))
    {
        return refuse(
            "it contains a character an address may not contain: a control character, a \
             space, or one of `< > , ; : ( ) \" \\ %`",
        );
    }
    let Some((local, domain)) = text.rsplit_once('@') else {
        return refuse("it is not `local@domain`");
    };
    if local.is_empty() || local.contains('@') {
        return refuse("it is not `local@domain`");
    }
    let domain = if domain.is_ascii() {
        domain.to_owned()
    } else {
        match idna::domain_to_ascii_strict(domain) {
            Ok(ascii) => ascii,
            Err(_) => return refuse("its domain is not a valid international domain name"),
        }
    };
    if !domain.contains('.') {
        return refuse("it is not `local@domain`");
    }
    let address = format!("{local}@{domain}");
    if address.len() > LONGEST_ADDRESS {
        return refuse("it is longer than 254 bytes");
    }
    Ok(address)
}

/// One recipient: the address [`address_of`] returned, and the display name written before it.
pub(super) struct Mailbox {
    /// The address, through [`address_of`].
    pub address: String,
    /// The name, any text. [`mailbox`] quotes or encodes it. `None` and an empty name are no name.
    pub name: Option<String>,
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
    format!("{NAME}{}\r\n", encoded_words(text))
}

/// `text` as RFC 2047 `=?UTF-8?B?...?=` encoded words of [`WORD_BYTES`] each, joined by a fold.
///
/// A word never splits a character, and each one is a single header token, so nothing in `text`
/// can end the header line. The result has no `\r\n` at its end.
fn encoded_words(text: &str) -> String {
    let mut out = String::with_capacity(text.len() / 3 * 5 + 16);
    let mut rest = text;
    loop {
        let mut end = rest.len().min(WORD_BYTES);
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        let (word, after) = rest.split_at(end);
        out.push_str("=?UTF-8?B?");
        out.push_str(&STANDARD.encode(word.as_bytes()));
        out.push_str("?=");
        rest = after;
        if rest.is_empty() {
            return out;
        }
        out.push_str("\r\n ");
    }
}

/// `address` in angle brackets, with `display` before it when there is one, as one header value
/// with no line break. [`address_line`] folds it at its spaces.
///
/// A display name of printable ASCII with no `"`, no `\` and no word longer than [`LINE`] is
/// written as a quoted string. Any other name is written as [`encoded_words`], so a quote, a line
/// break or a non-ASCII letter in it is content and never structure. An empty name is no name.
fn mailbox(display: Option<&str>, address: &str) -> String {
    let Some(display) = display.filter(|display| !display.is_empty()) else {
        return format!("<{address}>");
    };
    let quotable = display
        .bytes()
        .all(|byte| (0x20..0x7f).contains(&byte) && byte != b'"' && byte != b'\\')
        && display.split(' ').all(|word| word.len() < LINE);
    if quotable {
        return format!("\"{display}\" <{address}>");
    }
    format!(
        "{} <{address}>",
        encoded_words(display).replace("\r\n ", " ")
    )
}

/// The bytes [`message_id_of`] refuses in a message id, beside every control byte and every byte
/// that is not ASCII. Each one ends the id, starts a comment or a list, or quotes, in a header a
/// mail reader parses.
const NOT_IN_A_MESSAGE_ID: &[u8] = b"<>()[]\",;:\\ ";

/// The longest message id [`message_id_of`] takes, so that `In-Reply-To: <id>` fits on one line
/// under [`LONGEST_LINE`].
const LONGEST_MESSAGE_ID: usize = 250;

/// `text` as a message id for `In-Reply-To` or `References`: `local@domain`, written without its
/// angle brackets, as [`super`]'s `send` returns it.
///
/// # Errors
///
/// A catchable `RuntimeError` naming `what` for an empty id, one longer than
/// [`LONGEST_MESSAGE_ID`], one with a byte [`NOT_IN_A_MESSAGE_ID`] names or a byte outside
/// printable ASCII, and one that is not `local@domain` with a single `@`.
pub(super) fn message_id_of(text: &str, what: &str, member: &str) -> Result<String, Fault> {
    let refuse = |why: &str| {
        Err(Fault::thrown(format!(
            "{member}: `{what}` takes message ids, and `{text}` is not one. {why} Pass the id \
             `send` returned, without angle brackets."
        )))
    };
    if text.is_empty() || text.len() > LONGEST_MESSAGE_ID {
        return refuse("A message id has 1 to 250 characters.");
    }
    if text
        .bytes()
        .any(|byte| !(0x21..0x7f).contains(&byte) || NOT_IN_A_MESSAGE_ID.contains(&byte))
    {
        return refuse(
            "It contains a space, a control character, a character that is not ASCII or one of \
             `< > ( ) [ ] \" , ; : \\`.",
        );
    }
    match text.split_once('@') {
        Some((local, domain))
            if !local.is_empty() && !domain.is_empty() && !domain.contains('@') =>
        {
            Ok(text.to_owned())
        }
        _ => refuse("A message id is `local@domain`."),
    }
}

/// The longest `unsubscribe` URL [`unsubscribe_of`] takes, so that `List-Unsubscribe: <url>` fits
/// on one line under [`LONGEST_LINE`]. A URL cannot be folded.
const LONGEST_UNSUBSCRIBE: usize = LONGEST_LINE - "List-Unsubscribe: <>".len();

/// `text` as the one-click unsubscribe URL of RFC 8058.
///
/// # Errors
///
/// A catchable `RuntimeError` for a URL that does not start with `https://` and name a host, that
/// contains a space, a control character, `<`, `>` or a character that is not ASCII, or that is
/// longer than [`LONGEST_UNSUBSCRIBE`].
pub(super) fn unsubscribe_of(text: &str, member: &str) -> Result<String, Fault> {
    let refuse = |why: &str| {
        Err(Fault::thrown(format!(
            "{member}: `unsubscribe` is not a one-click unsubscribe URL. {why}"
        )))
    };
    let Some(rest) = text.strip_prefix("https://") else {
        return refuse("It must start with `https://`. Mail programs send the click as a POST.");
    };
    if rest.is_empty() || rest.starts_with('/') {
        return refuse("It names no host.");
    }
    if text.len() > LONGEST_UNSUBSCRIBE {
        return refuse("It is longer than 976 characters.");
    }
    if text
        .bytes()
        .any(|byte| !(0x21..0x7f).contains(&byte) || byte == b'<' || byte == b'>')
    {
        return refuse(
            "It contains a space, a control character, `<`, `>` or a character that is not \
             ASCII. Percent-encode it.",
        );
    }
    Ok(text.to_owned())
}

/// The id [`Headers::message_id`] is built from: `random` as 32 lowercase hex digits, `@`, and
/// the domain of the block's `from`.
///
/// The domain is the operator's, so a reply or a bounce names the deployment that sent the
/// message. The 128 random bits are what make the id unique without a counter.
pub(super) fn message_id(random: [u8; 16], from: &str) -> String {
    let domain = from
        .rsplit_once('@')
        .map_or("localhost", |(_, domain)| domain);
    let mut out = String::with_capacity(32 + 1 + domain.len());
    for byte in random {
        out.push_str(&format!("{byte:02x}"));
    }
    out.push('@');
    out.push_str(domain);
    out
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

/// `name: ` and every mailbox, written by [`mailbox`], comma-separated and folded,
/// as header lines ending `\r\n`.
///
/// Each address came through [`address_of`], so it has no space and is at most
/// [`LONGEST_ADDRESS`] long. Each word of a name is shorter than [`LINE`] or an
/// encoded word, so [`folded`] always finds a place to break.
fn address_line(name: &str, mailboxes: &[Mailbox]) -> String {
    let list = mailboxes
        .iter()
        .map(|each| mailbox(each.name.as_deref(), &each.address))
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

/// Every header value one message has, each already checked by the function that read it.
pub(super) struct Headers<'a> {
    /// The block's `from`, through [`address_of`].
    pub from: &'a str,
    /// The display name for `from`: the call's `fromName`, else the block's `from_name`.
    pub from_name: Option<&'a str>,
    /// The `to` recipients.
    pub to: &'a [Mailbox],
    /// The `cc` recipients.
    pub cc: &'a [Mailbox],
    /// The `replyTo` mailbox.
    pub reply_to: Option<&'a Mailbox>,
    /// The subject, any text. [`subject_line`] encodes it.
    pub subject: &'a str,
    /// What [`message_id`] built.
    pub message_id: &'a str,
    /// The `inReplyTo` id, through [`message_id_of`].
    pub in_reply_to: Option<&'a str>,
    /// The `references` ids, through [`message_id_of`].
    pub references: &'a [String],
    /// The `unsubscribe` URL, through [`unsubscribe_of`].
    pub unsubscribe: Option<&'a str>,
    /// Whether `Auto-Submitted: auto-generated` is written.
    pub auto_submitted: bool,
    /// The `Date:` header's time.
    pub now: SystemTime,
}

/// The message `DATA` carries — every header this class writes, then the body.
///
/// The **only** place a `\r\n` is produced. Every value reaching it has been
/// through [`subject_line`], [`mailbox`], [`address_of`], [`message_id_of`] or
/// [`unsubscribe_of`], so there is no path from an argument to a line break here,
/// which is the property the module doc's first section names.
///
/// `Auto-Submitted: auto-generated` (RFC 3834) tells an out-of-office responder not to answer.
/// It is on unless the program turns it off for mail a person wrote.
pub(super) fn compose(headers: &Headers<'_>, text: &str, html: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str(&format!("Date: {}\r\n", rfc5322_date(headers.now)));
    out.push_str(&format!("Message-ID: <{}>\r\n", headers.message_id));
    let from = Mailbox {
        address: headers.from.to_owned(),
        name: headers.from_name.map(str::to_owned),
    };
    out.push_str(&address_line("From", std::slice::from_ref(&from)));
    out.push_str(&address_line("To", headers.to));
    if !headers.cc.is_empty() {
        out.push_str(&address_line("Cc", headers.cc));
    }
    if let Some(reply_to) = headers.reply_to {
        out.push_str(&address_line("Reply-To", std::slice::from_ref(reply_to)));
    }
    out.push_str(&subject_line(headers.subject));
    if let Some(id) = headers.in_reply_to {
        out.push_str(&format!("In-Reply-To: <{id}>\r\n"));
    }
    if !headers.references.is_empty() {
        // Each id is at most `LONGEST_MESSAGE_ID` long and has no space, so `folded` always
        // finds a place to break between two of them.
        let list = headers
            .references
            .iter()
            .map(|id| format!("<{id}>"))
            .collect::<Vec<_>>()
            .join(" ");
        let folded = folded("References: ".len(), &list).unwrap_or(list);
        out.push_str(&format!("References: {folded}\r\n"));
    }
    if let Some(url) = headers.unsubscribe {
        out.push_str(&format!("List-Unsubscribe: <{url}>\r\n"));
        out.push_str("List-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n");
    }
    if headers.auto_submitted {
        out.push_str("Auto-Submitted: auto-generated\r\n");
    }
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
        let many: Vec<Mailbox> = (0..count)
            .map(|n| unnamed(&format!("user{n}@example.com")))
            .collect();
        let unfolded = many
            .iter()
            .map(|each| format!("<{}>", each.address))
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
        let to = [unnamed("to@example.com")];
        let cc = [unnamed("cc@example.com")];
        let message = compose(
            &Headers {
                to: &to,
                cc: &cc,
                ..headers()
            },
            "body",
            None,
        );
        assert!(message.contains("To: <to@example.com>\r\n"));
        assert!(message.contains("Cc: <cc@example.com>\r\n"));
        assert!(!message.contains("bcc@example.com"));
    }

    /// `address` with no display name.
    fn unnamed(address: &str) -> Mailbox {
        Mailbox {
            address: address.to_owned(),
            name: None,
        }
    }

    /// A recipient's name is quoted when it is plain and encoded when it is not, in `To:`, `Cc:`
    /// and `Reply-To:` alike. A name with a line break, a quote or a second address in it stays
    /// one header value, and the address after it is the one the program gave.
    // covers: Core\Mail::send
    #[test]
    fn a_recipient_name_is_encoded_and_cannot_add_a_header() {
        let named = |name: &str| Mailbox {
            address: "ada@example.com".to_owned(),
            name: Some(name.to_owned()),
        };
        let line = |header: &str, mailbox: Mailbox| {
            let one = [mailbox];
            let message = compose(
                &Headers {
                    to: if header == "To" { &one } else { &[] },
                    cc: if header == "Cc" { &one } else { &[] },
                    reply_to: (header == "Reply-To").then(|| &one[0]),
                    ..headers()
                },
                "body",
                None,
            );
            assert!(
                header_lines(&message).iter().all(|each| each
                    .split_once(": ")
                    .is_some_and(|(key, _)| !key.contains(' '))),
                "a name added a header: {message}"
            );
            header_lines(&message)
                .into_iter()
                .find(|each| each.starts_with(&format!("{header}: ")))
                .unwrap_or_default()
        };
        for header in ["To", "Cc", "Reply-To"] {
            assert_eq!(
                line(header, named("Ada Lovelace")),
                format!("{header}: \"Ada Lovelace\" <ada@example.com>")
            );
            assert_eq!(
                line(header, unnamed("ada@example.com")),
                format!("{header}: <ada@example.com>")
            );
            for name in [
                "Zoë",
                "Ada\r\nBcc: attacker@example.com",
                "Ada\" <attacker@example.com>, \"x",
                &"long".repeat(40),
            ] {
                let written = line(header, named(name));
                assert!(
                    written.starts_with(&format!("{header}: =?UTF-8?B?"))
                        && written.ends_with(" <ada@example.com>"),
                    "{name:?} gave {written:?}"
                );
                assert!(!written.contains("attacker"), "{written:?}");
            }
        }

        // A long list of named recipients still folds under 78 columns.
        let many: Vec<Mailbox> = (0..40)
            .map(|n| named(&format!("Reader number {n}")))
            .collect();
        let folded = address_line("To", &many);
        assert!(only_folded(&folded), "{folded:?}");
        assert!(folded.split("\r\n").all(|each| each.len() <= LINE));
    }

    /// A domain that is not ASCII is written as punycode, in the address `send` gives the server
    /// and the headers. A domain with no punycode form throws, and an ASCII one is unchanged.
    // covers: Core\Mail::send
    #[test]
    fn an_international_domain_is_sent_as_punycode() {
        let member = "Core\\Mail::send";
        assert_eq!(
            address_of("info@bücher.example", "to", member)
                .ok()
                .as_deref(),
            Some("info@xn--bcher-kva.example")
        );
        assert_eq!(
            address_of("info@Bücher.Example", "to", member)
                .ok()
                .as_deref(),
            Some("info@xn--bcher-kva.example")
        );
        assert_eq!(
            address_of("Info@Example.COM", "to", member).ok().as_deref(),
            Some("Info@Example.COM"),
            "an ASCII address is kept as written"
        );
        for bad in ["info@bü_cher.example", "info@bücher", "a@b@bücher.example"] {
            assert!(
                address_of(bad, "to", member).is_err(),
                "`{bad}` was accepted as an address"
            );
        }
        let message = compose(
            &Headers {
                to: &[unnamed(
                    &address_of("info@bücher.example", "to", member).unwrap_or_default(),
                )],
                ..headers()
            },
            "body",
            None,
        );
        assert!(message.is_ascii(), "{message}");
        assert!(message.contains("To: <info@xn--bcher-kva.example>\r\n"));
    }

    /// The headers of a message from `noreply@example.com`, with no recipient and nothing
    /// optional. Each test sets the field it is about.
    fn headers() -> Headers<'static> {
        Headers {
            from: "noreply@example.com",
            from_name: None,
            to: &[],
            cc: &[],
            reply_to: None,
            subject: "Hello",
            message_id: "0123456789abcdef0123456789abcdef@example.com",
            in_reply_to: None,
            references: &[],
            unsubscribe: None,
            auto_submitted: true,
            now: UNIX_EPOCH,
        }
    }

    /// The header part of `message`, unfolded, one header per entry.
    fn header_lines(message: &str) -> Vec<String> {
        let head = message.split("\r\n\r\n").next().unwrap_or_default();
        head.replace("\r\n ", " ")
            .split("\r\n")
            .map(str::to_owned)
            .collect()
    }

    /// Every message has a `Message-ID` header: 32 hex digits from the random bytes, then the
    /// domain of `from`.
    // covers: Core\Mail::send
    #[test]
    fn a_message_has_a_message_id_on_the_from_domain() {
        let random = [0xab_u8; 16];
        let id = message_id(random, "noreply@mail.example.com");
        assert_eq!(id, format!("{}@mail.example.com", "ab".repeat(16)));
        assert!(message_id_of(&id, "inReplyTo", "Core\\Mail::send").is_ok());

        let message = compose(
            &Headers {
                message_id: &id,
                ..headers()
            },
            "body",
            None,
        );
        assert!(
            header_lines(&message).contains(&format!("Message-ID: <{id}>")),
            "{message}"
        );
    }

    /// A display name is quoted when it is plain and encoded when it is not, and the address in
    /// `From:` is always the block's `from`.
    // covers: Core\Mail::send
    #[test]
    fn the_from_name_is_encoded_and_the_address_is_the_blocks() {
        let from = |name: &str| {
            let message = compose(
                &Headers {
                    from_name: Some(name),
                    ..headers()
                },
                "body",
                None,
            );
            header_lines(&message)
                .into_iter()
                .find(|line| line.starts_with("From: "))
                .unwrap_or_default()
        };
        assert_eq!(
            from("Example Shop"),
            "From: \"Example Shop\" <noreply@example.com>"
        );
        assert_eq!(from(""), "From: <noreply@example.com>");

        for name in [
            "Jürgen",
            "Shop\r\nBcc: attacker@example.com",
            "Shop\" <attacker@example.com>",
            "Back\\slash",
        ] {
            let line = from(name);
            assert!(
                line.starts_with("From: =?UTF-8?B?") && line.ends_with(" <noreply@example.com>"),
                "{name:?} gave {line:?}"
            );
            assert!(!line.contains("attacker"), "{line:?}");
        }
    }

    /// `In-Reply-To` and `References` carry message ids in angle brackets. A value that is not
    /// `local@domain`, or that has a byte that would end the id, throws.
    // covers: Core\Mail::send
    #[test]
    fn in_reply_to_and_references_take_only_message_ids() {
        let member = "Core\\Mail::send";
        for good in ["abc@example.com", "CAF+x=y.z$1@mail.example.com"] {
            assert_eq!(
                message_id_of(good, "inReplyTo", member).ok().as_deref(),
                Some(good)
            );
        }
        for bad in [
            "",
            "<abc@example.com>",
            "abc",
            "abc@",
            "@example.com",
            "a@b@example.com",
            "abc@example.com\r\nBcc: x@example.com",
            "abc @example.com",
            "abc(comment)@example.com",
            "jürgen@example.com",
        ] {
            assert!(
                message_id_of(bad, "references", member).is_err(),
                "`{bad}` was taken as a message id"
            );
        }

        let references = ["one@example.com".to_owned(), "two@example.com".to_owned()];
        let message = compose(
            &Headers {
                in_reply_to: Some("two@example.com"),
                references: &references,
                ..headers()
            },
            "body",
            None,
        );
        let lines = header_lines(&message);
        assert!(lines.contains(&"In-Reply-To: <two@example.com>".to_owned()));
        assert!(lines.contains(&"References: <one@example.com> <two@example.com>".to_owned()));
    }

    /// `unsubscribe` writes `List-Unsubscribe` and `List-Unsubscribe-Post` (RFC 8058), and only an
    /// `https://` URL is taken.
    // covers: Core\Mail::send
    #[test]
    fn unsubscribe_writes_both_headers_and_takes_only_https() {
        let member = "Core\\Mail::send";
        let url = "https://example.com/unsubscribe?u=42&t=abc";
        assert_eq!(unsubscribe_of(url, member).ok().as_deref(), Some(url));
        for bad in [
            "http://example.com/unsubscribe",
            "mailto:unsubscribe@example.com",
            "https://",
            "https:///path",
            "https://example.com/a b",
            "https://example.com/>\r\nBcc: x@example.com",
            "HTTPS://example.com/",
        ] {
            assert!(
                unsubscribe_of(bad, member).is_err(),
                "`{bad}` was taken as an unsubscribe URL"
            );
        }

        let message = compose(
            &Headers {
                unsubscribe: Some(url),
                ..headers()
            },
            "body",
            None,
        );
        let lines = header_lines(&message);
        assert!(lines.contains(&format!("List-Unsubscribe: <{url}>")));
        assert!(lines.contains(&"List-Unsubscribe-Post: List-Unsubscribe=One-Click".to_owned()));
        let plain = compose(&headers(), "body", None);
        assert!(!plain.contains("List-Unsubscribe"));
    }

    /// `Auto-Submitted: auto-generated` is on every message unless `autoSubmitted` is `false`.
    // covers: Core\Mail::send
    #[test]
    fn auto_submitted_is_written_unless_turned_off() {
        let on = compose(&headers(), "body", None);
        assert!(header_lines(&on).contains(&"Auto-Submitted: auto-generated".to_owned()));
        let off = compose(
            &Headers {
                auto_submitted: false,
                ..headers()
            },
            "body",
            None,
        );
        assert!(!off.contains("Auto-Submitted"));
    }
}
