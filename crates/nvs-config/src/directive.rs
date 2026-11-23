//! The directive registry: what applying a change to a `nvs.toml` directive requires.
//!
//! A [`Directive`] carries its dotted [`key`](Directive::key) and the reloadability field
//! `rule:config/reloadability-is-its-own-field` defines, and nothing else. *Who may set it* is a
//! different question with a home of its own: [`crate::settable`]'s list of the keys a request may
//! change. The row comments below still say why a key is the operator's alone where that reason is
//! the row's, because a reader looking up a directive reads this table first.
//!
//! **A row covers the keys beneath it.** [`lookup`] is longest-prefix on dot boundaries, so the one
//! `limits` row answers for `limits.memory` and every other key in that block, while the more
//! specific `limits.hard` row answers for `limits.hard.memory`. That is how the rules state
//! reloadability in the first place — `[server] listen` is `Boot`
//! (`rule:config/reloadability-is-its-own-field`) — so the registry holds one row per *stated* rule
//! rather than a row per key invented to fill the table out. It is therefore **not** the list of legal keys: refusing an unknown key is
//! `serde`'s `deny_unknown_fields` over the typed tree, which is `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s.
//!
//! Cost: one `&'static` slice, no allocation and nothing per request. A lookup is a linear scan of
//! the rows below, run at boot and on each reload and never on the request path.

/// What applying a change to a directive requires — `rule:config/reloadability-is-its-own-field`,
/// orthogonal to whether a request may set it, which is [`crate::settable`]'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Apply {
    /// A new snapshot is enough. This is nearly everything, including most of what only the
    /// operator sets.
    Reload,
    /// Applying it would rebind an OS resource or re-create the runtime, so it takes a restart.
    /// A reload names such a directive in its result rather than silently ignoring the change.
    Boot,
}

/// One directive: what it is called, and what applying it requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Directive {
    /// The dotted key, `limits.hard.memory`. A key naming a block governs every key beneath it —
    /// see the module doc on why the registry is stated that way.
    pub key: &'static str,
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

