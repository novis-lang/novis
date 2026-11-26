//! One SMTP session: the connection, `STARTTLS`, `AUTH PLAIN` and the envelope, over
//! [`nvs_host`]'s parking stream.
//!
//! [`deliver`] is the whole exchange for one message. The module doc of [`super`] says why a
//! credential needs TLS, and why a block with no credential stays in the clear.

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::time::Instant;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;
use nvs_runtime::{Fault, ThrownClass};

use super::Endpoint;

/// One SMTP conversation over [`nvs_host`]'s parking stream.
///
/// Held as a struct for the read buffer alone: a reply is line-oriented and a
/// multiline one is only finished when a line's fourth byte is a space, so a
/// reader that did not keep what it over-read would lose the head of the next
/// reply.
struct Session {
    /// The socket, or the TLS session `STARTTLS` replaced it with — either way
    /// it hands the core back rather than blocking it.
    stream: Wire,
    /// Whatever the last read took past the end of the reply it was completing.
    held: Vec<u8>,
}

/// What a [`Session`] is talking over.
///
/// An enum rather than a boxed trait object because there are two variants and
/// there will not be a third, and because the upgrade *consumes* the socket:
/// [`NvsTls::over`] takes an [`NvsTcp`] by value, which is what leaves nobody
/// holding a plaintext stream to an endpoint that has been secured.
///
/// `Secured` is about a kilobyte wider than `Plain` — `rustls`'s session state
/// is held inline — and that is the trade AGENTS.md's priority ordering asks
/// for, spent deliberately: there is one `Wire` per in-flight `send`, so the
/// cost is O(in-flight) rather than O(messages sent), and boxing it would buy an
/// allocation and an indirection on every record read to save a kilobyte
/// of memory footprint, which the ordering says not to chase.
#[allow(clippy::large_enum_variant)]
enum Wire {
    /// Before `STARTTLS`, and for the whole of a session with no credential.
    Plain(NvsTcp),
    /// After it. Every wait inside is still [`nvs_host::net`]'s park, which
    /// [`nvs_host::tls`]'s module doc is the home of.
    Secured(NvsTls),
}

impl Read for Wire {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(buf),
            Self::Secured(stream) => stream.read(buf),
        }
    }
}

impl Write for Wire {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(buf),
            Self::Secured(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            Self::Secured(stream) => stream.flush(),
        }
    }
}

