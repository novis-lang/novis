//! `rule:config/a-secret-is-a-file-whose-content-is-the-value`: a secret arrives as a file whose content is the value.
//!
//! A directive the configuration marks secret gains a `_file` sibling, and **exactly one of the
//! pair may be set** — both is a refusal, which is what keeps this two sources for one value rather
//! than the second spelling `rule:statements/nothing-gets-a-second-name` refuses. The file's *whole* content is the value, with one
//! trailing `\n` — and a `\r` immediately before it — stripped and nothing else trimmed. Not a
//! general trim: a password may legitimately begin or end with a space, and removing it is
//! `rule:errors/ambiguous-input-refused`'s failure of repairing input instead of reading it. One newline goes because
//! `echo secret > f` produces one and every injection system does; a value that genuinely ends in a
//! newline is written with two.
//!
//! **Every pair is a row of [`SECRETS`]** and [`materialize`] is a sweep over that table, so a
//! credential is covered by adding one row and not by editing a walk — the database password
//! (`rule:core-classes/db-capabilities`) and every credential written beside it since, in a block an operator names
//! and in a block they do not. `rule:config/a-secret-is-a-file-whose-content-is-the-value` speaks of a directive "the registry marks secret", and this
//! table is that marking.
//!
//! It is **not** another field on [`mod@crate::directive`]'s rows, for two reasons that both make
//! that table the wrong shape rather than merely a different one. A
//! [`Directive`](crate::directive::Directive) row is a *prefix* — one row answers for every key
//! beneath it — and secrecy is the opposite of prefix-shaped: `db.main.password` is secret and
//! `db.main.host`, one segment away inside the same block, is not. And a credential is written in an
//! operator-*named* block, so its key holds a segment no table can spell literally, while that
//! registry has no wildcard and needs none, because a changeability class governs a whole block
//! either way. What the two tables do share is the census habit: a credential on the tree with no
//! row here, or a row whose value has no `_file` sibling, is a failing assertion in
//! `tests/secret.rs` rather than a `_file` that silently does nothing.
//!
//! **The secret file is a configuration input like any other**, so it goes through
//! [`Files::trust`] before it is read: an account that can rewrite it
//! chooses the credential the server connects with, which is exactly what `rule:config/ownership-is-the-trust-boundary` refuses about the
//! files naming it. What `rule:config/a-secret-is-a-file-whose-content-is-the-value` does *not* inherit is confidentiality — a readable secret file is
//! `W1005` and not a refusal, because a Compose secret is mounted `0444`. [`mod@crate::trust`]'s
//! module doc owns that split.
//!
//! **A credential is never repaired, and an edge space is said out loud instead.** `rule:config/a-secret-is-a-file-whose-content-is-the-value` keeps the
//! value exactly as it arrived, so the one thing left to do about a password that begins or ends
//! with whitespace is to name it: [`padding`] finds it and `W1007` reports it, for the inline half
//! as well as the file half, and the boot goes on with the value. Refusing it instead would wall
//! off a credential some other system issued, with no remedy in the file that names it; trimming it
//! is what `rule:errors/ambiguous-input-refused` calls repairing input in place of reading it, and it
//! turns a working credential into an authentication failure at the far end that no message anywhere
//! would explain.
//!
//! **The value stays out of every message this module writes.** A refusal names the file, the key
//! and the shape of the problem — empty, whitespace-only, oversized, not UTF-8 — and never a byte
//! of the content, which is `rule:security/secret-qualifier`'s type-level meaning applied one layer below the language.
//! `password_file` is left set after the value is materialized for the same reason: `rule:config/check-and-dump-audit-the-tree-offline`'s dump
//! renders the value `<secret>` and names the file it came from, and it can only do that if the
//! file is still recorded.
//!
//! **The value never enters the merged table**, and that is what [`Secret`] exists for. The table is
//! the stream `rule:config/check-and-dump-audit-the-tree-offline` serializes whole for `nvs config dump --toml` and the one
//! [`Snapshot::retype`](crate::Snapshot) rebuilds the typed tree out of, so a content put into it
//! would have to be redacted again by every reader that walks it — and the one that forgets writes a
//! credential into a file an operator diffs. Carried beside it instead, a secret reaches exactly the
//! readers that ask for it by name: [`apply`] puts it back onto a typed tree, `Core\Config::get`
//! answers `db.main.password` with it, and the dump and `Core\Config::all` render it `<secret>`
//! ([`is_secret_key`]). That is also why
//! [`apply`] is a second function rather than the tail of [`materialize`] — a snapshot deserializes
//! the table more than once (a reload carries `Boot` values across and retypes), and each of those
//! rounds has to put the secrets back without re-reading a file.
//!
//! Cost: one trust check, one advisory and one whole-file read per secret file, plus a walk of
//! [`SECRETS`] against the named blocks the tree has — at boot and again at each reload.
//! Nothing here runs per request.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Files, Origin, origin_note};
use crate::tree::Config;

