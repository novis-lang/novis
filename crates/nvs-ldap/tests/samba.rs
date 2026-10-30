//! `nvs-ldap` against a real directory: the `samba-ad` domain controller in `tests/db/compose.yaml`, and a scripted peer for what that server will not do
//!
//! The server's facts are the compose file's header comment: the base
//! `DC=example,DC=test`, the administrator and its password, LDAPS on
//! `127.0.0.1:16636`, and plain LDAP on `16389`, where a cleartext bind is
//! accepted. Its certificate is the `certs` leaf, whose SAN has `localhost`,
//! and `tests/db/ca.crt` is the anchor copied out of that volume.
//!
//! **A case that needs the server skips when it is not there**, the way
//! `nvs-cli`'s `compose_postgres` does: no `tests/db/ca.crt`, or nothing
//! listening on the LDAPS port, prints a line and returns. `docker compose -f
//! tests/db/compose.yaml up` brings the server up, and then these assert. CI's
//! `database` job sets `NVS_LDAP_SAMBA`, which turns the skip into a failure.
//!
//! **Six cases use a local listener and always run.** Three prove that
//! nothing reached the wire, which only a listener on our side can show. The
//! fourth is a scripted peer that answers a cleartext bind
//! `strongerAuthRequired`: this Samba is configured to accept one, so that
//! refusal has to come from somewhere else. The fifth is a peer that answers
//! late, which no real server does on demand. The sixth is a peer whose ranged
//! attribute never ends: Samba sends a range only when one is asked for, and
//! `Core\Ldap`'s `select` takes no range option, so no program can make it.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use nvs_ldap::ber::{self, Writer, tag};
use nvs_ldap::{Connection, Endpoint, Filter, Kind, Scope, SearchRequest, Tls, Url};

const ADMIN: &str = "CN=Administrator,CN=Users,DC=example,DC=test";
const PASSWORD: &[u8] = b"Novis-test-1";
const BASE: &str = "DC=example,DC=test";
const LDAPS_PORT: u16 = 16636;
const LDAP_PORT: u16 = 16389;

/// How many Samba cases run at once on this machine, across this binary,
/// `nvs-stdlib`'s `ldap` and every worktree's run. It is `nvs-stdlib`'s
/// `SAMBA_AT_ONCE`, and the file that defines it says why.
const SAMBA_AT_ONCE: usize = 4;

/// The anchor, when the server is up and the anchor is on disk, and this
/// case's turn at the server, held until the case drops it.
#[allow(
    clippy::print_stderr,
    reason = "a green line for a test that reached no server has to say so"
)]
fn samba() -> Option<(PathBuf, nvs_repo::Turn)> {
    let ca = nvs_repo::path("tests/db/ca.crt");
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, LDAPS_PORT));
    if ca.is_file() && TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_ok() {
        return Some((ca, nvs_repo::turn("samba", SAMBA_AT_ONCE)));
    }
    assert!(
        std::env::var_os("NVS_LDAP_SAMBA").is_none(),
        "`NVS_LDAP_SAMBA` is set, and there is no `samba-ad` on 127.0.0.1:{LDAPS_PORT} \
         or no `tests/db/ca.crt`"
    );
    eprintln!(
        "skipped: no `samba-ad` on 127.0.0.1:{LDAPS_PORT}, or no `tests/db/ca.crt` — \
         `docker compose -f tests/db/compose.yaml up -d --wait samba-ad` is what this needs"
    );
    None
}

/// Opens `url` on loopback with the anchor `ca`.
fn open(
    url: &str,
    tls: Tls,
    granted: bool,
    ca: Option<&std::path::Path>,
) -> Result<Connection, nvs_ldap::Error> {
    let url = Url::parse(url).expect("a test URL parses");
    Connection::open(&Endpoint {
        address: SocketAddr::from((Ipv4Addr::LOCALHOST, url.port)),
        url: &url,
        tls,
        ca_file: ca,
        cleartext_granted: granted,
        deadline: Some(Instant::now() + Duration::from_secs(10)),
    })
}

