//! `Core\Ldap`'s connections, run through `nvs_stdlib::ldap`: which grant each of `connect` and `open` asks, the address policy `open` passes and `connect` does not, the pool's identity, and what `search` and `read` return
//!
//! The grant and policy cases need no server: every refusal they assert
//! happens before a byte is sent, and the one dial they make goes to a port
//! nothing listens on. The size limit case talks to a scripted peer on
//! loopback, because Samba ignores the limit a client sends. The pool and
//! search cases need the `samba-ad` service from `tests/db/compose.yaml`, and
//! skip when nothing listens on its LDAPS port unless `NVS_LDAP_SAMBA` is set,
//! as `crates/nvs-ldap/tests/samba.rs` does.

use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use nvs_config::tree::{CapLdap, Capabilities, Config, LdapDirectory, Setting};
use nvs_ldap::ber::{self, Writer, tag};
use nvs_ldap::{Dn, Filter, Scope, SearchRequest};
use nvs_runtime::HeldConnection as _;
use nvs_stdlib::ldap;

const BASE: &str = "DC=example,DC=test";
const ADMIN: &str = "CN=Administrator,CN=Users,DC=example,DC=test";
const PASSWORD: &str = "Novis-test-1";
const LDAPS_PORT: u16 = 16636;

/// The anchor, when the server is up and the anchor is on disk.
#[allow(
    clippy::print_stderr,
    reason = "a green line for a test that reached no server has to say so"
)]
fn samba() -> Option<PathBuf> {
    let ca = nvs_repo::path("tests/db/ca.crt");
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, LDAPS_PORT));
    if ca.is_file() && TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_ok() {
        return Some(ca);
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

/// A throw's class and message as one line, which is what these cases assert on.
trait Said {
    fn message(&self) -> String;
}

impl Said for nvs_runtime::Fault {
    fn message(&self) -> String {
        format!("{self:?}")
    }
}

fn list(items: &[&str]) -> Option<Setting> {
    Some(Setting::List(
        items.iter().map(|&item| item.to_owned()).collect(),
    ))
}

/// A configuration with `blocks` under `[ldap.<name>]` and `grants` under `[capabilities.ldap]`.
fn snapshot(blocks: Vec<(&str, LdapDirectory)>, grants: CapLdap) -> Arc<nvs_config::Snapshot> {
    Arc::new(nvs_config::Snapshot {
        config: Config {
            capabilities: Some(Capabilities {
                ldap: Some(grants),
                ..Capabilities::default()
            }),
            ldap: blocks
                .into_iter()
                .map(|(name, block)| (name.to_owned(), block))
                .collect::<BTreeMap<_, _>>(),
            ..Config::default()
        },
        ..nvs_config::Snapshot::default()
    })
}

fn ctx_over(snapshot: &Arc<nvs_config::Snapshot>) -> nvs_runtime::Ctx {
    let mut ctx = nvs_runtime::Ctx::buffered();
    ctx.set_config(Arc::clone(snapshot));
    ctx
}

/// A block bound as the administrator at `url`.
fn block_at(url: &str, ca: Option<&PathBuf>) -> LdapDirectory {
    LdapDirectory {
        url: Some(Setting::Text(url.to_owned())),
        user: Some(ADMIN.to_owned()),
        password: Some(PASSWORD.to_owned()),
        tls_ca_file: ca.map(|path| path.to_string_lossy().into_owned()),
        timeout: Some(Setting::Text("5s".to_owned())),
        ..LdapDirectory::default()
    }
}

// covers: Core\Ldap::connect
#[test]
fn connect_reads_its_block_and_needs_the_ldap_connect_grant() {
    // Port 1 on loopback: nothing listens there, so the dial fails at once.
    let closed = "ldaps://127.0.0.1:1";

    let ungranted = snapshot(vec![("corp", block_at(closed, None))], CapLdap::default());
    let refused = ldap::connect(&mut ctx_over(&ungranted), "corp")
        .expect_err("no `ldap.connect` grant, so no connection");
    assert!(
        refused.message().contains("`ldap.connect`"),
        "the refusal names the grant: {}",
        refused.message()
    );

    let granted = snapshot(
        vec![("corp", block_at(closed, None))],
        CapLdap {
            connect: list(&["corp", "staff"]),
            ..CapLdap::default()
        },
    );
    let outside = ldap::connect(&mut ctx_over(&granted), "shop")
        .expect_err("the grant names blocks, and `shop` is not one of them");
    assert!(
        outside.message().contains("`ldap.connect`"),
        "{}",
        outside.message()
    );

    let unwritten = ldap::connect(&mut ctx_over(&granted), "staff")
        .expect_err("`staff` is granted and no block sets it up");
    assert!(
        unwritten.message().contains("`[ldap.staff]`"),
        "a granted name with no block says so: {}",
        unwritten.message()
    );

    // The block is read and its URL dialled. Loopback is a denied range for a
    // program's own address, and this one is the operator's, so the dial is
    // what fails and not the policy.
    let unanswered = ldap::connect(&mut ctx_over(&granted), "corp")
        .expect_err("nothing listens at the block's URL");
    assert!(
        unanswered
            .message()
            .contains("no URL in `[ldap.corp]` answered")
            && !unanswered.message().contains("net.internal"),
        "the block's own address is dialled with no policy in front of it: {}",
        unanswered.message()
    );
}

// covers: Core\Ldap::open
#[test]
fn open_needs_the_ldap_open_grant_and_passes_the_address_policy() {
    let settings = ldap::Settings {
        url: "ldaps://127.0.0.1:1",
        user: Some(ADMIN),
        password: PASSWORD.as_bytes(),
        tls: nvs_config::ldap::Tls::Required,
        tls_ca_file: None,
        timeout: Some(Duration::from_secs(5)),
        base: None,
    };

    let ungranted = snapshot(Vec::new(), CapLdap::default());
    let refused = ldap::open(&mut ctx_over(&ungranted), &settings)
        .expect_err("no `ldap.open` grant, so no connection");
    assert!(
        refused.message().contains("`ldap.open`") && refused.message().contains("127.0.0.1"),
        "the refusal names the grant and the host: {}",
        refused.message()
    );

    // The host is granted by name, and its address is still loopback, which
    // `rule:security/net-address-policy` denies to a program-supplied target.
    let granted = snapshot(
        Vec::new(),
        CapLdap {
            open: list(&["127.0.0.1"]),
            ..CapLdap::default()
        },
    );
    let denied = ldap::open(&mut ctx_over(&granted), &settings)
        .expect_err("a granted host at a denied address does not open");
    assert!(
        denied.message().contains("net.internal"),
        "the refusal names the key that would allow the range: {}",
        denied.message()
    );

    let unparsed = ldap::open(
        &mut ctx_over(&granted),
        &ldap::Settings {
            url: "https://127.0.0.1",
            ..settings
        },
    )
    .expect_err("only `ldap://` and `ldaps://` are URLs `open` dials");
    assert!(
        unparsed.message().contains("`url`"),
        "{}",
        unparsed.message()
    );
}

/// A configuration with one `[ldap.corp]` block at the test server, granted by name.
fn corp(ca: &PathBuf) -> Arc<nvs_config::Snapshot> {
    let url = format!("ldaps://localhost:{LDAPS_PORT}");
    snapshot(
        vec![("corp", block_at(&url, Some(ca)))],
        CapLdap {
            connect: list(&["corp"]),
            ..CapLdap::default()
        },
    )
}

/// A subtree search from the domain root for every entry, `page_size` to a page.
fn everything(filter: &Filter, page_size: u32, size_limit: u32) -> SearchRequest<'_> {
    SearchRequest {
        base: BASE,
        scope: Scope::Subtree,
        filter,
        attributes: &["cn"],
        page_size,
        size_limit,
        time_limit: 0,
        sort: None,
        window: None,
        show_deleted: false,
    }
}

#[test]
fn a_pooled_connection_is_always_bound_as_its_block() {
    let Some(ca) = samba() else { return };
    let config = corp(&ca);

    // The same request asks twice and gets the one connection it already holds.
    let mut first = ctx_over(&config);
    let key = ldap::connect(&mut first, "corp").expect("the block opens");
    assert_eq!(ldap::connect(&mut first, "corp").expect("held"), key);
    let bound = ldap::who_am_i(&mut first, key).expect("whoami answers");
    assert!(
        bound.to_ascii_lowercase().contains("administrator"),
        "the connection is bound as the block's user: {bound}"
    );

    // The request ends, and the connection goes back to this core's pool. The
    // next request on the core is served from it, bound as the same user.
    drop(first);
    for _ in 0..3 {
        let mut next = ctx_over(&config);
        let key = ldap::connect(&mut next, "corp").expect("the block opens again");
        assert_eq!(
            ldap::who_am_i(&mut next, key).expect("whoami answers"),
            bound,
            "a connection taken from the pool is bound as the block, every time"
        );
    }

    // A search read to its end leaves the connection poolable. One left with
    // pages still on the server does not, so teardown closes it.
    let every = Filter::Present("objectClass".to_owned());
    let mut ctx = ctx_over(&config);
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let mut finished =
        ldap::search(&mut ctx, key, &everything(&every, 1000, 0)).expect("the search starts");
    while finished
        .next(&mut ctx)
        .expect("the search succeeds")
        .is_some()
    {}
    assert!(
        ldap::held(&mut ctx, key, "test")
            .expect("open")
            .is_poolable(),
        "a search read to its end leaves the connection settled"
    );
    let mut paging =
        ldap::search(&mut ctx, key, &everything(&every, 2, 0)).expect("the search starts");
    assert!(paging.next(&mut ctx).expect("one entry").is_some());
    drop(paging);
    assert!(
        !ldap::held(&mut ctx, key, "test")
            .expect("open")
            .is_poolable(),
        "a search left paging keeps the connection out of the pool"
    );
}