/// Every directive whose reloadability a rule states. Order is presentation only:
/// [`lookup`] takes the longest matching row, not the first.
#[rustfmt::skip] // one row per line: the registry is a table and reads as one.
pub const DIRECTIVES: &[Directive] = &[
    // `[limits]` states the default a request starts with, `[limits.hard]` the ceiling it may raise
    // itself to (`rule:config/ceilings-are-their-own-directives`). The same key names sit in both
    // blocks, which is why the registry is keyed on the whole dotted path and not on the last
    // segment. Both reload: each is read per request. Which limits a request may set is `crate::settable`'s.
    Directive { key: "limits", apply: Apply::Reload },
    Directive { key: "limits.hard", apply: Apply::Reload },
    // The tier-1 handler's reserved slice (`rule:errors/on-limit`). Not on `crate::settable::SETTABLE`,
    // because a script choosing the size of its own safety net is the case where the choice most
    // needs to be made by someone else. Both halves of the slice are the same net. These rows,
    // like the `limits.*` rows below, carry the reason the key is not settable.
    Directive { key: "limits.fatal_reserve_memory", apply: Apply::Reload },
    Directive { key: "limits.fatal_reserve_time", apply: Apply::Reload },
    // The recursion ceiling, not settable and not under `[limits.hard]` for the reserve's reason:
    // a script able to raise its own would exhaust the tree's heap before any depth stopped it,
    // which is the confusion `rule:security/isolate-budget-is-the-trees` asks this key to remove.
    Directive { key: "limits.max_script_depth", apply: Apply::Reload },
    // The decompression bound (`rule:core-classes/decompression-bound`), not settable and not under
    // `[limits.hard]`: a call lowers what it decompresses under through its own argument, so a
    // request may only ever move this ceiling up, which is the direction the rule forbids.
    Directive { key: "limits.max_decompressed", apply: Apply::Reload },
    Directive { key: "limits.max_decompression_ratio", apply: Apply::Reload },
    // `rule:observability/memory-high-water-writes-a-warn`'s threshold, not settable on the
    // reserve's grounds: a program able to raise the share that warns about it could silence the warning.
    Directive { key: "limits.memory_high_water", apply: Apply::Reload },
    // `rule:http-server/a-request-outlives-a-client-that-goes-away`'s two keys, not settable: a
    // request may not change whether its own abandonment cancels it, or how long it may outlive its client.
    Directive { key: "limits.cancel_on_disconnect", apply: Apply::Reload },
    Directive { key: "limits.disconnect_grace", apply: Apply::Reload },
    // `[mode]` has the same two halves, a default and its ceiling
    // (`rule:config/two-modes-and-the-default-is-production`). Both reload. `crate::settable` lets a
    // request move `default` up to `ceiling`, and `ceiling` is the operator's alone.
    Directive { key: "mode.default", apply: Apply::Reload },
    Directive { key: "mode.ceiling", apply: Apply::Reload },
    // The grants a script holds (`rule:security/isolate-shares-nothing`). Not settable, because a
    // list is not a value `set` narrows. Reloads: each request reads the grants from its snapshot.
    Directive { key: "capabilities", apply: Apply::Reload },
    // `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s policy directives, which a
    // request may set for itself (`crate::settable`) because it could already write the header
    // directly. Reloads: each is read per request.
    Directive { key: "http", apply: Apply::Reload },
    // Not settable: the door verifies *every* request's token against this key
    // (`rule:security/csrf-is-on-by-default`), so a request that could set it would be choosing
    // which forgeries its co-residents accept. `Reload` and not `Boot` because the door reads the
    // standing tree per request, so a rotated key is in force for the next one.
    Directive { key: "http.csrf_key", apply: Apply::Reload },
    Directive { key: "http.csrf_key_file", apply: Apply::Reload },
    // The outbound pool's two caps, not settable: they bound a **core's** memory, not a request's,
    // and a request widening one is `rule:config/ceilings-are-their-own-directives`'s failure
    // (`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`). The rows
    // carry that reason, and reload as their block does.
    Directive { key: "http.client.pool_idle", apply: Apply::Reload },
    Directive { key: "http.client.pool_idle_timeout", apply: Apply::Reload },
    // Not settable: `[http.client.tls]` settles the single `ClientConfig` every session in the
    // process shares, so a request that could set one of its keys would be choosing trust anchors
    // for every co-resident request. `Reload`, because a
    // reload builds the new client before it publishes and installs it after (`nvs_host::tls`), and
    // the pool files every connection under the client it was opened with, so the next connection
    // is judged by the new anchors and no socket the old ones accepted serves a later call.
    Directive { key: "http.client.tls", apply: Apply::Reload },
    // `[http.client.proxy]` is not settable: where every outbound byte goes is the deployment's
    // decision, and a per-call value would be a per-call way to narrow
    // `rule:security/net-address-policy`. `Reload` and not `Boot`, because the next call
    // reads the block and the pool's key carries the proxy, so nothing the old value made can serve
    // one (`rule:http-server/an-outbound-proxy-is-operator-configured`).
    Directive { key: "http.client.proxy", apply: Apply::Reload },
    // Each key bounds one socket this request opens
    // (`rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`), so
    // `crate::settable` lists both. Reloads as `http` does; the row is written because a block after
    // three operator-only ones is where a reader assumes the exception carries on.
    Directive { key: "http.client.socket", apply: Apply::Reload },
    // `[log] format` and `level` are rows of `rule:config/a-mode-is-five-defaults`'s mode table.
    // Reloads: each is read per request. Which `[log]` keys a request may set is `crate::settable`'s.
    Directive { key: "log", apply: Apply::Reload },
    // `rule:errors/a-use-of-deprecated-code-may-log-or-throw`: what a use of deprecated code does
    // while it runs (settable, per `crate::settable`). It is read per request, so a reload is in
    // force at the next one. The row names the key and not the block, so a second key in
    // `[errors]` is refused for want of a row.
    Directive { key: "errors.deprecated", apply: Apply::Reload },
    // `rule:core-classes/regex-compile-time-tiering`: whether a backtracking pattern compiles. Not
    // settable: it decides what a unit compiles to, so a request that could set it would undo the
    // operator's `deny` for itself. `Reload` because the value is in the environment hash, so a
    // changed value rekeys every unit and the next resolve recompiles under it.
    Directive { key: "regex.backtracking", apply: Apply::Reload },
    // The tier-3 handler (`rule:errors/handler-script`), not settable on the grounds
    // `limits.fatal_reserve_memory` above is not: a script naming the script that reports its own
    // failure is the case where the choice most needs to be made by someone else, and this one
    // names a file to *run*. `Reload` and not `Boot`: the path is resolved when a failure reaches
    // the ladder, so a new value is in force at the next one and nothing is re-created.
    Directive { key: "log.handler", apply: Apply::Reload },
    // Its ceilings, not settable for a reason of their own: `rule:errors/handler-script`'s reserve
    // exists so that the tier reporting a request's failure is not stopped by that request, and a
    // script that could widen or narrow it would be deciding how loudly its own failure is
    // reported. `Reload` for the handler's reason: both are read when a failure reaches the ladder.
    Directive { key: "log.handler_reserve_memory", apply: Apply::Reload },
    Directive { key: "log.handler_reserve_time", apply: Apply::Reload },
    // The floor's destination, which `rule:errors/engine-floor` gives to the operator alone. A
    // request that could move it could send the record of its own failure somewhere nobody reads,
    // which is the same authority `log.handler` withholds one rung up. `Reload` for that row's reason as well — the target is
    // resolved when a record is first written, so a new value is in force for the next context and
    // nothing is re-created. `nvs_runtime::Ctx::write_log_record` is its only reader.
    Directive { key: "log.target", apply: Apply::Reload },
    // The target's disk bound, `rule:http-server/the-floor-cannot-fill-the-disk`: not settable,
    // because a request that could raise either could fill the disk the floor
    // writes to. `Reload` because the bound is read where the target is, when a context first
    // writes a record, so the next context writes under the new one.
    Directive { key: "log.max_size", apply: Apply::Reload },
    Directive { key: "log.keep", apply: Apply::Reload },
    // `rule:http-server/a-log-line-can-leave-out-the-client-address`: not settable, because a request
    // that could turn the address back on would undo the operator's promise to the people whose
    // addresses these are. `Reload` because it is read off the snapshot each request ran under, so
    // the next request writes under the new value.
    Directive { key: "log.client_ip", apply: Apply::Reload },
    // `[cache]`'s three tiers, one row each. The artifact cache's directory is written
    // `opcache.file_cache_dir` further down: no `[cache]` key is an artifact directory at all
    // (`rule:config/opcache-file-cache-directives-are-system`). The only `Boot` rows are `[server]`'s, written with its block.
    // `rule:core-api/two-cache-tiers`'s shared tier is not settable, because where a fleet's
    // coherent state lives is not a decision a request may make for itself. `Reload` because a request dials the store its own
    // snapshot names, and the schedule ticker opens its fleet lease again when the block moves: the
    // next fire takes its key in the new store, and a fire already running renews in the old one.
    Directive { key: "cache.shared", apply: Apply::Reload },
    // `rule:concurrency/the-memory-tier-is-one-store-per-process`'s two keys are not settable: the
    // map `max_size` bounds is held once
    // for the whole process, so a request raising it would spend what every request on the box then
    // goes without, and `fill_wait` is how long a request on one core may be held waiting on a fetch
    // another core started. `Reload` rather than `Boot` because a new ceiling is read by the next
    // write and enforced by forgetting entries, and a new wait by the next fill — neither re-creates
    // the map, which exists before the workers do, and neither re-dials anything.
    Directive { key: "cache.memory", apply: Apply::Reload },
    // `rule:core-classes/temporary-dir-sweep`. Not settable, because the root is the runtime's and
    // not a request's: a request that could move it would be choosing where every *other* request's temporaries land. `Reload`
    // because `nvs_runtime::capability::temp_dir` reads it from the snapshot its request cloned,
    // so a new root applies to the next temporary directory. A directory made under the old root
    // is deleted by path when its script ends, so nothing live is left there.
    Directive { key: "io.temp_root", apply: Apply::Reload },
    // `rule:core-classes/image-pixel-cap`, not settable on the decompression bound's grounds: a call lowers the
    // cap through its own argument, so a request-set value could only move it up. `Reload` because
    // the component's caller reads it from the snapshot its request cloned. The row names the key
    // and not the block, so a second key in `[image]` has no row until a rule argues one.
    Directive { key: "image.max_pixels", apply: Apply::Reload },
    // `rule:core-classes/temporary-dir-sweep` gives the key to the operator alone: a request that
    // could set it would be exempting its own files from cleanup, which is the hoarding the rule
    // refuses. `Reload` because it is read by the next script that ends and applying it
    // re-creates nothing, which is what lets it be flipped on around one problematic request.
    Directive { key: "debug.keep_temporary", apply: Apply::Reload },
    // `rule:errors/debug-dump`: whether a request prints its dumps inline. A request may only turn
    // it off (`crate::settable`). `Reload` because it is read by the next request and applying it
    // re-creates nothing. Without this row `crate::mode::DERIVED`'s first table entry would be a
    // default for a key `lookup` answers `None` for, which is the drift
    // `every_derived_default_names_a_directive_the_registry_holds` stands in the way of.
    Directive { key: "debug.inline", apply: Apply::Reload },
    // `rule:testing/debug-mode-directive`. Not settable, because a list is not a value
    // `Core\Config::set` narrows. `Reload` because the next context the
    // configuration is handed to reads it, and a server refuses any value at boot and at reload
    // (`crate::debug::refuse_serving`).
    Directive { key: "debug.mode", apply: Apply::Reload },
    // `rule:testing/writing-a-debug-record-to-disk-is-its-own-gate`: whether a request's record is
    // written to disk. A request may only turn it off (`crate::settable`). `Reload` because the next request reads it and applying it re-creates nothing.
    Directive { key: "debug.store", apply: Apply::Reload },
    // `rule:testing/the-debug-stream-reaches-the-service-through-the-disk`. Not settable, because the
    // bound is the disk's and a request that could raise it could fill that disk. `Reload` because
    // a writer reads its share when it next seals a file.
    Directive { key: "debug.spool_max", apply: Apply::Reload },
    // `rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time`, both bounds.
    // Not settable, because recorded bodies are request data and how long they stay on disk is the
    // operator's to decide. `Reload` because `nvs debug` reads them at each deletion pass.
    Directive { key: "debug.index_age", apply: Apply::Reload },
    Directive { key: "debug.index_max", apply: Apply::Reload },
    // `rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`'s listener. Not settable,
    // because the address and its certificate and token decide who can read recorded requests.
    // `Boot` because `nvs debug` binds the address and loads the files once, when it starts.
    Directive { key: "debug.listen", apply: Apply::Boot },
    Directive { key: "debug.tls_cert_file", apply: Apply::Boot },
    Directive { key: "debug.tls_key_file", apply: Apply::Boot },
    Directive { key: "debug.token_file", apply: Apply::Boot },
    // `rule:http-server/the-server-block-is-boot-class`. No key in the block is settable, because
    // every one is the deployment's. The block row is `Reload`, and governs the keys a request reads from the
    // snapshot it cloned (`dispatch`, `static`, `health_path`, `trusted_proxies`,
    // `development_hosts` and `max_in_flight`), the ones a connection reads from the snapshot published when it is
    // accepted (the four waits, `drain_timeout` and `[server.connection]`), and `root` and `mount`,
    // which `nvs serve`'s background expansion reads from the published snapshot and expands again
    // when they changed. The longer rows below it are `Boot`.
    Directive { key: "server", apply: Apply::Reload },
    // `rule:config/reloadability-is-its-own-field`'s four server restart keys. `listen` binds the sockets,
    // and a port below 1024 needs a privilege the process dropped after it bound. `socket_mode` is
    // applied when a Unix listener is bound, so it moves only with `listen`. `workers` sizes the
    // runtime state every core holds. `watchdog_margin` is read once, by the one watchdog the
    // process builds when it starts.
    Directive { key: "server.listen", apply: Apply::Boot },
    Directive { key: "server.socket_mode", apply: Apply::Boot },
    Directive { key: "server.workers", apply: Apply::Boot },
    Directive { key: "server.watchdog_margin", apply: Apply::Boot },
    // Operator-only and `Reload` together: the pairing `rule:config/reloadability-is-its-own-field` exists to make expressible.
    Directive { key: "opcache", apply: Apply::Reload },
    // A row of its own under the block above, with the block's apply, because a reload names this
    // key when it cannot apply it: a directory the ownership check refuses keeps the running one
    // (`rule:config/a-reload-names-what-it-could-not-apply`), and a carried key needs a row. The
    // next compile reads and writes the new directory, and units already in memory stay. Not
    // settable for the whole block's reason (`rule:config/opcache-file-cache-directives-are-system`):
    // a request that could redirect where already-compiled native code is read from would hold a
    // code-injection primitive rather than a performance knob. `nvs_cli::cache::placed` is its reader.
    Directive { key: "opcache.file_cache_dir", apply: Apply::Reload },
    // `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`: where a fleet's sessions live is a deployment decision, so it is not settable. `Reload`
    // because a request's first `Core\Session` call reads the block out of the snapshot its request cloned, and
    // the store it reaches is this core's connection, dialled again when the published tree names
    // another. A changed backend signs every user out, because no record is carried from the old
    // store to the new one; that is the operator's choice to make.
    Directive { key: "session", apply: Apply::Reload },
    Directive { key: "deferred.max_concurrent", apply: Apply::Reload },
    // `rule:concurrency/deferred-is-bounded-by-two-directives` splits `[deferred]` in two: the cap
    // above sizes the host and is the operator's, and the deadline is a per-request default a
    // request may set (`crate::settable`). Both reload.
    Directive { key: "deferred.deadline", apply: Apply::Reload },
    Directive { key: "extension", apply: Apply::Reload },
    // `rule:packaging/a-guest-has-no-ambient-authority`: an extension's settings block is the
    // operator's, as the `[[extension]]` entry that loads it is, so it is not settable. `Reload` because a guest reads it out of its request's
    // snapshot (`nvs:ext/settings`), and a reload checks the new block against the manifests the
    // new set loads before it publishes either.
    Directive { key: "ext", apply: Apply::Reload },
    Directive { key: "schedule", apply: Apply::Reload },
    // No key is settable, for one reason: `rule:core-classes/queue-storage-is-a-table` puts the
    // jobs in a connection the operator names, so work a request could redirect is work a request could redirect into a
    // database it was never granted. Every key reloads. `connection` and `workers` are what a worker
    // is built from, so a new value starts or stops worker tasks: a stopped worker writes back the
    // job it holds first, and a new connection's workers claim from there
    // (`nvs-cli`'s `worker::Crew`). `max_attempts` and `visibility` are read per job out of the
    // snapshot, so a new value is in force for the next job.
    Directive { key: "queue.connection", apply: Apply::Reload },
    Directive { key: "queue.workers", apply: Apply::Reload },
    Directive { key: "queue.max_attempts", apply: Apply::Reload },
    Directive { key: "queue.visibility", apply: Apply::Reload },
    Directive { key: "app", apply: Apply::Reload },
    Directive { key: "include", apply: Apply::Reload },
    Directive { key: "metrics", apply: Apply::Reload },
    Directive { key: "trace", apply: Apply::Reload },
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
