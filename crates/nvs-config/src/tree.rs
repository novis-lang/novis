//! The typed block tree one configuration file deserializes into: every block an ADR states, with
//! its own fields, and an unknown key refused.
//!
//! This is the type [`crate::file::parse`] was written generic over. Substituting it for the
//! `toml::Table` stand-in is the whole of what makes [ADR 0064 § 3]'s unknown-key refusal real:
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
//! ([ADR 0074] § 4). Two reasons the split is deliberate: a value refusal wants to name the unit it
//! expected, which `serde`'s "invalid type" cannot, and the override stream of
//! [ADR 0103 § 3](../../../docs/adr/0103-configuration-is-a-tree-of-files.md) resolves *before*
//! anything is interpreted, so a value overridden by a later file must not have had to parse.
//!
//! Which is also why every field is an [`Option`]: unset and set-to-the-shipped-default are
//! different facts to the override record, and only one of them has an origin to report.
//!
//! Cost: one owned tree per configuration file, held for the length of the boot or reload that
//! reads it and then dropped once the snapshot is built. Nothing here runs per request, and the
//! `Option`-per-field shape is chosen for that reason — it would be the wrong trade on a hot path.
//!
//! [ADR 0064 § 2a]: ../../../docs/adr/0064-configuration-file-format.md
//! [ADR 0064 § 3]: ../../../docs/adr/0064-configuration-file-format.md
//! [ADR 0074]: ../../../docs/adr/0074-http-defaults-safe-and-finite.md

use std::collections::BTreeMap;

use serde::Deserialize;

/// One directive's value, in the shapes the ADRs actually write.
///
/// Several directives are spelled two ways on purpose and the second spelling is load-bearing:
/// `[limits.hard] memory = false` removes a ceiling ([ADR 0005], and 0064 § 1 chose TOML partly
/// because "off" is a boolean there), `[metrics] exporter = false` disables an export,
/// `[http.headers] frame_ancestors` is `"none"`, `"self"` or a list of origins. A field typed
/// `String` would refuse the operator's own documented example, so those fields are typed here.
///
/// A table is deliberately absent: every block-shaped value in the configuration is a block with a
/// struct of its own, so a table arriving where a directive is expected is a mistake worth
/// refusing rather than a shape worth carrying.
///
/// [ADR 0005]: ../../../docs/adr/0005-config-changeability.md
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
/// The root table holds no directives of its own — [ADR 0064] § 2 puts every directive inside a
/// block — so every field here names a block, and the array-of-tables blocks are the four that are
/// repeated records: `[[include]]`, `[[app]]`, `[[extension]]` and `[[schedule]]`.
///
/// A file that sets nothing deserializes into [`Config::default`], which is what makes an empty
/// include legal rather than a parse failure.
///
/// [ADR 0064]: ../../../docs/adr/0064-configuration-file-format.md
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// `[[include]]` — the tree of files (ADR 0103 § 2).
    pub include: Vec<Include>,
    /// `[[app]]` — one application, keyed on an entry file path (ADR 0104 § 1).
    pub app: Vec<App>,
    /// `[limits]` and the `[limits.hard]` ceiling under it (ADR 0005).
    pub limits: Option<Limits>,
    /// `[mode]` — the default a request starts in and the ceiling it may select (ADRs 0005, 0091).
    pub mode: Option<Mode>,
    /// `[capabilities]` — deny-by-default grants (ADRs 0005, 0006).
    pub capabilities: Option<Capabilities>,
    /// `[[extension]]` — a precompiled binary and its pin (ADR 0003 § 3).
    pub extension: Vec<Extension>,
    /// `[debug]` — the probe set, default and ceiling in one (ADR 0018).
    pub debug: Option<Debug>,
    /// `[log]` — the handler ladder's rungs (ADR 0020) and the record's shape (ADR 0092).
    pub log: Option<Log>,
    /// `[http.*]` — the five sub-blocks ADRs 0020 § 7 and 0074 own.
    pub http: Option<Http>,
    /// `[db.<name>]` — one named connection per sub-table (ADR 0067 § 2).
    pub db: BTreeMap<String, Database>,
    /// `[deferred]` — the after-response executor's bounds (ADR 0072 § 7).
    pub deferred: Option<Deferred>,
    /// `[[schedule]]` — scheduled work, which is configuration and not an API (ADR 0073).
    pub schedule: Vec<Schedule>,
    /// `[metrics]` — the metrics exporter (ADR 0076 § 6).
    pub metrics: Option<Metrics>,
    /// `[trace]` — the trace exporter (ADR 0076 § 6).
    pub trace: Option<Trace>,
    /// `[server]` and the `[[server.mount]]` array under it (ADR 0097 §§ 4, 5).
    pub server: Option<Server>,
    /// `[cache]` — the artifact cache's directory (ADRs 0042, 0078 § 2).
    pub cache: Option<Cache>,
    /// `[control]` — the local control socket (ADR 0078 § 3).
    pub control: Option<Control>,
    /// `[opcache]` — revalidation and the file cache (ADRs 0017, 0042 § 9).
    pub opcache: Option<Opcache>,
}