#[test]
fn an_attribute_name_matches_without_case_and_keeps_the_servers_case() {
    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");

    let entry = ldap::read(&mut ctx, key, ADMIN, &["SAMACCOUNTNAME"])
        .expect("the read succeeds")
        .expect("the administrator exists");
    assert_eq!(
        entry.get("samaccountname"),
        Some(&[b"Administrator".to_vec()][..])
    );
    let names: Vec<&str> = entry.attributes.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        ["sAMAccountName"],
        "the name is kept as the server spells it, whatever case the request used"
    );

    let missing = format!("CN=Nobody,CN=Users,{BASE}");
    assert_eq!(
        ldap::read(&mut ctx, key, &missing, &[]).expect("no such object is not an error"),
        None
    );
    assert!(
        ldap::held(&mut ctx, key, "test")
            .expect("open")
            .is_poolable(),
        "a read, found or not, leaves the connection settled"
    );
}

/// A peer on loopback that reads one whole message and answers it with
/// `answer`, then holds the connection until the client closes it.
fn peer(answer: Vec<u8>) -> u16 {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
    let port = listener.local_addr().expect("its address").port();
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
        let mut received = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut answered = false;
        loop {
            if !answered
                && matches!(ber::header(&received), Ok(Some((head, length))) if received.len() >= head + length)
            {
                stream.write_all(&answer).ok();
                answered = true;
            }
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => return,
                Ok(read) => received.extend_from_slice(&chunk[..read]),
            }
        }
    });
    port
}

#[test]
fn a_size_limit_the_server_hit_throws() {
    // One entry, then a `SearchResultDone` with `sizeLimitExceeded` (4): what
    // a directory sends when the search reached the limit it was given.
    let mut answer = Writer::new();
    answer.constructed(tag::SEQUENCE, |msg| {
        msg.integer(tag::INTEGER, 1);
        msg.constructed(0x64, |entry| {
            entry.octets(tag::OCTET_STRING, b"CN=One,DC=example,DC=test");
            entry.constructed(tag::SEQUENCE, |_| {});
        });
    });
    answer.constructed(tag::SEQUENCE, |msg| {
        msg.integer(tag::INTEGER, 1);
        msg.constructed(0x65, |done| {
            done.integer(tag::ENUMERATED, 4);
            done.octets(tag::OCTET_STRING, b"");
            done.octets(tag::OCTET_STRING, b"Size limit exceeded");
        });
    });
    let port = peer(answer.into_bytes());

    // An anonymous block over plain LDAP on loopback, which the cleartext
    // grant allows, so the search is the first message the peer reads.
    let config = snapshot(
        vec![(
            "corp",
            LdapDirectory {
                url: Some(Setting::Text(format!("ldap://127.0.0.1:{port}"))),
                tls: Some("none".to_owned()),
                timeout: Some(Setting::Text("5s".to_owned())),
                ..LdapDirectory::default()
            },
        )],
        CapLdap {
            connect: list(&["corp"]),
            cleartext: list(&["127.0.0.1"]),
            ..CapLdap::default()
        },
    );
    let mut ctx = ctx_over(&config);
    let key = ldap::connect(&mut ctx, "corp").expect("the peer accepts");

    // The page arrived with the limit, so the entry in it is not returned:
    // the search throws before the program reads anything.
    let every = Filter::Present("objectClass".to_owned());
    let stopped = ldap::search(&mut ctx, key, &everything(&every, 1000, 1))
        .expect_err("a search the server stopped at its size limit throws");
    assert!(
        stopped.message().contains("SizeLimitExceeded"),
        "the limit throws as its kind: {}",
        stopped.message()
    );
    assert!(
        ldap::held(&mut ctx, key, "test")
            .expect("open")
            .is_poolable(),
        "the server finished the search, so the connection is settled"
    );
}

/// The DNs a subtree search from the domain root returns for `filter`.
fn found(ctx: &mut nvs_runtime::Ctx, key: u64, filter: &Filter) -> Vec<String> {
    let mut entries =
        ldap::search(ctx, key, &everything(filter, 1000, 0)).expect("the search starts");
    let mut dns = Vec::new();
    while let Some(entry) = entries.next(ctx).expect("the search succeeds") {
        dns.push(entry.dn);
    }
    dns
}

// covers: Core\Ldap\Filter::all, Core\Ldap\Filter::startsWith, Core\Ldap\Filter::toString
#[test]
fn a_filter_encodes_to_ber_with_no_text_step() {
    // `(&(objectClass=user)(sAMAccountName=Admin*))`, as RFC 4511 § 4.5.1
    // encodes it: every byte below is a tag, a length, or a value's own byte.
    let filter = Filter::And(vec![
        Filter::Equal("objectClass".to_owned(), b"user".to_vec()),
        Filter::Substrings {
            attribute: "sAMAccountName".to_owned(),
            initial: Some(b"Admin".to_vec()),
            any: Vec::new(),
            last: None,
        },
    ]);
    let mut expected = vec![0xa0, 0x30, 0xa3, 0x13, 0x04, 0x0b];
    expected.extend_from_slice(b"objectClass");
    expected.extend_from_slice(&[0x04, 0x04]);
    expected.extend_from_slice(b"user");
    expected.extend_from_slice(&[0xa4, 0x19, 0x04, 0x0e]);
    expected.extend_from_slice(b"sAMAccountName");
    expected.extend_from_slice(&[0x30, 0x07, 0x80, 0x05]);
    expected.extend_from_slice(b"Admin");
    assert_eq!(filter.to_ber(), expected);
    assert_eq!(
        Filter::from_ber(&expected).expect("the encoding reads back"),
        filter
    );
    // The text form is rendered from the tree for a log. A search sends the
    // bytes above, never this.
    assert_eq!(
        filter.to_text(),
        "(&(objectClass=user)(sAMAccountName=Admin*))"
    );

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    assert_eq!(
        found(&mut ctx, key, &Filter::Encoded(filter.to_ber())),
        [ADMIN],
        "the server reads the encoding as the filter it was built from"
    );
}

// covers: Core\Ldap\Filter::equals
#[test]
fn a_tainted_value_cannot_change_a_filters_structure() {
    // As RFC 4515 text, this value closes the equality filter and opens one
    // that every entry matches. As a value, it is one octet string.
    let hostile = b"*)(objectClass=*".to_vec();
    let filter = Filter::And(vec![
        Filter::Equal("objectClass".to_owned(), b"user".to_vec()),
        Filter::Equal("sAMAccountName".to_owned(), hostile.clone()),
    ]);
    let Filter::And(parts) = Filter::from_ber(&filter.to_ber()).expect("reads back") else {
        panic!("the outer element is still the `and`");
    };
    assert_eq!(parts.len(), 2, "the value added no filter");
    assert_eq!(
        parts[1],
        Filter::Equal("sAMAccountName".to_owned(), hostile)
    );
    // The log text escapes it, so it reads back as the same one value.
    assert_eq!(
        filter.to_text(),
        "(&(objectClass=user)(sAMAccountName=\\2a\\29\\28objectClass=\\2a))"
    );

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    assert!(
        found(&mut ctx, key, &filter).is_empty(),
        "no account is named `*)(objectClass=*`, so nothing matches"
    );
}

// covers: Core\Ldap\Filter::endsWith, Core\Ldap\Filter::contains, Core\Ldap\Filter::present
// covers: Core\Ldap\Filter::atLeast, Core\Ldap\Filter::atMost, Core\Ldap\Filter::approx
// covers: Core\Ldap\Filter::any, Core\Ldap\Filter::parse
#[test]
fn every_filter_kind_reads_back_from_the_text_it_writes() {
    // One of each kind `Ldap\Filter` builds, with a value that needs escaping.
    let name = || "cn".to_owned();
    let filter = Filter::Or(vec![
        Filter::Substrings {
            attribute: name(),
            initial: None,
            any: Vec::new(),
            last: Some(b"son)".to_vec()),
        },
        Filter::Substrings {
            attribute: name(),
            initial: None,
            any: vec![b"a*b".to_vec()],
            last: None,
        },
        Filter::Present("mail".to_owned()),
        Filter::GreaterOrEqual("uSNChanged".to_owned(), b"100".to_vec()),
        Filter::LessOrEqual("uSNChanged".to_owned(), b"200".to_vec()),
        Filter::Approx("sn".to_owned(), b"Smith".to_vec()),
    ]);
    let text = filter.to_text();
    assert_eq!(
        text,
        "(|(cn=*son\\29)(cn=*a\\2ab*)(mail=*)(uSNChanged>=100)(uSNChanged<=200)(sn~=Smith))"
    );
    // `parse` of the text is the filter it was written from, byte for byte.
    let parsed = Filter::parse(&text).expect("the text reads back");
    assert_eq!(parsed.to_ber(), filter.to_ber());
}

// covers: Core\Ldap\Filter::not, Core\Ldap\Filter::parse, Core\Ldap\Filter::toString
#[test]
fn a_filter_nests_at_most_max_filter_depth_levels() {
    let nested = |levels: usize| {
        let mut filter = Filter::Present("mail".to_owned());
        for _ in 1..levels {
            filter = Filter::Not(Box::new(Filter::Encoded(filter.to_ber())));
        }
        filter.to_ber()
    };
    let deepest = nested(nvs_ldap::MAX_FILTER_DEPTH);
    assert_eq!(
        Filter::depth_of(&deepest).expect("the limit itself is allowed"),
        nvs_ldap::MAX_FILTER_DEPTH
    );
    let past = nested(nvs_ldap::MAX_FILTER_DEPTH + 1);
    assert!(
        Filter::depth_of(&past).is_err(),
        "one level more is refused"
    );
    assert!(Filter::from_ber(&past).is_err(), "and does not decode");
    // `toString` of such bytes writes `(?)` and does not recurse past the limit.
    assert_eq!(Filter::Encoded(past).to_text(), "(?)");

    // The text form has the same limit, and the error is at the `(` past it.
    let text = |levels: usize| {
        format!(
            "{}(mail=*){}",
            "(!".repeat(levels - 1),
            ")".repeat(levels - 1)
        )
    };
    let read = Filter::parse(&text(nvs_ldap::MAX_FILTER_DEPTH)).expect("the limit reads");
    assert_eq!(read.to_ber(), deepest);
    let error = Filter::parse(&text(nvs_ldap::MAX_FILTER_DEPTH + 1)).expect_err("past it");
    assert_eq!(error.position, 2 * nvs_ldap::MAX_FILTER_DEPTH + 1);
    // A text a hundred thousand levels deep stops at the same place.
    let error = Filter::parse(&text(100_000)).expect_err("far past it");
    assert_eq!(error.position, 2 * nvs_ldap::MAX_FILTER_DEPTH + 1);
}

