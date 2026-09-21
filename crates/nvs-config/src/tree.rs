//! The typed block tree one configuration file deserializes into: every block an ADR states, with
//! its own fields, and an unknown key refused.
//!
//! This is the type [`crate::file::parse`] is generic over, and its standing where a bare
//! `toml::Table` could is the whole of what makes [ADR 0064 § 3]'s unknown-key refusal real:
//! `deny_unknown_fields` sits on every struct here, so a typo'd `capabilties` fails at boot with
//! the line under it instead of reading as "granted nothing". That refusal is `serde`'s, not ours —
//! this module is the *roster*, and the roster is the security-relevant half.
//!
//! **Every field comes from the ADR that owns its block**, and [ADR 0064 § 2a] is the table saying
//! which ADR that is. Nothing here invents a key, because a key invented in this file becomes a key
//! the operator may write and no other code reads. Where an ADR states a block but no field set,
//! the gap is recorded in that struct's doc comment rather than filled in.
//!
//! **This tree answers which keys exist, not whether a value is usable.** A `memory = "12 bananas"`
//! deserializes into a [`Setting::Text`] here and is refused where sizes are parsed; a
//! `same_site = "None"` with `secure = false` is refused by the HTTP layer that reads the pair
//! (`rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`). The split is deliberate: a value refusal wants to name the unit it
//! expected, which `serde`'s "invalid type" cannot, and the override stream of
//! [ADR 0103 § 3](/docs/decisions/0103.md) resolves *before*
//! anything is interpreted, so a value overridden by a later file must not have had to parse.
//!
//! Which is also why every field is an [`Option`]: unset and set-to-the-shipped-default are
//! different facts to the override record, and only one of them has an origin to report.
//!
//! Cost: one owned tree per configuration file, held for the length of the boot or reload that
//! reads it and then dropped once the snapshot is built. Nothing here runs per request, and the
//! `Option`-per-field shape is chosen for that reason — it would be the wrong trade on a hot path.
//!
//! [ADR 0064 § 2a]: ../../../docs/decisions/0064.md
//! [ADR 0064 § 3]: ../../../docs/decisions/0064.md

use std::collections::BTreeMap;

use serde::Deserialize;

/// One directive's value, in the shapes the ADRs actually write.
///
/// Several directives are spelled two ways on purpose and the second spelling is load-bearing:
/// `[limits.hard] memory = false` removes a ceiling (`rule:config/three-changeability-classes`, and 0064 § 1 chose TOML partly
/// because "off" is a boolean there), `[metrics] exporter = false` disables an export,
/// `[http.headers] frame_ancestors` is `"none"`, `"self"` or a list of origins. A field typed
/// `String` would refuse the operator's own documented example, so those fields are typed here.
///
/// A table is deliberately absent: every block-shaped value in the configuration is a block with a
/// struct of its own, so a table arriving where a directive is expected is a mistake worth
/// refusing rather than a shape worth carrying.
///
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Setting {
    /// `false` for "no ceiling" / "no exporter", `true` for an unrestricted grant.
    Bool(bool),
    /// A count: `max_tasks = 64`.
    Integer(i64),
    /// A ratio: `[trace] sample = 0.01`.
    Float(f64),
    /// A size, a duration, a path, an enumerated word — every one of which is written as a string.
    Text(String),
    /// A list of paths or origins: `script.spawn = ["/srv/www/jobs"]`.
    List(Vec<String>),
}

/// One configuration file, whole.
///
/// The root table holds no directives of its own — `rule:config/lists-are-arrays-and-repeated-records-are-arrays-of-tables` puts every directive inside a
/// block — so every field here names a block, and the array-of-tables blocks are the repeated
/// records: `[[include]]`, `[[app]]`, `[[extension]]` and `[[schedule]]`.
///
/// A file that sets nothing deserializes into [`Config::default`], which is what makes an empty
/// include legal rather than a parse failure.
///
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// `[[include]]` — the tree of files (`rule:config/include-takes-a-path-or-a-dir`).
    pub include: Vec<Include>,
    /// `[[app]]` — one application, keyed on an entry file path (`rule:config/an-application-is-its-entry-file-path`).
    pub app: Vec<App>,
    /// `[limits]` and the `[limits.hard]` ceiling under it (`rule:config/three-changeability-classes`).
    pub limits: Option<Limits>,
    /// `[mode]` — the default a request starts in and the ceiling it may select (ADRs 0005, 0091).
    pub mode: Option<Mode>,
    /// `[capabilities]` — deny-by-default grants (ADRs 0005, 0006).
    pub capabilities: Option<Capabilities>,
    /// `[[extension]]` — a precompiled binary and its pin (`rule:packaging/extension-loading-is-root-controlled`).
    pub extension: Vec<Extension>,
    /// `[debug]` — the probe set, default and ceiling in one (`rule:testing/debug-probes`).
    pub debug: Option<Debug>,
    /// `[io]` — the root the runtime creates temporary directories under (`rule:core-classes/temporary-dir-sweep`).
    pub io: Option<Io>,
    /// `[log]` — the handler ladder's rungs (`rule:errors/escalation-ladder`) and the record's shape (`rule:errors/diagnostic-record`).
    pub log: Option<Log>,
    /// `[http.*]` — the sub-blocks ADRs 0020 § 7 and 0074 own.
    pub http: Option<Http>,
    /// `[db.<name>]` — one named connection per sub-table (`rule:core-classes/db-connection-is-named`), and the `pool = false`
    /// § 13 lets an operator write beside them rather than inside one.
    pub db: Databases,
    /// `[mail.<name>]` — one named SMTP endpoint per sub-table (`rule:programs/framework-core-half`).
    pub mail: BTreeMap<String, MailEndpoint>,
    /// `[storage.<name>]` — one named object-storage disk per sub-table (`rule:programs/framework-core-half`).
    pub storage: BTreeMap<String, StorageDisk>,
    /// `[deferred]` — the after-response executor's bounds (`rule:concurrency/deferred-is-bounded-by-two-directives`).
    pub deferred: Option<Deferred>,
    /// `[[schedule]]` — scheduled work, which is configuration and not an API (`rule:config/scheduled-work-is-a-config-block`).
    pub schedule: Vec<Schedule>,
    /// `[queue]` — the durable job queue and the `[db.<name>]` it stores rows in (`rule:core-classes/queue-storage-is-a-table`).
    pub queue: Option<Queue>,
    /// `[metrics]` — the metrics exporter (`rule:observability/metrics-and-trace-blocks-are-system`).
    pub metrics: Option<Metrics>,
    /// `[trace]` — the trace exporter (`rule:observability/metrics-and-trace-blocks-are-system`).
    pub trace: Option<Trace>,
    /// `[server]` and the `[[server.mount]]` array under it (`rule:http-server/a-request-resolves-in-five-steps` and `rule:http-server/the-server-block-is-boot-class`).
    pub server: Option<Server>,
    /// `[cache]` — the artifact cache's directory (ADRs 0042, 0078 § 2).
    pub cache: Option<Cache>,
    /// `[control]` — the local control socket (`rule:config/one-local-control-socket`).
    pub control: Option<Control>,
    /// `[opcache]` — revalidation and the file cache (`rule:config/an-edit-reaches-the-next-request-without-a-restart`, `rule:config/opcache-file-cache-directives-are-system`).
    pub opcache: Option<Opcache>,
    /// `[session]` — where `Core\Session`'s records live, and how long one survives (`rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot`).
    pub session: Option<Session>,
}

/// One `[[include]]` entry — `rule:config/include-takes-a-path-or-a-dir`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Include {
    /// One file, relative to the directory of the file this entry is written in (§ 5).
    pub path: Option<String>,
    /// Every `*.toml` directly inside, ascending by filename and not recursing. Not a glob.
    pub dir: Option<String>,
    /// Absence is fine; unreadable, unowned or unparseable is not, and the ownership check moves to
    /// the directory that would hold the file (§ 6).
    pub optional: Option<bool>,
}