/// A connection over LDAPS, bound as the administrator.
fn admin(ca: &std::path::Path) -> Connection {
    let mut connection = open(
        &format!("ldaps://localhost:{LDAPS_PORT}"),
        Tls::Required,
        false,
        Some(ca),
    )
    .expect("LDAPS opens");
    connection
        .bind(ADMIN, PASSWORD)
        .expect("the administrator binds");
    connection
}

/// A peer on loopback that records every byte it receives and answers each
/// whole message with what `reply` returns for its message ID.
fn peer(reply: fn(i64) -> Option<Vec<u8>>) -> (u16, mpsc::Receiver<Vec<u8>>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
    let port = listener.local_addr().expect("its address").port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
        let mut received = Vec::new();
        let mut answered = 0;
        let mut chunk = [0u8; 4096];
        loop {
            while let Ok(Some((head, length))) = ber::header(&received[answered..]) {
                let end = answered + head + length;
                if received.len() < end {
                    break;
                }
                let mut message = ber::Reader::new(&received[answered..end]);
                let mut body = ber::Reader::new(message.expect(tag::SEQUENCE).expect("a message"));
                let id = ber::integer(body.expect(tag::INTEGER).expect("an ID")).expect("an ID");
                if let Some(answer) = reply(id) {
                    stream.write_all(&answer).ok();
                }
                answered = end;
            }
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(read) => received.extend_from_slice(&chunk[..read]),
            }
        }
        tx.send(received).ok();
    });
    (port, rx)
}

#[test]
fn an_ldaps_simple_bind_and_whoami_succeed() {
    let Some((ca, _turn)) = samba() else { return };
    let mut connection = admin(&ca);
    assert!(connection.is_encrypted());
    let identity = connection.who_am_i().expect("whoami answers");
    assert!(
        identity.to_ascii_lowercase().contains("administrator"),
        "whoami answered `{identity}`"
    );
    connection.unbind().expect("the unbind is written");
}

#[test]
fn starttls_upgrades_the_connection_then_binds() {
    let Some((ca, _turn)) = samba() else { return };
    let mut connection = open(
        &format!("ldap://localhost:{LDAP_PORT}"),
        Tls::Required,
        false,
        Some(&ca),
    )
    .expect("StartTLS completes");
    assert!(
        connection.is_encrypted(),
        "`ldap://` with `tls` required is upgraded"
    );
    connection
        .bind(ADMIN, PASSWORD)
        .expect("the bind over StartTLS succeeds");
    let identity = connection.who_am_i().expect("whoami answers");
    assert!(
        identity.to_ascii_lowercase().contains("administrator"),
        "{identity}"
    );
}

#[test]
fn an_empty_password_is_refused_before_a_byte_is_sent() {
    // The scripted peer counts what reached it. The connection is plain and
    // granted, so the only thing that can stop the bind is the empty password.
    let (port, received) = peer(|_| None);
    let mut connection =
        open(&format!("ldap://127.0.0.1:{port}"), Tls::None, true, None).expect("the peer accepts");
    let error = connection
        .bind(ADMIN, b"")
        .expect_err("an empty password is refused");
    assert_eq!(error.kind(), Kind::InvalidCredentials);
    assert_eq!(error.code(), None, "the client refused it, not a server");
    drop(connection);
    let bytes = received
        .recv_timeout(Duration::from_secs(10))
        .expect("the peer reports");
    assert!(
        bytes.is_empty(),
        "the peer received {} byte(s)",
        bytes.len()
    );

    // Samba refuses the same bind itself, but AD answers it as an anonymous
    // success, so the client's refusal is the one that counts.
    if let Some((ca, _turn)) = samba() {
        let mut connection = open(
            &format!("ldaps://localhost:{LDAPS_PORT}"),
            Tls::Required,
            false,
            Some(&ca),
        )
        .expect("LDAPS opens");
        let error = connection
            .bind(ADMIN, b"")
            .expect_err("refused over LDAPS too");
        assert_eq!(
            (error.kind(), error.code()),
            (Kind::InvalidCredentials, None)
        );
    }
}