/// One secret, materialized: the value `rule:config/a-secret-is-a-file-whose-content-is-the-value` read, and the file it came from.
///
/// The file is kept because it is half of what `rule:config/check-and-dump-audit-the-tree-offline`'s dump prints — `<secret>` says a value is in
/// force and the path says which file decides it, and an audit that cannot name the file cannot act
/// on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Secret {
    /// The file's whole content, cut to `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s value.
    pub value: String,
    /// The trusted, absolute path it was read from.
    pub file: PathBuf,
}

/// What [`materialize`] read out of the tree: every secret, by the dotted key it is the value of.
#[derive(Clone, Debug, Default)]
pub struct Materialized {
    /// Each secret under the key it answers — `db.main.password`, never the `_file` sibling that
    /// named it, which is still in the table under its own key.
    pub secrets: BTreeMap<String, Secret>,
    /// `rule:config/a-secret-file-is-checked-for-integrity-and-advised-on-exposure`'s advisories: `W1005` for a secret file another account can read, `W1007` for a
    /// credential whose value begins or ends with whitespace. Never a refusal: that arrives as the
    /// `Err` instead.
    pub warnings: Vec<Diagnostic>,
}

/// One secret directive: the `value`/`value_file` pair `rule:config/a-secret-is-a-file-whose-content-is-the-value` gives a key that holds a credential, as
/// the things a sweep needs to know about it and nothing else.
///
/// The two accessors are function pointers rather than a block name some `match` turns back into a
/// field, which is what makes **a pair exactly one row and no second place**: the row that names
/// `password_file` is the row that reads it, and there is no arm anywhere else to forget. The cost
/// is a `&'static` table of thin pointers, walked at boot and at each reload and never on the
/// request path.
#[derive(Clone, Copy, Debug)]
pub struct SecretPair {
    /// The block the pair is written in, where `*` stands for the one segment an operator names —
    /// `db.*` for `rule:core-classes/db-one-api`'s `[db.<name>]`. Every key and every diagnostic is built out of this.
    ///
    /// A block with no `*` is a single block rather than a map, and it answers at most one site,
    /// named with the empty string: there is no segment for a name to go in, so every key this row
    /// builds is the block's own.
    pub block: &'static str,
    /// The value's own segment. Its `_file` sibling is this plus `_file`, and `rule:config/a-secret-is-a-file-whose-content-is-the-value` gives no way to
    /// spell either half differently.
    pub value: &'static str,
    /// Every block of this pair's map the tree has, named or not, secret or not.
    sites: fn(&Config) -> Vec<Site<'_>>,
    /// Puts a read value onto the typed tree, at the block named.
    set: fn(&mut Config, &str, &str),
}

/// One block a pair could be written in, as the facts `rule:config/a-secret-is-a-file-whose-content-is-the-value` asks of it.
#[derive(Clone, Copy, Debug)]
struct Site<'a> {
    /// The operator's name for the block — `main`, in `[db.main]`.
    name: &'a str,
    /// The `_file` half, where this block names one.
    file: Option<&'a str>,
    /// The inline half, where this block sets one. Both halves at once is `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s `E0608`, and the
    /// value itself is here rather than a `bool` because the advisory below asks about the value
    /// whichever of the two spellings put it in force.
    inline: Option<&'a str>,
}

impl SecretPair {
    /// The block a site is written in, with the operator's name in place of the `*` — `db.main`.
    #[must_use]
    pub fn block_of(&self, name: &str) -> String {
        self.block.replace('*', name)
    }

    /// The key the pair is the value of — `db.main.password`.
    #[must_use]
    pub fn key(&self, name: &str) -> String {
        format!("{}.{}", self.block_of(name), self.value)
    }