#[test]
fn ad_member_of_nested_uses_the_in_chain_rule() {
    // `Ldap\Ad::memberOf($group, {nested: true})`: RFC 4511's extensible
    // match, with AD's `LDAP_MATCHING_RULE_IN_CHAIN` and the DN as its value.
    let rule = "1.2.840.113556.1.4.1941";
    let staff = "CN=Staff,DC=example,DC=test";
    let nested = |group: &str| Filter::Extensible {
        rule: Some(rule.to_owned()),
        attribute: Some("memberOf".to_owned()),
        value: group.as_bytes().to_vec(),
        dn_attributes: false,
    };
    let filter = nested(staff);
    let mut expected = vec![0xa9, 0x40, 0x81, 0x17];
    expected.extend_from_slice(rule.as_bytes());
    expected.extend_from_slice(&[0x82, 0x08]);
    expected.extend_from_slice(b"memberOf");
    expected.extend_from_slice(&[0x83, 0x1b]);
    expected.extend_from_slice(staff.as_bytes());
    assert_eq!(filter.to_ber(), expected);
    assert_eq!(
        Filter::from_ber(&expected).expect("the encoding reads back"),
        filter
    );
    assert_eq!(
        filter.to_text(),
        "(memberOf:1.2.840.113556.1.4.1941:=CN=Staff,DC=example,DC=test)"
    );

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let admin = ldap::read(&mut ctx, key, ADMIN, &["memberOf"])
        .expect("the read succeeds")
        .expect("the entry exists");
    let group = admin
        .get("memberOf")
        .and_then(|groups| {
            groups
                .iter()
                .find(|group| group.starts_with(b"CN=Domain Admins,"))
        })
        .map(|group| String::from_utf8(group.clone()).expect("a DN is UTF-8"))
        .expect("the administrator is in `Domain Admins`");
    let direct = Filter::Equal("memberOf".to_owned(), group.clone().into_bytes());
    assert!(
        found(&mut ctx, key, &direct).iter().any(|dn| dn == ADMIN),
        "the direct form finds the administrator"
    );
    // The server accepts the rule. What it returns for it is the server's
    // own evaluation, so only the request's success is asserted.
    found(&mut ctx, key, &Filter::Encoded(nested(&group).to_ber()));
}

#[test]
fn dn_escapes_each_value_and_round_trips() {
    let users = Dn::parse("CN=Users, DC=example, DC=test").expect("an operator's DN parses");
    assert_eq!(users.to_text(), "CN=Users,DC=example,DC=test");

    // Concatenated into text, this value would end the first level and name
    // `CN=Users` as the second. Built from parts, it is one value.
    let hostile = "Administrator,CN=Users";
    let dn = users.child("CN", hostile).expect("builds");
    assert_eq!(
        dn.to_text(),
        "CN=Administrator\\,CN\\=Users,CN=Users,DC=example,DC=test"
    );
    assert_eq!(dn.rdns().len(), 4, "the value added no level");
    assert_eq!(dn.rdn().first().value, hostile);
    assert_eq!(Dn::parse(&dn.to_text()), Ok(dn.clone()));
    assert!(dn.is_within(&users));
    assert_eq!(dn.parent(), Some(users.clone()));

    // Every character RFC 4514 § 2.4 names, each at the place it matters.
    for value in [
        " lead", "trail ", "#hash", "a+b", "a;b", "a<b>", "a\"b", "a\\b", "a\nb", "Åsa",
    ] {
        let dn = Dn::of("CN", value).expect("builds");
        let back = Dn::parse(&dn.to_text()).expect("reads back");
        assert_eq!(back.rdn().first().value, value, "{}", dn.to_text());
        assert_eq!(back, dn);
    }
    assert_eq!(Dn::of("CN;x", "a"), Err(nvs_ldap::PartError::Attribute));
    assert_eq!(Dn::of("CN", ""), Err(nvs_ldap::PartError::EmptyValue));

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let admin = users.child("CN", "Administrator").expect("builds");
    let entry = ldap::read(&mut ctx, key, &admin.to_text(), &[])
        .expect("the read succeeds")
        .expect("the entry exists");
    assert_eq!(
        Dn::parse(&entry.dn),
        Ok(admin),
        "the server's DN reads as the one built"
    );
    assert!(
        ldap::read(&mut ctx, key, &dn.to_text(), &[])
            .expect("the read succeeds")
            .is_none(),
        "no entry is named `Administrator,CN=Users`, so nothing is read"
    );
}

/// The one value of `name` on the administrator's entry, read with `select`
/// naming it alone.
fn admin_value(ctx: &mut nvs_runtime::Ctx, key: u64, dn: &str, name: &str) -> Vec<u8> {
    let entry = ldap::read(ctx, key, dn, &[name])
        .expect("the read succeeds")
        .expect("the entry exists");
    let values = entry.get(name).expect("the entry has the attribute");
    assert_eq!(values.len(), 1, "`{name}` has one value");
    values[0].clone()
}

#[test]
fn object_guid_reads_as_a_uuid_with_ads_byte_order() {
    use nvs_ldap::value::{ValueError, uuid_from_guid};

    // MS-DTYP § 2.3.4's own example: `{6F9619FF-8B86-D011-B42D-00C04FC964FF}`
    // is stored with its first three groups little-endian.
    let stored = [
        0xff, 0x19, 0x96, 0x6f, 0x86, 0x8b, 0x11, 0xd0, 0xb4, 0x2d, 0x00, 0xc0, 0x4f, 0xc9, 0x64,
        0xff,
    ];
    let octets = uuid_from_guid(&stored).expect("16 bytes");
    assert_eq!(
        octets,
        [
            0x6f, 0x96, 0x19, 0xff, 0x8b, 0x86, 0xd0, 0x11, 0xb4, 0x2d, 0x00, 0xc0, 0x4f, 0xc9,
            0x64, 0xff
        ]
    );
    assert_eq!(
        uuid_from_guid(&octets),
        Ok(stored),
        "the swap is its own inverse"
    );
    assert_eq!(uuid_from_guid(&stored[..15]), Err(ValueError::NotAGuid));
    assert_eq!(uuid_from_guid(&[0; 17]), Err(ValueError::NotAGuid));

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let guid = admin_value(&mut ctx, key, ADMIN, "objectGUID");
    let octets = uuid_from_guid(&guid).expect("the server sent 16 bytes");
    // A directory issues random GUIDs, version 4. The version is the high
    // nibble of octet 6 only once the third group is swapped back.
    assert_eq!(octets[6] >> 4, 4, "{octets:02x?}");
    assert_eq!(octets[8] >> 6, 0b10, "the RFC 9562 variant, {octets:02x?}");
}

#[test]
fn object_sid_reads_as_its_s_1_5_21_text() {
    use nvs_ldap::{Sid, SidTextError, ValueError};

    // BUILTIN\Administrators, MS-DTYP § 2.4.2.4.
    let builtin = [1, 2, 0, 0, 0, 0, 0, 5, 0x20, 0, 0, 0, 0x20, 0x02, 0, 0];
    let sid = Sid::from_bytes(&builtin).expect("a SID");
    assert_eq!(sid.to_text(), "S-1-5-32-544");
    assert_eq!(sid.to_bytes(), builtin);
    assert_eq!(sid.rid(), 544);
    assert_eq!(
        sid.domain().map(|d| d.to_text()).as_deref(),
        Some("S-1-5-32")
    );
    assert_eq!(Sid::parse("S-1-5-32-544"), Ok(sid));
    assert_eq!(Sid::parse("S-1-1-0").expect("Everyone").domain(), None);
    assert_eq!(
        Sid::parse("S-1-0x010000000000-7").map(|sid| sid.to_text()),
        Ok("S-1-0x010000000000-7".to_owned())
    );
    assert_eq!(Sid::parse("s-1-5-32"), Err(SidTextError::Prefix));
    assert_eq!(Sid::parse("S-1-5-x"), Err(SidTextError::Part));
    assert_eq!(Sid::parse("S-1-0x1000000000000-1"), Err(SidTextError::Part));
    assert_eq!(Sid::parse("S-1-5"), Err(SidTextError::Count));
    assert_eq!(Sid::from_bytes(&builtin[..15]), Err(ValueError::NotASid));
    assert_eq!(
        Sid::from_bytes(&[1, 0, 0, 0, 0, 0, 0, 5]),
        Err(ValueError::NotASid)
    );
    assert_eq!(
        Sid::from_bytes(&[2, 1, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0]),
        Err(ValueError::NotASid)
    );

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let admin = Sid::from_bytes(&admin_value(&mut ctx, key, ADMIN, "objectSid"))
        .expect("the server sent a SID");
    let text = admin.to_text();
    assert!(text.starts_with("S-1-5-21-"), "{text}");
    assert_eq!(admin.rid(), 500, "the built-in administrator is RID 500");
    let domain = Sid::from_bytes(&admin_value(&mut ctx, key, BASE, "objectSid"))
        .expect("the domain has a SID");
    assert_eq!(admin.domain(), Some(domain));
    assert_eq!(Sid::parse(&text), Ok(admin));
}

