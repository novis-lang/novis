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