    /// Its `_file` sibling's key — `db.main.password_file`.
    #[must_use]
    pub fn file_key(&self, name: &str) -> String {
        format!("{}_file", self.key(name))
    }
}

/// Every secret directive `rule:config/a-secret-is-a-file-whose-content-is-the-value` marks, one row each.
pub const SECRETS: &[SecretPair] = &[
    // `rule:core-classes/db-capabilities`: the database password, the pair this whole mechanism is
    // written for.
    SecretPair {
        block: "db.*",
        value: "password",
        sites: |config| {
            config
                .db
                .iter()
                .map(|(name, db)| Site {
                    name,
                    file: db.password_file.as_deref(),
                    inline: db.password.as_deref(),
                })
                .collect()
        },
        set: |config, name, value| {
            if let Some(db) = config.db.get_mut(name) {
                db.password = Some(value.to_owned());
            }
        },
    },
    // An `[ldap.<name>]` block's credential: the account every pooled connection of the block binds
    // as, for its whole life (`rule:security/ldap-pool-is-bound-as-its-block`).
    SecretPair {
        block: "ldap.*",
        value: "password",
        sites: |config| {
            config
                .ldap
                .iter()
                .map(|(name, ldap)| Site {
                    name,
                    file: ldap.password_file.as_deref(),
                    inline: ldap.password.as_deref(),
                })
                .collect()
        },
        set: |config, name, value| {
            if let Some(ldap) = config.ldap.get_mut(name) {
                ldap.password = Some(value.to_owned());
            }
        },
    },
    // `rule:programs/framework-core-half`'s SMTP endpoint, which holds a submission credential of exactly the kind above:
    // written by an operator into a named block, sent as `AUTH PLAIN` over `STARTTLS`, and delivered
    // to a container by the same injected file. `nvs_stdlib::mail` needs no code of its own to see
    // it — that module reads `mail.<name>.password` through `Core\Config`, and
    // [`mod@crate::request`] answers a materialized secret before it consults the table.
    SecretPair {
        block: "mail.*",
        value: "password",
        sites: |config| {
            config
                .mail
                .iter()
                .map(|(name, mail)| Site {
                    name,
                    file: mail.password_file.as_deref(),
                    inline: mail.password.as_deref(),
                })
                .collect()
        },
        set: |config, name, value| {
            if let Some(mail) = config.mail.get_mut(name) {
                mail.password = Some(value.to_owned());
            }
        },
    },
    // `rule:http-server/an-outbound-proxy-is-operator-configured`'s proxy credential, which is the
    // same value in a block that is not a map: it becomes `Proxy-Authorization: Basic` on the
    // `CONNECT` request and nothing else ever sees it. The block names no `*`, so its one site is
    // the unnamed one and every key this row builds is the block's own.
    SecretPair {
        block: "http.client.proxy",
        value: "password",
        sites: |config| {
            crate::http::written_proxy(config)
                .map(|proxy| Site {
                    name: "",
                    file: proxy.password_file.as_deref(),
                    inline: proxy.password.as_deref(),
                })
                .into_iter()
                .collect()
        },
        set: |config, _name, value| {
            if let Some(proxy) = config
                .http
                .as_mut()
                .and_then(|http| http.client.as_mut())
                .and_then(|client| client.proxy.as_mut())
            {
                proxy.password = Some(value.to_owned());
            }
        },
    },
    // `rule:security/csrf-is-on-by-default`'s key: what the server door verifies a presented token
    // against, and what an application issues one under. Key material rather than a password, and
    // the pair is what makes it deliverable the way every other credential in this table is — a
    // mounted file, never a line in a tree an operator diffs. The block names no `*`, so its one
    // site is the unnamed one, as `http.client.proxy` above.
    SecretPair {
        block: "http",
        value: "csrf_key",
        sites: |config| {
            config
                .http
                .as_ref()
                .map(|http| Site {
                    name: "",
                    file: http.csrf_key_file.as_deref(),
                    inline: http.csrf_key.as_deref(),
                })
                .into_iter()
                .collect()
        },
        set: |config, _name, value| {
            if let Some(http) = config.http.as_mut() {
                http.csrf_key = Some(value.to_owned());
            }
        },
    },
    // `rule:config/cache-shared-is-the-grant-over-the-configured-store`'s store, behind the
    // password its `AUTH` takes: the same credential as the rows above, in a block an operator does
    // not name, and delivered by the same mounted file. It is a pair rather than URL userinfo
    // because a credential written into a URL is a credential in a string that is logged, and
    // because that would be the second spelling this rule exists to refuse.
    SecretPair {
        block: "cache.shared",
        value: "password",
        sites: |config| {
            config
                .cache
                .as_ref()
                .and_then(|cache| cache.shared.as_ref())
                .map(|shared| Site {
                    name: "",
                    file: shared.password_file.as_deref(),
                    inline: shared.password.as_deref(),
                })
                .into_iter()
                .collect()
        },
        set: |config, _name, value| {
            if let Some(shared) = config
                .cache
                .as_mut()
                .and_then(|cache| cache.shared.as_mut())
            {
                shared.password = Some(value.to_owned());
            }
        },
    },
];