#[test]
fn a_filetime_of_zero_or_max_reads_as_null() {
    use nvs_ldap::value::{ValueError, filetime, is_integer};

    assert_eq!(filetime(b"0"), Ok(None));
    assert_eq!(filetime(b"9223372036854775807"), Ok(None));
    // 1970-01-01 is 116444736000000000 ticks after 1601-01-01.
    assert_eq!(filetime(b"116444736000000000"), Ok(Some(0)));
    assert_eq!(filetime(b"116444736000000001"), Ok(Some(100)));
    assert_eq!(filetime(b"1"), Ok(Some(-11_644_473_600_000_000_000 + 100)));
    assert_eq!(filetime(b"-1"), Err(ValueError::NotAFiletime));
    assert_eq!(
        filetime(b"20240101120000.0Z"),
        Err(ValueError::NotAFiletime)
    );
    assert_eq!(filetime(b""), Err(ValueError::NotAFiletime));
    assert!(is_integer(b"133500000000000000"));
    assert!(!is_integer(b"20240101120000Z"));

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    assert_eq!(
        filetime(&admin_value(&mut ctx, key, ADMIN, "accountExpires")),
        Ok(None),
        "the administrator's account never expires"
    );
    let set = filetime(&admin_value(&mut ctx, key, ADMIN, "pwdLastSet"))
        .expect("a FILETIME")
        .expect("the password was set");
    // After 2020-01-01, the year this server image was built after.
    assert!(set > 1_577_836_800_000_000_000, "{set}");
}

#[test]
fn a_negative_interval_reads_as_a_duration() {
    use nvs_ldap::value::{ValueError, interval};

    // 42 days, as AD writes the default `maxPwdAge`.
    assert_eq!(
        interval(b"-36288000000000"),
        Ok(Some(42 * 24 * 3600 * 1_000_000_000))
    );
    assert_eq!(interval(b"0"), Ok(Some(0)));
    assert_eq!(
        interval(b"-9223372036854775808"),
        Ok(None),
        "AD's \"never\""
    );
    assert_eq!(interval(b"1"), Err(ValueError::NotAnInterval));
    assert_eq!(interval(b"-x"), Err(ValueError::NotAnInterval));
    assert_eq!(
        interval(b"-9223372036854775807"),
        Err(ValueError::IntervalTooLong)
    );

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    assert_eq!(
        interval(&admin_value(&mut ctx, key, BASE, "lockoutDuration")),
        Ok(Some(30 * 60 * 1_000_000_000)),
        "Samba's default lockout is 30 minutes"
    );
}

#[test]
fn generalized_time_reads_as_an_instant() {
    use nvs_ldap::value::{GeneralizedTime, ValueError, generalized_time};

    let at = |year, month, day, hour, minute, second, nanosecond, offset| GeneralizedTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        nanosecond,
        offset,
    };
    assert_eq!(
        generalized_time(b"20240101120000.0Z"),
        Ok(at(2024, 1, 1, 12, 0, 0, 0, 0))
    );
    assert_eq!(
        generalized_time(b"20240229235959.123Z"),
        Ok(at(2024, 2, 29, 23, 59, 59, 123_000_000, 0))
    );
    assert_eq!(
        generalized_time(b"199412161032-0500"),
        Ok(at(1994, 12, 16, 10, 32, 0, 0, -5 * 3600))
    );
    // A fraction of an hour is minutes and seconds.
    assert_eq!(
        generalized_time(b"2024010112.5+0130"),
        Ok(at(2024, 1, 1, 12, 30, 0, 0, 5400))
    );
    for wrong in [
        &b"20240101120000"[..],
        b"20241301120000Z",
        b"20240101246000Z",
        b"2024010112.Z",
        b"20240101120000Z ",
        b"20240101120000+055",
        b"133500000000000000",
    ] {
        assert_eq!(
            generalized_time(wrong),
            Err(ValueError::NotAGeneralizedTime),
            "{}",
            String::from_utf8_lossy(wrong)
        );
    }

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let created = admin_value(&mut ctx, key, ADMIN, "whenCreated");
    let read = generalized_time(&created).expect("whenCreated is a GeneralizedTime");
    assert_eq!(read.offset, 0, "AD writes UTC");
    assert!(read.year >= 2020, "{read:?}");
}

#[test]
fn account_flags_keep_the_bits_they_do_not_name() {
    use nvs_ldap::value::{
        ACCOUNT_FLAGS, AccountType, GROUP_TYPE_FLAGS, ValueError, account_type, flag_field,
        has_flag, named_bits, with_flags,
    };

    let flag = |flags: &[nvs_ldap::value::Flag], name: &str| {
        flags
            .iter()
            .find(|flag| flag.name == name)
            .map_or_else(|| panic!("no flag `{name}`"), |flag| flag.bit)
    };
    // `PASSWD_CANT_CHANGE` (`0x40`) and `0x8000000` are named by no reader,
    // and a value read with them keeps them.
    let unnamed = 0x40 | 0x800_0000;
    assert_eq!(named_bits(ACCOUNT_FLAGS) & unnamed, 0);
    let bits = flag_field(format!("{}", 0x202 | unnamed).as_bytes()).expect("a flag field");
    assert_eq!(bits, i64::from(0x202 | unnamed), "every bit is kept");
    assert!(has_flag(bits, flag(ACCOUNT_FLAGS, "disabled")));
    assert!(has_flag(bits, flag(ACCOUNT_FLAGS, "normalAccount")));
    assert!(!has_flag(bits, flag(ACCOUNT_FLAGS, "lockedOut")));
    // `with` changes only the bits it names, and keeps the unnamed ones.
    let enabled = with_flags(bits, [(flag(ACCOUNT_FLAGS, "disabled"), false)]);
    assert_eq!(enabled, 0x200 | unnamed);
    let no_expiry = flag(ACCOUNT_FLAGS, "passwordNeverExpires");
    assert_eq!(
        with_flags(bits, [(no_expiry, true)]),
        0x202 | unnamed | no_expiry
    );
    assert_eq!(with_flags(bits, []), 0x202 | unnamed);

    // A security group is negative, because AD writes `groupType` signed.
    let group = flag_field(b"-2147483646").expect("a group type");
    assert!(has_flag(group, flag(GROUP_TYPE_FLAGS, "security")));
    assert!(has_flag(group, flag(GROUP_TYPE_FLAGS, "global")));
    assert!(!has_flag(group, flag(GROUP_TYPE_FLAGS, "universal")));
    // Clearing the security bit makes it positive, and setting it again
    // returns the number AD wrote.
    let distribution = with_flags(group, [(flag(GROUP_TYPE_FLAGS, "security"), false)]);
    assert_eq!(distribution, 2);
    let security = with_flags(
        i64::from(distribution),
        [(flag(GROUP_TYPE_FLAGS, "security"), true)],
    );
    assert_eq!(security.cast_signed(), -2_147_483_646);
    assert_eq!(flag_field(b"4294967295"), Ok(i64::from(u32::MAX)));
    for wrong in [&b"4294967296"[..], b"-2147483649", b"0x2", b""] {
        assert_eq!(flag_field(wrong), Err(ValueError::NotAFlagField));
    }

    assert_eq!(account_type(b"805306368"), Ok(AccountType::User));
    assert_eq!(account_type(b"268435456"), Ok(AccountType::Group));
    assert_eq!(account_type(b"7"), Err(ValueError::NotAnAccountType));

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let guest = "CN=Guest,CN=Users,DC=example,DC=test";
    let guest = flag_field(&admin_value(&mut ctx, key, guest, "userAccountControl"))
        .expect("Guest has flags");
    assert!(
        has_flag(guest, flag(ACCOUNT_FLAGS, "disabled")),
        "Guest is disabled: {guest}"
    );
    let admin = flag_field(&admin_value(&mut ctx, key, ADMIN, "userAccountControl"))
        .expect("the administrator has flags");
    assert!(!has_flag(admin, flag(ACCOUNT_FLAGS, "disabled")), "{admin}");
    assert!(
        has_flag(admin, flag(ACCOUNT_FLAGS, "normalAccount")),
        "{admin}"
    );
    assert_eq!(
        account_type(&admin_value(&mut ctx, key, ADMIN, "sAMAccountType")),
        Ok(AccountType::User)
    );
}

/// A peer on loopback that answers the n-th whole message it reads with
/// `answers[n]`, and returns its port and every byte it read.
fn scripted(answers: Vec<Vec<u8>>) -> (u16, Arc<std::sync::Mutex<Vec<u8>>>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
    scripted_on(listener, answers)
}

/// [`scripted`] on a listener the caller bound.
fn scripted_on(
    listener: TcpListener,
    answers: Vec<Vec<u8>>,
) -> (u16, Arc<std::sync::Mutex<Vec<u8>>>) {
    let port = listener.local_addr().expect("its address").port();
    let read = Arc::new(std::sync::Mutex::new(Vec::new()));
    let kept = Arc::clone(&read);
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
        let mut answers = answers.into_iter();
        let mut received = Vec::new();
        let mut at = 0;
        let mut chunk = [0u8; 4096];
        loop {
            while let Ok(Some((head, length))) = ber::header(&received[at..]) {
                if received.len() < at + head + length {
                    break;
                }
                at += head + length;
                if let Some(answer) = answers.next() {
                    stream.write_all(&answer).ok();
                }
            }
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => return,
                Ok(count) => {
                    received.extend_from_slice(&chunk[..count]);
                    kept.lock()
                        .expect("unpoisoned")
                        .extend_from_slice(&chunk[..count]);
                }
            }
        }
    });
    (port, read)
}

/// One search's answer under message `id`: an entry at `dn` with `attributes`,
/// then a `SearchResultDone` with success and no paging cookie.
fn one_entry(id: i64, dn: &str, attributes: &[(&str, &[&str])]) -> Vec<u8> {
    let mut answer = Writer::new();
    answer.constructed(tag::SEQUENCE, |msg| {
        msg.integer(tag::INTEGER, id);
        msg.constructed(0x64, |entry| {
            entry.octets(tag::OCTET_STRING, dn.as_bytes());
            entry.constructed(tag::SEQUENCE, |list| {
                for (name, values) in attributes {
                    list.constructed(tag::SEQUENCE, |attribute| {
                        attribute.octets(tag::OCTET_STRING, name.as_bytes());
                        attribute.constructed(tag::SET, |set| {
                            for value in *values {
                                set.octets(tag::OCTET_STRING, value.as_bytes());
                            }
                        });
                    });
                }
            });
        });
    });
    answer.constructed(tag::SEQUENCE, |msg| {
        msg.integer(tag::INTEGER, id);
        msg.constructed(0x65, |done| {
            done.integer(tag::ENUMERATED, 0);
            done.octets(tag::OCTET_STRING, b"");
            done.octets(tag::OCTET_STRING, b"");
        });
    });
    answer.into_bytes()
}