/// One `[[app]]` block — `rule:config/an-application-is-its-entry-file-path`.
///
/// `root` or `entry`, never both and never neither; `mode` and `origin` sit directly on the block
/// while its directives live in the sub-tables below. Which of those two rules this struct can hold
/// is the difference between a type and a check: TOML gives no way to spell "one of these two
/// keys", so the exclusivity is the resolver's (§ 2) and only the field set is here.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct App {
    /// Every entry file beneath this directory, matched on path-component boundaries.
    pub root: Option<String>,
    /// One entry file exactly — the most specific form of the same test.
    pub entry: Option<String>,
    /// The mode this application starts in, under `[mode] ceiling` like any other.
    pub mode: Option<String>,
    /// What `Core\Router::urlAbsolute` prepends (`rule:http-server/a-mount-table-expands-at-boot`'s fallback reads this key).
    pub origin: Option<String>,
    /// `[app.limits]`, and `[app.limits.hard]` beneath it.
    pub limits: Option<Limits>,
    /// `[app.capabilities]` — grants for this application only.
    pub capabilities: Option<Capabilities>,
    /// `[app.log]` — `rule:errors/handler-script`'s escalation handler for this application, and the rungs beside
    /// it. Per application rather than per file because that is the unit an operator reports a
    /// failure *of*: one handler answers for every entry `rule:config/an-application-is-its-entry-file-path`'s block covers, and a
    /// deployment running two applications out of one tree gets two, which a root-only key could
    /// not spell. It folds onto the global `[log]` block like every other sub-table here —
    /// `crate::snapshot`'s per-app merge takes the whole block and names none of these fields.
    pub log: Option<Log>,
}

/// `[limits]` — what a request starts with, plus the ceiling it may raise itself to (`rule:config/three-changeability-classes`).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    /// `Runtime` — the heap a request starts with.
    pub memory: Option<Setting>,
    /// `Runtime` — CPU time.
    pub cpu_time: Option<Setting>,
    /// `Runtime` — wall time.
    pub wall_time: Option<Setting>,
    /// `Runtime` — concurrent tasks. Parsed, rostered and named to users, and **enforced
    /// nowhere yet**: no scheduler or budget reads it, though
    /// `rule:security/isolate-budget-is-the-trees` names it as the root's bound on child tasks.
    pub max_tasks: Option<Setting>,
    /// `Runtime` — bytes written to the response.
    pub max_output: Option<Setting>,
    /// `Runtime` — how many steps `rule:core-classes/regex-two-tiers`'s backtracking tier may spend
    /// on one subject before it throws. Unset, the budget `crates/nvs-stdlib/src/regex.rs` states,
    /// which is the figure that engine's own adversarial-pattern tests are written against. The
    /// linear tier reads nothing here, because it runs under no budget at all, so this bounds only
    /// the patterns the linear engine cannot express.
    pub max_regex_steps: Option<Setting>,
    /// `System` — `rule:errors/on-limit`'s reserved slice: the bytes carved out of [`memory`](Self::memory)
    /// at request start and left for the tier-1 handler, which is the one thing that may still
    /// allocate once the rest of the ceiling is gone. `System` rather than `Runtime` because it is
    /// the request's own safety net, and it is not under `[limits.hard]` for the same reason —
    /// there is no request-set value for a ceiling to bound.
    pub fatal_reserve_memory: Option<Setting>,
    /// `System` — the other half of `rule:errors/on-limit`'s reserved slice: the CPU time carved out of
    /// [`cpu_time`](Self::cpu_time) and left for the same handler, for the same reason and under the
    /// same class. A duration where its sibling is a size; neither is under `[limits.hard]`.
    pub fatal_reserve_time: Option<Setting>,
    /// `System` — how deep a chain of `spawn script` may nest before the next one is refused.
    /// `System` on the reserve's grounds, which is why it sits beside them: a script choosing its
    /// own recursion ceiling is the case that class exists for. Not under `[limits.hard]` either,
    /// and for the reserve's reason — no request may set it, so there is no value for a ceiling to
    /// bound. Unlike every key above it this one has a *default*: an uncapped recursion of isolates
    /// does not run forever, it exhausts the tree's heap, and m6.md's *Verify* asks for that spawn
    /// to be stopped as a depth rather than reported as an out-of-memory.
    pub max_script_depth: Option<Setting>,
    /// `System` — the most any one `Core\Compress` or `Core\Zip` decompression may produce
    /// (`rule:core-classes/decompression-bound`). `System` on the reserve's grounds and beside it
    /// for the same reason: a call chooses how much *less* than this it wants through its own
    /// argument, and a program raising the ceiling it decompresses hostile input under is exactly
    /// the choice that has to be made by someone else. Not under `[limits.hard]`, because this key
    /// *is* the ceiling — there is no request-set value for a second one to bound — and `false`
    /// does not remove it: `crates/nvs-stdlib/src/compress.rs`'s `Bound` reads an unbounded value
    /// as the shipped default, so no configuration spells an unbounded decompression.
    pub max_decompressed: Option<Setting>,
    /// `System` — the other half of the same bound: the most output per octet of input. A bomb is
    /// small on the wire, so a byte ceiling alone is one a small request still reaches.
    pub max_decompression_ratio: Option<Setting>,
    /// `[limits.hard]` — the same keys, `System`-class, and `false` removes a ceiling.
    pub hard: Option<LimitSet>,
}

/// `[limits.hard]`, `[app.limits.hard]` and a `[[schedule]]`'s `limits` — the keys with no
/// ceiling nested under them.
///
/// A separate struct rather than [`Limits`] recursing, because `[limits.hard.hard]` is not a thing
/// anyone should be able to write and a self-referential field would spell exactly that.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct LimitSet {
    /// The ceiling on the heap; `false` removes it (`rule:config/three-changeability-classes`).
    pub memory: Option<Setting>,
    /// The ceiling on CPU time.
    pub cpu_time: Option<Setting>,
    /// The ceiling on wall time.
    pub wall_time: Option<Setting>,
    /// The ceiling on concurrent tasks.
    pub max_tasks: Option<Setting>,
    /// The ceiling on response bytes.
    pub max_output: Option<Setting>,
    /// The ceiling on backtracking steps.
    pub max_regex_steps: Option<Setting>,
}

/// `[mode]` — `rule:http-server/the-mode-ceiling-defaults-to-the-startup-mode`, whose keys `rule:config/three-changeability-classes` gives different classes.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Mode {
    /// `Runtime` — the mode an application starts in.
    pub default: Option<String>,
    /// `System` — the most permissive mode any code may select. Unset means the mode the server
    /// started in, which is what makes a production host unreachable from code with nothing written.
    pub ceiling: Option<String>,
}

/// `[capabilities]`, `[app.capabilities]` and a `[[schedule]]`'s `grants` — deny-by-default,
/// `RuntimeTighten`, and dotted.
///
/// A capability's name is dotted and a dotted TOML key *is* table nesting (`rule:config/lists-are-arrays-and-repeated-records-are-arrays-of-tables`), so
/// `script.spawn = [...]` under `[capabilities]` and a `[capabilities.script]` block with a `spawn`
/// key are the same input and both land in these sub-structs. Nothing has to choose between them.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Capabilities {
    /// `script.spawn` (`rule:security/script-spawn-capability`).
    pub script: Option<CapScript>,
    /// `fs.read` and `fs.write`.
    pub fs: Option<CapFs>,
    /// `net.connect` (`rule:http-server/allow-url-pins-the-address`'s outbound policy is what it is checked against).
    pub net: Option<CapNet>,
    /// `tls.anchors`, `tls.pin`, `tls.any_name` and `tls.insecure`
    /// (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`).
    pub tls: Option<CapTls>,
    /// `process.exec`.
    pub process: Option<CapProcess>,
    /// `debug.trace` and `debug.profile` (`rule:testing/debug-probes`).
    pub debug: Option<CapDebug>,
    /// `db.connect`, `db.open` and `db.schema` (`rule:core-classes/db-capabilities`).
    pub db: Option<CapDb>,
    /// `mail.send` (`rule:programs/framework-core-half`).
    pub mail: Option<CapMail>,
    /// `cache.shared` (`rule:config/cache-shared-is-the-grant-over-the-configured-store`).
    pub cache: Option<CapCache>,
    /// `queue.purge` (`rule:concurrency/queue-deletion-is-explicit-and-bounded`).
    pub queue: Option<CapQueue>,
}

/// The `script.*` grants.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapScript {
    /// The roots a `spawn script` target may live under. Being able to *read* a file is not
    /// permission to run it, so this does not follow from `fs.read` (`rule:security/script-spawn-capability`).
    pub spawn: Option<Setting>,
}

/// The `fs.*` grants.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapFs {
    /// The roots readable.
    pub read: Option<Setting>,
    /// The roots writable.
    pub write: Option<Setting>,
}

