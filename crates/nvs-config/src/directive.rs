//! The directive registry: what class a `nvs.toml` directive is, and what applying a change to it
//! requires.
//!
//! A [`Directive`] carries three fields and no more: its dotted [`key`](Directive::key), the
//! changeability class ADR 0005 defines, and the reloadability field ADR 0078 § 2 adds
//! **orthogonal** to it. The two answer different questions — [`Class`] is *who may set it*,
//! [`Apply`] is *what applying a change requires* — and the whole reason 0078 gave reloadability a
//! field of its own is that `System` had been carrying both meanings at once. A registry that
//! derived either field from the other would re-create that conflation while still typechecking, so
//! `tests/directives.rs` holds a census of the pairs and fails if it ever does.
//!
//! **A row covers the keys beneath it.** [`lookup`] is longest-prefix on dot boundaries, so the one
//! `limits` row answers for `limits.memory` and every other key in that block, while the more
//! specific `limits.hard` row answers for `limits.hard.memory`. That is how the ADRs state the
//! classes in the first place — "every `[[schedule]]` key" is `System` (0005), `[server]` is `Boot`
//! (0097 § 5) — so the registry holds one row per *stated* rule rather than a row per key invented
//! to fill the table out. It is therefore **not** the list of legal keys: refusing an unknown key is
//! `serde`'s `deny_unknown_fields` over the typed tree, which is ADR 0064 § 3's.
//!
//! One roster gap is recorded rather than guessed: ADR 0078 § 2's `Boot` set names "the
//! thread-per-core count", and no ADR spells that as a key, so it has no row here yet.
//!
//! Cost: one `&'static` slice, no allocation and nothing per request. A lookup is a linear scan of
//! nineteen rows, run at boot and on each reload and never on the request path.

/// Who may set a directive — ADR 0005's changeability class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    /// `nvs.toml` is the only place it can be set; `Core\Config::set` fails with `E0602` and
    /// returns `false`. The rule for the class is that changing it from inside a request would
    /// affect something other than that request.
    System,
    /// `nvs.toml` gives the **default** a request starts with, and the request may then set any
    /// value for itself, wider or narrower, up to the `[limits.hard]` ceiling.
    Runtime,
    /// As [`Runtime`](Class::Runtime), but narrowing only: a widening set fails with `E0602`. It is
    /// argued per directive and never as a policy — capability grants, and the directives where PHP
    /// itself behaves this way.
    RuntimeTighten,
}

impl Class {
    /// Whether a request may set a directive of this class at all, which is the one question the
    /// class answers (ADR 0005). Whether the *value* it asked for is accepted is the ceiling's
    /// question and this one's caller's.
    #[must_use]
    pub fn settable_by_a_request(self) -> bool {
        !matches!(self, Class::System)
    }
}

/// What applying a change to a directive requires — ADR 0078 § 2, orthogonal to [`Class`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Apply {
    /// A new snapshot is enough. This is nearly everything, including most of what is `System`.
    Reload,
    /// Applying it would rebind an OS resource or re-create the runtime, so it takes a restart.
    /// `nvs ctl reload` names such a directive in its result rather than silently ignoring the
    /// change.
    Boot,
}

/// One directive, as three fields: what it is called, who may set it, and what applying it requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Directive {
    /// The dotted key, `limits.hard.memory`. A key naming a block governs every key beneath it —
    /// see the module doc on why the registry is stated that way.
    pub key: &'static str,
    /// Who may set it (ADR 0005).
    pub class: Class,
    /// What applying a change to it requires (ADR 0078 § 2).
    pub apply: Apply,
}

impl Directive {
    /// The block the key is written in — everything before its last dot, and `""` for a key written
    /// in the root table or for a row that *is* a block.
    #[must_use]
    pub fn block(&self) -> &'static str {
        match self.key.rfind('.') {
            Some(dot) => &self.key[..dot],
            None => "",
        }
    }
}