impl Session {
    /// The connected session, with the server's banner already accepted.
    ///
    /// Every wait of the exchange ends at `until`: the connect, the banner,
    /// each command and the `DATA` body, and the TLS session over the same
    /// socket after `STARTTLS`.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` for a socket that will not open and for a banner
    /// that is not a 220.
    fn open(address: SocketAddr, until: Instant, member: &str) -> Result<Self, Fault> {
        let budget = until.saturating_duration_since(Instant::now());
        let mut stream = NvsTcp::connect_timeout(address, budget).map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!("{member} could not reach {address}: {why}"),
            )
        })?;
        stream.set_deadline(Some(until));
        let mut session = Self {
            stream: Wire::Plain(stream),
            held: Vec::new(),
        };
        session.expect(220, "the greeting", member)?;
        Ok(session)
    }

    /// The same conversation over TLS: `STARTTLS`, a handshake verified against
    /// `name`, and the session that answers on the other side of it.
    ///
    /// Taken by value, which is the property rather than a style choice — the
    /// plaintext session is gone by the time this returns, so no later step can
    /// reach for it.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the endpoint refuses `STARTTLS`, when it sends
    /// anything between agreeing to it and the handshake, and when the handshake
    /// does not complete — including a certificate that does not verify against
    /// [`nvs_host::tls`]'s compiled-in anchors.
    fn secure(mut self, name: &str, member: &str) -> Result<Self, Fault> {
        self.command("STARTTLS", 220, "the upgrade", member)?;
        // The whole reason `held` is inspected anywhere. A man in the middle
        // who writes commands *after* the endpoint's 220 and before the
        // handshake has them buffered here, and a session that carried the
        // buffer across would replay them as though the secured peer had sent
        // them. The endpoint owes silence between the two, so anything at all is
        // refused rather than discarded — discarding is the same defence with
        // the evidence thrown away.
        if !self.held.is_empty() {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: the endpoint sent {} more byte(s) after agreeing to `STARTTLS`, \
                     which is how a plaintext command is smuggled into a secured session",
                    self.held.len()
                ),
            ));
        }
        let Wire::Plain(stream) = self.stream else {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!("{member}: the session is already secured"),
            ));
        };
        let secured = NvsTls::over(stream, name).map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!("{member}: the TLS handshake with `{name}` did not complete — {why}"),
            )
        })?;
        Ok(Self {
            stream: Wire::Secured(secured),
            held: Vec::new(),
        })
    }

    /// One command, and the lines of the reply it was answered with.
    ///
    /// `line` never reaches a diagnostic — `what` names the step instead —
    /// because one of the commands this sends is `AUTH PLAIN`.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the write fails, when the connection ends
    /// mid-reply, or when the code is not `want`.
    fn command(
        &mut self,
        line: &str,
        want: i64,
        what: &str,
        member: &str,
    ) -> Result<Vec<String>, Fault> {
        self.stream
            .write_all(format!("{line}\r\n").as_bytes())
            .and_then(|()| self.stream.flush())
            .map_err(|why| {
                Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{member} could not send {what}: {why}"),
                )
            })?;
        self.expect(want, what, member)
    }

    /// Reads one whole reply, holds it to `want`, and answers with its lines —
    /// which for `EHLO` are the extension list [`advertised`] reads.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` for a closed connection, a reply that is not
    /// three digits, or a code other than `want` — the last carrying the
    /// server's own sentence, because that is the only part of the failure the
    /// operator did not already write.
    fn expect(&mut self, want: i64, what: &str, member: &str) -> Result<Vec<String>, Fault> {
        let mut reply = Vec::new();
        loop {
            if reply.len() == REPLY_LINES {
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!(
                        "{member}: the reply to {what} has more than the {REPLY_LINES} lines \
                         an SMTP reply may have"
                    ),
                ));
            }
            let line = self.line(what, member)?;
            let code = line
                .get(..3)
                .and_then(|digits| digits.parse::<i64>().ok())
                .ok_or_else(|| {
                    Fault::thrown_as(
                        ThrownClass::Io,
                        format!(
                            "{member}: the endpoint answered {what} with `{line}`, which is \
                                 not an SMTP reply"
                        ),
                    )
                })?;
            // A continuation line writes `250-`; the last one writes `250 `.
            if line.as_bytes().get(3) != Some(&b'-') {
                if code != want {
                    return Err(Fault::thrown_as(
                        ThrownClass::Io,
                        format!("{member}: the endpoint refused {what} — {line}"),
                    ));
                }
                reply.push(line);
                return Ok(reply);
            }
            reply.push(line);
        }
    }

    /// The next CRLF-terminated line, without its terminator.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the connection closes before one arrives.
    fn line(&mut self, what: &str, member: &str) -> Result<String, Fault> {
        loop {
            let found = self.held.iter().position(|byte| *byte == b'\n');
            if found.unwrap_or(self.held.len()) > REPLY_LINE_CEILING {
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!(
                        "{member}: a line of the reply to {what} is longer than the \
                         {REPLY_LINE_CEILING} bytes an SMTP reply line may be"
                    ),
                ));
            }
            if let Some(end) = found {
                let mut line: Vec<u8> = self.held.drain(..=end).collect();
                while line
                    .last()
                    .is_some_and(|byte| *byte == b'\n' || *byte == b'\r')
                {
                    line.pop();
                }
                return Ok(String::from_utf8_lossy(&line).into_owned());
            }
            let mut chunk = [0_u8; 512];
            let read = self.stream.read(&mut chunk).map_err(|why| {
                Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{member} could not read the reply to {what}: {why}"),
                )
            })?;
            if read == 0 {
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{member}: the endpoint closed the connection during {what}"),
                ));
            }
            self.held.extend_from_slice(&chunk[..read]);
        }
    }
}