/// What `Core\Config::all` and the dump render a secret's value as. Never the content, and never
/// a fixed-width mask that would say how long it is.
pub const REDACTED: &str = "<secret>";

/// Whether the dotted `key` is the value half of a row of [`SECRETS`] — `db.main.password`,
/// `http.csrf_key` — whichever of the two spellings put it in force.
///
/// A `*` in a row's block matches exactly one segment, so `db.main.password` is one and
/// `db.password` and `db.a.b.password` are not.
#[must_use]
pub fn is_secret_key(key: &str) -> bool {
    SECRETS.iter().any(|pair| {
        let Some(block) = key
            .strip_suffix(pair.value)
            .and_then(|head| head.strip_suffix('.'))
        else {
            return false;
        };
        let mut want = pair.block.split('.');
        let mut have = block.split('.');
        loop {
            match (want.next(), have.next()) {
                (None, None) => return true,
                (Some("*"), Some(segment)) if !segment.is_empty() => {}
                (Some(wanted), Some(segment)) if wanted == segment => {}
                _ => return false,
            }
        }
    })
}

/// `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s cap on a secret file, in bytes.
///
/// Not a resource bound — a credential is never this large, so the cap is what catches a
/// `password_file` pointed at a log, a database or a device before its content becomes a password.
pub const MAX_SECRET_BYTES: usize = 64 * 1024;