/// The `net.*` grants.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapNet {
    /// The hosts an outbound connection may reach.
    pub connect: Option<Setting>,
    /// The hosts a call may name its own address for with `connectTo`
    /// (`rule:http-server/an-outbound-call-names-its-address-only-under-a-grant`). The address
    /// written is judged by `rule:security/net-address-policy` and by `internal` below exactly as a
    /// resolved one is, and the certificate is still checked against the URL's host, so this
    /// chooses among the addresses a deployment already reaches rather than widening the set.
    ///
    /// `true` is not a spelling it has, for `internal`'s reason.
    pub connect_to: Option<Setting>,
    /// The hosts a redirect from `https` to `http` may land on
    /// (`rule:http-server/an-https-redirect-never-becomes-plaintext`), which the call must ask for
    /// as well. A plain `http` URL asked for directly needs nothing here — a program fetching an
    /// internal `http` endpoint is not being attacked by itself; what needs naming is a server
    /// stripping a call's TLS with one `Location` header while the caller sees a `200`.
    ///
    /// `true` is not a spelling it has, for `internal`'s reason.
    pub downgrade: Option<Setting>,
    /// The addresses inside `rule:security/net-address-policy`'s denied ranges this deployment reaches anyway — the
    /// operator's exception, written as IP address literals and never as hostnames, because the
    /// policy is asked of a resolved address and a name can resolve anywhere.
    ///
    /// It widens nothing on its own: an address named here is still only reachable under a host
    /// `connect` grants. `true` is not a spelling it has — an exception names the address it wants,
    /// so that what a deployment gave back is legible in review.
    pub internal: Option<Setting>,
    /// The endpoints a program may bind, written as `address:port` literals and matched exactly
    /// (`rule:security/net-listen-is-a-separate-grant-from-net-connect`). An entry that does not
    /// parse as an endpoint matches nothing, and `true` is every endpoint this process may bind.
    ///
    /// It carries no address policy, because the policy's terms invert under a bind: loopback is
    /// the contained endpoint and the unspecified address the exposed one, so `connect`'s denied
    /// ranges would refuse the safe spelling and admit the dangerous one.
    pub listen: Option<Setting>,
    /// The socket paths a program may connect to or bind, written as absolute paths or directory
    /// prefixes and canonicalized before matching like every other path-scoped grant
    /// (`rule:config/net-local-is-named-and-not-on-the-roster`).
    ///
    /// It carries no address policy, because a path has no address, and it follows from `connect`
    /// no more than `connect` follows from it: reaching the network and opening a socket on this
    /// machine are different powers.
    pub local: Option<Setting>,
}

/// The `tls.*` grants — where an outbound call may relax certificate verification
/// (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`).
///
/// Each is a list of hosts, matched as [`CapNet::connect`] matches them, and **none has a `true`
/// spelling**, for [`CapNet::internal`]'s reason: what a deployment relaxed stays legible host by
/// host in review, where a boolean is one line nobody reads again.
///
/// A grant here relaxes nothing on its own. It says *where* verification may be relaxed, and only a
/// call naming the option beside it says *here*; a call that names no option keeps the strict
/// process-wide trust `[http.client.tls]` configures.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapTls {
    /// The hosts a call may trust PEM certificates of its own for with `tlsCa`, in place of
    /// `[http.client.tls] roots`.
    pub anchors: Option<Setting>,
    /// The hosts a call may accept on a `sha256//` public-key pin alone with `tlsPin`, building no
    /// chain — the spelling a self-signed origin is reached by.
    pub pin: Option<Setting>,
    /// The hosts a call may skip the name check for with `tlsVerifyHost: false`, the chain still
    /// built and checked.
    pub any_name: Option<Setting>,
    /// The hosts a call may check neither chain nor name for with `tlsVerify: false`.
    pub insecure: Option<Setting>,
}

/// The `process.*` grants.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapProcess {
    /// Whether a subprocess may be started, written as `true` or as a list of programs.
    pub exec: Option<Setting>,
}

/// The `debug.*` grants — `rule:testing/debug-probes`'s probe sinks.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapDebug {
    /// Where a trace may be written.
    pub trace: Option<Setting>,
    /// Where a profile may be written.
    pub profile: Option<Setting>,
}

/// The `db.*` grants — `rule:core-classes/db-capabilities`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapDb {
    /// Which `[db.<name>]` blocks a program may open by name. An endpoint named here is
    /// operator-written and so is pre-approved against `rule:http-server/allow-url-pins-the-address`'s denied ranges.
    pub connect: Option<Setting>,
    /// Which hosts a program-supplied `Db\Settings` may reach; these stay subject to that policy.
    pub open: Option<Setting>,
    /// Which `[db.<name>]` blocks a program may issue DDL to — `Core\Db\Schema::applySafe` and its
    /// risky twin (`rule:core-classes/schema-apply-capability`). Named like `connect` and not like `open`, because what it gates is
    /// a block and not an address; opening a connection is not permission to change what is behind
    /// it, so it does not follow from `connect`.
    pub schema: Option<Setting>,
}

/// One `[[extension]]` entry — `rule:packaging/extension-loading-is-root-controlled`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Extension {
    /// The `.nvsx` to load.
    pub path: Option<String>,
    /// The pin re-verified on every `nvs ctl reload`; a mismatch refuses the whole swap.
    pub sha256: Option<String>,
}

/// `[debug]` — `rule:testing/debug-mode-directive`, where `nvs.toml` states the default and the ceiling in one value.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Debug {
    /// `[]` is off; any subset of `coverage`, `branch`, `trace`, `profile`. `RuntimeTighten`, so a
    /// request may narrow this and can never turn a bit on.
    ///
    /// [unread: the probes are compiled into every unit and `nvs_runtime`'s `DebugFlags` arms them, but nothing turns a bit on from the file and the directive registry carries no row for this key, so a value written here starts no coverage run and narrows nothing. owner: rule:testing/debug-mode-directive]
    pub mode: Option<Vec<String>>,
    /// `rule:errors/debug-dump` — `true` and a `Core\Debug::dump` made while a request writes an
    /// HTML body is appended to that body as a collapsible block as well as written as a record;
    /// `false` and the record is all there is. A JSON body is never modified either way, in either
    /// mode. `RuntimeTighten`, so a request may turn its own inline output off and can never turn
    /// it on, which leaves the run mode's default (`crate::mode::DERIVED`, whose first row this is)
    /// as the only thing that enables it — and a host that wrote no configuration starts in
    /// `production`, where it is off.
    ///
    /// A mode flip derives this key, `Core\Config` answers it, and `nvs_stdlib::debug` reads it in
    /// force at every dump a request makes — which is the whole of what it does. The block that
    /// results is held on the context and appended by the isolate's finish path
    /// (`nvs_runtime::Ctx::flush_inline_debug`), because whether the body is HTML at all is a
    /// declaration the body member may not have made yet when the dump was written.
    pub inline: Option<bool>,
    /// `rule:core-classes/temporary-dir-sweep` — `true` and the end-of-script sweep logs each path it would have deleted
    /// instead of deleting it, so the absence of cleanup is deliberate and visible rather than a
    /// silent leak. `System` and reloadable per `crate::directive`, which is what lets an operator
    /// flip it on around one problematic request and off again; there is deliberately no
    /// in-language setter and no per-call persist, because a program that could exempt its own
    /// files from cleanup is a program that can be made to hoard them.
    pub keep_temporary: Option<bool>,
}

/// `[io]` — `rule:core-classes/temporary-dir-sweep`'s owned root: the one directory `Core\IO::temporaryDir` creates under.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Io {
    /// Where a temporary directory is created. Unset is a `novis` subdirectory of the platform
    /// temporary directory, which is what an unconfigured deployment gets; an operator writes this
    /// to put temporaries on a particular filesystem — a larger disk, or a tmpfs.
    ///
    /// `Boot`, per `crate::directive`'s `io.temp_root` row: § 4's orphan sweep runs at `nvs serve`
    /// boot over the root it was started with, so moving the root under a running server would
    /// leave the old one holding entries nothing sweeps.
    pub temp_root: Option<String>,
}

/// `[log]` — the escalation ladder's configured rungs (`rule:errors/handler-script` and `rule:errors/panics-bypass-user-code`) and the record's own
/// keys (`rule:errors/log-level`).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Log {
    /// Tier 3: a `.nvs` invoked as a `spawn script` isolate with one `ErrorReport` argument.
    pub handler: Option<String>,
    /// The memory reserved for that handler, sized once per core so the tier that reports an
    /// out-of-memory is not itself out of memory.
    pub handler_reserve_memory: Option<Setting>,
    /// The time reserved for it, on the same argument.
    pub handler_reserve_time: Option<Setting>,
    /// Tier 4, the floor: `stderr`, `file:<path>` or `syslog`, hardcoded in Rust and bounded
    /// against the disk it writes to.
    pub target: Option<String>,
    /// Which rendering the target emits — `rule:errors/renderings`'s set **minus** the HTML one, since
    /// that one is a response's and never a destination's.
    pub format: Option<String>,
    /// The minimum level written; its per-mode default is `rule:config/a-mode-is-five-defaults`'s.
    pub level: Option<String>,
}

