//! The session against a fake IMAP server on a loopback port: TLS, `STARTTLS`, the three sign-ins,
//! the folder list, `SELECT`, the search, the fetch ceiling and how a folder name is written.
//!
//! The fake reads each command as the client wrote it, with its literals, and answers a
//! synchronized literal with `+ go ahead` before it reads the literal's bytes. It returns every
//! command it read, so a test checks what reached the wire.

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nvs_config::mail::{Auth, Tls};
use nvs_runtime::Fault;

use super::{Filter, Server, Session, SignIn, utf7};
use crate::mail::localhost;

const MEMBER: &str = "Core\\Imap::open";

/// A stream the fake server reads and writes, plain or TLS.
trait Duplex: Read + Write {}
impl<T: Read + Write> Duplex for T {}

/// What the fake server does.
#[derive(Default)]
struct Fake {
    /// TLS from the first byte.
    implicit: bool,
    /// `STARTTLS` is offered before the session is secured.
    starttls: bool,
    /// The capabilities beside `IMAP4rev1` and `STARTTLS`.
    capabilities: &'static [&'static str],
    /// The certificate it presents.
    certificate: Option<Arc<rustls::ServerConfig>>,
    /// The `* LIST` lines it sends, after `* LIST `.
    folders: &'static [&'static str],
    /// The UIDs `* SEARCH` lists.
    found: &'static [u32],
    /// The `UIDVALIDITY` of every folder.
    validity: u32,
    /// The message every `UID FETCH` returns, with the UID it was asked for.
    message: &'static str,
    /// The UIDs, as written, that `UID STORE`, `UID MOVE` and `UID EXPUNGE` find no message for.
    missing: &'static [&'static str],
}

/// What the fake read: each command, with its literals written in place of their `{n}`.
struct Heard {
    commands: Vec<String>,
}

impl Heard {
    /// The command whose verb (the word after the tag) starts with `verb`.
    fn find(&self, verb: &str) -> Option<&str> {
        self.commands
            .iter()
            .map(String::as_str)
            .find(|command| verb_of(command).starts_with(verb))
    }
}

/// Everything after a command's tag.
fn verb_of(command: &str) -> &str {
    command.split_once(' ').map_or("", |(_, rest)| rest)
}

fn say(wire: &mut dyn Duplex, line: &str) -> std::io::Result<()> {
    wire.write_all(format!("{line}\r\n").as_bytes())?;
    wire.flush()
}

/// One line without its CRLF, or `None` once the client closed.
fn line(wire: &mut dyn Duplex) -> Option<Vec<u8>> {
    let mut line = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        match wire.read(&mut byte) {
            Ok(1) if byte[0] == b'\n' => break,
            Ok(1) => line.push(byte[0]),
            _ => return None,
        }
    }
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    Some(line)
}

/// The length of the literal a line ends with, and whether it is synchronized.
fn literal_at_end(line: &[u8]) -> Option<(usize, bool)> {
    let text = std::str::from_utf8(line).ok()?.strip_suffix('}')?;
    let open = text.rfind('{')?;
    let inside = &text[open + 1..];
    let (digits, sync) = match inside.strip_suffix('+') {
        Some(digits) => (digits, false),
        None => (inside, true),
    };
    Some((digits.parse().ok()?, sync))
}

/// One whole command: its lines and literals, with each literal answered as the client needs.
fn command(wire: &mut dyn Duplex) -> Option<String> {
    let mut whole = Vec::new();
    loop {
        let part = line(wire)?;
        whole.extend_from_slice(&part);
        let Some((length, sync)) = literal_at_end(&part) else {
            break;
        };
        if sync {
            say(wire, "+ go ahead").ok()?;
        }
        whole.extend_from_slice(b"\r\n");
        let mut literal = vec![0_u8; length];
        wire.read_exact(&mut literal).ok()?;
        whole.extend_from_slice(&literal);
    }
    Some(String::from_utf8_lossy(&whole).into_owned())
}