/// Reads every secret file the flattened tree names, filling each `_file` sibling's value.
///
/// `origins` is the merge's key-to-file record: it says which file each `password_file` was written
/// in, which is both the directory `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` resolves a relative path against and the file a refusal has
/// to name. A key with no origin is one no file set, so it is not read.
///
/// `config` comes back with every value filled — [`apply`] does that half, and is called again by
/// whoever deserializes the tree a second time.
///
/// # Errors
///
/// One [`Diagnostic`]: `E0608` for a pair set twice or a file `rule:config/a-secret-is-a-file-whose-content-is-the-value` will not take a value from,
/// `E0607` for a secret file outside the trust boundary, `E0605` for one that cannot be read.
///
/// Returns `rule:config/a-secret-file-is-checked-for-integrity-and-advised-on-exposure`'s advisory warnings with the values — `W1005` for a secret file another account can
/// read — which are never a reason to stop.
pub fn materialize(
    config: &mut Config,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<Materialized, Diagnostic> {
    let mut out = Materialized::default();
    for pair in SECRETS {
        // The sites hold a shared borrow of the tree for the whole inner loop, which is why nothing
        // in it touches the tree: reading a file needs the path and the key and no more, and
        // [`apply`] does the writing once the last borrow is gone.
        for site in (pair.sites)(config) {
            let key = pair.key(site.name);
            let file_key = pair.file_key(site.name);
            match (site.file, site.inline) {
                (Some(_), Some(_)) => {
                    return Err(both_set(
                        &pair.block_of(site.name),
                        pair.value,
                        origins.get(&file_key),
                    ));
                }
                (Some(named), None) => {
                    let written_in = origins.get(&file_key);
                    let secret = read(named, &file_key, written_in, files, &mut out.warnings)?;
                    if let Some(edge) = padding(&secret.value) {
                        out.warnings
                            .push(padded(&key, edge, Some(&secret.file), written_in));
                    }
                    out.secrets.insert(key, secret);
                }
                // The inline half is in force and there is nothing for `rule:config/a-secret-is-a-file-whose-content-is-the-value` to read — the value is
                // already the table's. It gets the advisory all the same, because the rule an
                // operator has to hold is then one sentence with no exception in it, and a space
                // before a closing quote is missed about as easily as one inside a file.
                (None, Some(value)) => {
                    if let Some(edge) = padding(value) {
                        out.warnings
                            .push(padded(&key, edge, None, origins.get(&key)));
                    }
                }
                (None, None) => {}
            }
        }
    }
    program_secrets(config, origins, files, &mut out)?;
    apply(config, &out.secrets);
    Ok(out)
}

/// `[secrets]` and each `[app.secrets]`, read into `out` under the key each is the value of:
/// `secrets.<name>`, and `app.<index>.secrets.<name>` for a block's own, which
/// [`Snapshot`](crate::Snapshot)'s fold puts over the global one.
///
/// The same pair as a secret directive, with the same reader: `name` or `name_file`, exactly one.
/// An inline value is carried here too, with the file it was written in, because
/// [`strip_inline`] takes it out of the table.
fn program_secrets(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
    out: &mut Materialized,
) -> Result<(), Diagnostic> {
    let blocks = std::iter::once((
        crate::settings::SECRETS.to_owned(),
        "secrets",
        config.secrets.as_ref(),
    ))
    .chain(config.app.iter().enumerate().map(|(index, app)| {
        (
            format!("app.{index}.secrets"),
            "app.secrets",
            app.secrets.as_ref(),
        )
    }));
    for (prefix, shown, block) in blocks {
        let Some(block) = block else {
            continue;
        };
        let names: std::collections::BTreeSet<&str> = block
            .keys()
            .map(|key| key.strip_suffix("_file").unwrap_or(key))
            .collect();
        for name in names {
            let key = format!("{prefix}.{name}");
            let file_key = format!("{key}_file");
            let inline = block.get(name).and_then(toml::Value::as_str);
            let named = block
                .get(&format!("{name}_file"))
                .and_then(toml::Value::as_str);
            match (named, inline) {
                (Some(_), Some(_)) => {
                    return Err(both_set(shown, name, origins.get(&file_key)));
                }
                (Some(named), None) => {
                    let written_in = origins.get(&file_key);
                    let secret = read(named, &file_key, written_in, files, &mut out.warnings)?;
                    if let Some(edge) = padding(&secret.value) {
                        out.warnings
                            .push(padded(&key, edge, Some(&secret.file), written_in));
                    }
                    out.secrets.insert(key, secret);
                }
                (None, Some(value)) => {
                    let written_in = origins.get(&key);
                    if let Some(edge) = padding(value) {
                        out.warnings.push(padded(&key, edge, None, written_in));
                    }
                    let file = written_in.map(|origin| origin.path.clone());
                    out.secrets.insert(
                        key,
                        Secret {
                            value: value.to_owned(),
                            file: file.unwrap_or_default(),
                        },
                    );
                }
                // `settings::check_file` refused every value that is not a string.
                (None, None) => {}
            }
        }
    }
    Ok(())
}

/// Takes every inline `[secrets]` and `[app.secrets]` value out of the table and the typed tree,
/// once [`materialize`] has carried each beside them. Each `_file` half stays, so the dump can name
/// the file a value came from, as `password_file` stays.
pub fn strip_inline(config: &mut Config, table: &mut toml::Table) {
    let keep_files = |block: &mut toml::Table| block.retain(|key, _| key.ends_with("_file"));
    let strip = |parent: &mut toml::Table| {
        if let Some(toml::Value::Table(block)) = parent.get_mut(crate::settings::SECRETS) {
            keep_files(block);
            if block.is_empty() {
                parent.remove(crate::settings::SECRETS);
            }
        }
    };
    strip(table);
    if let Some(toml::Value::Array(apps)) = table.get_mut("app") {
        for app in apps {
            if let toml::Value::Table(app) = app {
                strip(app);
            }
        }
    }
    let typed = std::iter::once(&mut config.secrets)
        .chain(config.app.iter_mut().map(|app| &mut app.secrets));
    for block in typed {
        if let Some(inner) = block.as_mut() {
            keep_files(inner);
            if inner.is_empty() {
                *block = None;
            }
        }
    }
}

/// Puts every materialized secret onto a typed tree, at the key it is the value of.
///
/// A sweep over [`SECRETS`] like [`materialize`]'s, and this is the *only* place a value
/// reaches [`Config`]: a tree deserialized from the merged table has `password_file` and no
/// `password`, so every deserialization is followed by this call — [`materialize`]'s own, and each
/// one a [`Snapshot`](crate::Snapshot) makes when it retypes.
///
/// A key naming a `[db]` block this tree does not have is skipped rather than creating one: the
/// secrets came from a table this same tree was built from, so that can only be a caller applying
/// one tree's secrets to another's configuration.
pub fn apply(config: &mut Config, secrets: &BTreeMap<String, Secret>) {
    for pair in SECRETS {
        // The names come out owned because the setter takes the tree mutably and a site borrows it.
        let names: Vec<String> = (pair.sites)(config)
            .into_iter()
            .map(|site| site.name.to_owned())
            .collect();
        for name in names {
            if let Some(secret) = secrets.get(&pair.key(&name)) {
                (pair.set)(config, &name, &secret.value);
            }
        }
    }
}

/// One secret file: trusted, advised on, read, and cut to its value.
fn read(
    named: &str,
    key: &str,
    written_in: Option<&Origin>,
    files: &dyn Files,
    warnings: &mut Vec<Diagnostic>,
) -> Result<Secret, Diagnostic> {
    // `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`: relative to the file it was written in, which is the same rule an `[[include]]` path
    // follows. A key with no origin cannot have been written anywhere, so there is nothing but the
    // path itself to resolve against.
    let base = written_in
        .and_then(|origin| origin.path.parent())
        .unwrap_or(Path::new("."));
    let path = crate::resolve::absolute(base, Path::new(named));

    let trusted = files.trust(&path).map_err(|why| {
        crate::resolve::untrusted(
            &path,
            &why,
            "a secret directive's `_file` names it, and whoever can write it chooses the \
             credential the server connects with",
        )
    })?;
    if let Some(how) = files.exposure(&trusted) {
        warnings.push(exposed(&trusted, key, &how));
    }
    let bytes = files.read_bytes(&trusted).map_err(|err| {
        crate::resolve::unreadable(&trusted, &err, "a secret directive's `_file` names it")
    })?;

    if bytes.len() > MAX_SECRET_BYTES {
        return Err(refusal(
            &trusted,
            key,
            &format!(
                "is larger than {} KiB ({} bytes)",
                MAX_SECRET_BYTES / 1024,
                bytes.len()
            ),
            written_in,
        )
        .with_help(
            "a credential is never this large; a `_file` this size is a path pointed at something \
             else"
                .to_string(),
        ));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| refusal(&trusted, key, "is not valid UTF-8", written_in))?;

    let value = strip_one_newline(&text);
    if value.is_empty() {
        return Err(refusal(&trusted, key, "is empty", written_in));
    }
    if value.trim().is_empty() {
        return Err(refusal(&trusted, key, "holds only whitespace", written_in));
    }
    Ok(Secret {
        value: value.to_string(),
        file: trusted,
    })
}

/// How a value is padded, as the phrase a message says it with, and `None` when it is not.
///
/// Asked of the value `rule:config/a-secret-is-a-file-whose-content-is-the-value` arrived at, so the newline that rule already removed is not padding: a
/// file holding `hunter2\n` is the ordinary case and says nothing. A file holding `hunter2\n\n` is
/// `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s spelling of a credential that genuinely ends in a newline, and that one does warn — it is
/// exactly the value someone will later wonder about.
fn padding(value: &str) -> Option<&'static str> {
    match (
        value.starts_with(char::is_whitespace),
        value.ends_with(char::is_whitespace),
    ) {
        (true, true) => Some("begins and ends with whitespace"),
        (true, false) => Some("begins with whitespace"),
        (false, true) => Some("ends with whitespace"),
        (false, false) => None,
    }
}