/// The `[http.*]` blocks: one refusal policy (`rule:errors/compile-failure`) and `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s defaults blocks.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Http {
    /// `[http] csrf_key` — the key the server door verifies a CSRF token
    /// against (`rule:security/csrf-is-on-by-default`). A secret, so
    /// [`crate::secret`]'s `_file` sibling is the other half of it.
    pub csrf_key: Option<String>,
    /// `[http] csrf_key_file`.
    pub csrf_key_file: Option<String>,
    /// `[http.errors]`.
    pub errors: Option<HttpErrors>,
    /// `[http.headers]`.
    pub headers: Option<HttpHeaders>,
    /// `[http.cors]`.
    pub cors: Option<HttpCors>,
    /// `[http.cookies]`.
    pub cookies: Option<HttpCookies>,
    /// `[http.client]`.
    pub client: Option<HttpClient>,
}

/// `[http.errors]` — `rule:errors/compile-failure`, the block that decides what a request which never got a frame
/// shows.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HttpErrors {
    /// `generic` or `full`; `Runtime`-class, defaulting by run mode (`rule:config/a-mode-is-five-defaults`).
    pub detail: Option<String>,
}

/// `[http.headers]` — `rule:http-server/secure-headers-with-nothing-written`'s shipped defaults, applied with no configuration present.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HttpHeaders {
    /// `X-Content-Type-Options: nosniff`.
    pub content_type_options: Option<bool>,
    /// `none`, `self`, or a list of origins.
    pub frame_ancestors: Option<Setting>,
    /// The `Referrer-Policy` value.
    pub referrer_policy: Option<String>,
    /// `Strict-Transport-Security` max-age; `false` disables it.
    pub hsts: Option<Setting>,
    /// Whether that header carries `includeSubDomains`.
    pub hsts_subdomains: Option<bool>,
    /// Empty emits nothing beyond `frame-ancestors`.
    pub content_security_policy: Option<String>,
    /// Empty emits nothing.
    pub permissions_policy: Option<String>,
}

/// `[http.cors]` — `rule:http-server/cors-is-closed-until-origins-are-named`, closed until origins are named.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HttpCors {
    /// Exact origins; `["*"]` is permitted only with `credentials = false`.
    pub origins: Option<Vec<String>>,
    /// The methods a preflight may allow.
    pub methods: Option<Vec<String>>,
    /// The request headers a preflight may allow.
    pub headers: Option<Vec<String>>,
    /// The response headers exposed to script.
    pub expose: Option<Vec<String>>,
    /// Whether credentials are allowed.
    pub credentials: Option<bool>,
    /// How long a preflight may be cached.
    pub max_age: Option<String>,
}

/// `[http.cookies]` — `rule:http-server/cookies-are-secure-httponly-and-lax`, the defaults every `Core\Response::addCookie` inherits.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HttpCookies {
    /// The `Secure` attribute.
    pub secure: Option<bool>,
    /// The `HttpOnly` attribute.
    pub http_only: Option<bool>,
    /// `Lax`, `Strict` or `None`; `None` with `secure = false` is refused where the pair is read.
    pub same_site: Option<String>,
    /// The default path.
    pub path: Option<String>,
}

/// `[http.client]` — `rule:http-server/no-spelling-for-an-unbounded-wait`, where nothing has a spelling for an unbounded outbound wait.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HttpClient {
    /// The connect wait.
    pub connect_timeout: Option<String>,
    /// The total, covering every attempt and every redirect hop.
    pub deadline: Option<String>,
    /// The longest silence a streamed body may go through once its head has arrived
    /// (`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`), where `deadline`
    /// covers the connection and the head and stops.
    pub idle: Option<String>,
    /// The longest a streamed body may take altogether — see [`HttpClient::idle`], whose bound
    /// alone an origin dribbling a byte at a time would never trip.
    pub max_duration: Option<String>,
    /// Redirects are off by default (`rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`).
    pub max_redirects: Option<u32>,
    /// How many idle connections one core may hold between requests
    /// (`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`). `System`
    /// class with [`HttpClient::pool_idle_timeout`], because the pair bounds a core's memory
    /// rather than a request's; `0` keeps none, which is connect-per-request.
    pub pool_idle: Option<u32>,
    /// How long one may sit idle before it is closed — see [`HttpClient::pool_idle`], under whose
    /// count alone a connection the far end retired hours ago would still be drawn.
    pub pool_idle_timeout: Option<String>,
    /// `[http.client.tls]`.
    pub tls: Option<HttpClientTls>,
    /// `[http.client.proxy]`.
    pub proxy: Option<HttpClientProxy>,
    /// `[http.client.socket]`.
    pub socket: Option<HttpClientSocket>,
}

/// `[http.client.tls]` — whose certificates an outbound `https` call believes, the version floor it
/// speaks over, and the debugging file it may write its secrets to.
///
/// `System` as a block, and for the reason `rule:config/three-changeability-classes` gives: these
/// three settle the one `ClientConfig` every session in the process shares
/// (`nvs_host::tls`'s module doc § *The trust anchors are compiled in*), so a request setting one
/// would be setting it for every co-resident request. There is no code-side spelling for any of
/// them, which is the same closure `rule:security/one-tls-client` puts on a driver: a program does
/// not widen a decision the deployment made.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HttpClientTls {
    /// The trust anchors, in order. Each entry is `"bundled"` — the compiled-in Mozilla set — or a
    /// PEM file, so `["bundled", "/etc/novis/corp-ca.pem"]` adds a company CA to the shipped set
    /// and a list naming only files trusts only those files. Unset is `["bundled"]`.
    ///
    /// Every file entry is resolved against the configuration file that wrote it and trust-checked
    /// there (`rule:config/ownership-is-the-trust-boundary`), exactly as
    /// [`Database::tls_ca_file`] is and for the same reason: whoever can rewrite one chooses which
    /// server this deployment's outbound calls may be talking to. It is not *read* here —
    /// `nvs_host::tls` parses it, because that crate owns the one answer to whose certificates this
    /// process believes.
    pub roots: Option<Vec<String>>,
    /// The version floor every outbound call speaks over — `"1.2"` or `"1.3"`, and nothing below,
    /// because the client implements neither TLS 1.0 nor 1.1. Unset is `"1.2"`.
    pub min_version: Option<String>,
    /// A file each session's secrets are appended to in the `SSLKEYLOGFILE` format, so an operator
    /// can read their own traffic in Wireshark. Unset, and nothing is written.
    ///
    /// Refused at boot when the host runs in `production` (`E0640`): the file decrypts everything
    /// this deployment sends, credentials included, for whoever can read it.
    pub keylog: Option<String>,
}

/// `[http.client.proxy]` — the forward proxy every `Core\Http\Client` call leaves through, and the
/// one word saying whether the address pin survives it.
///
/// `System` as a block and `Reload`
/// (`rule:http-server/an-outbound-proxy-is-operator-configured`): where every outbound byte goes is
/// a deployment's decision, so there is no call option, no client option and no program-side
/// spelling, and no environment variable is read. A changed value is in force for the next call,
/// because the pool's key carries the proxy and nothing the old value made can serve one.
///
/// An absent block is no proxy at all, which is why a written one that names no [`url`](Self::url)
/// is refused rather than read as one: leaving the block out is how a deployment says it wants
/// none.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HttpClientProxy {
    /// The proxy itself, as an `http://` URL with a host and an optional port — `CONNECT` over
    /// plain TCP, since TLS *to* the proxy is a second trust decision with no spelling here. A
    /// credential belongs in [`username`](Self::username) and [`password`](Self::password) and
    /// never in this URL.
    pub url: Option<String>,
    /// Who resolves the destination — `"local"` or `"proxy"`, with no default and no third word
    /// (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
    /// `"local"` keeps everything the pin buys: Novis resolves, checks every address against
    /// `rule:security/net-address-policy` and asks for a `CONNECT` to one it approved. `"proxy"` is
    /// for the network where only the proxy can resolve a name, and it moves the address question
    /// to the proxy — which every boot says out loud.
    pub resolve: Option<String>,
    /// The hosts reached directly, matched against the URL's host text before anything is resolved:
    /// each entry exact, or with a leading `.` for a suffix. A bypassed destination is dialled
    /// under the full address policy. No port, no scheme, no wildcard and no CIDR — an entry
    /// carrying one is refused, because a range here hands back more than an operator can see.
    pub bypass: Option<Vec<String>>,
    /// The user half of the `Proxy-Authorization: Basic` header the `CONNECT` request carries, and
    /// which nothing else does.
    pub username: Option<String>,
    /// The password half, inline. It is `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s
    /// kind of value, so [`password_file`](Self::password_file) is the spelling an audited
    /// deployment wants and setting both is refused.
    pub password: Option<String>,
    /// The file whose whole content is the password, on [`Database::password_file`]'s footing and
    /// read by the same pass.
    pub password_file: Option<String>,
}

