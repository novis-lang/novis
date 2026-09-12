//! The directive registry: what class a `nvs.toml` directive is, and what applying a change to it
//! requires.
//!
//! A [`Directive`] carries its dotted [`key`](Directive::key), the changeability class
//! `rule:config/three-changeability-classes` defines, and the reloadability field `rule:config/reloadability-is-its-own-field` adds
//! **orthogonal** to it, and nothing else. The two answer different questions — [`Class`] is *who
//! may set it*, [`Apply`] is *what applying a change requires* — and the reason 0078 gives
//! reloadability a field of its own is that `System` would otherwise carry both meanings at once.
//! A registry that derived either field from the other would re-create that conflation while still
//! typechecking, so `tests/directives.rs` holds a census of the pairs and fails if it ever does.
//!
//! **A row covers the keys beneath it.** [`lookup`] is longest-prefix on dot boundaries, so the one
//! `limits` row answers for `limits.memory` and every other key in that block, while the more
//! specific `limits.hard` row answers for `limits.hard.memory`. That is how the ADRs state the
//! classes in the first place — "every `[[schedule]]` key" is `System` (0005), `[server]` is `Boot`
//! (0097 § 5) — so the registry holds one row per *stated* rule rather than a row per key invented
//! to fill the table out. It is therefore **not** the list of legal keys: refusing an unknown key is
//! `serde`'s `deny_unknown_fields` over the typed tree, which is `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s.
//!
//! One roster gap is recorded rather than guessed: `rule:config/reloadability-is-its-own-field`'s `Boot` set names "the
//! thread-per-core count", and no ADR spells that as a key, so it has no row here yet.
//!
//! Cost: one `&'static` slice, no allocation and nothing per request. A lookup is a linear scan of
//! the rows below, run at boot and on each reload and never on the request path.

/// Who may set a directive — `rule:config/three-changeability-classes`'s changeability class.
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
    /// class answers (`rule:config/three-changeability-classes`). Whether the *value* it asked for is accepted is the ceiling's
    /// question and this one's caller's.
    #[must_use]
    pub fn settable_by_a_request(self) -> bool {
        !matches!(self, Class::System)
    }
}

/// What applying a change to a directive requires — `rule:config/reloadability-is-its-own-field`, orthogonal to [`Class`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Apply {
    /// A new snapshot is enough. This is nearly everything, including most of what is `System`.
    Reload,
    /// Applying it would rebind an OS resource or re-create the runtime, so it takes a restart.
    /// `nvs ctl reload` names such a directive in its result rather than silently ignoring the
    /// change.
    Boot,
}