/// One IMAP session on a loopback port, served as `fake` says.
fn serve(fake: Fake) -> (SocketAddr, std::thread::JoinHandle<Heard>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let address = listener.local_addr().expect("its address");
    let thread = std::thread::spawn(move || {
        let mut heard = Heard {
            commands: Vec::new(),
        };
        if let Ok((socket, _)) = listener.accept() {
            converse(&fake, &socket, &mut heard);
        }
        heard
    });
    (address, thread)
}

/// A line the fake read that is not a command tagged `N<n>`, counted from 1.
const UNTAGGED: &str = "<not a tagged command>";

/// The conversation over `socket`. A line whose tag is not the next `N<n>` ends it, as
/// [`UNTAGGED`].
fn converse(fake: &Fake, socket: &std::net::TcpStream, heard: &mut Heard) {
    drop(socket.set_nonblocking(false));
    drop(socket.set_read_timeout(Some(Duration::from_secs(10))));
    let secure = |socket: &std::net::TcpStream| -> Option<Box<dyn Duplex>> {
        let config = Arc::clone(fake.certificate.as_ref()?);
        let session = rustls::ServerConnection::new(config).ok()?;
        Some(Box::new(rustls::StreamOwned::new(
            session,
            socket.try_clone().ok()?,
        )))
    };
    let mut wire: Box<dyn Duplex> = if fake.implicit {
        match secure(socket) {
            Some(wire) => wire,
            None => return,
        }
    } else {
        Box::new(socket.try_clone().expect("a second handle"))
    };
    let mut secured = fake.implicit;
    if say(&mut *wire, "* OK fake.example ready").is_err() {
        return;
    }
    while let Some(read) = command(&mut *wire) {
        let expected = format!("N{} ", heard.commands.len() + 1);
        if !read.starts_with(&expected) {
            heard.commands.push(UNTAGGED.to_owned());
            return;
        }
        heard.commands.push(read.clone());
        let (tag, rest) = read.split_once(' ').unwrap_or((read.as_str(), ""));
        let upper = rest.to_ascii_uppercase();
        let mut reply: Vec<String> = Vec::new();
        if upper == "CAPABILITY" {
            let mut words = vec!["IMAP4rev1"];
            if fake.starttls && !secured {
                words.push("STARTTLS");
            }
            words.extend(fake.capabilities);
            reply.push(format!("* CAPABILITY {}", words.join(" ")));
        } else if upper == "STARTTLS" {
            if say(&mut *wire, &format!("{tag} OK begin TLS")).is_err() {
                return;
            }
            match secure(socket) {
                Some(upgraded) => {
                    wire = upgraded;
                    secured = true;
                    continue;
                }
                None => return,
            }
        } else if upper.starts_with("AUTHENTICATE ") && rest.split(' ').count() == 2 {
            // No initial response: ask for it, and keep it as part of the command.
            if say(&mut *wire, "+ ").is_err() {
                return;
            }
            let Some(answer) = line(&mut *wire) else {
                return;
            };
            if let Some(last) = heard.commands.last_mut() {
                last.push(' ');
                last.push_str(&String::from_utf8_lossy(&answer));
            }
        } else if upper.starts_with("LIST ") {
            reply.extend(fake.folders.iter().map(|each| format!("* LIST {each}")));
        } else if upper.starts_with("SELECT ") {
            reply.push("* 3 EXISTS".to_owned());
            reply.push(format!("* OK [UIDVALIDITY {}] UIDs valid", fake.validity));
        } else if upper.starts_with("UID SEARCH ") {
            let found: Vec<String> = fake.found.iter().map(u32::to_string).collect();
            reply.push(
                format!("* SEARCH {}", found.join(" "))
                    .trim_end()
                    .to_owned(),
            );
        } else if upper.starts_with("UID FETCH ") {
            let uid = rest.split(' ').nth(2).unwrap_or("1");
            if upper.contains("BODY.PEEK[]") {
                let length = fake.message.len();
                reply.push(format!(
                    "* 1 FETCH (UID {uid} BODY[] {{{length}}}\r\n{})",
                    fake.message
                ));
            } else {
                let length = fake.message.len();
                reply.push(format!("* 1 FETCH (UID {uid} RFC822.SIZE {length})"));
            }
        } else if upper.starts_with("UID STORE ") {
            let uid = rest.split(' ').nth(2).unwrap_or("1");
            if !fake.missing.contains(&uid) {
                reply.push(format!("* 1 FETCH (UID {uid} FLAGS (\\Seen))"));
            }
        } else if upper.starts_with("UID MOVE ") || upper.starts_with("UID EXPUNGE ") {
            let uid = rest.split(' ').nth(2).unwrap_or("1");
            if !fake.missing.contains(&uid) {
                reply.push("* 1 EXPUNGE".to_owned());
            }
        } else if upper == "LOGOUT" {
            drop(say(&mut *wire, "* BYE fake.example closes"));
            drop(say(&mut *wire, &format!("{tag} OK done")));
            return;
        }
        reply.push(format!("{tag} OK done"));
        if say(&mut *wire, &reply.join("\r\n")).is_err() {
            return;
        }
    }
}