/// `[http.client.socket]` — the two bounds an outbound WebSocket has that no other outbound call
/// does: the largest message it will reassemble, and how long one frame may wait to be written.
///
/// `Runtime` as a block, and for `[http.client] deadline`'s reason
/// (`rule:config/three-changeability-classes`): each key bounds one call, a program that knows its
/// own peer names its own value at the call site, and neither is a resource one request could spend
/// on another's behalf. They are written here rather than among [`HttpClient`]'s own keys because
/// they mean nothing to a row that is not `openSocket`, and a block is the smallest thing a
/// `nvs config dump` reader can skip.
///
/// Neither has a spelling that removes the bound: `false` and zero are both `E0647`, because
/// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` is about exactly this pair of
/// waits and `rule:http-server/no-spelling-for-an-unbounded-wait` is the same closure one level up.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HttpClientSocket {
    /// The largest message the socket reassembles, in bytes. Unset is `4194304` — four mebibytes,
    /// which is what this process already applies to the inbound half of RFC 6455, since one
    /// process holding two opinions about the size of one message is how a program comes to work in
    /// one direction and not the other. A message past it closes the socket with `1009` rather than
    /// growing the opening task's memory to whatever the peer sends.
    pub max_message: Option<Setting>,
    /// How long one frame may wait to be written, after which the send fails. Unset is `"30s"`,
    /// the wait the inbound half takes, on [`max_message`](Self::max_message)'s reasoning.
    pub send_timeout: Option<Setting>,
}

/// The `mail.*` grants.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapMail {
    /// Which `[mail.<name>]` blocks a program may send through, by name — never by host, for
    /// [`crate::Cap::MailSend`]'s reason.
    pub send: Option<Setting>,
}

/// The `cache.*` grants.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapCache {
    /// Whether this program may reach the coherent tier — the store `[cache.shared] url` names,
    /// whichever address or socket that is. Unscoped, for [`crate::Cap::CacheShared`]'s reason:
    /// a deployment has one shared store, so there is nothing to name here.
    pub shared: Option<Setting>,
}

/// The `queue.*` grants.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapQueue {
    /// The queues whose rows a program may remove — `Core\Queue`'s `delete` and `purge`, and the
    /// only two members of that class taking a grant at all
    /// (`rule:concurrency/queue-deletion-is-explicit-and-bounded`).
    ///
    /// Named by queue and matched exactly, which is [`CapDb::connect`]'s shape and for the same
    /// reason: a queue name is a name the deployment wrote rather than a host, so two queues
    /// differing only in case are two queues. `true` is every queue this program enqueues to, and a
    /// deployment that never removes a row writes nothing here — the retention entry is the one
    /// place a grant belongs, since a web entry that enqueues needs none.
    pub purge: Option<Setting>,
}

/// One `[mail.<name>]` block — `rule:programs/framework-core-half`'s operator-named SMTP endpoint, whose shape is
/// [`Database`]'s and for the same reason: the name is the key and the settings are the
/// operator's alone, so nothing a program writes can reach past this struct.
///
/// `user` and `password` are the pair that asks for TLS: a block naming them is sent through
/// `STARTTLS` and refused where the endpoint cannot carry one, and a block naming neither stays in
/// the clear. `nvs_stdlib::mail`'s module doc is the home of that decision, including why the
/// upgrade is not opportunistic. The password is `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s kind of value, so `password_file` is
/// beside it and [`mod@crate::secret`] owns every rule about the pair.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct MailEndpoint {
    /// The relay's host. Never program-supplied and never laundered — an operator wrote it.
    pub host: Option<String>,
    /// The submission port, 25 where the block names none.
    pub port: Option<u16>,
    /// The envelope sender and the `From:` header, both. There is no call-site override: domain
    /// alignment is a fact about the deployment.
    pub from: Option<String>,
    /// The submission user. Set with `password` or not at all — half a credential is refused, and a
    /// `password_file` is that half arriving as a file.
    pub user: Option<String>,
    /// Its password, sent as `AUTH PLAIN` over the `STARTTLS` its presence requires.
    pub password: Option<String>,
    /// The file whose whole content is the password, on [`Database::password_file`]'s footing and
    /// under every rule [`mod@crate::secret`] states: exactly one of this and `password` may be set,
    /// and it stays set after the value is read so `rule:config/check-and-dump-audit-the-tree-offline`'s dump can name where the secret
    /// came from.
    pub password_file: Option<String>,
    /// How long the whole exchange may take, 30s where the block names none.
    pub timeout: Option<String>,
}

/// One `[storage.<name>]` block — `rule:programs/framework-core-half`'s object-storage disk, whose shape is
/// [`MailEndpoint`]'s minus everything an endpoint needs and a directory does not.
///
/// **One field, and there is deliberately no `capabilities.storage` beside it.** The grant over a
/// disk is the `fs.read`/`fs.write` the operator already writes about `root`, which is what ADR
/// 0082 § 2's "over `rule:core-api/tier-placement`'s existing `fs.*` capabilities" means and what
/// `nvs_stdlib::storage`'s module doc argues at length: a second grant over one door is the shape
/// where a deployment is tightened in one of them and stays open through the other.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct StorageDisk {
    /// The directory the disk's objects are files in. Every object is one entry directly under
    /// it, because a key is one segment and never a path — `nvs_stdlib::storage` owns why.
    pub root: Option<String>,
}

/// `[db]` — every named block, and the one directive `rule:security/db-pool-reset-is-a-boundary` lets an operator write
/// *unscoped*.
///
/// **A bare map has no home for an unscoped `pool = false`.** § 13's switch is written per block as
/// `[db.<name>] pool = false`, and an audited deployment — one where every connection must map to
/// one request — needs it to reach every connection the process opens, including the one a program
/// described for itself through `Core\Db::open`. That connection names no block, so no per-block key
/// can ever reach it. So `[db]` carries the switch and flattens the blocks beside it.
///
/// **Reserving a *name* inside the map is the other shape, and it collides.** `pool` is a name an
/// operator may already have given a block, and a map that reinterpreted it would silently stop
/// opening that connection. Against this shape a block written `[db.pool]` fails to deserialize as a
/// [`Pool`] and names the key it could not read, which is the loud half of the same trade.
///
/// It [`Deref`](std::ops::Deref)s to [`blocks`](Self::blocks), so `config.db.get("main")` is still
/// the whole of how a name is looked up; the field is named only by the passes that iterate, which
/// cannot borrow through a deref.
///
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Databases {
    /// `[db] pool = false` — § 13's switch with no block in front of it, and the only key this
    /// table holds that is not a block.
    ///
    /// It is the same [`Pool`] a block's own key is, so that `false` is spelled once; a *table* of
    /// bounds written here is refused by `nvs_config::db::validate` rather than given a second
    /// meaning, because bounds unscoped would be a default set for a pool whose key is a
    /// credential hash and could not be sized against any one server.
    pub pool: Option<Pool>,
    /// `[db.<name>]` — one named connection per sub-table (`rule:core-classes/db-connection-is-named`).
    #[serde(flatten)]
    pub blocks: BTreeMap<String, Database>,
}

impl std::ops::Deref for Databases {
    type Target = BTreeMap<String, Database>;

    fn deref(&self) -> &Self::Target {
        &self.blocks
    }
}

impl std::ops::DerefMut for Databases {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.blocks
    }
}