/// One trailing `\n`, and the `\r` before it, and nothing else — `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s whole trimming rule.
///
/// Only the last newline: a file ending in two of them yields a value ending in one, which is how
/// `rule:config/a-secret-is-a-file-whose-content-is-the-value` spells a credential that genuinely ends in a newline. Leading and interior bytes are never
/// touched, spaces included.
fn strip_one_newline(text: &str) -> &str {
    let cut = text.strip_suffix('\n').unwrap_or(text);
    cut.strip_suffix('\r').unwrap_or(cut)
}

/// `E0608` when both halves of a pair are set: two sources for one value, and `rule:config/a-secret-is-a-file-whose-content-is-the-value` allows one.
fn both_set(block: &str, value: &str, written_in: Option<&Origin>) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_SECRET_FILE,
        format!("`[{block}]` sets both `{value}` and `{value}_file`"),
    )
    .with_note(format!(
        "set exactly one of the two. `{value}` is the value, and `{value}_file` names a file \
         whose content is the value{}",
        origin_note(written_in)
    ))
    .with_help(format!(
        "remove whichever of the two the deployment does not mean; a `{value}` left behind from \
         before the file existed is the usual one"
    ))
}

/// `E0608` about the file's content, phrased as a predicate the path is prefixed onto — the same
/// shape [`mod@crate::trust`] reports a breach in, so the two read alike in a boot log.
fn refusal(path: &Path, key: &str, problem: &str, written_in: Option<&Origin>) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_SECRET_FILE,
        format!("`{key}` names `{}`, which {problem}", path.display()),
    )
    .with_note(format!(
        "the value is the file's whole content, without one trailing newline. A value that \
         cannot be used stops the server at boot, so it does not fail at the first request{}",
        origin_note(written_in)
    ))
}