/// An `[imap.inbox]` block that reaches `localhost` with `tls`.
fn server(tls: Tls) -> Server {
    Server {
        name: "inbox".to_owned(),
        host: "localhost".to_owned(),
        tls,
        ca_file: None,
        auth: None,
    }
}

fn password() -> SignIn {
    SignIn::Password {
        user: "ada@example.com".to_owned(),
        password: "hunter2".to_owned(),
    }
}

fn open(address: SocketAddr, server: &Server, sign_in: &SignIn) -> Result<Session, Fault> {
    let until = Instant::now() + Duration::from_secs(10);
    Session::open(address, server, sign_in, until, MEMBER)
}

/// A session signed in over implicit TLS against `fake`, which presents a certificate the
/// block trusts. The scratch directory holds the certificate and lives as long as the test.
fn signed_in(mut fake: Fake) -> (Session, std::thread::JoinHandle<Heard>, nvs_repo::Scratch) {
    let (certificate, dir, ca) = localhost();
    fake.implicit = true;
    fake.certificate = Some(certificate);
    let (address, thread) = serve(fake);
    let mut block = server(Tls::Implicit);
    block.ca_file = Some(ca);
    let session = open(address, &block, &password()).expect("the session opens");
    (session, thread, dir)
}

/// Ends `session` and returns what the fake read, every line of it a tagged command.
fn close(mut session: Session, thread: std::thread::JoinHandle<Heard>) -> Heard {
    session.logout();
    drop(session);
    let heard = thread.join().expect("the fake server");
    assert!(
        !heard.commands.iter().any(|each| each == UNTAGGED),
        "{:?}",
        heard.commands
    );
    heard
}

fn said(fault: &Fault) -> String {
    format!("{fault:?}")
}

/// `tls = "implicit"`, the default on port 993, starts the handshake before the greeting. A
/// server that greets in plain text gets no command, and no password.
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn imap_speaks_tls_from_the_first_byte_on_993() {
    let (session, thread, _dir) = signed_in(Fake::default());
    let heard = close(session, thread);
    assert_eq!(heard.commands[0], "N1 CAPABILITY", "{:?}", heard.commands);
    assert!(heard.find("AUTHENTICATE").is_none() && heard.find("LOGIN").is_some());

    let (_, _dir, ca) = localhost();
    let (address, thread) = serve(Fake::default());
    let mut block = server(Tls::Implicit);
    block.ca_file = Some(ca);
    let Err(fault) = open(address, &block, &password()) else {
        panic!("a plain-text server was read as a TLS one");
    };
    assert!(said(&fault).contains("TLS handshake"), "{}", said(&fault));
    let heard = thread.join().expect("the fake server");
    // The fake reads the client's TLS hello as a line, and nothing after it.
    assert_eq!(heard.commands, [UNTAGGED]);
}