#[test]
fn a_password_with_a_nul_byte_is_refused_before_a_byte_is_sent() {
    // Samba reads a simple-bind password up to its first NUL, so each of
    // these would log in as the administrator if it reached the server.
    let mut with_nul = PASSWORD.to_vec();
    with_nul.push(0);
    let mut nul_then_more = with_nul.clone();
    nul_then_more.extend_from_slice(b"anything");
    let passwords = [with_nul, nul_then_more, b"\0".to_vec()];
    if let Some((ca, _turn)) = samba() {
        for password in &passwords {
            let mut connection = open(
                &format!("ldaps://localhost:{LDAPS_PORT}"),
                Tls::Required,
                false,
                Some(&ca),
            )
            .expect("LDAPS opens");
            let error = connection
                .bind(ADMIN, password)
                .expect_err("refused over LDAPS");
            assert_eq!(
                (error.kind(), error.code()),
                (Kind::InvalidCredentials, None),
                "the client refused it, not the server"
            );
        }
        // Another control character is sent, and the server judges it.
        let mut connection = open(
            &format!("ldaps://localhost:{LDAPS_PORT}"),
            Tls::Required,
            false,
            Some(&ca),
        )
        .expect("LDAPS opens");
        let mut with_tab = PASSWORD.to_vec();
        with_tab.push(b'\t');
        let error = connection
            .bind(ADMIN, &with_tab)
            .expect_err("a password with a tab is wrong");
        assert_eq!(error.code(), Some(49), "the server judged it");
    }

    // The scripted peer counts what reached it, as for the empty password.
    for password in &passwords {
        let (port, received) = peer(|_| None);
        let mut connection = open(&format!("ldap://127.0.0.1:{port}"), Tls::None, true, None)
            .expect("the peer accepts");
        let error = connection
            .bind(ADMIN, password)
            .expect_err("a password with a NUL byte is refused");
        assert_eq!(
            (error.kind(), error.code()),
            (Kind::InvalidCredentials, None),
            "the client refused it, not a server"
        );
        drop(connection);
        let bytes = received
            .recv_timeout(Duration::from_secs(10))
            .expect("the peer reports");
        assert!(
            bytes.is_empty(),
            "the peer received {} byte(s)",
            bytes.len()
        );
    }
}

#[test]
fn a_cleartext_bind_to_a_host_off_the_grant_is_refused_in_the_client() {
    // A listener nothing accepts on: a connection the client made would wait
    // in its queue, where the check below finds it.
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
    let port = listener.local_addr().expect("its address").port();
    let error = open(&format!("ldap://127.0.0.1:{port}"), Tls::None, false, None)
        .expect_err("`tls = \"none\"` without the grant is refused");
    assert_eq!(error.kind(), Kind::EncryptionRequired);
    assert!(error.message().contains("cleartext"), "{}", error.message());
    listener
        .set_nonblocking(true)
        .expect("a non-blocking listener");
    thread::sleep(Duration::from_millis(100));
    assert!(listener.accept().is_err(), "the client dialled the host");
}

#[test]
fn a_cleartext_bind_under_the_grant_and_the_blocks_ask_succeeds() {
    let Some((_, _turn)) = samba() else { return };
    // This connection has no TLS: the password crosses the network readable.
    let mut connection = open(
        &format!("ldap://localhost:{LDAP_PORT}"),
        Tls::None,
        true,
        None,
    )
    .expect("a granted plain connection opens");
    assert!(!connection.is_encrypted());
    connection
        .bind(ADMIN, PASSWORD)
        .expect("Samba accepts the cleartext bind");
    let identity = connection.who_am_i().expect("whoami answers");
    assert!(
        identity.to_ascii_lowercase().contains("administrator"),
        "{identity}"
    );
}