/// Every directive whose class and reloadability an ADR states. Order is presentation only:
/// [`lookup`] takes the longest matching row, not the first.
#[rustfmt::skip] // one row per line: the registry is a table and reads as one.
pub const DIRECTIVES: &[Directive] = &[
    // `[limits]` states the default a request starts with, `[limits.hard]` the ceiling it may raise
    // itself to (ADR 0005). Same key names under two different classes, which is why the registry
    // is keyed on the whole dotted path and not on the last segment.
    Directive { key: "limits", class: Class::Runtime, apply: Apply::Reload },
    Directive { key: "limits.hard", class: Class::System, apply: Apply::Reload },
    // The two keys in `[limits]` that are not `Runtime`: ADR 0020 § 1 makes the tier-1 handler's
    // reserved slice `System` on the grounds that a script choosing the size of its own safety net
    // is the case where the choice most needs to be made by someone else. Both halves of the slice
    // are the same net, so they are the same class.
    Directive { key: "limits.fatal_reserve_memory", class: Class::System, apply: Apply::Reload },
    Directive { key: "limits.fatal_reserve_time", class: Class::System, apply: Apply::Reload },
    // The third, on the same grounds and not under `[limits.hard]` for the same reason: a script
    // able to raise its own recursion ceiling would exhaust the tree's heap before any depth
    // stopped it, which is the confusion m6.md's *Verify* asks this key to remove.
    Directive { key: "limits.max_script_depth", class: Class::System, apply: Apply::Reload },
    // `[mode]` is the second and last block with that same two-halves shape (ADR 0005, ADR 0091).
    Directive { key: "mode.default", class: Class::Runtime, apply: Apply::Reload },
    Directive { key: "mode.ceiling", class: Class::System, apply: Apply::Reload },
    // Every grant in the block is the same class — a script may drop a right it holds and never add
    // one it does not (ADR 0005, ADR 0006) — so which grants exist is not this registry's question.
    Directive { key: "capabilities", class: Class::RuntimeTighten, apply: Apply::Reload },
    // ADR 0005 names a response header as the counter-example to `System`: a request may set any of
    // ADR 0074's policy directives for itself, because it could already write the header directly.
    Directive { key: "http", class: Class::Runtime, apply: Apply::Reload },
    // `[log] format` and `level` are rows of ADR 0091 § 3's mode table, and no row in that table is
    // `System`-class.
    Directive { key: "log", class: Class::Runtime, apply: Apply::Reload },
    // The four `Boot` rows ADR 0078 § 2 names, less the thread-per-core count the module doc
    // records as unspelled. `[server]`'s whole block is `Boot` per ADR 0097 § 5, which is more than
    // 0078's "the server's listen addresses" and includes them.
    Directive { key: "cache.dir", class: Class::System, apply: Apply::Boot },
    Directive { key: "control.socket", class: Class::System, apply: Apply::Boot },
    Directive { key: "server", class: Class::System, apply: Apply::Boot },
    // `System` and `Reload` together: the pairing ADR 0078 § 2 exists to make expressible.
    Directive { key: "opcache", class: Class::System, apply: Apply::Reload },
    Directive { key: "deferred.max_concurrent", class: Class::System, apply: Apply::Reload },
    Directive { key: "extension", class: Class::System, apply: Apply::Reload },
    Directive { key: "schedule", class: Class::System, apply: Apply::Reload },
    Directive { key: "app", class: Class::System, apply: Apply::Reload },
    Directive { key: "include", class: Class::System, apply: Apply::Reload },
    Directive { key: "metrics", class: Class::System, apply: Apply::Reload },
    Directive { key: "trace", class: Class::System, apply: Apply::Reload },
];

/// The directive governing `key`: the row with the longest key that is `key` itself or a
/// dot-boundary prefix of it, and `None` when no row governs it at all.
#[must_use]
pub fn lookup(key: &str) -> Option<&'static Directive> {
    DIRECTIVES
        .iter()
        .filter(|row| governs(row.key, key))
        .max_by_key(|row| row.key.len())
}

/// Whether `row` is `key` or names a block `key` is written inside. The dot-boundary test is what
/// keeps `limits` from governing a `limitshard` that was never written.
///
/// Reused by [`snapshot`](crate::snapshot) for the same question asked of a dotted origin key,
/// because a second dot-boundary test is a second chance to get the boundary wrong.
pub(crate) fn governs(row: &str, key: &str) -> bool {
    key == row || (key.starts_with(row) && key.as_bytes().get(row.len()) == Some(&b'.'))
}