/// `tls = "starttls"` upgrades before the sign-in. A server that offers no `STARTTLS` gets no
/// credential, and a certificate the block does not trust names `tls_ca_file` as the fix.
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn imap_starttls_is_required_and_verified() {
    let (certificate, _dir, ca) = localhost();
    let (address, thread) = serve(Fake {
        starttls: true,
        certificate: Some(Arc::clone(&certificate)),
        ..Fake::default()
    });
    let mut block = server(Tls::StartTls);
    block.ca_file = Some(ca);
    let session = open(address, &block, &password()).expect("the upgraded session");
    let heard = close(session, thread);
    let verbs: Vec<&str> = heard.commands.iter().map(|each| verb_of(each)).collect();
    assert_eq!(
        &verbs[..3],
        ["CAPABILITY", "STARTTLS", "CAPABILITY"],
        "{verbs:?}"
    );
    assert!(verbs[3].starts_with("LOGIN"), "{verbs:?}");

    let (address, thread) = serve(Fake::default());
    let Err(fault) = open(address, &block, &password()) else {
        panic!("a server without STARTTLS was given the password");
    };
    assert!(
        said(&fault).contains("offers no `STARTTLS`"),
        "{}",
        said(&fault)
    );
    let heard = thread.join().expect("the fake server");
    assert_eq!(heard.commands, ["N1 CAPABILITY"]);

    let (address, thread) = serve(Fake {
        starttls: true,
        certificate: Some(certificate),
        ..Fake::default()
    });
    let Err(fault) = open(address, &server(Tls::StartTls), &password()) else {
        panic!("a certificate from an unknown issuer was accepted");
    };
    assert!(said(&fault).contains("tls_ca_file"), "{}", said(&fault));
    let heard = thread.join().expect("the fake server");
    assert!(heard.find("LOGIN").is_none(), "{:?}", heard.commands);
}

/// `PLAIN` sends an empty authorization identity, the user and the password, at once with
/// `SASL-IR` and after the server's `+` without it. `LOGIN` is used where `PLAIN` is not
/// offered, and `XOAUTH2` sends the user and the bearer token.
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn imap_signs_in_with_plain_login_and_xoauth2() {
    use base64::Engine as _;
    let base64 = |text: &str| base64::engine::general_purpose::STANDARD.encode(text);
    let plain = base64("\0ada@example.com\0hunter2");

    let (session, thread, _dir) = signed_in(Fake {
        capabilities: &["AUTH=PLAIN", "SASL-IR"],
        ..Fake::default()
    });
    let heard = close(session, thread);
    assert_eq!(
        heard.find("AUTHENTICATE"),
        Some(format!("N2 AUTHENTICATE PLAIN {plain}").as_str())
    );

    let (session, thread, _dir) = signed_in(Fake {
        capabilities: &["AUTH=PLAIN"],
        ..Fake::default()
    });
    let heard = close(session, thread);
    assert_eq!(
        heard.find("AUTHENTICATE"),
        Some(format!("N2 AUTHENTICATE PLAIN {plain}").as_str())
    );

    let (session, thread, _dir) = signed_in(Fake::default());
    let heard = close(session, thread);
    assert_eq!(
        heard.find("LOGIN"),
        Some("N2 LOGIN ada@example.com hunter2")
    );

    let (certificate, _dir, ca) = localhost();
    let (address, thread) = serve(Fake {
        implicit: true,
        certificate: Some(certificate),
        capabilities: &["AUTH=XOAUTH2", "SASL-IR"],
        ..Fake::default()
    });
    let mut block = server(Tls::Implicit);
    block.ca_file = Some(ca);
    block.auth = Some(Auth::XOAuth2);
    let token = SignIn::Token {
        user: "ada@example.com".to_owned(),
        token: "ya29.token".to_owned(),
    };
    let session = open(address, &block, &token).expect("the token signs in");
    let heard = close(session, thread);
    let answer = base64("user=ada@example.com\x01auth=Bearer ya29.token\x01\x01");
    assert_eq!(
        heard.find("AUTHENTICATE"),
        Some(format!("N2 AUTHENTICATE XOAUTH2 {answer}").as_str())
    );
    assert!(heard.find("LOGIN").is_none());
}

