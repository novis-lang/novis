//! ADR 0278 § 2's `[ldap.<name>]` block and `[capabilities.ldap]`'s three grants, as the boot reads
//! them: the empty password refused when the block is read
//! (`rule:security/ldap-empty-password-is-refused`), `tls = "none"` as one half of a cleartext bind
//! (`rule:security/ldap-cleartext-bind-is-granted-per-host`), and the grants in `db.*`'s shape.

use std::collections::BTreeMap;
use std::time::Duration;

use nvs_config::Config;
use nvs_config::capability::Cap;
use nvs_config::ldap::{DEFAULT_TIMEOUT, Tls, timeout_for, tls_of, urls_of, validate};
use nvs_diagnostics::{Diagnostic, SourceMap, code};

/// The tree `text` parses to, panicking with the refusal's message when it does not.
fn config(text: &str) -> Config {
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text);
    parsed.unwrap_or_else(|err| panic!("{text}\n-- refused: {}", err.message))
}

/// The refusal [`validate`] gives `text`, panicking when it accepts it.
fn refusal(text: &str) -> Diagnostic {
    validate(&config(text), &BTreeMap::new())
        .err()
        .unwrap_or_else(|| panic!("{text}\n-- was accepted, and should not have been"))
}

fn accepted(text: &str) {
    validate(&config(text), &BTreeMap::new())
        .unwrap_or_else(|err| panic!("{text}\n-- refused: {}", err.message));
}

const URL: &str = "[ldap.corp]\nurl = \"ldaps://dc1.example.test\"\n";

/// `rule:security/ldap-empty-password-is-refused`'s boot half: a `user` with no password, with
/// `password = ""` and with a password of nothing at all are the same unauthenticated bind, and each
/// is refused by name. Counted, so a check that lost one spelling fails here.
#[test]
fn a_block_with_a_user_and_an_empty_password_is_refused_when_it_is_read() {
    let empty = [
        format!("{URL}user = \"svc@example.test\"\n"),
        format!("{URL}user = \"svc@example.test\"\npassword = \"\"\n"),
    ];
    let refused = empty
        .iter()
        .filter(|text| {
            let err = refusal(text);
            err.code == Some(code::E_BAD_DIRECTIVE) && err.message.contains("empty password")
        })
        .count();
    assert_eq!(
        refused,
        empty.len(),
        "every empty-password spelling is refused"
    );

    accepted(&format!(
        "{URL}user = \"svc@example.test\"\npassword = \"s3cret\"\n"
    ));
    accepted(URL);
}

/// A password with no `user` would be read and never sent, and an empty `user` is neither an
/// account nor the anonymous bind.
#[test]
fn half_a_credential_is_refused() {
    assert!(
        refusal(&format!("{URL}password = \"s3cret\"\n"))
            .message
            .contains("no `user`")
    );
    assert!(
        refusal(&format!("{URL}user = \"\"\npassword = \"s3cret\"\n"))
            .message
            .contains("empty")
    );
}

/// The URL list: one string or a list, tried in order, and every entry `ldap://` or `ldaps://`.
#[test]
fn the_url_is_one_or_a_list_and_each_is_an_ldap_url() {
    let one = config(URL);
    assert_eq!(urls_of(&one.ldap["corp"]), ["ldaps://dc1.example.test"]);
    let many = config(
        "[ldap.corp]\nurl = [\"ldaps://dc1.example.test\", \"ldap://dc2.example.test:389\"]\n",
    );
    assert_eq!(
        urls_of(&many.ldap["corp"]),
        ["ldaps://dc1.example.test", "ldap://dc2.example.test:389"],
        "the order written is the order tried",
    );

    for bad in [
        "[ldap.corp]\nbase = \"DC=example,DC=test\"\n",
        "[ldap.corp]\nurl = []\n",
        "[ldap.corp]\nurl = \"https://dc1.example.test\"\n",
        "[ldap.corp]\nurl = \"ldaps://\"\n",
        "[ldap.corp]\nurl = [\"ldaps://dc1.example.test\", \"dc2.example.test\"]\n",
    ] {
        assert_eq!(refusal(bad).code, Some(code::E_BAD_DIRECTIVE), "{bad}");
    }
}