/// An anonymous `[ldap.corp]` block over plain LDAP to the peer on `port`,
/// which the cleartext grant allows.
fn cleartext_corp(port: u16) -> Arc<nvs_config::Snapshot> {
    snapshot(
        vec![(
            "corp",
            LdapDirectory {
                url: Some(Setting::Text(format!("ldap://127.0.0.1:{port}"))),
                tls: Some("none".to_owned()),
                timeout: Some(Setting::Text("5s".to_owned())),
                ..LdapDirectory::default()
            },
        )],
        CapLdap {
            connect: list(&["corp"]),
            cleartext: list(&["127.0.0.1"]),
            ..CapLdap::default()
        },
    )
}

#[test]
fn a_ranged_member_list_is_merged_to_its_end() {
    // A group whose `member` list is longer than one answer holds: the search
    // returns the first two values as `member;range=0-1`, and each follow-up
    // asks for the rest from where the last answer ended.
    let group = "CN=Staff,DC=example,DC=test";
    let (port, read) = scripted(vec![
        one_entry(
            1,
            group,
            &[("cn", &["Staff"]), ("member;range=0-1", &["CN=A", "CN=B"])],
        ),
        one_entry(2, group, &[("member;range=2-3", &["CN=C", "CN=D"])]),
        one_entry(3, group, &[("member;range=4-*", &["CN=E"])]),
    ]);
    let mut ctx = ctx_over(&cleartext_corp(port));
    let key = ldap::connect(&mut ctx, "corp").expect("the peer accepts");

    let every = Filter::Present("objectClass".to_owned());
    let mut entries =
        ldap::search(&mut ctx, key, &everything(&every, 1000, 0)).expect("the search starts");
    let entry = entries
        .next(&mut ctx)
        .expect("the ranges are fetched")
        .expect("one entry");
    let names: Vec<&str> = entry
        .attributes
        .iter()
        .map(|attribute| attribute.name.as_str())
        .collect();
    assert_eq!(names, ["cn", "member"], "the list has its plain name");
    let members: Vec<String> = entry
        .get("member")
        .expect("the merged list")
        .iter()
        .map(|value| String::from_utf8_lossy(value).into_owned())
        .collect();
    assert_eq!(members, ["CN=A", "CN=B", "CN=C", "CN=D", "CN=E"]);
    assert!(entries.next(&mut ctx).expect("the search ends").is_none());

    let sent = read.lock().expect("unpoisoned").clone();
    let asked = |text: &str| {
        sent.windows(text.len())
            .any(|window| window == text.as_bytes())
    };
    assert!(asked("member;range=2-*") && asked("member;range=4-*"));
    assert!(
        ldap::held(&mut ctx, key, "test")
            .expect("open")
            .is_poolable(),
        "every follow-up ran to its end, so the connection is settled"
    );

    // Samba returns a range when the request names one, so asking the
    // built-in Administrators group for `member;range=0-0` makes the merge
    // fetch the other members.
    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let administrators = "CN=Administrators,CN=Builtin,DC=example,DC=test";
    let whole = ldap::read(&mut ctx, key, administrators, &["member"])
        .expect("the read succeeds")
        .expect("the group exists");
    let ranged = ldap::read(&mut ctx, key, administrators, &["member;range=0-0"])
        .expect("the read succeeds")
        .expect("the group exists");
    let names: Vec<&str> = ranged
        .attributes
        .iter()
        .map(|attribute| attribute.name.as_str())
        .collect();
    assert_eq!(names, ["member"], "the merged list has its plain name");
    assert!(whole.get("member").expect("members").len() > 1);
    assert_eq!(ranged.get("member"), whole.get("member"));
}

#[test]
fn the_schema_is_read_once_per_pool() {
    let Some(ca) = samba() else { return };
    let url = format!("ldaps://localhost:{LDAPS_PORT}");
    let config = snapshot(
        vec![
            ("corp", block_at(&url, Some(&ca))),
            ("staff", block_at(&url, Some(&ca))),
        ],
        CapLdap {
            connect: list(&["corp", "staff"]),
            ..CapLdap::default()
        },
    );

    // Two requests at once hold two connections of the one pool, and the
    // second finds the schema the first read.
    let mut first = ctx_over(&config);
    let mut second = ctx_over(&config);
    let one = ldap::connect(&mut first, "corp").expect("the block opens");
    let two = ldap::connect(&mut second, "corp").expect("the block opens again");
    let read = ldap::schema(&mut first, one, "test").expect("the schema reads");
    let found = ldap::schema(&mut second, two, "test").expect("the schema is found");
    assert!(Arc::ptr_eq(&read, &found), "one pool reads its schema once");

    use nvs_ldap::Syntax;
    assert_eq!(read.syntax("pwdLastSet"), Syntax::LargeInteger);
    assert_eq!(read.syntax("WHENCREATED"), Syntax::GeneralizedTime);
    assert_eq!(read.syntax("isDeleted"), Syntax::Boolean);
    assert_eq!(read.syntax("userAccountControl"), Syntax::Integer);
    assert_eq!(read.syntax("cn"), Syntax::Other);

    // A request after them takes a pooled connection and the same schema.
    drop(first);
    drop(second);
    let mut later = ctx_over(&config);
    let key = ldap::connect(&mut later, "corp").expect("the block opens");
    let again = ldap::schema(&mut later, key, "test").expect("the schema is found");
    assert!(Arc::ptr_eq(&read, &again));

    // Another block is another pool, and reads its own.
    let key = ldap::connect(&mut later, "staff").expect("the block opens");
    let other = ldap::schema(&mut later, key, "test").expect("the schema reads");
    assert!(!Arc::ptr_eq(&read, &other), "each pool reads its own");
    assert_eq!(*read, *other, "the same server declares the same schema");
}

/// A write's answer under message `id`: `tag` with a result of `code`.
fn written(id: i64, tag: u8, code: i64) -> Vec<u8> {
    let mut answer = Writer::new();
    answer.constructed(tag::SEQUENCE, |msg| {
        msg.integer(tag::INTEGER, id);
        msg.constructed(tag, |done| {
            done.integer(tag::ENUMERATED, code);
            done.octets(tag::OCTET_STRING, b"");
            done.octets(tag::OCTET_STRING, b"");
        });
    });
    answer.into_bytes()
}

/// The values of `name` on the entry at `dn`, as text, or `None` when it has none.
fn texts(ctx: &mut nvs_runtime::Ctx, key: u64, dn: &str, name: &str) -> Option<Vec<String>> {
    let entry = ldap::read(ctx, key, dn, &[name])
        .expect("the read succeeds")
        .expect("the entry exists");
    entry.get(name).map(|values| {
        values
            .iter()
            .map(|value| String::from_utf8_lossy(value).into_owned())
            .collect()
    })
}

#[test]
fn modify_applies_its_changes_in_one_request() {
    use nvs_ldap::{Change, ChangeKind, proto};

    let change = |kind, attribute: &str, values: &[&str]| Change {
        kind,
        attribute: attribute.to_owned(),
        values: values
            .iter()
            .map(|value| value.as_bytes().to_vec())
            .collect(),
    };
    let dn = "OU=Framework,DC=example,DC=test";
    let changes = [
        change(ChangeKind::Add, "description", &["two"]),
        change(ChangeKind::Remove, "description", &["one"]),
        change(ChangeKind::Replace, "street", &[]),
    ];

    // The peer reads one message, and it is one Modify request holding all
    // three changes in order.
    let (port, read) = scripted(vec![written(1, 0x67, 0)]);
    let mut ctx = ctx_over(&cleartext_corp(port));
    let key = ldap::connect(&mut ctx, "corp").expect("the peer accepts");
    ldap::modify(&mut ctx, key, dn, &changes).expect("the peer says success");
    let sent = read.lock().expect("unpoisoned").clone();
    assert_eq!(
        sent,
        proto::message(1, &proto::modify_request(dn, &changes), &[]),
        "one message, one request"
    );
    let (head, length) = ber::header(&sent).expect("BER").expect("whole");
    assert_eq!(head + length, sent.len());

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    // A run that stopped half-way leaves the entry behind.
    ldap::delete(&mut ctx, key, dn).ok();
    ldap::add(
        &mut ctx,
        key,
        dn,
        &[
            nvs_ldap::Attribute {
                name: "objectClass".to_owned(),
                values: vec![b"top".to_vec(), b"organizationalUnit".to_vec()],
            },
            nvs_ldap::Attribute {
                name: "description".to_owned(),
                values: vec![b"one".to_vec()],
            },
        ],
    )
    .expect("the entry is added");

    // The second change fails, so the first is not applied either.
    let refused = ldap::modify(
        &mut ctx,
        key,
        dn,
        &[
            change(ChangeKind::Add, "description", &["two"]),
            change(ChangeKind::Remove, "description", &["absent"]),
        ],
    )
    .expect_err("there is no value `absent` to remove");
    assert!(
        refused.message().contains("ConstraintViolation"),
        "{}",
        refused.message()
    );
    assert_eq!(
        texts(&mut ctx, key, dn, "description"),
        Some(vec!["one".to_owned()]),
        "nothing of a refused modify is applied"
    );

    ldap::modify(&mut ctx, key, dn, &changes[..2]).expect("both changes apply");
    assert_eq!(
        texts(&mut ctx, key, dn, "description"),
        Some(vec!["two".to_owned()])
    );
    ldap::delete(&mut ctx, key, dn).expect("the entry is deleted");
    assert!(
        ldap::read(&mut ctx, key, dn, &[])
            .expect("the read succeeds")
            .is_none()
    );
}