/// `LIST "" *` lists every folder. A name in modified UTF-7 is decoded, a literal name is read,
/// and a `\Noselect` folder is left out because it holds no messages.
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn folders_are_listed_and_decoded_from_modified_utf7() {
    let (mut session, thread, _dir) = signed_in(Fake {
        folders: &[
            "(\\HasNoChildren) \"/\" INBOX",
            "(\\HasNoChildren) \"/\" \"Entw&APw-rfe\"",
            "(\\Noselect \\HasChildren) \"/\" \"[Gmail]\"",
            "(\\HasNoChildren) \"/\" \"Tom &- Jerry\"",
            "(\\HasNoChildren) \"/\" {6}\r\nReport",
            "(\\HasNoChildren) \"/\" \"&ZeVnLIqe-\"",
        ],
        ..Fake::default()
    });
    let folders = session.folders(MEMBER).expect("the folder list");
    let heard = close(session, thread);
    assert_eq!(
        folders,
        ["INBOX", "Entwürfe", "Tom & Jerry", "Report", "日本語"]
    );
    assert_eq!(heard.find("LIST"), Some("N3 LIST \"\" *"));

    for name in ["Entwürfe", "Tom & Jerry", "日本語", "a/b~c", "😀 Fun"] {
        assert_eq!(utf7::decode(&utf7::encode(name)), name);
    }
    assert_eq!(utf7::encode("Entwürfe"), "Entw&APw-rfe");
    assert_eq!(utf7::encode("日本語"), "&ZeVnLIqe-");
    assert_eq!(utf7::decode("&Jjo-broken&"), "☺broken&");
}

/// Each filter is one search key, every non-ASCII text is sent with `CHARSET UTF-8` as a
/// literal, and the UIDs come back sorted with no repeats, the oldest first, up to `limit`.
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn search_sends_each_filter_and_returns_uids() {
    let (mut session, thread, _dir) = signed_in(Fake {
        found: &[9, 3, 5, 3, 12],
        ..Fake::default()
    });
    let all = session.search(&Filter::default(), MEMBER).expect("all");
    let filtered = session
        .search(
            &Filter {
                unseen: true,
                since: Some((2026, 2, 1)),
                before: Some((2026, 11, 26)),
                from: Some("shop@example.com".to_owned()),
                subject: Some("Grüße".to_owned()),
                limit: Some(2),
            },
            MEMBER,
        )
        .expect("the filtered search");
    let heard = close(session, thread);
    assert_eq!(all, [3, 5, 9, 12]);
    assert_eq!(filtered, [3, 5]);
    let searches: Vec<&str> = heard
        .commands
        .iter()
        .filter(|each| verb_of(each).starts_with("UID SEARCH"))
        .map(String::as_str)
        .collect();
    assert_eq!(searches[0], "N3 UID SEARCH ALL");
    let wide = searches[1];
    for key in [
        "CHARSET UTF-8",
        "UNSEEN",
        "SINCE \"01-Feb-2026\"",
        "BEFORE \"26-Nov-2026\"",
        "FROM shop@example.com",
        "SUBJECT {7}\r\nGrüße",
    ] {
        assert!(wide.contains(key), "{key} in {wide:?}");
    }
}

/// `SELECT` returns the folder's `UIDVALIDITY`, and a server that sends none is an error.
// covers: Core\Imap\Folder::uidValidity
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn uid_validity_is_returned() {
    let (mut session, thread, _dir) = signed_in(Fake {
        validity: 1_700_000_001,
        ..Fake::default()
    });
    let validity = session.select("INBOX", MEMBER).expect("the folder");
    let heard = close(session, thread);
    assert_eq!(validity, 1_700_000_001);
    assert_eq!(heard.find("SELECT"), Some("N3 SELECT INBOX"));

    let (mut session, thread, _dir) = signed_in(Fake::default());
    let Err(fault) = session.select("INBOX", MEMBER) else {
        panic!("a folder with no UIDVALIDITY was opened");
    };
    drop(close(session, thread));
    assert!(said(&fault).contains("UIDVALIDITY"), "{}", said(&fault));
}