/// `tls` is two words. `"none"` is one half of a cleartext bind and is refused beside an `ldaps://`
/// URL, which always handshakes first.
#[test]
fn tls_is_required_by_default_and_none_only_beside_ldap_urls() {
    assert_eq!(tls_of(&config(URL).ldap["corp"]), Tls::Required);
    let plain = "[ldap.corp]\nurl = \"ldap://dc1.example.test\"\ntls = \"none\"\n";
    accepted(plain);
    assert_eq!(tls_of(&config(plain).ldap["corp"]), Tls::None);

    assert!(
        refusal(&format!("{URL}tls = \"none\"\n"))
            .message
            .contains("tls = \"none\"")
    );
    assert!(
        refusal(&format!("{URL}tls = \"optional\"\n"))
            .message
            .contains("not a TLS mode")
    );
}

/// A timeout has a default and no spelling for unlimited: `false` and zero are both refused.
#[test]
fn the_timeout_has_a_default_and_no_unlimited_spelling() {
    let none = BTreeMap::new();
    assert_eq!(
        timeout_for("corp", &config(URL).ldap["corp"], &none).unwrap(),
        DEFAULT_TIMEOUT
    );
    let written = config(&format!("{URL}timeout = \"5s\"\n"));
    assert_eq!(
        timeout_for("corp", &written.ldap["corp"], &none).unwrap(),
        Duration::from_secs(5)
    );
    for bad in [
        "timeout = false\n",
        "timeout = \"0s\"\n",
        "timeout = \"soon\"\n",
    ] {
        assert_eq!(
            refusal(&format!("{URL}{bad}")).code,
            Some(code::E_BAD_DIRECTIVE),
            "{bad}"
        );
    }
}

/// Every table refuses an unknown key, the block and its grants alike.
#[test]
fn an_unknown_key_is_refused_in_the_block_and_in_the_grants() {
    for text in [
        "[ldap.corp]\nreferrals = true\n",
        "[capabilities.ldap]\nsearch = true\n",
    ] {
        let mut sources = SourceMap::new();
        let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text);
        assert!(parsed.is_err(), "{text} parsed");
    }
}

/// The three grants in `db.*`'s shape: `connect` names blocks, `open` names hosts and takes `*.` at
/// a label boundary, and `cleartext` names hosts with no `true`.
#[test]
fn the_three_grants_are_db_shaped_and_cleartext_names_each_host() {
    let caps = config(
        "[capabilities.ldap]\nconnect = [\"corp\"]\nopen = [\"*.tenants.example.test\"]\n\
         cleartext = [\"dc1.example.test\"]\n",
    )
    .capabilities
    .expect("the block was written");
    assert_eq!(Cap::LdapConnect.name(), "ldap.connect");
    assert_eq!(Cap::LdapCleartext.family(), "ldap");
    assert!(caps.allows_name(Cap::LdapConnect, "corp"));
    assert!(!caps.allows_name(Cap::LdapConnect, "other"));
    assert!(caps.allows_host(Cap::LdapOpen, "a.tenants.example.test"));
    assert!(!caps.allows_host(Cap::LdapOpen, "tenants.example.test"));
    assert!(!caps.allows_host(Cap::LdapOpen, "evil-tenants.example.test"));
    assert!(caps.allows_host(Cap::LdapCleartext, "dc1.example.test"));
    assert!(!caps.allows_host(Cap::LdapCleartext, "dc2.example.test"));

    let everything = config("[capabilities.ldap]\nconnect = true\ncleartext = true\n")
        .capabilities
        .expect("the block was written");
    assert!(everything.allows_name(Cap::LdapConnect, "corp"));
    assert!(
        !everything.allows_host(Cap::LdapCleartext, "dc1.example.test"),
        "`cleartext = true` grants nothing: each host that sends a password readable is named",
    );
    let empty = config("[capabilities.ldap]\n")
        .capabilities
        .unwrap_or_default();
    assert!(
        !empty.allows_name(Cap::LdapConnect, "corp"),
        "deny by default"
    );
}