#[test]
fn a_controller_refusing_a_cleartext_bind_is_encryption_required() {
    // What AD and an unconfigured Samba send for a simple bind on 389.
    fn stronger_auth_required(id: i64) -> Option<Vec<u8>> {
        let mut out = Writer::new();
        out.constructed(tag::SEQUENCE, |msg| {
            msg.integer(tag::INTEGER, id);
            msg.constructed(0x61, |result| {
                result.integer(tag::ENUMERATED, 8);
                result.octets(tag::OCTET_STRING, b"");
                result.octets(
                    tag::OCTET_STRING,
                    b"BindSimple: Transport encryption required.",
                );
            });
        });
        Some(out.into_bytes())
    }
    let (port, _received) = peer(stronger_auth_required);
    let mut connection =
        open(&format!("ldap://127.0.0.1:{port}"), Tls::None, true, None).expect("the peer accepts");
    let error = connection
        .bind(ADMIN, PASSWORD)
        .expect_err("the peer refuses the bind");
    assert_eq!(error.kind(), Kind::EncryptionRequired);
    assert_eq!(error.code(), Some(8));
    assert!(
        !error.message().contains("Novis-test-1"),
        "the message carries the password: {}",
        error.message()
    );
}

/// A request whose deadline passed leaves its answer on the wire. The next
/// request reads it, drops it and reads its own.
#[test]
fn a_late_answer_to_a_request_that_timed_out_is_dropped() {
    // Answers every bind with success, and the first one a second late.
    fn late_first(id: i64) -> Option<Vec<u8>> {
        if id == 1 {
            thread::sleep(Duration::from_secs(1));
        }
        let mut out = Writer::new();
        out.constructed(tag::SEQUENCE, |msg| {
            msg.integer(tag::INTEGER, id);
            msg.constructed(0x61, |result| {
                result.integer(tag::ENUMERATED, 0);
                result.octets(tag::OCTET_STRING, b"");
                result.octets(tag::OCTET_STRING, b"");
            });
        });
        Some(out.into_bytes())
    }
    let (port, _received) = peer(late_first);
    let mut connection =
        open(&format!("ldap://127.0.0.1:{port}"), Tls::None, true, None).expect("the peer accepts");
    connection.set_deadline(Some(Instant::now() + Duration::from_millis(200)));
    let error = connection
        .bind(ADMIN, PASSWORD)
        .expect_err("the first answer is too late");
    assert_eq!(error.kind(), Kind::Timeout);
    connection.set_deadline(Some(Instant::now() + Duration::from_secs(10)));
    connection
        .bind(ADMIN, PASSWORD)
        .expect("the second bind reads past the late answer");
}

#[test]
fn a_ranged_attribute_that_never_ends_stops_at_the_memory_ceiling() {
    const CEILING: usize = 8 << 20;
    const PER_ANSWER: u64 = 100;
    // Answers the search with one value of a range, and every follow-up with
    // the next hundred values of 10 KB each, never with the range's end. Past
    // 200 answers it stops answering, so a loop the ceiling did not stop
    // ends at its deadline instead of filling the machine.
    fn endless(id: i64) -> Option<Vec<u8>> {
        let id = u64::try_from(id).ok()?;
        if id > 200 {
            return None;
        }
        let (name, count) = if id == 1 {
            ("description;range=0-0".to_owned(), 1)
        } else {
            let from = 1 + (id - 2) * PER_ANSWER;
            (
                format!("description;range={from}-{}", from + PER_ANSWER - 1),
                PER_ANSWER,
            )
        };
        let value = vec![b'x'; 10_000];
        let mut out = Writer::new();
        out.constructed(tag::SEQUENCE, |msg| {
            msg.integer(tag::INTEGER, i64::try_from(id).unwrap_or(0));
            msg.constructed(0x64, |entry| {
                entry.octets(tag::OCTET_STRING, b"OU=Range,DC=example,DC=test");
                entry.constructed(tag::SEQUENCE, |attributes| {
                    attributes.constructed(tag::SEQUENCE, |attribute| {
                        attribute.octets(tag::OCTET_STRING, name.as_bytes());
                        attribute.constructed(tag::SET, |values| {
                            for _ in 0..count {
                                values.octets(tag::OCTET_STRING, &value);
                            }
                        });
                    });
                });
            });
        });
        out.constructed(tag::SEQUENCE, |msg| {
            msg.integer(tag::INTEGER, i64::try_from(id).unwrap_or(0));
            msg.constructed(0x65, |result| {
                result.integer(tag::ENUMERATED, 0);
                result.octets(tag::OCTET_STRING, b"");
                result.octets(tag::OCTET_STRING, b"");
            });
        });
        Some(out.into_bytes())
    }
    let (port, received_by_peer) = peer(endless);
    let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Buffer(Vec::new()));
    ctx.set_memory_limit(CEILING);
    let mut connection =
        open(&format!("ldap://127.0.0.1:{port}"), Tls::None, true, None).expect("the peer accepts");
    let filter = Filter::Present("objectClass".into());
    let error = connection
        .search(&SearchRequest {
            base: BASE,
            scope: Scope::Base,
            filter: &filter,
            attributes: &["description"],
            page_size: 1,
            size_limit: 0,
            time_limit: 0,
            sort: None,
            window: None,
            show_deleted: false,
        })
        .next()
        .expect("the search returns its entry or an error")
        .expect_err("a range that never ends cannot fit under the ceiling");
    assert_eq!(error.kind(), Kind::SizeLimitExceeded, "{error}");
    let peak = usize::try_from(nvs_runtime::budget::peak_bytes()).unwrap_or(0);
    assert!(
        peak <= CEILING + CEILING / 2 + (8 << 20),
        "the merged list reached {peak} bytes under a ceiling of {CEILING}"
    );
    drop(connection);
    let received = received_by_peer
        .recv()
        .expect("the peer reports what it read");
    let asked = received
        .windows(b"description;range=".len())
        .filter(|window| *window == b"description;range=")
        .count();
    assert!(
        asked > 2,
        "the client asked {asked} follow-up range(s), so the merge never grew"
    );
    drop(ctx);
}