/// A folder name is written by the codec as an atom or a quoted string, and a search text as a
/// quoted string or a literal. A quote, a CRLF or a second command in a name stays inside the
/// one command: the fake reads exactly one command for each call.
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn a_folder_name_is_sent_as_a_literal_or_quoted_string() {
    let (mut session, thread, _dir) = signed_in(Fake {
        validity: 7,
        ..Fake::default()
    });
    session
        .select("Spam\" N9 DELETE \"INBOX", MEMBER)
        .expect("the quoted name");
    session
        .select("Spam\r\nN9 DELETE INBOX", MEMBER)
        .expect("the encoded name");
    session
        .search(
            &Filter {
                subject: Some("a\" N9 DELETE INBOX\r\nN10 LOGOUT".to_owned()),
                ..Filter::default()
            },
            MEMBER,
        )
        .expect("the search");
    let heard = close(session, thread);
    let verbs: Vec<&str> = heard.commands.iter().map(|each| verb_of(each)).collect();
    assert_eq!(verbs.len(), 6, "{verbs:?}");
    assert_eq!(verbs[2], "SELECT \"Spam\\\" N9 DELETE \\\"INBOX\"");
    assert_eq!(verbs[3], "SELECT \"Spam&AA0ACg-N9 DELETE INBOX\"");
    assert_eq!(
        verbs[4],
        "UID SEARCH SUBJECT {30}\r\na\" N9 DELETE INBOX\r\nN10 LOGOUT"
    );
    assert!(!verbs.iter().any(|verb| verb.starts_with("DELETE")));
}

/// `fetch` reads `RFC822.SIZE` first. A message larger than the ceiling is an `IOError`, and its
/// body is never asked for. One that fits is read whole, without setting `\Seen`.
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn a_message_over_the_request_ceiling_is_an_error_before_it_is_read() {
    const MESSAGE: &str = "Subject: hi\r\nMessage-ID: <1@example.com>\r\n\r\nhello\r\n";
    let (mut session, thread, _dir) = signed_in(Fake {
        message: MESSAGE,
        ..Fake::default()
    });
    let small = (MESSAGE.len() - 1) as u64;
    let Err(fault) = session.fetch(42, small, MEMBER) else {
        panic!("a message over the ceiling was read");
    };
    let raw = session
        .fetch(42, MESSAGE.len() as u64, MEMBER)
        .expect("a message that fits");
    let heard = close(session, thread);
    assert!(
        said(&fault).contains(&format!("is {} bytes", MESSAGE.len())),
        "{}",
        said(&fault)
    );
    assert_eq!(raw, MESSAGE.as_bytes());
    let fetches: Vec<&str> = heard
        .commands
        .iter()
        .map(|each| verb_of(each))
        .filter(|verb| verb.starts_with("UID FETCH"))
        .collect();
    assert_eq!(
        fetches,
        [
            "UID FETCH 42 (UID RFC822.SIZE)",
            "UID FETCH 42 (UID RFC822.SIZE)",
            "UID FETCH 42 (UID BODY.PEEK[])",
        ]
    );
}

/// `session` as a request holds it, with nothing selected.
fn held(session: Session) -> crate::imap::Held {
    crate::imap::Held {
        mailbox: crate::imap::Backend::Server(session),
        selected: None,
        timeout: Duration::from_secs(10),
    }
}

/// `fetch` parses the message it read, and the message's `uid()` is the UID it was fetched
/// with. A message that `parse` reads has no UID.
// covers: Core\Imap\Folder::fetch, Core\Mail\Message::uid
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn fetch_returns_a_parsed_message_with_its_uid() {
    const MESSAGE: &str = "Subject: hi\r\nMessage-ID: <1@example.com>\r\n\r\nhello\r\n";
    let (session, thread, _dir) = signed_in(Fake {
        message: MESSAGE,
        validity: 7,
        ..Fake::default()
    });
    let mut mailbox = held(session);
    mailbox.start();
    assert_eq!(mailbox.select("INBOX", MEMBER).expect("the folder"), 7);
    let message = mailbox.fetch(42, u64::MAX, MEMBER).expect("the message");
    let class = &crate::mail::receive::MESSAGE;
    let slot = |name: &str| {
        let at = class
            .slots
            .iter()
            .position(|each| *each == name)
            .expect("a slot");
        let object = crate::instance::receiver(message, class, MEMBER).expect("a message");
        crate::instance::slot(object, at)
    };
    assert_eq!(slot("uid").as_int(), Some(42));
    assert_eq!(slot("subject").as_text(), Some("hi"));
    assert_eq!(slot("messageId").as_text(), Some("1@example.com"));
    #[expect(
        unsafe_code,
        reason = "the message was built here and nothing else holds it"
    )]
    // SAFETY: `fetch` returns one reference, owned by this test.
    unsafe {
        message.release();
    }

    let parsed = crate::mail::receive::message_of(MESSAGE.as_bytes());
    let object = crate::instance::receiver(parsed, class, MEMBER).expect("a message");
    let at = class
        .slots
        .iter()
        .position(|each| *each == "uid")
        .expect("a slot");
    assert_eq!(
        crate::instance::slot(object, at).tag(),
        Some(nvs_runtime::Tag::Null)
    );
    #[expect(
        unsafe_code,
        reason = "the message was built here and nothing else holds it"
    )]
    // SAFETY: `message_of` returns one reference, owned by this test.
    unsafe {
        parsed.release();
    }

    drop(mailbox);
    let heard = thread.join().expect("the fake server");
    assert!(
        heard.find("UID FETCH 42 (UID BODY.PEEK[])").is_some(),
        "{:?}",
        heard.commands
    );
}