/// One `[[include]]` entry — ADR 0103 § 2.
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

/// One `[[app]]` block — ADR 0104 § 1.
///
/// `root` or `entry`, never both and never neither; `mode` and `origin` sit directly on the block
/// while its directives live in the three sub-tables. Which of those two rules this struct can hold
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
    /// What `Core\Router::urlAbsolute` prepends (ADR 0097 § 3's fallback reads this key).
    pub origin: Option<String>,
    /// `[app.limits]`, and `[app.limits.hard]` beneath it.
    pub limits: Option<Limits>,
    /// `[app.capabilities]` — grants for this application only.
    pub capabilities: Option<Capabilities>,
}

/// `[limits]` — what a request starts with, plus the ceiling it may raise itself to (ADR 0005).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    /// `Runtime` — the heap a request starts with.
    pub memory: Option<Setting>,
    /// `Runtime` — CPU time.
    pub cpu_time: Option<Setting>,
    /// `Runtime` — wall time.
    pub wall_time: Option<Setting>,
    /// `Runtime` — concurrent tasks.
    pub max_tasks: Option<Setting>,
    /// `Runtime` — bytes written to the response.
    pub max_output: Option<Setting>,
    /// `System` — ADR 0020 § 1's reserved slice: the bytes carved out of [`memory`](Self::memory)
    /// at request start and left for the tier-1 handler, which is the one thing that may still
    /// allocate once the rest of the ceiling is gone. `System` rather than `Runtime` because it is
    /// the request's own safety net, and it is not under `[limits.hard]` for the same reason —
    /// there is no request-set value for a ceiling to bound.
    pub fatal_reserve_memory: Option<Setting>,
    /// `[limits.hard]` — the same five keys, `System`-class, and `false` removes a ceiling.
    pub hard: Option<LimitSet>,
}

/// `[limits.hard]`, `[app.limits.hard]` and a `[[schedule]]`'s `limits` — the five keys with no
/// ceiling nested under them.
///
/// A separate struct rather than [`Limits`] recursing, because `[limits.hard.hard]` is not a thing
/// anyone should be able to write and a self-referential field would spell exactly that.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct LimitSet {
    /// The ceiling on the heap; `false` removes it (ADR 0005).
    pub memory: Option<Setting>,
    /// The ceiling on CPU time.
    pub cpu_time: Option<Setting>,
    /// The ceiling on wall time.
    pub wall_time: Option<Setting>,
    /// The ceiling on concurrent tasks.
    pub max_tasks: Option<Setting>,
    /// The ceiling on response bytes.
    pub max_output: Option<Setting>,
}

/// `[mode]` — ADR 0091 § 5, whose two keys ADR 0005 gives two different classes.
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
/// A capability's name is dotted and a dotted TOML key *is* table nesting (ADR 0064 § 2), so
/// `script.spawn = [...]` under `[capabilities]` and a `[capabilities.script]` block with a `spawn`
/// key are the same input and both land in these sub-structs. Nothing has to choose between them.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Capabilities {
    /// `script.spawn` (ADR 0006 § 5).
    pub script: Option<CapScript>,
    /// `fs.read` and `fs.write`.
    pub fs: Option<CapFs>,
    /// `net.connect` (ADR 0058's outbound policy is what it is checked against).
    pub net: Option<CapNet>,
    /// `process.exec`.
    pub process: Option<CapProcess>,
    /// `debug.trace` and `debug.profile` (ADR 0018).
    pub debug: Option<CapDebug>,
    /// `db.connect` and `db.open` (ADR 0067 § 3).
    pub db: Option<CapDb>,
}

/// The `script.*` grants.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapScript {
    /// The roots a `spawn script` target may live under. Being able to *read* a file is not
    /// permission to run it, so this does not follow from `fs.read` (ADR 0006 § 5).
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
}

/// The `process.*` grants.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapProcess {
    /// Whether a subprocess may be started, written as `true` or as a list of programs.
    pub exec: Option<Setting>,
}