/// One `[db.<name>]` block — `rule:core-classes/db-connection-is-named`, where the name and not the settings is the key.
///
/// **Recorded gap: `rule:core-classes/db-one-api` states `Db\Settings` as a language type and never writes the config
/// block out**, so this roster is every field that ADR names in prose (§ 2's "SQLite takes a `path`
/// and has no `host`, `port`, `user` or `password`", § 4's `Settings.database` and `.user`,
/// § 3a's `password_file`, § 1's `statement_cache`, § 9's `time_zone`, § 11's `slow_query`,
/// § 13's `pool`) plus the
/// `driver` a discriminated union needs to be discriminated on.
/// A field the ADR turns out to have meant and this list omits is a boot refusal naming the line,
/// which is loud and one edit to fix; the fix is to add the field here *and* the example to 0067.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Database {
    /// Which of `rule:core-classes/db-one-api`'s drivers this block selects.
    pub driver: Option<String>,
    /// SQLite's file, resolved against the directory of the file it is written in (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`).
    pub path: Option<String>,
    /// The server host. Refuses `tainted` at the language level and has no launderer (§ 3).
    pub host: Option<String>,
    /// The server port.
    pub port: Option<u16>,
    /// The user.
    pub user: Option<String>,
    /// The password, inline.
    pub password: Option<String>,
    /// The file whose whole content is the password, minus one trailing newline
    /// (`rule:config/a-secret-is-a-file-whose-content-is-the-value`). Exactly one of this and `password` may be set, and it stays set after the
    /// value is read so § 9's dump can name where the secret came from. [`mod@crate::secret`] is
    /// every rule about it; this is the only field on this struct another module writes to.
    pub password_file: Option<String>,
    /// The database name.
    pub database: Option<String>,
    /// The PEM file of trust anchors this server's certificate is verified against, resolved
    /// against the directory of the file it is written in (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`).
    ///
    /// **Written, it replaces the compiled-in Mozilla set for this block and does not add to it** —
    /// `nvs_host::tls`'s module doc owns why, and it is `sslrootcert`'s meaning on every other
    /// client an operator has used. A server behind a private CA is exactly the case where a public
    /// CA vouching for it is the attack.
    ///
    /// It is a trust-boundary file on the footing of [`Self::password_file`]: whoever can write it
    /// chooses which server this connection may be talking to. So [`mod@crate::db`] resolves and
    /// trust-checks it at boot rather than at connect time, and a bundle another account can write
    /// is a boot refusal.
    pub tls_ca_file: Option<String>,
    /// How many server-side prepared statements one connection keeps alive (`rule:core-classes/db-one-api`).
    ///
    /// Unset is the driver's own default rather than a number written here, because the size that
    /// suits a request is a property of the protocol and not of this file. A written `0` is not a
    /// broken cache: it is the unnamed statement every time, which is what a connection keeping no
    /// statements alive does. The reader is `nvs_db::sql::StatementCache::capacity_for`, which is
    /// also where the default lives — this crate names no driver's constant.
    pub statement_cache: Option<u32>,
    /// How long a statement on this connection may take before it is also written to `Core\Log`
    /// (`rule:observability/a-slow-query-is-logged-past-a-threshold`), as `200ms` or `1s`.
    ///
    /// Unset is off, and that is the ADR's own default rather than a number: § 11 gives the
    /// threshold no value, and a slow-query log every deployment gets without asking would be the
    /// ungated output that section refuses. A written `0` is legal and logs every statement, which
    /// is the honest reading of "slower than nothing" and the spelling an operator debugging one
    /// request reaches for. `nvs_config::db::slow_query_for` is the reader, and the line itself is
    /// `Core\Db`'s — it carries the span `rule:observability/trace-events-carry-a-kind`'s `query` event carries, which is what `rule:observability/a-slow-query-is-logged-past-a-threshold` means
    /// by *the same facts*.
    pub slow_query: Option<Setting>,
    /// The zone this database's zone-less `DATETIME`/`TIMESTAMP` columns are written in
    /// (`rule:core-classes/db-column-types`), defaulting to UTC.
    ///
    /// An offset and never a zone name — `"+02:00"`, `"-05:30"`, `"UTC"` — because § 9 sends it to
    /// the server as a numeric offset, and a named zone needs server-side tables that usually are
    /// not populated. `nvs_db::sql::time_zone_for` is the reader and owns the accepted spellings
    /// and the bound; it answers *no* offset for anything else, which is a value refused rather
    /// than a zone read silently wrong.
    pub time_zone: Option<String>,
    /// `[db.<name>.pool]`'s bounds, or the `pool = false` that turns pooling off (`rule:security/db-pool-reset-is-a-boundary`).
    ///
    /// One key in two shapes, because § 13 writes both against the same name and TOML has one `pool`
    /// for a table and a boolean alike. [`Pool`] is that pair; `nvs_config::db::pool_for` is the
    /// reader, and it owns the default every bound takes when this is unset.
    pub pool: Option<Pool>,
}

/// `[db.<name>] pool` — `rule:security/db-pool-reset-is-a-boundary`'s switch, or the table of bounds written under the same key.
///
/// § 13 writes `pool = false` to restore connect-per-request and `[db.<name>.pool] max = 16` for the
/// bounds, and neither spelling can be moved without contradicting the ADR. `true` is the default
/// said out loud rather than a meaning of its own.
#[derive(Clone, Debug, PartialEq)]
pub enum Pool {
    /// `pool = false`, and the `pool = true` that changes nothing.
    Switch(bool),
    /// `[db.<name>.pool]` — one or more of the bounds.
    Bounds(DatabasePool),
}

/// Hand-written rather than `#[serde(untagged)]`, and it is the only one in this module.
///
/// Untagged buys the same shapes for a fraction of the code, but it buffers the value through
/// `serde`'s private `Content` first, so a typo inside the table is reported as *data did not match
/// any variant* with no key and no line — throwing away the unknown-key refusal this module's doc
/// calls its security-relevant half. A visitor dispatches on the shape instead and hands a table
/// straight to [`DatabasePool`]'s own derive, where `deny_unknown_fields` still names the key.
impl<'de> Deserialize<'de> for Pool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(PoolVisitor)
    }
}

/// The dispatch itself: a boolean is the switch, a table is the bounds, and anything else is
/// `expecting`'s sentence.
struct PoolVisitor;

impl<'de> serde::de::Visitor<'de> for PoolVisitor {
    type Value = Pool;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("`false`, or a `[db.<name>.pool]` table of bounds")
    }

    fn visit_bool<E>(self, written: bool) -> Result<Pool, E>
    where
        E: serde::de::Error,
    {
        Ok(Pool::Switch(written))
    }

    fn visit_map<M>(self, map: M) -> Result<Pool, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        DatabasePool::deserialize(serde::de::value::MapAccessDeserializer::new(map))
            .map(Pool::Bounds)
    }
}

/// `[db.<name>.pool]` — `rule:security/db-pool-reset-is-a-boundary`'s bounds.
///
/// The numbers are not here: this struct is the roster, exactly as every other block's is, and
/// `nvs_config::db::PoolBounds` holds the default set beside the parse that reads `"30m"`. Which is
/// also why the counts are typed and the durations are a [`Setting`]: a count has one
/// spelling, so the field's own type is the whole refusal, while a duration has a suffix set and
/// a bare-seconds form that only [`mod@crate::value`] can tell apart.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct DatabasePool {
    /// Connections one core may hold. It is per core, so a deployment's ceiling on the server is
    /// `cores × max` — the number an operator sizes `max_connections` against.
    pub max: Option<u32>,
    /// How many of those stay open with nothing to do; `0` keeps none warm.
    pub idle: Option<u32>,
    /// How long a connection may live before it is retired regardless of health.
    pub lifetime: Option<Setting>,
    /// How long an acquire waits for a free connection before it throws rather than hanging; `0`
    /// never waits, so a request arriving at `max` is refused at once.
    pub acquire: Option<Setting>,
}

/// `[deferred]` — `rule:concurrency/deferred-is-bounded-by-two-directives`'s bounds on after-response work.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Deferred {
    /// `System` — request trees per core kept alive for deferred work. Past the cap
    /// `afterResponse` throws rather than queueing.
    pub max_concurrent: Option<u64>,
    /// `Runtime` — the default a call inherits when it names none.
    pub deadline: Option<String>,
}

/// `[queue]` — `rule:core-classes/queue-storage-is-a-table`'s durable job queue, which is a table in a database an operator names.
///
/// Every key is `System`: the queue is armed at boot and a request may not move it, for `rule:config/scheduled-work-is-a-config-block`'s
/// reason on `[[schedule]]` beside it — work a request could redirect is work a request could
/// redirect into a database it was never granted.
///
/// There is no shape here that turns the queue off, because the block's own absence is that: nothing
/// is enqueued and nothing runs. `workers = 0` is the other half of § 2's answer and is a different
/// fact — this instance enqueues and lets another one work the rows.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Queue {
    /// The `[db.<name>]` block whose connection holds the jobs and dead-letter tables. Required,
    /// and § 2 recommends it be the application's own, because that is what makes an enqueue commit
    /// with the write that caused it.
    pub connection: Option<String>,
    /// Workers this instance runs, per § 2. `0` makes the instance enqueue-only, which is a
    /// supported deployment and not a disabled queue.
    pub workers: Option<u64>,
    /// Attempts a job gets before § 6 moves it to the dead-letter table. Finite with nothing
    /// configured, per `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`, and there is no spelling for unbounded: § 6's whole shape is
    /// that nothing is retried forever and nothing is discarded silently.
    pub max_attempts: Option<u64>,
    /// How long a claimed job stays invisible to other workers before it may be claimed again
    /// (§ 4's lease). A duration, so `5m` and `300s` read the same.
    pub visibility: Option<Setting>,
}