/// `W1007`: the credential in force has an edge space, which `rule:config/a-secret-is-a-file-whose-content-is-the-value` keeps and this names.
///
/// `from` is the secret file when the file half is in force, because in that spelling there is
/// nothing an operator could have looked at; the inline spelling names only the key, which is
/// already in the file the note points at.
fn padded(key: &str, edge: &str, from: Option<&Path>, written_in: Option<&Origin>) -> Diagnostic {
    Diagnostic::warning(
        code::W_CREDENTIAL_HAS_EDGE_WHITESPACE,
        match from {
            Some(path) => format!("`{key}` names `{}`, whose content {edge}", path.display()),
            None => format!("`{key}` {edge}"),
        },
    )
    .with_note(format!(
        "a credential is used exactly as it is written, because a password may start or end \
         with a space. The value is used as it is, and this is only a warning{}",
        origin_note(written_in)
    ))
    .with_help(
        "if the space is a mistake, write the value again without it. Write a secret file with \
         `printf %s`. Only one trailing newline is removed from the file"
            .to_string(),
    )
}

/// `W1005`: the advisory half, which names the mode and stops.
fn exposed(path: &Path, key: &str, how: &str) -> Diagnostic {
    Diagnostic::warning(
        code::W_SECRET_FILE_READABLE,
        format!("`{key}` names `{}`, which {how}", path.display()),
    )
    .with_note(
        "this is a warning and not an error. A Compose secret is mounted `0444`, and a \
         Kubernetes secret volume is `0644` by default. That is normal in a container, but not \
         on a shared host"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::{is_secret_key, strip_one_newline};

    /// Every row's value key is a secret key, its `_file` sibling and its neighbours are not, and
    /// a `*` matches one segment only.
    #[test]
    fn a_secret_key_is_a_rows_value_half_with_one_segment_for_its_star() {
        for key in [
            "db.main.password",
            "ldap.corp.password",
            "mail.relay.password",
            "http.client.proxy.password",
            "http.csrf_key",
            "cache.shared.password",
        ] {
            assert!(is_secret_key(key), "{key} is a secret directive");
        }
        for key in [
            "db.main.password_file",
            "db.main.driver",
            "db.password",
            "db..password",
            "db.a.b.password",
            "http.csrf_key_file",
            "xhttp.csrf_key",
        ] {
            assert!(!is_secret_key(key), "{key} is not a secret directive");
        }
    }

    /// `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s trimming, on both sides of the one rule it has: one newline goes, a second stays, and
    /// nothing else is touched — the space a password may legitimately end with least of all.
    #[test]
    fn one_trailing_newline_is_stripped_and_nothing_else_is() {
        assert_eq!(strip_one_newline("hunter2\n"), "hunter2");
        assert_eq!(strip_one_newline("hunter2\r\n"), "hunter2");
        assert_eq!(strip_one_newline("hunter2\n\n"), "hunter2\n");
        assert_eq!(strip_one_newline("hunter2"), "hunter2");
        assert_eq!(strip_one_newline(" hunter2 \n"), " hunter2 ");
        assert_eq!(strip_one_newline("\nhunter2"), "\nhunter2");
    }
}
