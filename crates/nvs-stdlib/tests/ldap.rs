//! `Core\Ldap`'s connections, run through `nvs_stdlib::ldap`: which grant each of `connect` and `open` asks, the address policy `open` passes and `connect` does not, and the pool's identity
//!
//! The grant and policy cases need no server: every refusal they assert
//! happens before a byte is sent, and the one dial they make goes to a port
//! nothing listens on. The pool case needs the `samba-ad` service from
//! `tests/db/compose.yaml`, and skips when nothing listens on its LDAPS port
//! unless `NVS_LDAP_SAMBA` is set, as `crates/nvs-ldap/tests/samba.rs` does.

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use nvs_config::tree::{CapLdap, Capabilities, Config, LdapDirectory, Setting};
use nvs_stdlib::ldap;

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

#[test]
fn a_pooled_connection_is_always_bound_as_its_block() {
    let Some(ca) = samba() else { return };
    let url = format!("ldaps://localhost:{LDAPS_PORT}");
    let config = snapshot(
        vec![("corp", block_at(&url, Some(&ca)))],
        CapLdap {
            connect: list(&["corp"]),
            ..CapLdap::default()
        },
    );

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
}