#[test]
fn typed_values_round_trip_through_a_write() {
    use nvs_ldap::value::{
        boolean, boolean_text, filetime, filetime_text, flag_field, flag_text, interval,
        interval_text, uuid_from_guid,
    };

    // Each writer is the inverse of the reader `Ldap\Entry` uses.
    for nanos in [0, 1_700_000_000_000_000_000, -86_400_000_000_000] {
        let text = filetime_text(nanos).expect("after 1601");
        assert_eq!(filetime(text.as_bytes()), Ok(Some(nanos)), "{text}");
    }
    assert_eq!(filetime_text(-(116_444_736_000_000_000 * 100)), None);
    let six_weeks = 42 * 86_400 * 1_000_000_000;
    assert_eq!(interval_text(six_weeks), "-36288000000000");
    assert_eq!(
        interval(interval_text(six_weeks).as_bytes()),
        Ok(Some(six_weeks))
    );
    for on in [true, false] {
        assert_eq!(boolean(boolean_text(on).as_bytes()), Ok(on));
    }
    // A security group's `groupType` is negative, and `with` returns it as
    // the unsigned 32 bits.
    assert_eq!(flag_text(0x8000_0002), "-2147483646");
    assert_eq!(flag_text(-2_147_483_646), "-2147483646");
    assert_eq!(flag_field(flag_text(0x202).as_bytes()), Ok(0x202));
    let guid = [7u8; 16];
    let swapped = uuid_from_guid(&guid).expect("16 bytes");
    assert_eq!(uuid_from_guid(&swapped), Ok(guid));

    // An entry written with each form reads back the same values.
    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let dn = "OU=Example,DC=example,DC=test";
    ldap::delete(&mut ctx, key, dn).ok();
    ldap::add(
        &mut ctx,
        key,
        dn,
        &[
            nvs_ldap::Attribute {
                name: "objectClass".to_owned(),
                values: vec![b"organizationalUnit".to_vec()],
            },
            nvs_ldap::Attribute {
                name: "gPOptions".to_owned(),
                values: vec![b"1".to_vec()],
            },
            nvs_ldap::Attribute {
                name: "managedBy".to_owned(),
                values: vec![ADMIN.as_bytes().to_vec()],
            },
        ],
    )
    .expect("the entry is added");
    assert_eq!(
        texts(&mut ctx, key, dn, "gPOptions"),
        Some(vec!["1".to_owned()])
    );
    assert_eq!(
        texts(&mut ctx, key, dn, "managedBy"),
        Some(vec![ADMIN.to_owned()])
    );
    ldap::delete(&mut ctx, key, dn).expect("the entry is deleted");
}

/// A new, disabled user account `CN=<cn>,OU=<cn>` in an OU of its own, after
/// deleting both where a run that stopped half-way left them behind. Not
/// `CN=Users`, because other cases count the entries there.
fn user(ctx: &mut nvs_runtime::Ctx, key: u64, cn: &str) -> String {
    let dn = format!("CN={cn},OU={cn},{BASE}");
    ldap::delete(ctx, key, &dn).ok();
    ldap::delete(ctx, key, &format!("OU={cn},{BASE}")).ok();
    ldap::add(
        ctx,
        key,
        &format!("OU={cn},{BASE}"),
        &[nvs_ldap::Attribute {
            name: "objectClass".to_owned(),
            values: vec![b"organizationalUnit".to_vec()],
        }],
    )
    .expect("the OU is added");
    ldap::add(
        ctx,
        key,
        &dn,
        &[
            nvs_ldap::Attribute {
                name: "objectClass".to_owned(),
                values: vec![b"user".to_vec()],
            },
            nvs_ldap::Attribute {
                name: "sAMAccountName".to_owned(),
                values: vec![cn.as_bytes().to_vec()],
            },
        ],
    )
    .expect("the user is added");
    dn
}

/// Deletes what [`user`] added.
fn delete_user(ctx: &mut nvs_runtime::Ctx, key: u64, cn: &str) {
    ldap::delete(ctx, key, &format!("CN={cn},OU={cn},{BASE}")).expect("the user is deleted");
    ldap::delete(ctx, key, &format!("OU={cn},{BASE}")).expect("the OU is deleted");
}

/// Whether a bind as `dn` with `password` succeeds over LDAPS.
fn binds(ca: &PathBuf, dn: &str, password: &str) -> Result<(), String> {
    let mut block = block_at(&format!("ldaps://localhost:{LDAPS_PORT}"), Some(ca));
    block.user = Some(dn.to_owned());
    block.password = Some(password.to_owned());
    let snapshot = snapshot(
        vec![("staff", block)],
        CapLdap {
            connect: list(&["staff"]),
            ..CapLdap::default()
        },
    );
    let mut ctx = ctx_over(&snapshot);
    let key = ldap::connect(&mut ctx, "staff").map_err(|fault| fault.message())?;
    ldap::who_am_i(&mut ctx, key)
        .map(|_| ())
        .map_err(|fault| fault.message())
}

#[test]
fn set_password_and_change_password_use_unicode_pwd() {
    use nvs_ldap::conn::unicode_pwd;

    assert_eq!(unicode_pwd("Ab1"), b"\"\0A\0b\x001\0\"\0");

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let dn = user(&mut ctx, key, "Team");
    let first = "Shop-first-Pass-7";
    ldap::set_password(&mut ctx, key, &dn, first).expect("the reset is accepted");
    ldap::modify(
        &mut ctx,
        key,
        &dn,
        &[nvs_ldap::Change {
            kind: nvs_ldap::ChangeKind::Replace,
            attribute: "userAccountControl".to_owned(),
            values: vec![b"512".to_vec()],
        }],
    )
    .expect("the account is enabled");
    assert_eq!(binds(&ca, &dn, first), Ok(()));
    // A password must be a day old before its owner changes it, unless the
    // account is due to change it.
    ldap::modify(
        &mut ctx,
        key,
        &dn,
        &[nvs_ldap::Change {
            kind: nvs_ldap::ChangeKind::Replace,
            attribute: "pwdLastSet".to_owned(),
            values: vec![b"0".to_vec()],
        }],
    )
    .expect("the account is due to change its password");

    // The change needs the password the account has now.
    let second = "Shop-second-Pass-8";
    let wrong = ldap::change_password(&mut ctx, key, &dn, "Not-the-Pass-9", second)
        .expect_err("the old password is wrong");
    assert!(
        !wrong.message().contains("Not-the-Pass-9") && !wrong.message().contains(second),
        "{}",
        wrong.message()
    );
    ldap::change_password(&mut ctx, key, &dn, first, second).expect("the change is accepted");
    assert_eq!(binds(&ca, &dn, second), Ok(()));
    delete_user(&mut ctx, key, "Team");
}

#[test]
fn a_password_write_on_an_unencrypted_connection_is_refused() {
    // The cleartext grant covers the peer, and the password is still not sent.
    let (port, read) = scripted(vec![written(1, 0x67, 0)]);
    let mut ctx = ctx_over(&cleartext_corp(port));
    let key = ldap::connect(&mut ctx, "corp").expect("the peer accepts");
    let dn = format!("CN=Staff,CN=Users,{BASE}");
    let reset = ldap::set_password(&mut ctx, key, &dn, "Staff-Pass-1")
        .expect_err("the connection is not encrypted");
    let change = ldap::change_password(&mut ctx, key, &dn, "Staff-Pass-1", "Staff-Pass-2")
        .expect_err("the connection is not encrypted");
    for refused in [reset, change] {
        assert!(
            refused.message().contains("EncryptionRequired")
                && !refused.message().contains("Staff-Pass"),
            "{}",
            refused.message()
        );
    }
    assert!(
        read.lock().expect("unpoisoned").is_empty(),
        "nothing reached the peer"
    );
}

#[test]
fn a_write_to_a_global_catalog_block_throws_before_it_is_sent() {
    // Port 3268 is the catalog, and 3269 its TLS form: either makes the block read-only.
    let listener = [3268, 3269]
        .into_iter()
        .find_map(|port| TcpListener::bind((Ipv4Addr::LOCALHOST, port)).ok())
        .expect("port 3268 or 3269 is free on loopback");
    let (port, read) = scripted_on(listener, vec![written(1, 0x67, 0)]);
    let mut ctx = ctx_over(&cleartext_corp(port));
    let key = ldap::connect(&mut ctx, "corp").expect("the peer accepts");
    let dn = format!("OU=Shop,{BASE}");
    let attribute = nvs_ldap::Attribute {
        name: "objectClass".to_owned(),
        values: vec![b"organizationalUnit".to_vec()],
    };
    let change = nvs_ldap::Change {
        kind: nvs_ldap::ChangeKind::Replace,
        attribute: "description".to_owned(),
        values: vec![b"Shop".to_vec()],
    };
    let refused = [
        ldap::add(&mut ctx, key, &dn, &[attribute]),
        ldap::modify(&mut ctx, key, &dn, &[change]),
        ldap::delete(&mut ctx, key, &dn),
        ldap::rename(&mut ctx, key, &dn, &format!("OU=Blog,{BASE}")),
        ldap::set_password(&mut ctx, key, &dn, "Shop-Pass-1"),
        ldap::change_password(&mut ctx, key, &dn, "Shop-Pass-1", "Shop-Pass-2"),
    ];
    for result in refused {
        let message = result.expect_err("a Global Catalog only reads").message();
        assert!(message.contains("ReadOnly"), "{message}");
    }
    assert!(
        read.lock().expect("unpoisoned").is_empty(),
        "nothing reached the peer"
    );
}

#[test]
fn a_password_policy_refusal_is_its_own_kind() {
    assert_eq!(
        nvs_ldap::Kind::of_result(
            19,
            "0000052D: Constraint violation - the password is too short"
        ),
        nvs_ldap::Kind::PasswordPolicy
    );

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let dn = user(&mut ctx, key, "Editors");
    let refused =
        ldap::set_password(&mut ctx, key, &dn, "a").expect_err("the password is too short");
    assert!(
        refused.message().contains("PasswordPolicy"),
        "{}",
        refused.message()
    );
    delete_user(&mut ctx, key, "Editors");
}