/// The longest reply line [`Session::line`] reads, terminator excluded.
///
/// RFC 5321 § 4.5.3.1.5 allows 512 octets with the terminator. The cap is
/// eight times that, so an endpoint that is a little over the limit still
/// works, and one that never ends a line is refused at 4 KiB.
const REPLY_LINE_CEILING: usize = 4096;

/// The most lines one reply may have.
///
/// An `EHLO` reply is one line per extension, and endpoints advertise about
/// twenty. With [`REPLY_LINE_CEILING`] this caps a reply at 1 MiB, which an
/// endpoint that sends continuation lines forever runs into.
const REPLY_LINES: usize = 256;

/// The parameters `keyword` was advertised with in an `EHLO` reply, or `None`
/// where the endpoint did not advertise it at all.
///
/// The reply's first line is the endpoint's own greeting rather than a keyword,
/// so it is skipped; every other line is `250-KEYWORD PARAM PARAM`, and
/// RFC 5321 § 2.4 makes the keyword case-insensitive.
fn advertised<'a>(reply: &'a [String], keyword: &str) -> Option<&'a str> {
    reply.iter().skip(1).find_map(|line| {
        let rest = line.get(4..)?;
        let (word, params) = rest.split_once(' ').unwrap_or((rest, ""));
        word.eq_ignore_ascii_case(keyword).then_some(params)
    })
}

/// RFC 4616's `PLAIN` message, base64'd: an empty authorization identity — so
/// the endpoint uses the authentication one — then the user and the password,
/// NUL-separated.
fn plain(user: &str, password: &str) -> String {
    STANDARD.encode(format!("\0{user}\0{password}"))
}

/// The `MAIL FROM` command for `from` and `recipients`, given the `extensions` the session's
/// last `EHLO` advertised.
///
/// An address whose local part is not ASCII needs RFC 6531's `SMTPUTF8`, which the command then
/// asks for. Every address reaching here came through [`super::compose::address_of`], so its
/// domain is already ASCII.
///
/// # Errors
///
/// A catchable `IOError` naming the first such address when `host` does not advertise
/// `SMTPUTF8`. Nothing has been sent but the greeting.
fn envelope_sender(
    from: &str,
    recipients: &[String],
    extensions: &[String],
    host: &str,
    member: &str,
) -> Result<String, Fault> {
    let Some(international) = std::iter::once(from)
        .chain(recipients.iter().map(String::as_str))
        .find(|address| !address.is_ascii())
    else {
        return Ok(format!("MAIL FROM:<{from}>"));
    };
    if advertised(extensions, "SMTPUTF8").is_none() {
        return Err(Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{member}: the address {international} is not ASCII, and {host} does not offer \
                 `SMTPUTF8`, which such an address needs. Nothing was sent."
            ),
        ));
    }
    Ok(format!("MAIL FROM:<{from}> SMTPUTF8"))
}