#[test]
fn a_search_pages_past_a_thousand_entries_holding_one_page() {
    let Some((ca, _turn)) = samba() else { return };
    let mut connection = admin(&ca);
    // The schema partition has well over a thousand entries on every Samba
    // and AD, so a page size of 1000 needs at least two pages.
    let filter = Filter::Present("objectClass".into());
    let base = format!("CN=Schema,CN=Configuration,{BASE}");
    let mut search = connection.search(&SearchRequest {
        base: &base,
        scope: Scope::One,
        filter: &filter,
        attributes: &["cn"],
        page_size: 1000,
        size_limit: 0,
        time_limit: 0,
        sort: None,
        window: None,
        show_deleted: false,
    });
    let mut count = 0;
    let mut most_held = 0;
    while let Some(entry) = search.next() {
        let entry = entry.expect("every page arrives");
        assert!(entry.dn.ends_with(&base), "{}", entry.dn);
        most_held = most_held.max(search.held() + 1);
        count += 1;
    }
    assert!(count > 1000, "the schema returned {count} entries");
    assert!(
        search.pages() >= 2,
        "{count} entries came in {} page(s)",
        search.pages()
    );
    assert!(
        most_held <= 1000,
        "the search held {most_held} entries at once"
    );
    drop(search);
    assert!(
        connection.is_settled(),
        "a finished search leaves the connection settled"
    );
}

// covers: Core\Ldap\Entries::references
#[test]
fn a_continuation_reference_is_returned_and_not_followed() {
    let Some((ca, _turn)) = samba() else { return };
    let mut connection = admin(&ca);
    // A subtree search from the domain root crosses into the configuration
    // and DNS partitions, which the server names by reference.
    let filter = Filter::Equal("objectClass".into(), b"domain".to_vec());
    let mut search = connection.search(&SearchRequest {
        base: BASE,
        scope: Scope::Subtree,
        filter: &filter,
        attributes: &["1.1"],
        page_size: 1000,
        size_limit: 0,
        time_limit: 0,
        sort: None,
        window: None,
        show_deleted: false,
    });
    let entries: Vec<_> = search
        .by_ref()
        .map(|entry| entry.expect("the search succeeds").dn)
        .collect();
    assert_eq!(entries, [BASE], "only the entry under the base is returned");
    let references = search.references();
    assert!(
        references
            .iter()
            .any(|url| url.ends_with(&format!("CN=Configuration,{BASE}"))),
        "the references were {references:?}"
    );
    assert!(
        references.iter().all(|url| url.starts_with("ldap")),
        "{references:?}"
    );
}