#[test]
fn compare_returns_a_bool_and_throws_on_error() {
    use nvs_ldap::proto;

    // `compareTrue` (6) and `compareFalse` (5) are answers, and any other
    // result is an error.
    let dn = "OU=Catalog,DC=example,DC=test";
    let (port, read) = scripted(vec![
        written(1, 0x6f, 6),
        written(2, 0x6f, 5),
        written(3, 0x6f, 32),
    ]);
    let mut ctx = ctx_over(&cleartext_corp(port));
    let key = ldap::connect(&mut ctx, "corp").expect("the peer accepts");
    let compare = |ctx: &mut nvs_runtime::Ctx, key, value: &[u8]| {
        ldap::compare(ctx, key, dn, "description", value).map_err(|fault| fault.message())
    };
    assert_eq!(compare(&mut ctx, key, b"one"), Ok(true));
    assert_eq!(compare(&mut ctx, key, b"two"), Ok(false));
    let missing = compare(&mut ctx, key, b"one").expect_err("the peer says there is no such entry");
    assert!(missing.contains("NoSuchObject"), "{missing}");
    let sent = read.lock().expect("unpoisoned").clone();
    let first = proto::message(1, &proto::compare_request(dn, "description", b"one"), &[]);
    assert_eq!(
        &sent[..first.len()],
        first.as_slice(),
        "one request per compare"
    );

    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    ldap::delete(&mut ctx, key, dn).ok();
    ldap::add(
        &mut ctx,
        key,
        dn,
        &[
            nvs_ldap::Attribute {
                name: "objectClass".to_owned(),
                values: vec![b"organizationalUnit".to_vec()],
            },
            nvs_ldap::Attribute {
                name: "description".to_owned(),
                values: vec![b"Books".to_vec()],
            },
        ],
    )
    .expect("the entry is added");
    // The server's matching rule for `description` ignores case.
    assert_eq!(compare(&mut ctx, key, b"Books"), Ok(true));
    assert_eq!(compare(&mut ctx, key, b"books"), Ok(true));
    assert_eq!(compare(&mut ctx, key, b"Music"), Ok(false));
    ldap::delete(&mut ctx, key, dn).expect("the entry is deleted");
    let gone = compare(&mut ctx, key, b"Books").expect_err("the entry is gone");
    assert!(gone.contains("NoSuchObject"), "{gone}");
}

/// A `BindResponse` to message 1 with `code` and `diagnostic`.
fn bound(code: i64, diagnostic: &str) -> Vec<u8> {
    let mut answer = Writer::new();
    answer.constructed(tag::SEQUENCE, |msg| {
        msg.integer(tag::INTEGER, 1);
        msg.constructed(0x61, |done| {
            done.integer(tag::ENUMERATED, code);
            done.octets(tag::OCTET_STRING, b"");
            done.octets(tag::OCTET_STRING, diagnostic.as_bytes());
        });
    });
    answer.into_bytes()
}

/// What a login peer read on one connection, and whether the client closed it.
type Login = (Vec<u8>, bool);

/// A peer on loopback for `authenticate`. It keeps the first connection open
/// and answers nothing on it, which is the block's own connection. Every
/// later connection is one login: the peer answers its first message with
/// the next of `answers`, reads until the client closes it, and sends what
/// it read.
fn login_peer(answers: Vec<Vec<u8>>) -> (u16, std::sync::mpsc::Receiver<Login>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
    let port = listener.local_addr().expect("its address").port();
    let (send, logins) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let Ok((block, _)) = listener.accept() else {
            return;
        };
        for answer in answers {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
            let mut received = Vec::new();
            let mut answered = false;
            let mut chunk = [0u8; 4096];
            let closed = loop {
                if !answered
                    && let Ok(Some((head, length))) = ber::header(&received)
                    && received.len() >= head + length
                {
                    stream.write_all(&answer).ok();
                    answered = true;
                }
                match stream.read(&mut chunk) {
                    Ok(0) => break true,
                    Err(_) => break false,
                    Ok(count) => received.extend_from_slice(&chunk[..count]),
                }
            };
            send.send((received, closed)).ok();
        }
        drop(block);
    });
    (port, logins)
}

/// A user account `CN=<cn>,OU=<cn>` with `password` that can log in.
fn login_user(ctx: &mut nvs_runtime::Ctx, key: u64, cn: &str, password: &str) -> String {
    let dn = user(ctx, key, cn);
    ldap::set_password(ctx, key, &dn, password).expect("the reset is accepted");
    ldap::modify(
        ctx,
        key,
        &dn,
        &[nvs_ldap::Change {
            kind: nvs_ldap::ChangeKind::Replace,
            attribute: "userAccountControl".to_owned(),
            values: vec![b"512".to_vec()],
        }],
    )
    .expect("the account is enabled");
    dn
}

#[test]
fn authenticate_binds_on_its_own_connection_and_closes_it() {
    use nvs_ldap::proto;

    // The block's connection is the peer's first, and the login is its
    // second: one bind, one unbind, and then the client closes it.
    let login = format!("CN=Staff,OU=Staff,{BASE}");
    let (port, logins) = login_peer(vec![bound(0, "")]);
    let mut ctx = ctx_over(&cleartext_corp(port));
    let key = ldap::connect(&mut ctx, "corp").expect("the peer accepts");
    ldap::authenticate(&mut ctx, key, &login, "Staff-Pass-1").expect("the peer accepts the login");
    let (sent, closed) = logins
        .recv_timeout(Duration::from_secs(10))
        .expect("the login came on a connection of its own");
    let bind = proto::message(1, &proto::bind_request(&login, b"Staff-Pass-1"), &[]);
    let unbind = proto::message(2, &proto::unbind_request(), &[]);
    assert_eq!(sent, [bind, unbind].concat(), "one bind and one unbind");
    assert!(closed, "the client closed the login's connection");

    // Against the directory: the block's connection stays bound as the block.
    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let before = ldap::who_am_i(&mut ctx, key).expect("whoami answers");
    let dn = login_user(&mut ctx, key, "Visitors", "Visitors-Pass-7");
    ldap::authenticate(&mut ctx, key, &dn, "Visitors-Pass-7").expect("the login is accepted");
    assert_eq!(
        ldap::who_am_i(&mut ctx, key).expect("whoami answers"),
        before,
        "authenticate does not rebind the block's connection"
    );
    delete_user(&mut ctx, key, "Visitors");
}

#[test]
fn no_such_user_and_a_wrong_password_are_one_kind() {
    const AD: &str = "80090308: LdapErr: DSID-0C09050F, comment: AcceptSecurityContext error";
    let (port, logins) = login_peer(vec![
        bound(49, &format!("{AD}, data 525, v4f7c")),
        bound(49, &format!("{AD}, data 52e, v4f7c")),
    ]);
    let mut ctx = ctx_over(&cleartext_corp(port));
    let key = ldap::connect(&mut ctx, "corp").expect("the peer accepts");
    let mut refused = Vec::new();
    for _ in 0..2 {
        let fault = ldap::authenticate(&mut ctx, key, &format!("CN=Staff,{BASE}"), "Staff-Pass-1")
            .expect_err("the peer refuses the login");
        let (_, closed) = logins
            .recv_timeout(Duration::from_secs(10))
            .expect("a login");
        assert!(closed, "a refused login's connection is closed too");
        refused.push(fault.message());
    }
    assert!(refused[0].contains("InvalidCredentials"), "{}", refused[0]);
    assert_eq!(
        refused[0], refused[1],
        "no such user and a wrong password throw the same error"
    );

    // Against the directory: an account that does not exist, and one that
    // does with the wrong password.
    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let dn = login_user(&mut ctx, key, "Members", "Members-Pass-7");
    let missing = ldap::authenticate(
        &mut ctx,
        key,
        &format!("CN=Nobody,{BASE}"),
        "Members-Pass-7",
    )
    .expect_err("there is no such account");
    let wrong = ldap::authenticate(&mut ctx, key, &dn, "Members-Pass-8")
        .expect_err("the password is wrong");
    assert!(
        missing.message().contains("InvalidCredentials"),
        "{}",
        missing.message()
    );
    assert_eq!(missing.message(), wrong.message());
    delete_user(&mut ctx, key, "Members");
}

#[test]
fn ad_bind_sub_codes_map_to_kinds() {
    const AD: &str = "80090308: LdapErr: DSID-0C09050F, comment: AcceptSecurityContext error";
    let reasons = [
        ("530", "NotAllowedNow"),
        ("531", "NotAllowedNow"),
        ("532", "PasswordExpired"),
        ("533", "AccountDisabled"),
        ("701", "AccountExpired"),
        ("773", "MustChangePassword"),
        ("775", "AccountLocked"),
    ];
    let answers = reasons
        .iter()
        .map(|(sub, _)| bound(49, &format!("{AD}, data {sub}, v4f7c")))
        .collect();
    let (port, logins) = login_peer(answers);
    let mut ctx = ctx_over(&cleartext_corp(port));
    let key = ldap::connect(&mut ctx, "corp").expect("the peer accepts");
    for (sub, kind) in reasons {
        let fault = ldap::authenticate(&mut ctx, key, &format!("CN=Staff,{BASE}"), "Staff-Pass-1")
            .expect_err("the peer refuses the login");
        logins
            .recv_timeout(Duration::from_secs(10))
            .expect("a login");
        assert!(
            fault.message().contains(&format!(": {kind}:")),
            "data {sub}: {}",
            fault.message()
        );
    }

    // Against the directory: a disabled account, and one that must change
    // its password before it logs in.
    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let dn = login_user(&mut ctx, key, "Readers", "Readers-Pass-7");
    let replace = |attribute: &str, value: &[u8]| nvs_ldap::Change {
        kind: nvs_ldap::ChangeKind::Replace,
        attribute: attribute.to_owned(),
        values: vec![value.to_vec()],
    };
    ldap::modify(&mut ctx, key, &dn, &[replace("pwdLastSet", b"0")])
        .expect("the account must change its password");
    let must = ldap::authenticate(&mut ctx, key, &dn, "Readers-Pass-7")
        .expect_err("the password must change first");
    assert!(
        must.message().contains("MustChangePassword"),
        "{}",
        must.message()
    );
    ldap::modify(&mut ctx, key, &dn, &[replace("userAccountControl", b"514")])
        .expect("the account is disabled");
    let disabled = ldap::authenticate(&mut ctx, key, &dn, "Readers-Pass-7")
        .expect_err("the account is disabled");
    assert!(
        disabled.message().contains("AccountDisabled"),
        "{}",
        disabled.message()
    );
    delete_user(&mut ctx, key, "Readers");
}