/// The whole exchange, once every argument has been checked and every
/// deployment question answered. Every wait in it ends at `until`.
///
/// # Errors
///
/// A catchable `IOError` for anything the endpoint said or did — see
/// [`Session`] — and for an endpoint that cannot carry a configured credential:
/// the module doc's § *A credential asks for TLS* is why that is a refusal here
/// rather than a send in the clear.
pub(super) fn deliver(
    address: SocketAddr,
    endpoint: &Endpoint,
    until: Instant,
    envelope: &[String],
    message: &str,
    member: &str,
) -> Result<(), Fault> {
    let mut session = Session::open(address, until, member)?;
    // The domain the operator's own `from` names, so nothing about the host
    // serving the request is announced to the relay.
    let domain = endpoint
        .from
        .split_once('@')
        .map_or("localhost", |(_, d)| d);
    let greeting = format!("EHLO {domain}");
    let mut extensions = session.command(&greeting, 250, "the greeting", member)?;

    if let Some((user, password)) = &endpoint.credential {
        if advertised(&extensions, "STARTTLS").is_none() {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: `[mail]` configures a credential for {} and that endpoint offers \
                     no `STARTTLS`, so there is nothing to send one under — nothing was sent",
                    endpoint.host
                ),
            ));
        }
        // The certificate is checked against the name the *operator* wrote, the
        // same rule `crate::http::transport` states: the address was derived
        // from that name and answers for nothing on its own.
        session = session.secure(&endpoint.host, member)?;
        // A second `EHLO`, because the extension list belongs to the session and
        // this is a new one — an endpoint that offers `AUTH` only once the
        // stream is secured is the ordinary case, not an unusual one.
        let secured = session.command(&greeting, 250, "the greeting", member)?;
        let offers_plain = advertised(&secured, "AUTH").is_some_and(|mechanisms| {
            mechanisms
                .split_ascii_whitespace()
                .any(|mechanism| mechanism.eq_ignore_ascii_case("PLAIN"))
        });
        if !offers_plain {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: {} offers no `AUTH PLAIN` over TLS, and this transport has no \
                     second mechanism — nothing was sent",
                    endpoint.host
                ),
            ));
        }
        session.command(
            &format!("AUTH PLAIN {}", plain(user, password)),
            235,
            "the credential",
            member,
        )?;
        extensions = secured;
    }

    let mail_from = envelope_sender(&endpoint.from, envelope, &extensions, &endpoint.host, member)?;
    session.command(&mail_from, 250, "the envelope sender", member)?;
    for recipient in envelope {
        session.command(
            &format!("RCPT TO:<{recipient}>"),
            250,
            "a recipient",
            member,
        )?;
    }
    session.command("DATA", 354, "the body", member)?;
    session
        .stream
        .write_all(message.as_bytes())
        .and_then(|()| session.stream.write_all(b"\r\n.\r\n"))
        .and_then(|()| session.stream.flush())
        .map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!("{member} could not send the message: {why}"),
            )
        })?;
    session.expect(250, "the message", member)?;
    // A QUIT the endpoint never answers is not a message that failed to send:
    // it was accepted at the 250 above, so the reply here is not asked about.
    let _ = session.stream.write_all(b"QUIT\r\n");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// The extension reader, over the three things an `EHLO` reply can do to
    /// it: name the endpoint on its first line, spell a keyword in whatever
    /// case it likes, and carry parameters after it.
    ///
    /// The greeting line matters because it is the one line that is *not* a
    /// keyword: a reader that did not skip it would read an endpoint calling
    /// itself `starttls.example.com` as an offer of `STARTTLS`, and offer to
    /// upgrade a connection nothing would answer.
    #[test]
    fn the_ehlo_reply_is_read_as_keywords_after_its_first_line() {
        let reply: Vec<String> = [
            "250-starttls.example.com at your service",
            "250-SIZE 35882577",
            "250-StartTls",
            "250 AUTH LOGIN PLAIN XOAUTH2",
        ]
        .iter()
        .map(|line| (*line).to_owned())
        .collect();

        // Case-insensitive, per RFC 5321 § 2.4, and found on a continuation
        // line as readily as on the last one.
        assert_eq!(advertised(&reply, "STARTTLS"), Some(""));
        assert_eq!(advertised(&reply, "size"), Some("35882577"));
        assert_eq!(advertised(&reply, "AUTH"), Some("LOGIN PLAIN XOAUTH2"));

        // And the greeting is not a keyword, however much it looks like one.
        let greeting_only = vec![reply[0].clone()];
        assert_eq!(advertised(&greeting_only, "STARTTLS"), None);

        // A keyword nobody offered is absent rather than empty.
        assert_eq!(advertised(&reply, "DSN"), None);
    }

    /// A reply line that never ends and a reply that never stops continuing
    /// are both refused at their cap, so an endpoint cannot choose how much a
    /// session holds. The endpoint here reads until the session closes, so the
    /// refusal is never a reset.
    #[test]
    fn a_reply_is_capped_per_line_and_per_reply() {
        let endless_line = "a".repeat(REPLY_LINE_CEILING + 1024);
        let endless_reply = "220-mail.example.com\r\n".repeat(REPLY_LINES + 1);
        for (greeting, refused) in [
            (endless_line, "an SMTP reply line may be"),
            (endless_reply, "lines an SMTP reply may have"),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
            let address = listener.local_addr().expect("its address");
            let endpoint = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("the session");
                stream.write_all(greeting.as_bytes()).expect("the greeting");
                let mut rest = Vec::new();
                drop(stream.read_to_end(&mut rest));
            });
            let until = Instant::now() + Duration::from_secs(10);
            let Err(fault) = Session::open(address, until, "Core\\Mail::send") else {
                panic!("a greeting past its cap was read");
            };
            assert!(format!("{fault:?}").contains(refused), "{fault:?}");
            endpoint.join().expect("the endpoint thread");
        }
    }

    /// RFC 4616's framing, which is three fields and not two: the empty
    /// authorization identity in front is what makes the endpoint authorize as
    /// whoever authenticated, and a message missing it is refused by every
    /// server rather than read as a shorter one.
    #[test]
    fn auth_plain_sends_an_empty_authorization_identity_first() {
        let message = plain("postmaster@example.com", "hunter2");
        let decoded = STANDARD.decode(&message).expect("`plain` emits base64");

        assert_eq!(
            decoded, b"\0postmaster@example.com\0hunter2",
            "the PLAIN message is NUL-separated with an empty first field"
        );
        // The credential is never on the wire in any other spelling, so the
        // encoded form is the only thing a reply can quote back.
        assert!(!message.contains("hunter2"));
    }

    /// An address whose local part is not ASCII is kept as written, and the send asks for
    /// `SMTPUTF8` when the server offers it. A server that does not offer it gets no `MAIL FROM`,
    /// and the `IOError` names the address. An ASCII envelope never asks.
    // covers: Core\Mail::send
    #[test]
    fn a_non_ascii_local_part_needs_smtputf8() {
        let member = "Core\\Mail::send";
        let jose = super::super::compose::address_of("josé@bücher.example", "to", member)
            .expect("a non-ASCII local part is an address");
        assert_eq!(jose, "josé@xn--bcher-kva.example");

        let with = ["250-mail.example.com".to_owned(), "250 SMTPUTF8".to_owned()];
        let without = ["250-mail.example.com".to_owned(), "250 8BITMIME".to_owned()];
        let ascii = ["ada@example.com".to_owned()];
        let international = ["ada@example.com".to_owned(), jose.clone()];

        let line = |from: &str, to: &[String], offered: &[String]| {
            envelope_sender(from, to, offered, "mail.example.com", member)
        };
        assert_eq!(
            line("noreply@example.com", &ascii, &without).ok().as_deref(),
            Some("MAIL FROM:<noreply@example.com>")
        );
        assert_eq!(
            line("noreply@example.com", &ascii, &with).ok().as_deref(),
            Some("MAIL FROM:<noreply@example.com>")
        );
        assert_eq!(
            line("noreply@example.com", &international, &with).ok().as_deref(),
            Some("MAIL FROM:<noreply@example.com> SMTPUTF8")
        );
        assert_eq!(
            line("josé@xn--bcher-kva.example", &ascii, &with).ok().as_deref(),
            Some("MAIL FROM:<josé@xn--bcher-kva.example> SMTPUTF8")
        );
        let Err(fault) = line("noreply@example.com", &international, &without) else {
            panic!("a non-ASCII address was sent to a server without SMTPUTF8");
        };
        let said = format!("{fault:?}");
        assert!(
            said.contains("josé@xn--bcher-kva.example") && said.contains("SMTPUTF8"),
            "{said}"
        );
    }
}