/// The `debug.*` grants — ADR 0018's probe sinks.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapDebug {
    /// Where a trace may be written.
    pub trace: Option<Setting>,
    /// Where a profile may be written.
    pub profile: Option<Setting>,
}

/// The `db.*` grants — ADR 0067 § 3.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CapDb {
    /// Which `[db.<name>]` blocks a program may open by name. An endpoint named here is
    /// operator-written and so is pre-approved against ADR 0058's denied ranges.
    pub connect: Option<Setting>,
    /// Which hosts a program-supplied `Db\Settings` may reach; these stay subject to that policy.
    pub open: Option<Setting>,
}

/// One `[[extension]]` entry — ADR 0003 § 3.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Extension {
    /// The `.nvsx` to load.
    pub path: Option<String>,
    /// The pin re-verified on every `nvs ctl reload`; a mismatch refuses the whole swap.
    pub sha256: Option<String>,
}

/// `[debug]` — ADR 0018 § 4, where `nvs.toml` states the default and the ceiling in one value.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Debug {
    /// `[]` is off; any subset of `coverage`, `branch`, `trace`, `profile`. `RuntimeTighten`, so a
    /// request may narrow this and can never turn a bit on.
    pub mode: Option<Vec<String>>,
}

/// `[log]` — the escalation ladder's two configured rungs (ADR 0020 §§ 3, 5) and the record's own
/// two keys (ADR 0092 § 2).
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
    /// Which of ADR 0092 § 3's three renderings the sink emits.
    pub format: Option<String>,
    /// The minimum level written; its per-mode default is ADR 0091 § 3's.
    pub level: Option<String>,
}

/// The `[http.*]` blocks: one refusal policy (ADR 0020 § 7) and ADR 0074's four defaults blocks.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Http {
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

/// `[http.errors]` — ADR 0020 § 7, the block that decides what a request which never got a frame
/// shows.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HttpErrors {
    /// `generic` or `full`; `Runtime`-class, defaulting by run mode (ADR 0091 § 3).
    pub detail: Option<String>,
}

/// `[http.headers]` — ADR 0074 § 2's shipped defaults, applied with no configuration present.
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

/// `[http.cors]` — ADR 0074 § 3, closed until origins are named.
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

/// `[http.cookies]` — ADR 0074 § 4, the defaults every `Core\Response::addCookie` inherits.
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

/// `[http.client]` — ADR 0074 § 5, where nothing has a spelling for an unbounded outbound wait.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HttpClient {
    /// The connect wait.
    pub connect_timeout: Option<String>,
    /// The total, covering every attempt and every redirect hop.
    pub deadline: Option<String>,
    /// Redirects are off by default (ADR 0058 § 4).
    pub max_redirects: Option<u32>,
}

/// One `[db.<name>]` block — ADR 0067 § 2, where the name and not the settings is the key.
///
/// **Recorded gap: ADR 0067 states `Db\Settings` as a language type and never writes the config
/// block out**, so this roster is every field that ADR names in prose (§ 2's "SQLite takes a `path`
/// and has no `host`, `port`, `user` or `password`", § 4's `Settings.database` and `.user`,
/// § 3a's `password_file`) plus the `driver` a discriminated union needs to be discriminated on.
/// A field the ADR turns out to have meant and this list omits is a boot refusal naming the line,
/// which is loud and one edit to fix; the fix is to add the field here *and* the example to 0067.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Database {
    /// Which of ADR 0067's drivers this block selects.
    pub driver: Option<String>,
    /// SQLite's file, resolved against the directory of the file it is written in (ADR 0103 § 5).
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
    /// (ADR 0103 § 7). Exactly one of this and `password` may be set, and it stays set after the
    /// value is read so § 9's dump can name where the secret came from. [`mod@crate::secret`] is
    /// every rule about it; this is the only field on this struct another module writes to.
    pub password_file: Option<String>,
    /// The database name.
    pub database: Option<String>,
}

/// `[deferred]` — ADR 0072 § 7's two bounds on after-response work.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Deferred {
    /// `System` — request trees per core kept alive for deferred work. Past the cap
    /// `afterResponse` throws rather than queueing.
    pub max_concurrent: Option<u64>,
    /// `Runtime` — the default a call inherits when it names none.
    pub deadline: Option<String>,
}

/// One `[[schedule]]` entry — ADR 0073 § 1, every key `System`.
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