/// One directive: what it is called, who may set it, and what applying it requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Directive {
    /// The dotted key, `limits.hard.memory`. A key naming a block governs every key beneath it —
    /// see the module doc on why the registry is stated that way.
    pub key: &'static str,
    /// Who may set it (`rule:config/three-changeability-classes`).
    pub class: Class,
    /// What applying a change to it requires (`rule:config/reloadability-is-its-own-field`).
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
    // itself to (`rule:config/three-changeability-classes`). Same key names under two different classes, which is why the registry
    // is keyed on the whole dotted path and not on the last segment.
    Directive { key: "limits", class: Class::Runtime, apply: Apply::Reload },
    Directive { key: "limits.hard", class: Class::System, apply: Apply::Reload },
    // The reserve keys in `[limits]`, which are not `Runtime`: `rule:errors/on-limit` makes the tier-1
    // handler's reserved slice `System` on the grounds that a script choosing the size of its own
    // safety net is the case where the choice most needs to be made by someone else. Both halves
    // of the slice are the same net, so they are the same class.
    Directive { key: "limits.fatal_reserve_memory", class: Class::System, apply: Apply::Reload },
    Directive { key: "limits.fatal_reserve_time", class: Class::System, apply: Apply::Reload },
    // The recursion ceiling, on the same grounds and not under `[limits.hard]` for the same
    // reason: a script able to raise its own would exhaust the tree's heap before any depth
    // stopped it, which is the confusion m6.md's *Verify* asks this key to remove.
    Directive { key: "limits.max_script_depth", class: Class::System, apply: Apply::Reload },
    // The decompression bound (`rule:core-classes/decompression-bound`), on the same grounds and
    // not under `[limits.hard]` for the same reason: a call lowers what it decompresses under
    // through its own argument, so a request-set value for a ceiling to bound would be a second
    // way to say the same thing -- and the only direction a script could move a `Runtime` one is
    // the direction the rule forbids.
    Directive { key: "limits.max_decompressed", class: Class::System, apply: Apply::Reload },
    Directive { key: "limits.max_decompression_ratio", class: Class::System, apply: Apply::Reload },
    // `[mode]` is the other block with that same two-halves shape (`rule:config/three-changeability-classes`, `rule:config/two-modes-and-the-default-is-production`).
    Directive { key: "mode.default", class: Class::Runtime, apply: Apply::Reload },
    Directive { key: "mode.ceiling", class: Class::System, apply: Apply::Reload },
    // Every grant in the block is the same class — a script may drop a right it holds and never add
    // one it does not (`rule:config/three-changeability-classes`, `rule:security/isolate-shares-nothing`) — so which grants exist is not this registry's question.
    Directive { key: "capabilities", class: Class::RuntimeTighten, apply: Apply::Reload },
    // `rule:config/three-changeability-classes` names a response header as the counter-example to `System`: a request may set any of
    // `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s policy directives for itself, because it could already write the header directly.
    Directive { key: "http", class: Class::Runtime, apply: Apply::Reload },
    // The outbound pool's two caps, which are the exception inside that block: they bound a
    // **core's** memory rather than a request's, and a request widening one is
    // `rule:config/ceilings-are-their-own-directives`'s failure
    // (`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`).
    Directive { key: "http.client.pool_idle", class: Class::System, apply: Apply::Reload },
    Directive { key: "http.client.pool_idle_timeout", class: Class::System, apply: Apply::Reload },
    // `[log] format` and `level` are rows of `rule:config/a-mode-is-five-defaults`'s mode table, and no row in that table is
    // `System`-class.
    Directive { key: "log", class: Class::Runtime, apply: Apply::Reload },
    // Its sibling that is, and a more specific row for the reason the `[cache]` rows below
    // are: `rule:errors/handler-script` makes the tier-3 handler a `System` directive on exactly the grounds
    // `limits.fatal_reserve_memory` above is one — a script naming the script that reports its own
    // failure is the case where the choice most needs to be made by someone else, and this one
    // names a file to *run*. `Reload` and not `Boot`: the path is resolved when a failure reaches
    // the ladder, so a new value is in force at the next one and nothing is re-created.
    Directive { key: "log.handler", class: Class::System, apply: Apply::Reload },
    // Its ceilings, `System` for a reason of their own rather than by inheritance from the row
    // above: `rule:errors/handler-script`'s reserve exists so that the tier reporting a request's failure is not
    // stopped by that request, and a script that could widen or narrow it would be deciding how
    // loudly its own failure is reported. `Reload` for the handler's reason — both are read when a
    // failure reaches the ladder, so a new value is in force at the next one.
    Directive { key: "log.handler_reserve_memory", class: Class::System, apply: Apply::Reload },
    Directive { key: "log.handler_reserve_time", class: Class::System, apply: Apply::Reload },
    // The floor's destination, `System` because `rule:errors/engine-floor` says so in as many words: tier 4
    // "writes to an operator-owned, `System`-class sink". A request that could move it could send
    // the record of its own failure somewhere nobody reads, which is the same authority
    // `log.handler` withholds one rung up. `Reload` for that row's reason as well — the target is
    // resolved when a record is first written, so a new value is in force for the next context and
    // nothing is re-created. `nvs_runtime::Ctx::write_log_record` is its only reader.
    Directive { key: "log.target", class: Class::System, apply: Apply::Reload },
    // The `Boot` rows `rule:config/reloadability-is-its-own-field` names, less the thread-per-core count the module doc
    // records as unspelled and less `[queue]`'s two, which are written beside the rest of their own
    // block below. `[server]`'s whole block is `Boot` per `rule:http-server/the-server-block-is-boot-class`, which is more than
    // 0078's "the server's listen addresses" and includes them. The artifact cache's directory is
    // one of them and is written `opcache.file_cache_dir` further down: neither `[cache]` key is an
    // artifact directory at all, and the block holds no third one (`docs/decisions/0175.md`).
    // `rule:core-api/two-cache-tiers`'s shared tier is `System` because where a fleet's coherent state lives is not a
    // decision a request may make for itself, and `Boot` because each core holds one connection to
    // it — moving the store re-dials every one of them, which is the same "re-creates the runtime's
    // mapping" the artifact directory is `Boot` for.
    Directive { key: "cache.shared", class: Class::System, apply: Apply::Boot },
    // `rule:concurrency/cache-memory-is-charged-to-the-core`'s cap on the local tier is `System` by the rule `rule:config/three-changeability-classes` states — the memory it
    // bounds is the core's, so a request raising it would spend what every other request on that
    // core then goes without — and `Reload` rather than `Boot` because a new ceiling is read by the
    // next write and enforced by forgetting entries, which re-creates nothing and re-dials nothing.
    Directive { key: "cache.local", class: Class::System, apply: Apply::Reload },
    Directive { key: "control.socket", class: Class::System, apply: Apply::Boot },
    // `rule:core-classes/temporary-dir-sweep`. `System` because the root is the runtime's and not a request's — a request that
    // could move it would be choosing where every *other* request's temporaries land — and `Boot`
    // because § 4's orphan sweep runs once at `nvs serve` boot over the root it started with, so a
    // root swapped under a running server would leave the old one holding entries nothing sweeps.
    Directive { key: "io.temp_root", class: Class::System, apply: Apply::Boot },
    // `rule:core-classes/temporary-dir-sweep`. `System` because § 5 gives the key to the operator alone — a request that could
    // set it would be exempting its own files from cleanup, which is the hoarding that section
    // refuses — and `Reload` because it is read by the next script that ends and applying it
    // re-creates nothing, which is what lets it be flipped on around one problematic request.
    Directive { key: "debug.keep_temporary", class: Class::System, apply: Apply::Reload },
    Directive { key: "server", class: Class::System, apply: Apply::Boot },
    // `System` and `Reload` together: the pairing `rule:config/reloadability-is-its-own-field` exists to make expressible.
    Directive { key: "opcache", class: Class::System, apply: Apply::Reload },
    // The one `[opcache]` key that is not `Reload`, and a longer row than the block above, so
    // `lookup` finds it first: every other file-cache directive is read by the next compile, while
    // moving the directory re-creates the runtime's mapping of every cached unit. `System` for the
    // whole block's reason (`rule:config/opcache-file-cache-directives-are-system`) — a request that could redirect
    // where already-compiled native code is read from would hold a code-injection primitive rather
    // than a performance knob. `nvs_cli::cache::from_config` is its reader.
    Directive { key: "opcache.file_cache_dir", class: Class::System, apply: Apply::Boot },
    // `rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot`: where a fleet's sessions live is a deployment decision, so `System`; `Boot`
    // rather than `Reload` because a backend swapped under a running server strands every live
    // record in the store nothing reads any more, which is the one failure a session store has.
    Directive { key: "session", class: Class::System, apply: Apply::Boot },
    Directive { key: "deferred.max_concurrent", class: Class::System, apply: Apply::Reload },
    // Its sibling is `Runtime`, and `rule:concurrency/deferred-is-bounded-by-two-directives` is explicit that the two halves of `[deferred]`
    // are different classes: the cap is a host-sizing decision and the deadline is an ordinary
    // per-request default a call may name its own value for.
    Directive { key: "deferred.deadline", class: Class::Runtime, apply: Apply::Reload },
    Directive { key: "extension", class: Class::System, apply: Apply::Reload },
    Directive { key: "schedule", class: Class::System, apply: Apply::Reload },
    // `System` throughout, and for one reason: `rule:core-classes/queue-storage-is-a-table` puts the jobs in a connection the
    // operator names, so work a request could redirect is work a request could redirect into a
    // database it was never granted. The apply class then splits, which makes `[queue]` the second
    // block after `[deferred]` whose halves are two of them. `Boot` for the two a worker is built
    // out of: a connection swapped under running workers strands every claim in flight against a
    // database nothing will report to, and a worker is a spawned task, so applying a new count
    // means starting or stopping tasks.
    Directive { key: "queue.connection", class: Class::System, apply: Apply::Boot },
    Directive { key: "queue.workers", class: Class::System, apply: Apply::Boot },
    // `Reload` for the two that are read per job out of the snapshot — the attempts a job gets
    // before it is dead-lettered, and the lease length a claim takes — because a new value is in
    // force for the next job and applying it re-creates nothing.
    Directive { key: "queue.max_attempts", class: Class::System, apply: Apply::Reload },
    Directive { key: "queue.visibility", class: Class::System, apply: Apply::Reload },
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