#[test]
fn an_errors_message_never_carries_the_password() {
    // A refused login, a reason AD names, a server that does not answer the
    // bind, and an empty login, which is refused before anything is sent.
    let secret = "Shop-Secret-Pass-9";
    let (port, logins) = login_peer(vec![
        bound(49, "AcceptSecurityContext error, data 52e, v4f7c"),
        bound(49, "AcceptSecurityContext error, data 775, v4f7c"),
        bound(52, "the server is shutting down"),
    ]);
    let mut ctx = ctx_over(&cleartext_corp(port));
    let key = ldap::connect(&mut ctx, "corp").expect("the peer accepts");
    let login = format!("CN=Staff,{BASE}");
    let mut faults = Vec::new();
    for _ in 0..3 {
        faults
            .push(ldap::authenticate(&mut ctx, key, &login, secret).expect_err("the login fails"));
        logins
            .recv_timeout(Duration::from_secs(10))
            .expect("a login");
    }
    faults.push(ldap::authenticate(&mut ctx, key, "", secret).expect_err("the login is empty"));
    for fault in &faults {
        let message = fault.message();
        assert!(
            !message.contains(secret) && !message.contains("Secret"),
            "{message}"
        );
    }
    assert!(
        faults[3].message().contains("InvalidCredentials"),
        "{}",
        faults[3].message()
    );
}

/// The `ou` of each entry a one-level search below `base` returns, sorted by
/// `sort` and cut to `window` when one is given, and the total the server sent.
fn sorted_names(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    base: &str,
    sort: nvs_ldap::Sort<'_>,
    window: Option<nvs_ldap::Window>,
) -> (Vec<String>, Option<u32>) {
    let every = Filter::Present("objectClass".to_owned());
    let mut entries = ldap::search(
        ctx,
        key,
        &SearchRequest {
            base,
            scope: Scope::One,
            filter: &every,
            attributes: &["ou"],
            page_size: 2,
            size_limit: 0,
            time_limit: 0,
            sort: Some(sort),
            window,
            show_deleted: false,
        },
    )
    .expect("the search starts");
    let total = entries.total();
    let mut names = Vec::new();
    while let Some(entry) = entries.next(ctx).expect("the search succeeds") {
        let ou = entry.get("ou").expect("an OU has `ou`")[0].clone();
        names.push(String::from_utf8(ou).expect("UTF-8"));
    }
    (names, total)
}

#[test]
fn sort_and_a_vlv_window_return_the_requested_slice() {
    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let base = format!("OU=Archive,{BASE}");
    let names = ["Delta", "Alpha", "Echo", "Charlie", "Bravo"];
    for name in names {
        ldap::delete(&mut ctx, key, &format!("OU={name},{base}")).ok();
    }
    ldap::delete(&mut ctx, key, &base).ok();
    let unit = || {
        vec![nvs_ldap::Attribute {
            name: "objectClass".to_owned(),
            values: vec![b"organizationalUnit".to_vec()],
        }]
    };
    ldap::add(&mut ctx, key, &base, &unit()).expect("the OU is added");
    for name in names {
        ldap::add(&mut ctx, key, &format!("OU={name},{base}"), &unit()).expect("added");
    }

    // A sort with no window is sent with every page, here two entries to a page.
    let up = nvs_ldap::Sort {
        attribute: "ou",
        descending: false,
    };
    assert_eq!(
        sorted_names(&mut ctx, key, &base, up, None),
        (
            vec!["Alpha", "Bravo", "Charlie", "Delta", "Echo"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            None
        )
    );

    // A window skips `offset` sorted entries, returns `count`, and carries the total.
    let slice = |offset, count| Some(nvs_ldap::Window { offset, count });
    let (window, total) = sorted_names(&mut ctx, key, &base, up, slice(1, 2));
    assert_eq!(
        (window, total),
        (vec!["Bravo".into(), "Charlie".into()], Some(5))
    );
    let down = nvs_ldap::Sort {
        attribute: "ou",
        descending: true,
    };
    let (window, total) = sorted_names(&mut ctx, key, &base, down, slice(0, 2));
    assert_eq!(
        (window, total),
        (vec!["Echo".into(), "Delta".into()], Some(5))
    );
    assert!(
        ldap::held(&mut ctx, key, "test")
            .expect("open")
            .is_poolable(),
        "a window is one answer, so the connection is settled after it"
    );

    for name in names {
        ldap::delete(&mut ctx, key, &format!("OU={name},{base}")).expect("deleted");
    }
    ldap::delete(&mut ctx, key, &base).expect("the OU is deleted");
}

/// The DNs and `isDeleted` values of the entries under the domain root whose
/// `objectGUID` is `guid`, searched with and without the show deleted control.
fn with_guid(
    ctx: &mut nvs_runtime::Ctx,
    key: u64,
    guid: &[u8],
    show_deleted: bool,
) -> Vec<(String, bool)> {
    let filter = Filter::Equal("objectGUID".to_owned(), guid.to_vec());
    let mut entries = ldap::search(
        ctx,
        key,
        &SearchRequest {
            attributes: &["isDeleted"],
            show_deleted,
            ..everything(&filter, 1000, 0)
        },
    )
    .expect("the search starts");
    let mut found = Vec::new();
    while let Some(entry) = entries.next(ctx).expect("the search succeeds") {
        let deleted = entry
            .get("isDeleted")
            .is_some_and(|values| values.first().is_some_and(|value| value == b"TRUE"));
        found.push((entry.dn.clone(), deleted));
    }
    found
}

#[test]
fn show_deleted_finds_a_deleted_entry() {
    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let dn = format!("OU=Museum,{BASE}");
    ldap::delete(&mut ctx, key, &dn).ok();
    ldap::add(
        &mut ctx,
        key,
        &dn,
        &[nvs_ldap::Attribute {
            name: "objectClass".to_owned(),
            values: vec![b"organizationalUnit".to_vec()],
        }],
    )
    .expect("the OU is added");
    let read = ldap::read(&mut ctx, key, &dn, &["objectGUID"])
        .expect("the read succeeds")
        .expect("the OU exists");
    let guid = read.get("objectGUID").expect("every entry has a GUID")[0].clone();
    assert_eq!(
        with_guid(&mut ctx, key, &guid, false),
        vec![(dn.clone(), false)],
        "a live entry is found with or without the control"
    );

    ldap::delete(&mut ctx, key, &dn).expect("the OU is deleted");
    assert!(
        with_guid(&mut ctx, key, &guid, false).is_empty(),
        "a plain search does not see a tombstone"
    );
    let tombstones = with_guid(&mut ctx, key, &guid, true);
    assert_eq!(tombstones.len(), 1, "{tombstones:?}");
    let (moved, deleted) = &tombstones[0];
    assert!(*deleted, "a tombstone has `isDeleted: TRUE`");
    assert!(
        moved.contains("CN=Deleted Objects"),
        "a tombstone is moved under `CN=Deleted Objects`: {moved}"
    );
}

/// The DNs of the OUs a DirSync from `cookie` returns, and the cookie it ends with.
fn synced(ctx: &mut nvs_runtime::Ctx, key: u64, cookie: &[u8]) -> (Vec<String>, Vec<u8>) {
    let units = Filter::Equal("objectClass".to_owned(), b"organizationalUnit".to_vec());
    let mut changes = ldap::changes(
        ctx,
        key,
        &SearchRequest {
            attributes: &["ou"],
            ..everything(&units, 1000, 0)
        },
        cookie,
    )
    .expect("the sync starts");
    let mut found = Vec::new();
    while let Some(entry) = changes.next(ctx).expect("the sync succeeds") {
        found.push(entry.dn);
    }
    (found, changes.cookie().to_vec())
}

#[test]
fn dirsync_returns_only_what_changed_since_its_cookie() {
    let Some(ca) = samba() else { return };
    let mut ctx = ctx_over(&corp(&ca));
    let key = ldap::connect(&mut ctx, "corp").expect("the block opens");
    let dn = format!("OU=Harbor,{BASE}");
    let unchanged = format!("OU=Domain Controllers,{BASE}");
    ldap::delete(&mut ctx, key, &dn).ok();

    // An empty cookie returns every entry, and ends with a cookie.
    let (every, cookie) = synced(&mut ctx, key, &[]);
    assert!(every.contains(&unchanged), "{every:?}");
    assert!(!cookie.is_empty(), "a sync ends with a cookie");

    ldap::add(
        &mut ctx,
        key,
        &dn,
        &[nvs_ldap::Attribute {
            name: "objectClass".to_owned(),
            values: vec![b"organizationalUnit".to_vec()],
        }],
    )
    .expect("the OU is added");
    let (since, next) = synced(&mut ctx, key, &cookie);
    assert!(since.contains(&dn), "the new OU is a change: {since:?}");
    assert!(
        !since.contains(&unchanged),
        "an OU that did not change is not sent again: {since:?}"
    );
    let (after, _) = synced(&mut ctx, key, &next);
    assert!(
        !after.contains(&dn),
        "the next cookie is past the change: {after:?}"
    );
    assert!(
        ldap::held(&mut ctx, key, "test")
            .expect("open")
            .is_poolable(),
        "a finished sync leaves the connection settled"
    );
    ldap::delete(&mut ctx, key, &dn).expect("the OU is deleted");
}