/// A session filed against a request is ended with `LOGOUT` when the request's context is
/// dropped, with no call from the program.
// covers: Core\Imap::open
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn the_session_closes_when_the_request_ends() {
    let (session, thread, _dir) = signed_in(Fake {
        folders: &[r#"() "/" INBOX"#],
        ..Fake::default()
    });
    let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
    let key = ctx.hold_open_connection(None, None, Box::new(held(session)));
    let open = crate::imap::held(&mut ctx, key, MEMBER).expect("the session is filed");
    open.start();
    assert_eq!(open.folders(MEMBER).expect("the folders"), ["INBOX"]);
    drop(ctx);
    let heard = thread.join().expect("the fake server");
    assert!(
        !heard.commands.iter().any(|each| each == UNTAGGED),
        "{:?}",
        heard.commands
    );
    assert_eq!(
        heard.commands.last().map(|each| verb_of(each)),
        Some("LOGOUT"),
        "{:?}",
        heard.commands
    );
}

/// The verbs of every command after the sign-in and `SELECT`, without `LOGOUT`.
fn changes(heard: &Heard) -> Vec<&str> {
    heard
        .commands
        .iter()
        .map(|each| verb_of(each))
        .filter(|verb| verb.starts_with("UID ") || *verb == "EXPUNGE")
        .collect()
}

/// A server that offers `MOVE` gets one `UID MOVE`, and nothing is copied or expunged. A UID
/// the server finds no message for is an `IOError`.
// covers: Core\Imap\Folder::move
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn move_uses_move_when_offered() {
    let (mut session, thread, _dir) = signed_in(Fake {
        capabilities: &["MOVE", "UIDPLUS"],
        validity: 7,
        missing: &["8"],
        ..Fake::default()
    });
    session.select("INBOX", MEMBER).expect("the folder");
    session
        .move_to(7, "Archive", MEMBER)
        .expect("the message is moved");
    let Err(fault) = session.move_to(8, "Archive", MEMBER) else {
        panic!("a missing message was moved");
    };
    let heard = close(session, thread);
    assert!(
        said(&fault).contains("no message with the UID 8"),
        "{}",
        said(&fault)
    );
    assert_eq!(
        changes(&heard),
        ["UID MOVE 7 Archive", "UID MOVE 8 Archive"]
    );
}

/// A server without `MOVE` gets `UID COPY`, `\Deleted` and `UID EXPUNGE` of the one UID. A
/// server with neither `MOVE` nor `UIDPLUS` gets nothing, and the move is an `IOError`.
// covers: Core\Imap\Folder::move
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn move_falls_back_to_copy_and_uid_expunge() {
    let (mut session, thread, _dir) = signed_in(Fake {
        capabilities: &["UIDPLUS"],
        validity: 7,
        ..Fake::default()
    });
    session.select("INBOX", MEMBER).expect("the folder");
    session
        .move_to(7, "Archive", MEMBER)
        .expect("the message is moved");
    let heard = close(session, thread);
    assert_eq!(
        changes(&heard),
        [
            "UID COPY 7 Archive",
            "UID STORE 7 +FLAGS (\\Deleted)",
            "UID EXPUNGE 7",
        ]
    );

    let (mut session, thread, _dir) = signed_in(Fake {
        validity: 7,
        ..Fake::default()
    });
    session.select("INBOX", MEMBER).expect("the folder");
    let Err(fault) = session.move_to(7, "Archive", MEMBER) else {
        panic!("a server without MOVE or UIDPLUS moved a message");
    };
    let heard = close(session, thread);
    assert!(said(&fault).contains("UIDPLUS"), "{}", said(&fault));
    assert!(changes(&heard).is_empty(), "{:?}", heard.commands);
}

/// `UID STORE` adds a flag with `+FLAGS` and removes one with `-FLAGS`, a keyword as written.
/// A UID the server reports no message for is an `IOError`.
// covers: Core\Imap\Folder::markSeen, Core\Imap\Folder::markUnseen, Core\Imap\Folder::flag, Core\Imap\Folder::unflag
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn a_flag_is_set_and_cleared() {
    let (mut session, thread, _dir) = signed_in(Fake {
        validity: 7,
        missing: &["9"],
        ..Fake::default()
    });
    session.select("INBOX", MEMBER).expect("the folder");
    session
        .store(7, "\\Seen", true, MEMBER)
        .expect("the flag is set");
    session
        .store(7, "\\Seen", false, MEMBER)
        .expect("the flag is cleared");
    session
        .store(7, "$Important", true, MEMBER)
        .expect("the keyword is set");
    let Err(fault) = session.store(9, "\\Flagged", true, MEMBER) else {
        panic!("a missing message was flagged");
    };
    let heard = close(session, thread);
    assert!(
        said(&fault).contains("no message with the UID 9"),
        "{}",
        said(&fault)
    );
    assert_eq!(
        changes(&heard),
        [
            "UID STORE 7 +FLAGS (\\Seen)",
            "UID STORE 7 -FLAGS (\\Seen)",
            "UID STORE 7 +FLAGS ($Important)",
            "UID STORE 9 +FLAGS (\\Flagged)",
        ]
    );

    for (written, kept) in [
        ("\\seen", Some("\\Seen")),
        ("\\Flagged", Some("\\Flagged")),
        ("$Important", Some("$Important")),
        ("\\Deleted", None),
        ("\\Recent", None),
        ("two words", None),
        ("a)b", None),
        ("", None),
    ] {
        assert_eq!(
            super::super::flag_of(written, MEMBER).ok().as_deref(),
            kept,
            "{written}"
        );
    }
    let longest = "k".repeat(super::super::MAX_KEYWORD_BYTES);
    assert_eq!(
        super::super::flag_of(&longest, MEMBER).ok().as_deref(),
        Some(longest.as_str())
    );
    let Err(fault) = super::super::flag_of(&format!("{longest}k"), MEMBER) else {
        panic!("a keyword over the bound was taken");
    };
    assert!(
        format!("{fault:?}").contains("at most 255 bytes"),
        "{fault:?}"
    );
}

/// `delete` sets `\Deleted` and runs `UID EXPUNGE` of its own UID. A plain `EXPUNGE`, which
/// removes every message marked `\Deleted`, is never sent, and a server without `UIDPLUS` gets
/// no command at all.
// covers: Core\Imap\Folder::delete
#[cfg_attr(
    miri,
    ignore = "`ring` runs C and assembly, which Miri cannot interpret"
)]
#[test]
fn delete_expunges_only_its_own_uid() {
    let (mut session, thread, _dir) = signed_in(Fake {
        capabilities: &["UIDPLUS"],
        validity: 7,
        ..Fake::default()
    });
    session.select("INBOX", MEMBER).expect("the folder");
    session.delete(9, MEMBER).expect("the message is deleted");
    let heard = close(session, thread);
    assert_eq!(
        changes(&heard),
        ["UID STORE 9 +FLAGS (\\Deleted)", "UID EXPUNGE 9"]
    );

    let (mut session, thread, _dir) = signed_in(Fake {
        validity: 7,
        ..Fake::default()
    });
    session.select("INBOX", MEMBER).expect("the folder");
    let Err(fault) = session.delete(9, MEMBER) else {
        panic!("a server without UIDPLUS deleted a message");
    };
    let heard = close(session, thread);
    assert!(said(&fault).contains("UIDPLUS"), "{}", said(&fault));
    assert!(changes(&heard).is_empty(), "{:?}", heard.commands);
}