/// One `[[schedule]]` entry — `rule:config/scheduled-work-is-a-config-block`, every key `System`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Schedule {
    /// Required and unique — the log and metric label. A duplicate is a boot error naming both
    /// lines, which is the resolver's check and not this struct's.
    pub name: Option<String>,
    /// Five-field cron, and nothing more (§ 2).
    pub cron: Option<String>,
    /// The `.nvs` to fire, resolved against the `script.spawn` roots and prefix-checked. A path
    /// outside them is a *boot* error, not a first-fire one.
    pub script: Option<String>,
    /// Required, no default: `fleet` or `host`.
    pub scope: Option<String>,
    /// Defaults to `UTC`.
    pub timezone: Option<String>,
    /// `skip`, `queue` or `kill`; defaults to `skip`.
    pub overlap: Option<String>,
    /// An optional sub-cap, narrowing only.
    pub limits: Option<LimitSet>,
    /// Optional grants, narrowing only.
    pub grants: Option<Capabilities>,
}

/// `[metrics]` — `rule:observability/metrics-and-trace-blocks-are-system`, `System`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Metrics {
    /// `false`, `prometheus` or `otlp`.
    pub exporter: Option<Setting>,
    /// The Prometheus scrape endpoint, bound at boot by `nvs serve` and
    /// answered by `nvs_server::serve_scrapes_on_this_core`. Required where
    /// `exporter` is `prometheus`.
    pub listen: Option<String>,
    /// The OTLP collector URL, dialled at boot by `nvs serve` and pushed to by
    /// `nvs_server::push_registry_on_this_core`. Required where `exporter` is
    /// `otlp`.
    pub endpoint: Option<String>,
    /// Per core (§ 7).
    pub max_series: Option<u64>,
}

/// `[trace]` — `rule:observability/metrics-and-trace-blocks-are-system`, `System`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Trace {
    /// `false` or `otlp`.
    pub exporter: Option<Setting>,
    /// The OTLP collector URL, read by `nvs_server::otlp::Endpoint` — where it is parsed, resolved
    /// and dialled — through [`crate::export::Tracing`].
    pub endpoint: Option<String>,
    /// Head-based, 0.0–1.0. An inbound sampled trace is always continued regardless.
    pub sample: Option<f64>,
    /// Whether `traceparent` is sent on outbound `Core\Http\Client` calls.
    pub propagate: Option<bool>,
}

/// `[server]` — `rule:http-server/the-server-block-is-boot-class`, `Boot` as a whole block: a change here needs a restart.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Server {
    /// Every mount path must resolve inside this.
    pub root: Option<String>,
    /// One flat array: `host:port`, or an absolute path meaning a Unix socket. Unix sockets are
    /// Unix-only; Windows listens on TCP loopback.
    pub listen: Option<Vec<String>>,
    /// Unix-socket entries only. What the mode decides is which accounts may connect and so name
    /// their own client address, which is `rule:config/ownership-is-the-trust-boundary`'s question
    /// asked of a socket rather than of a file; [`crate::server::socket_mode_for`] reads it.
    pub socket_mode: Option<String>,
    /// `entry` or `path`; the development default is `path` (`rule:config/a-startup-default-is-never-flipped`).
    pub dispatch: Option<String>,
    /// Static file serving; the development default is on (`rule:config/a-startup-default-is-never-flipped`). Spelled `static` in the
    /// file, which is a Rust keyword.
    #[serde(rename = "static")]
    pub serve_static: Option<bool>,
    /// Fail-closed (§ 6): with nothing here, no forwarding header is believed.
    pub trusted_proxies: Option<Vec<String>>,
    /// Off when empty.
    pub health_path: Option<String>,
    /// The in-flight ceiling.
    pub max_in_flight: Option<u64>,
    /// How many cores accept, over this machine's own available parallelism
    /// (`rule:http-server/the-accept-fan-out-is-one-worker-per-core`).
    pub workers: Option<u64>,
    /// The header read wait — one of the waits below, all finite with nothing configured and all
    /// *idle* rather than total. [`mod@crate::server`] reads them into durations and owns what each
    /// one bounds; a `Setting` rather than a `String` so that `"10s"` and a bare `10` spell the
    /// same wait, which is [`mod@crate::value`]'s rule for every duration in the tree.
    pub header_timeout: Option<Setting>,
    /// The body idle wait.
    pub body_idle_timeout: Option<Setting>,
    /// The write idle wait.
    pub write_idle_timeout: Option<Setting>,
    /// The keep-alive idle wait.
    pub keepalive_timeout: Option<Setting>,
    /// How long a connection keeps being served after this server has begun
    /// draining, spelled like the waits above and bounding a stop rather than an idle socket
    /// ([ADR 0186](/docs/decisions/0186.md) § 3).
    pub drain_timeout: Option<Setting>,
    /// `[server.connection]` — what holds an upgraded connection, which is a different subject
    /// from the waits above (`rule:concurrency/connection-bounds-are-finite`).
    pub connection: Option<ServerConnection>,
    /// `[[server.mount]]` — one rule per mount (§ 4).
    pub mount: Vec<Mount>,
}

/// `[server.connection]` — the bounds one open connection is held inside
/// (`rule:concurrency/connection-bounds-are-finite`).
///
/// A block of its own rather than six more `[server]` keys, because what these bound is a
/// connection that outlived the request which upgraded it, while every wait above bounds a
/// request. An operator reading `send_timeout` beside `write_idle_timeout` in one flat block would
/// have two spellings of one idea in front of them and nothing to say which door each reaches.
///
/// Every key here is a bound and none has an unbounded spelling: `false` and zero are both
/// `E0649`, which [`crate::server::connection_bounds_for`] refuses them with. A key left out is
/// the finite number `nvs_server::bounds::Connection::default` ships, which is where each of those
/// numbers is chosen and argued — this block is what writes over one, never where one lives.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConnection {
    /// How many connections this process may hold open at once. A count, and deliberately not
    /// `[server] max_in_flight`: a request that upgraded has ended, so a connection holding a
    /// coroutine and a root isolate would otherwise be charged to nothing.
    pub max_open: Option<Setting>,
    /// The largest frame payload the codec accepts, in bytes.
    pub max_frame: Option<Setting>,
    /// The largest message — frames reassembled — the codec accepts, in bytes. A value below
    /// `max_frame` is not refused here: a message is at least one frame, and
    /// `nvs_server::bounds::Connection` is what holds the two together.
    pub max_message: Option<Setting>,
    /// How long a connection may go without a frame from its peer before it is closed. An event
    /// stream's peer never speaks, so this bound is unarmed on that door and writing it moves the
    /// WebSocket one alone.
    pub idle_timeout: Option<Setting>,
    /// How long a connection may stay open at all, however busy.
    pub max_lifetime: Option<Setting>,
    /// How long one `send` may take before it throws.
    pub send_timeout: Option<Setting>,
}

/// One `[[server.mount]]` entry — `rule:http-server/a-request-resolves-in-five-steps`.
///
/// There is no `mode` key: `rule:config/a-mount-routes-and-an-app-block-sets-policy` puts a mount's mode on the `[[app]]` block, so an
/// application's mode is one answer wherever the entry file is reached from.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Mount {
    /// A glob under `[server] root`; `*` captures one path segment.
    pub scan: Option<String>,
    /// The path prefix matched, with `{1}` referring to a captured segment and `{1:lower}` to the
    /// same segment in ASCII lower case.
    pub prefix: Option<String>,
    /// The host matched, on the same capture rule. A mount matches on `prefix`, on `host`, or both.
    pub host: Option<String>,
    /// One literal file, overriding a `scan` at this mount's key.
    pub entry: Option<String>,
    /// Optional — what `Core\Router::urlAbsolute` prepends for requests arriving here (§ 3).
    pub origin: Option<String>,
}