/// `[metrics]` — ADR 0076 § 6, `System`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Metrics {
    /// `false`, `prometheus` or `otlp`.
    pub exporter: Option<Setting>,
    /// The Prometheus scrape endpoint.
    pub listen: Option<String>,
    /// The OTLP collector URL.
    pub endpoint: Option<String>,
    /// Per core (§ 7).
    pub max_series: Option<u64>,
}

/// `[trace]` — ADR 0076 § 6, `System`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Trace {
    /// `false` or `otlp`.
    pub exporter: Option<Setting>,
    /// The OTLP collector URL.
    pub endpoint: Option<String>,
    /// Head-based, 0.0–1.0. An inbound sampled trace is always continued regardless.
    pub sample: Option<f64>,
    /// Whether `traceparent` is sent on outbound `Core\Http\Client` calls.
    pub propagate: Option<bool>,
}

/// `[server]` — ADR 0097 § 5, `Boot` as a whole block: a change here needs a restart.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Server {
    /// Every mount path must resolve inside this.
    pub root: Option<String>,
    /// One flat array: `host:port`, or an absolute path meaning a Unix socket. Unix sockets are
    /// Unix-only; Windows listens on TCP loopback.
    pub listen: Option<Vec<String>>,
    /// Unix-socket entries only.
    pub socket_mode: Option<String>,
    /// `entry` or `path`; the development default is `path` (ADR 0091 § 3a).
    pub dispatch: Option<String>,
    /// Static file serving; the development default is on (ADR 0091 § 3a). Spelled `static` in the
    /// file, which is a Rust keyword.
    #[serde(rename = "static")]
    pub serve_static: Option<bool>,
    /// Fail-closed (§ 6): with nothing here, no forwarding header is believed.
    pub trusted_proxies: Option<Vec<String>>,
    /// Off when empty.
    pub health_path: Option<String>,
    /// The in-flight ceiling.
    pub max_in_flight: Option<u64>,
    /// The header read wait — one of four waits, all finite with nothing configured and all *idle*
    /// rather than total.
    pub header_timeout: Option<String>,
    /// The body idle wait.
    pub body_idle_timeout: Option<String>,
    /// The write idle wait.
    pub write_idle_timeout: Option<String>,
    /// The keep-alive idle wait.
    pub keepalive_timeout: Option<String>,
    /// `[[server.mount]]` — one rule per mount (§ 4).
    pub mount: Vec<Mount>,
}

/// One `[[server.mount]]` entry — ADR 0097 § 4.
///
/// There is no `mode` key: ADR 0104 § 4 moved a mount's mode onto the `[[app]]` block, so an
/// application's mode is one answer wherever the entry file is reached from.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Mount {
    /// A glob under `[server] root`; `*` captures one path segment.
    pub scan: Option<String>,
    /// The path prefix matched, with `{1}` referring to a captured segment.
    pub prefix: Option<String>,
    /// The host matched, on the same capture rule. A mount matches on `prefix`, on `host`, or both.
    pub host: Option<String>,
    /// One literal file, overriding a `scan` at this mount's key.
    pub entry: Option<String>,
    /// Optional — what `Core\Router::urlAbsolute` prepends for requests arriving here (§ 3).
    pub origin: Option<String>,
}

/// `[cache]` — ADR 0042's artifact cache.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Cache {
    /// `System` **and** `Boot` — one of the four directives ADR 0078 § 2 names as needing a restart,
    /// because moving it re-creates the runtime's mapping of every cached unit.
    pub dir: Option<String>,
}

/// `[control]` — ADR 0078 § 3's one local socket.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Control {
    /// A path, `\\.\pipe\nvs-control` on Windows, or `false` to disable. There is no TCP listener,
    /// no token and no auth middleware: the socket's owner and mode are the authentication.
    pub socket: Option<Setting>,
}

/// `[opcache]` — revalidation (ADR 0017 § 2) and the file cache (ADR 0042 § 9), every key `System`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Opcache {
    /// When a source file is re-`stat`ed; `never` in production.
    pub validate: Option<Setting>,
    /// The rate cap bounding that `stat` overhead.
    pub revalidate_freq: Option<Setting>,
    /// Whether the on-disk artifact cache is used at all.
    pub file_cache: Option<bool>,
    /// Where it lives; root-owned, and defaulting to a fixed system location.
    pub file_cache_dir: Option<String>,
    /// Its ceiling in bytes.
    pub file_cache_max_size: Option<Setting>,
    /// The GC probability, mirroring PHP's session-GC pair with the divisor below.
    pub file_cache_gc_probability: Option<u32>,
    /// The GC divisor.
    pub file_cache_gc_divisor: Option<u32>,
}