/// `[cache]` — `Core\Cache`'s two tiers, and nothing else: `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s compiled
/// artifacts live in `[opcache]`, down to the directory they are read from, which is written
/// `opcache.file_cache_dir` and is the only spelling of it (`docs/decisions/0175.md`).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Cache {
    /// `[cache.local]` — `rule:concurrency/cache-memory-is-charged-to-the-core`'s bound on the per-core tier, for the reason `shared` below
    /// sits here: `Core\Cache` is one class, and its two tiers are looked for under its own name.
    pub local: Option<CacheLocal>,
    /// `[cache.process]` — `rule:concurrency/the-process-tier-is-one-store-per-process`'s tier between the other two, one map
    /// every core of this process reads. It sits here for the reason `local` above does: `Core\Cache`
    /// is one class, and every tier of it is looked for under that name.
    pub process: Option<CacheProcess>,
    /// `[cache.shared]` — `rule:core-api/two-cache-tiers`'s coherent tier, a *store* a fleet dials rather than a map
    /// in this core's memory. It sits here rather than in a block of its own because `Core\Cache` is
    /// one class and an operator looking for where its entries live looks under its own name;
    /// nothing else about the two halves is shared.
    pub shared: Option<CacheShared>,
}

/// `[cache.local]` — what bounds the tier `Core\Cache::local()` hands back.
///
/// One key, because `rule:concurrency/cache-memory-is-charged-to-the-core` leaves exactly one thing to configure about a store that is a map
/// in the calling core's own memory: it has no address, no credential and no timeout, and § 1 gives
/// it no coherence to tune. What it has is a footprint, charged to the core rather than to any
/// request, which is why the key is `System`-class per `crate::directive`'s `cache.local` row.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CacheLocal {
    /// What this core's entries may hold together — `32M`, or `false` for no ceiling at all.
    /// Omitted, the cap `nvs_stdlib::cache` ships, which is that module's to state because it is
    /// the one thing that enforces it. Exceeding this **forgets** entries rather than failing a
    /// write (§ 3), so it is never a reason a `put` throws.
    pub max_size: Option<Setting>,
}

/// `[cache.process]` — what bounds the tier `Core\Cache::process()` hands back, and how long a
/// caller waits on another's fill.
///
/// Two keys, and both are facts about the process rather than about a request: the memory one map
/// holds is spent by every request on the box, and the time one core's request may block on
/// another's fetch is not a bargain a single request strikes on everyone's behalf. Both are
/// `System`-class and `Reload`-apply per `crate::directive`'s `cache.process` row. There is no
/// shard count here and no capability: the first is a fixed constant of the map
/// (`rule:concurrency/the-process-tier-is-one-store-per-process`) and the second has nothing to
/// gate, because this tier has no door onto an effect.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CacheProcess {
    /// What this process's entries may hold together — `32M`, or `false` for no ceiling at all.
    /// Omitted, the cap `nvs_stdlib::cache` ships, which is that module's to state because it is
    /// the one thing that enforces it; it is the same figure the local tier ships, and the same
    /// number here means *less* memory rather than more, because this map is held once per process
    /// where that one is held once per core. Exceeding it forgets the entry written longest ago
    /// rather than failing a write, so it is never a reason a `put` throws.
    pub max_size: Option<Setting>,
    /// How long a caller waits for the one filler in this process before it throws `TimeoutError`
    /// (`rule:concurrency/a-secret-fill-runs-once-per-process`), and the default a `getSecret` that
    /// wants a different one overrides by writing `wait` at its own call site. Omitted, the wait
    /// `nvs_stdlib::cache` ships, for the reason its cap sits there too.
    ///
    /// A wait of nothing and an unbounded one are both refused at boot with `E0642`
    /// (`crate::store::validate`): the first makes every concurrent caller but one throw, and the
    /// second is the spelling `rule:http-server/no-spelling-for-an-unbounded-wait` does not give.
    pub fill_wait: Option<String>,
}

/// `[cache.shared]` — where `Core\Cache::shared()` connects, what the store is reached with, and
/// what bounds a command.
///
/// Which store a fleet's coherent state lives in is a deployment decision, and every key here is
/// `System`-class per `crate::directive`'s `cache.shared` row: a credential and an index are part
/// of that one decision rather than three, because together they are what reaching the store an
/// operator named takes. There is no key here for TLS: it says which transport this is rather
/// than carrying a value over one, so the URL's scheme answers for it — `nvs_stdlib::cache`'s
/// module doc owns why.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CacheShared {
    /// `redis://host[:port]`, `rediss://host[:port]` for the same store behind the one outbound
    /// TLS client, or `unix:/path/to.sock` for a store on this machine
    /// (`rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host`, and [`crate::store::validate`]
    /// for the platform that has no transport for one). Absent, there is no shared tier and
    /// `Core\Cache::shared()` throws saying so rather than answering a store that would behave like
    /// the local one.
    pub url: Option<String>,
    /// The credential the store is behind, sent as `AUTH` ahead of any command on every connection
    /// a core opens — `nvs_stdlib::cache`'s `Dial` is where the whole of it is applied, including
    /// on the reconnect after a dropped socket. Omitted, nothing is sent and the connect costs no
    /// round trip, which is what a store with no password accepts. One value and no user half:
    /// this is the store-wide credential its `AUTH` takes on its own.
    pub password: Option<String>,
    /// The file the credential is read out of, and the half a container injects
    /// (`rule:config/a-secret-is-a-file-whose-content-is-the-value`): exactly one of the pair may
    /// be set, the file's whole content is the value, and that value never enters the merged
    /// table. [`crate::secret::SECRETS`] holds the row that reads it.
    pub password_file: Option<String>,
    /// Which of the store's databases this deployment's entries live in, sent as `SELECT` on every
    /// connection beside the credential above. Omitted, the index the store opens a connection on,
    /// which costs no round trip and is what every deployment that named only a `url` reaches. A
    /// store that does not have the index written here refuses the connection in its own words,
    /// because how many databases it keeps is the store's answer and not this file's.
    pub database: Option<u32>,
    /// The bound on the handshake, and on each command. Omitted, the shipped five seconds.
    pub timeout: Option<String>,
}

/// `[session]` — where a `Core\Session` record lives, what it is called on the way back, and how
/// long an untouched one survives (`rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot`).
///
/// Every key here is `System`/`Boot` per `crate::directive`'s `session` row: where a
/// fleet's sessions live is a deployment decision, and moving it while requests are in flight would
/// strand every live record in the store nobody reads any more. There is no `gc_probability` pair
/// and no `save_path` — § 5 gives expiry to the store, and § 3 gives it no backend that keeps files.
///
/// The block being absent is **not** a default backend: `Core\Session::start()` throws naming this
/// block, which is `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s "nothing configured is already safe" applied to a store nobody chose.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Session {
    /// `shared` or `db`. Neither weak cache tier is spellable here and writing one is `E0626` —
    /// `crate::session::Backend` is the roster and `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent` is the reason.
    pub backend: Option<String>,
    /// How long an untouched record survives, written onto the entry so the store expires it.
    /// Omitted, the two hours `nvs_stdlib::session` ships, which is that module's to state because
    /// it is the one thing that writes the expiry.
    pub ttl: Option<String>,
    /// The cookie name the identifier rides under. Omitted, `nvs_stdlib::session`'s default; the
    /// cookie's *attributes* are not here, because `rule:http-server/cookies-are-secure-httponly-and-lax` already fixes them for every cookie
    /// this server writes and a second spelling would be a way to weaken them.
    pub cookie: Option<String>,
}

/// `[control]` — `rule:config/one-local-control-socket`'s one local socket.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Control {
    /// A path, `\\.\pipe\nvs-control` on Windows, or `false` to disable. There is no TCP listener,
    /// no token and no auth middleware: the socket's owner and mode are the authentication.
    pub socket: Option<Setting>,
}

/// `[opcache]` — revalidation (`rule:config/opcache-revalidation-is-system-class`) and the file cache
/// (`rule:config/opcache-file-cache-directives-are-system`), every key `System`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Opcache {
    /// When a source file is re-`stat`ed; `never` in production.
    pub validate: Option<Setting>,
    /// The rate cap bounding that `stat` overhead.
    pub revalidate_freq: Option<Setting>,
    /// Whether the on-disk artifact cache is used at all.
    pub file_cache: Option<bool>,
    /// Where it lives; root-owned, and defaulting to the per-build location `nvs-cli`'s cache module
    /// derives inside the account already running the compile. The only spelling of the artifact
    /// cache's directory — `[cache]` is `Core\Cache`'s two tiers and holds none — and the one key in
    /// this block that applies at boot rather than at reload, because moving it re-creates the
    /// runtime's mapping of every cached unit.
    pub file_cache_dir: Option<String>,
    /// Its ceiling in bytes.
    pub file_cache_max_size: Option<Setting>,
    /// The GC probability, mirroring PHP's session-GC pair with the divisor below.
    pub file_cache_gc_probability: Option<u32>,
    /// The GC divisor.
    pub file_cache_gc_divisor: Option<u32>,
}
