//! The built-in defaults: what every `nvs.toml` key with a default is when no file writes it, in one
//! table.
//!
//! **This table is the one home of every default value.** Each number is a `const` here, the
//! [`DEFAULTS`] row for its key names that `const`, and every reader that needs the number in its own
//! type reads the `const`. [`Request::get`](crate::Request::get) and
//! [`Request::all`](crate::Request::all) render the same row, so the value a program reads is the
//! value the code enforces. The shipped `default.toml` shows each one beside its key, and
//! `tests/defaults.rs` holds every `# default` line of it to the row's rendering, so the file and the
//! code cannot drift apart.
//!
//! **Why a table of its own, and not a field on [`Directive`](crate::Directive).** A directive row
//! states one rule and covers every key beneath it: one `limits` row answers for every limit. A
//! default belongs to one leaf key. A `default` field there would split the registry into a row per
//! key, which is the shape its module doc refuses. So a default is keyed on the leaf, here, and the
//! class of the same key is still [`lookup`](crate::directive::lookup)'s.
//!
//! **A default that is not one fixed value is computed when it is read** ([`Value`]): `cpu_time`
//! from whether the run is server work, the mode table's rows from the mode in force
//! ([`crate::mode::DERIVED`], which stays their one home), `[server] dispatch`, `static` and
//! `[opcache] settle` from the mode the run started in, `[mode] ceiling` from that mode itself,
//! `[server] workers` from this machine, an unwritten `[limits.hard]` key from the `[limits]` value
//! it bounds (`memory`'s only for server work), and a data-folder path from the data folder. A key whose unwritten state is "no
//! value" — `wall_time`, `memory_high_water`, every `# example` key — has no row.
//!
//! **A row has a [`Scope`].** A key in a named block (`[db.<name>]`) has its default only where that
//! block is written, so a host with no `[db.main]` reads no `db.main.*` key. A key `get` cannot
//! address — one inside an `[[entry]]`, one the boot refuses where the shipped file shows it, or
//! `[log] target`, whose unwritten state is a split between two channels — has a row for its
//! readers and for the drift test, and `get` never renders it.
//!
//! Cost: one `&'static` slice and no allocation until a value is rendered. A value fixed by the
//! binary is rendered once per process, and [`in_force`] copies it. `get` scans it only when
//! neither the request, a secret nor the file has the key, and `all` renders every row once. Neither
//! runs on the request path unless a program calls `Core\Config`.

use std::path::PathBuf;
use std::time::Duration;

use crate::data::Folder;
use crate::mode;
use crate::snapshot::value_at;

/// One minute.
const MINUTE: u64 = 60;
/// One hour.
const HOUR: u64 = 60 * MINUTE;
/// One day.
const DAY: u64 = 24 * HOUR;
/// One mebibyte.
const MIB: u64 = 1 << 20;

/// `[mode] default`: the mode a host that wrote nothing starts in.
pub const MODE_DEFAULT: &str = mode::PRODUCTION;

/// `[limits] memory`, for every kind of run
/// (`rule:config/no-configuration-file-is-a-complete-configuration`). **What it spends:** up to
/// 128 MiB per request in flight, at most. Admission counts what each request holds, and admits
/// one more only while there is room for it to grow to this cap.
pub const LIMITS_MEMORY: u64 = 128 * MIB;
/// `[limits] cpu_time` for server work. A CLI program and a test run take no CPU ceiling.
pub const LIMITS_CPU_TIME: Duration = Duration::from_secs(30);
/// `[limits] max_regex_steps`: the backtracking tier's step budget per match.
pub const LIMITS_MAX_REGEX_STEPS: usize = 1_000_000;
/// `[limits] fatal_reserve_memory`: the tier-1 handler's reserved slice of the memory ceiling.
pub const LIMITS_FATAL_RESERVE_MEMORY: usize = 1 << 20;
/// `[limits] fatal_reserve_time`: the tier-1 handler's reserved slice of the CPU ceiling.
pub const LIMITS_FATAL_RESERVE_TIME: Duration = Duration::from_millis(50);
/// `[limits] max_script_depth`: how deep script calls may nest.
pub const LIMITS_MAX_SCRIPT_DEPTH: u32 = 64;
/// `[limits] max_decompressed`: the most bytes one decompression may produce.
pub const LIMITS_MAX_DECOMPRESSED: u64 = 64 * MIB;
/// `[limits] max_decompression_ratio`: the most output per byte of input.
pub const LIMITS_MAX_DECOMPRESSION_RATIO: u64 = 1000;
/// `[limits] disconnect_grace`: how long a request may outlive a client that went away.
pub const LIMITS_DISCONNECT_GRACE: Duration = Duration::from_secs(30);
/// `[limits] request_body`: the bytes of a request body parsed into memory
/// (`rule:http-server/request-body-and-upload-total-are-two-caps`). **What it spends:** up to
/// 8 MiB per request in flight that reads its body whole.
pub const LIMITS_REQUEST_BODY: u64 = 8 * MIB;
/// `[limits] upload_total`: the total bytes of a streamed request body. Nothing is resident for
/// it: a streamed body holds one chunk at a time, whatever this is.
pub const LIMITS_UPLOAD_TOTAL: u64 = 256 * MIB;
/// `[limits.hard] request_body`: the most a request may raise its own `request_body` to.
pub const LIMITS_HARD_REQUEST_BODY: u64 = 64 * MIB;
/// `[limits.hard] upload_total`: the most a request may raise its own `upload_total` to.
pub const LIMITS_HARD_UPLOAD_TOTAL: u64 = 2048 * MIB;
/// `[limits] max_multipart_parts`: how many parts a multipart body may hold, field parts included
/// (`rule:errors/multipart-part-count`). **What it spends:** nothing resident. It bounds the
/// bookkeeping one walk of a body does.
pub const LIMITS_MAX_MULTIPART_PARTS: usize = 1000;
/// `[limits.hard] max_multipart_parts`: the most a request may raise its own part count to.
pub const LIMITS_HARD_MAX_MULTIPART_PARTS: usize = 10_000;

/// `[log] handler_reserve_memory`: the tier-3 handler's memory reserve.
pub const LOG_HANDLER_RESERVE_MEMORY: usize = 16 << 20;
/// `[log] handler_reserve_time`: the tier-3 handler's CPU reserve.
pub const LOG_HANDLER_RESERVE_TIME: Duration = Duration::from_secs(5);
/// `[log] target` as the shipped file shows it: the floor's destination where a service manager
/// did not start the process. `get` never renders it ([`Scope::Unaddressed`]).
pub const LOG_TARGET: &str = "stderr";
/// `[log] max_size`: the size at which a `file:` target is renamed.
pub const LOG_MAX_SIZE: u64 = 10 * MIB;
/// `[log] keep`: how many renamed files a `file:` target keeps.
pub const LOG_KEEP: usize = 5;
/// `[log] client_ip`: whether a record the server writes names the client address.
pub const LOG_CLIENT_IP: bool = true;

/// `[errors] deprecated`.
pub const ERRORS_DEPRECATED: &str = "ignore";
/// `[image] max_pixels`, read as every other `M` in the file is read.
pub const IMAGE_MAX_PIXELS: u64 = 24 * MIB;
/// `[debug] keep_temporary`.
pub const DEBUG_KEEP_TEMPORARY: bool = false;
/// `[debug] store`.
pub const DEBUG_STORE: bool = false;
/// `[debug] spool_max`.
pub const DEBUG_SPOOL_MAX: u64 = 256 * MIB;
/// `[debug] index_age`.
pub const DEBUG_INDEX_AGE: Duration = Duration::from_secs(7 * DAY);
/// `[debug] index_max`.
pub const DEBUG_INDEX_MAX: u64 = 1024 * MIB;
/// `[debug] listen`: fixed, so a link to the viewer still works after a restart.
pub const DEBUG_LISTEN: &str = "127.0.0.1:8137";

/// `[http.headers] content_type_options`.
pub const HTTP_HEADERS_CONTENT_TYPE_OPTIONS: bool = true;
/// `[http.headers] frame_ancestors`.
pub const HTTP_HEADERS_FRAME_ANCESTORS: &str = "none";
/// `[http.headers] referrer_policy`.
pub const HTTP_HEADERS_REFERRER_POLICY: &str = "strict-origin-when-cross-origin";
/// `[http.headers] hsts`.
pub const HTTP_HEADERS_HSTS: Duration = Duration::from_secs(365 * DAY);
/// `[http.headers] hsts_subdomains`.
pub const HTTP_HEADERS_HSTS_SUBDOMAINS: bool = false;
/// `[http.cors] methods`.
pub const HTTP_CORS_METHODS: &[&str] = &["GET", "HEAD", "POST"];
/// `[http.cors] credentials`.
pub const HTTP_CORS_CREDENTIALS: bool = false;
/// `[http.cors] max_age`.
pub const HTTP_CORS_MAX_AGE: Duration = Duration::from_secs(10 * MINUTE);
/// `[http.cookies] secure`.
pub const HTTP_COOKIES_SECURE: bool = true;
/// `[http.cookies] http_only`.
pub const HTTP_COOKIES_HTTP_ONLY: bool = true;
/// `[http.cookies] same_site`.
pub const HTTP_COOKIES_SAME_SITE: &str = "Lax";
/// `[http.cookies] path`.
pub const HTTP_COOKIES_PATH: &str = "/";
/// `[http.client] connect_timeout`.
pub const HTTP_CLIENT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// `[http.client] deadline`.
pub const HTTP_CLIENT_DEADLINE: Duration = Duration::from_secs(30);
/// `[http.client] max_redirects`.
pub const HTTP_CLIENT_MAX_REDIRECTS: u32 = 0;
/// `[http.client] idle`.
pub const HTTP_CLIENT_IDLE: Duration = Duration::from_secs(30);
/// `[http.client] max_duration`.
pub const HTTP_CLIENT_MAX_DURATION: Duration = Duration::from_secs(5 * MINUTE);
/// `[http.client] pool_idle`. **What it spends:** up to this many idle connections per core.
pub const HTTP_CLIENT_POOL_IDLE: usize = 16;
/// `[http.client] pool_idle_timeout`.
pub const HTTP_CLIENT_POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
/// `[http.client.socket] max_message`.
pub const HTTP_CLIENT_SOCKET_MAX_MESSAGE: u64 = 4 * MIB;
/// `[http.client.socket] send_timeout`.
pub const HTTP_CLIENT_SOCKET_SEND_TIMEOUT: Duration = Duration::from_secs(30);
/// `[http.client.tls] roots`.
pub const HTTP_CLIENT_TLS_ROOTS: &[&str] = &["bundled"];
/// `[http.client.tls] min_version`.
pub const HTTP_CLIENT_TLS_MIN_VERSION: &str = "1.2";

/// `[server] listen`.
pub const SERVER_LISTEN: &[&str] = &["127.0.0.1:8000"];
/// `[server] socket_mode`: owner and group may connect.
pub const SERVER_SOCKET_MODE: u32 = 0o660;
/// `[server] dispatch` in production and in development.
pub const SERVER_DISPATCH: [&str; 2] = ["entry", "path"];
/// `[server] static` in production and in development.
pub const SERVER_STATIC: [bool; 2] = [false, true];
/// `[server] max_in_flight`.
pub const SERVER_MAX_IN_FLIGHT: u64 = 10_000;
/// `[server] header_timeout`.
pub const SERVER_HEADER_TIMEOUT: Duration = Duration::from_secs(10);
/// `[server] body_idle_timeout`.
pub const SERVER_BODY_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
/// `[server] write_idle_timeout`.
pub const SERVER_WRITE_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
/// `[server] keepalive_timeout`.
pub const SERVER_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(75);
/// `[server] drain_timeout`.
pub const SERVER_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);
/// `[server] watchdog_margin`.
pub const SERVER_WATCHDOG_MARGIN: Duration = Duration::from_secs(5);
/// `[server.connection] max_open`.
pub const SERVER_CONNECTION_MAX_OPEN: u64 = 10_000;
/// `[server.connection] max_frame`.
pub const SERVER_CONNECTION_MAX_FRAME: usize = 1 << 20;
/// `[server.connection] max_message`.
pub const SERVER_CONNECTION_MAX_MESSAGE: usize = 4 << 20;
/// `[server.connection] idle_timeout`.
pub const SERVER_CONNECTION_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * MINUTE);
/// `[server.connection] max_lifetime`.
pub const SERVER_CONNECTION_MAX_LIFETIME: Duration = Duration::from_secs(DAY);
/// `[server.connection] send_timeout`.
pub const SERVER_CONNECTION_SEND_TIMEOUT: Duration = Duration::from_secs(30);

/// `[session] ttl`.
pub const SESSION_TTL: Duration = Duration::from_secs(2 * HOUR);
/// `[session] cookie`.
pub const SESSION_COOKIE: &str = "nvsid";

/// `[db.<name>.pool] max`. **What it spends:** up to this many connections per named block.
pub const DB_POOL_MAX: u32 = 16;
/// `[db.<name>.pool] idle`.
pub const DB_POOL_IDLE: u32 = 2;
/// `[db.<name>.pool] lifetime`.
pub const DB_POOL_LIFETIME: Duration = Duration::from_secs(30 * MINUTE);
/// `[db.<name>.pool] acquire`.
pub const DB_POOL_ACQUIRE: Duration = Duration::from_secs(5);
/// `[db.<name>] statement_cache`.
pub const DB_STATEMENT_CACHE: usize = 16;
/// `[db.<name>] time_zone`.
pub const DB_TIME_ZONE: &str = "+00:00";
/// `[db.<name>] journal_mode`. **What it spends:** a `-wal` and a `-shm` file beside each SQLite
/// database a program writes, and the file's journal mode for good.
pub const DB_JOURNAL_MODE: &str = "wal";
/// `[ldap.<name>] tls`.
pub const LDAP_TLS: &str = "required";
/// `[ldap.<name>] timeout`.
pub const LDAP_TIMEOUT: Duration = Duration::from_secs(30);
/// `[mail.<name>] port`.
pub const MAIL_PORT: u16 = 25;
/// `[mail.<name>] timeout`.
pub const MAIL_TIMEOUT: Duration = Duration::from_secs(30);

/// `[cache.memory] max_size`. **What it spends:** up to this much once per serving process.
pub const CACHE_MAX_SIZE: usize = 32 << 20;
/// `[cache.memory] fill_wait`.
pub const CACHE_MEMORY_FILL_WAIT: Duration = Duration::from_secs(5);
/// `[cache.shared] timeout`.
pub const CACHE_SHARED_TIMEOUT: Duration = Duration::from_secs(5);

/// `[queue] workers`.
pub const QUEUE_WORKERS: u32 = 4;
/// `[queue] max_attempts`.
pub const QUEUE_MAX_ATTEMPTS: u32 = 5;
/// `[queue] visibility`.
pub const QUEUE_VISIBILITY: Duration = Duration::from_secs(5 * MINUTE);
/// `[deferred] max_concurrent`.
pub const DEFERRED_MAX_CONCURRENT: u64 = 256;
/// `[[schedule]] timezone`.
pub const SCHEDULE_TIMEZONE: &str = "UTC";
/// `[[schedule]] overlap`.
pub const SCHEDULE_OVERLAP: &str = "skip";
/// `[[include]] optional`.
pub const INCLUDE_OPTIONAL: bool = false;

/// `[metrics] max_series`.
pub const METRICS_MAX_SERIES: u64 = 10_000;
/// `[trace] sample`.
pub const TRACE_SAMPLE: f64 = 0.0;
/// `[trace] propagate`.
pub const TRACE_PROPAGATE: bool = true;
/// `[trace] response_header`.
pub const TRACE_RESPONSE_HEADER: bool = true;

/// `[opcache] validate`.
pub const OPCACHE_VALIDATE: &str = "mtime";
/// `[opcache] revalidate_freq`.
pub const OPCACHE_REVALIDATE_FREQ: Duration = Duration::from_secs(2);
/// `[opcache] settle` in production and in development.
pub const OPCACHE_SETTLE: [Duration; 2] = [Duration::from_secs(1), Duration::from_millis(100)];
/// `[opcache] file_cache`.
pub const OPCACHE_FILE_CACHE: bool = true;
/// `[opcache] file_cache_max_size`.
pub const OPCACHE_FILE_CACHE_MAX_SIZE: u64 = 256 * MIB;
/// `[opcache] file_cache_gc_probability`.
pub const OPCACHE_FILE_CACHE_GC_PROBABILITY: u32 = 1;
/// `[opcache] file_cache_gc_divisor`.
pub const OPCACHE_FILE_CACHE_GC_DIVISOR: u32 = 100;

/// `duration` in nanoseconds, for a reader that holds a duration as a `u64`. Every default here is
/// far below the 584 years a `u64` of nanoseconds runs out at.
#[must_use]
pub const fn nanos(duration: Duration) -> u64 {
    duration.as_secs() * 1_000_000_000 + duration.subsec_nanos() as u64
}

/// What a default is: one fixed value, or the rule that computes it when it is read.
#[derive(Clone, Copy, Debug)]
pub enum Value {
    /// A size in bytes, rendered with the largest binary suffix that divides it: `128M`.
    Size(u64),
    /// A duration, rendered with the largest unit that divides it: `30s`, `50ms`, `365d`.
    Time(Duration),
    /// A whole number, rendered bare: `1000000`.
    Count(u64),
    /// A fraction, rendered as TOML writes a float: `0.0`.
    Ratio(f64),
    /// `true` or `false`.
    Flag(bool),
    /// A string, rendered without quotes.
    Text(&'static str),
    /// A list. `get` renders no list, so this row is for its readers and the drift test.
    List(&'static [&'static str]),
    /// A Unix file mode, rendered as four octal digits: `0660`.
    FileMode(u32),
    /// A duration for server work, and nothing for a CLI program or a test.
    Serving(Duration),
    /// The [`mode::DERIVED`] row of the same key, in the mode in force.
    ByMode,
    /// One of two values, chosen by the mode the run started in. A flip does not move it
    /// (`rule:config/a-startup-default-is-never-flipped`).
    Startup {
        /// The value in production.
        production: &'static Value,
        /// The value in development.
        development: &'static Value,
    },
    /// The mode the run started in.
    StartedMode,
    /// How many cores this machine has, and one where it cannot say.
    Cores,
    /// The value the file writes for another key, or else that key's default.
    Inherit(&'static str),
    /// [`Inherit`](Self::Inherit) for server work, and nothing for a CLI program or a test. It is
    /// `[limits.hard] memory`'s, so a CLI run's own code may raise its memory limit freely until
    /// the operator writes a ceiling (`rule:config/ceilings-are-their-own-directives`).
    InheritServing(&'static str),
    /// A path in the data folder, and nothing where there is no data folder.
    DataFolder(fn(&Folder) -> PathBuf),
}

/// Where a row's default is in force.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// On every host.
    Always,
    /// Only where the file writes the named block, as `[db]` for the bounds of every connection.
    Written(&'static str),
    /// In each named block the file writes. The key spells the name as `<name>`.
    Named,
    /// Never through `get`: a key in an `[[entry]]`, one the boot refuses where the shipped file
    /// shows it, or `[log] target`, whose unwritten state is not one value. An unwritten target
    /// sends a `Core\Log::write` record to the program's output and a floor record to the
    /// diagnostic channel, and a written `stderr` sends both to the process's stderr.
    Unaddressed,
}

/// One built-in default: the key as the shipped file writes it, where it applies, and its value.
#[derive(Clone, Copy, Debug)]
pub struct Builtin {
    /// The dotted key, with `<name>` for the segment an operator names.
    pub key: &'static str,
    /// Where the default is in force.
    pub scope: Scope,
    /// The value, or the rule computing it.
    pub value: Value,
}

/// What a computed default is computed from: the file's own table and the run reading it.
#[derive(Clone, Copy, Debug)]
pub struct At<'a> {
    /// The table the snapshot was built from, after every `[[app]]` block folded into it.
    pub table: &'a toml::Table,
    /// Whether the run is server work.
    pub serving: bool,
    /// The mode in force, after any flip.
    pub mode: &'a str,
    /// The mode the run started in.
    pub started: &'a str,
}

/// What a capability grants where its key is not written: the capability half of this table.
///
/// A capability is not a [`DEFAULTS`] row, because a grant is not a value `get` renders or `set`
/// writes. [`capability`] is its one home instead, and
/// [`Authority`](crate::capability::Authority) is the one reader, so every asker meets the same
/// default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Granted {
    /// Nothing: the capability is denied until a line grants it.
    Nothing,
    /// Every argument, as a written `true` grants.
    Everything,
    /// A path inside the run's app folder with no component below it that starts with `.` and
    /// none named `nvs.toml`, or a path inside a directory `Core\IO::temporaryDir` made for this
    /// run. Asked of the canonical path.
    AppFolder,
    /// A path inside the run's app folder with no component below it that starts with `.` and
    /// none named `nvs.toml`, and inside no directory `Core\IO::temporaryDir` made for this run.
    /// Asked of the canonical path.
    AppCode,
    /// A path inside a directory `Core\IO::temporaryDir` made for this run, and nothing else.
    Temporary,
}

/// The default of `cap` on `run`, read only where `cap`'s key is not written. A written key
/// replaces it whole, `[]` included.
///
/// - **Files, on a CLI run:** `fs.read` and `fs.write` are [`Granted::Everything`], as a command-line
///   program's own files are in any other language.
/// - **Files, in server work:** `fs.read` is [`Granted::AppFolder`] and `fs.write` is
///   [`Granted::Temporary`]. A server reads its own code and assets, never a dot-file such as
///   `.env` or `.git/config` and never `nvs.toml`, and writes only what it made for this run, so
///   nothing it writes lands beside code it would run, and one request cannot reach another's
///   temporary directory.
/// - **Outbound hosts:** `net.connect` is [`Granted::Everything`] on every kind of run. The address
///   table still refuses every address that is not public, with [`internal`] as its exceptions.
/// - **Programs:** `process.exec` is [`Granted::Everything`] on a CLI run, and a bare name such as
///   `git` is looked up on `PATH`. Server work runs no program without a written line.
/// - **Scripts:** `script.spawn` is [`Granted::AppCode`] on every kind of run. `require` already
///   runs that code, so spawning it gives nothing new. A script the run wrote into its own
///   temporary directory is not covered.
/// - **Every other capability** is [`Granted::Nothing`] on every kind of run.
///
/// **What it spends:** nothing held. The app folder is one path per context, and the temporary
/// directories are the list the context keeps for the sweep anyway.
#[must_use]
pub fn capability(cap: crate::Cap, run: crate::capability::Run<'_>) -> Granted {
    match (cap, run.serving) {
        (crate::Cap::FsRead | crate::Cap::FsWrite | crate::Cap::ProcessExec, false)
        | (crate::Cap::NetConnect, _) => Granted::Everything,
        (crate::Cap::ScriptSpawn, _) => Granted::AppCode,
        (crate::Cap::FsRead, true) => Granted::AppFolder,
        (crate::Cap::FsWrite, true) => Granted::Temporary,
        _ => Granted::Nothing,
    }
}

/// The loopback ranges, as `[capabilities.net] internal` entries.
pub const NET_INTERNAL_LOOPBACK: &[&str] = &["127.0.0.0/8", "::1"];

/// The `[capabilities.net] internal` entries of `run` where that key is not written: a startup row
/// (`rule:config/a-startup-default-is-never-flipped`).
///
/// Loopback ([`NET_INTERNAL_LOOPBACK`]) on a CLI run, and in server work whose configuration names
/// `development`. Nothing in production server work, where every private and loopback address needs a
/// written line. The mode is the one the run started in, so a program that flips itself to
/// `development` gains no loopback.
#[must_use]
pub fn internal(run: crate::capability::Run<'_>) -> &'static [&'static str] {
    if run.serving && !run.development {
        &[]
    } else {
        NET_INTERNAL_LOOPBACK
    }
}

/// One [`Builtin`] row: the key and the value, with [`Scope::Always`] unless a scope is given.
macro_rules! row {
    ($key:literal, $value:expr) => {
        Builtin {
            key: $key,
            scope: Scope::Always,
            value: $value,
        }
    };
    ($key:literal, $scope:expr, $value:expr) => {
        Builtin {
            key: $key,
            scope: $scope,
            value: $value,
        }
    };
}

/// Every built-in default, by the key the shipped file writes. Order follows that file.
#[rustfmt::skip] // one row per line: the table reads as one.
pub const DEFAULTS: &[Builtin] = &[
    row!("mode.default", Value::Text(MODE_DEFAULT)),
    row!("mode.ceiling", Value::StartedMode),
    row!("limits.memory", Value::Size(LIMITS_MEMORY)),
    row!("limits.cpu_time", Value::Serving(LIMITS_CPU_TIME)),
    row!("limits.max_regex_steps", Value::Count(LIMITS_MAX_REGEX_STEPS as u64)),
    row!("limits.fatal_reserve_memory", Value::Size(LIMITS_FATAL_RESERVE_MEMORY as u64)),
    row!("limits.fatal_reserve_time", Value::Time(LIMITS_FATAL_RESERVE_TIME)),
    row!("limits.max_script_depth", Value::Count(LIMITS_MAX_SCRIPT_DEPTH as u64)),
    row!("limits.max_decompressed", Value::Size(LIMITS_MAX_DECOMPRESSED)),
    row!("limits.max_decompression_ratio", Value::Count(LIMITS_MAX_DECOMPRESSION_RATIO)),
    row!("limits.disconnect_grace", Value::Time(LIMITS_DISCONNECT_GRACE)),
    row!("limits.request_body", Value::Size(LIMITS_REQUEST_BODY)),
    row!("limits.upload_total", Value::Size(LIMITS_UPLOAD_TOTAL)),
    row!("limits.max_multipart_parts", Value::Count(LIMITS_MAX_MULTIPART_PARTS as u64)),
    row!("limits.hard.memory", Value::InheritServing("limits.memory")),
    row!("limits.hard.cpu_time", Value::Inherit("limits.cpu_time")),
    row!("limits.hard.wall_time", Value::Inherit("limits.wall_time")),
    row!("limits.hard.max_tasks", Value::Inherit("limits.max_tasks")),
    row!("limits.hard.max_output", Value::Inherit("limits.max_output")),
    row!("limits.hard.request_body", Value::Size(LIMITS_HARD_REQUEST_BODY)),
    row!("limits.hard.upload_total", Value::Size(LIMITS_HARD_UPLOAD_TOTAL)),
    row!("limits.hard.max_multipart_parts", Value::Count(LIMITS_HARD_MAX_MULTIPART_PARTS as u64)),
    row!("limits.hard.max_regex_steps", Value::Inherit("limits.max_regex_steps")),
    row!("log.handler_reserve_memory", Value::Size(LOG_HANDLER_RESERVE_MEMORY as u64)),
    row!("log.handler_reserve_time", Value::Time(LOG_HANDLER_RESERVE_TIME)),
    row!("log.target", Scope::Unaddressed, Value::Text(LOG_TARGET)),
    row!("log.max_size", Value::Size(LOG_MAX_SIZE)),
    row!("log.keep", Value::Count(LOG_KEEP as u64)),
    row!("log.format", Value::ByMode),
    row!("log.level", Value::ByMode),
    row!("log.access", Value::ByMode),
    row!("log.client_ip", Value::Flag(LOG_CLIENT_IP)),
    row!("errors.deprecated", Value::Text(ERRORS_DEPRECATED)),
    row!("io.temp_root", Value::DataFolder(Folder::tmp)),
    row!("image.max_pixels", Value::Size(IMAGE_MAX_PIXELS)),
    row!("debug.inline", Value::ByMode),
    row!("debug.keep_temporary", Value::Flag(DEBUG_KEEP_TEMPORARY)),
    row!("debug.store", Value::Flag(DEBUG_STORE)),
    row!("debug.spool_max", Value::Size(DEBUG_SPOOL_MAX)),
    row!("debug.index_age", Value::Time(DEBUG_INDEX_AGE)),
    row!("debug.index_max", Value::Size(DEBUG_INDEX_MAX)),
    row!("debug.listen", Value::Text(DEBUG_LISTEN)),
    row!("http.errors.detail", Value::ByMode),
    row!("http.headers.content_type_options", Value::Flag(HTTP_HEADERS_CONTENT_TYPE_OPTIONS)),
    row!("http.headers.frame_ancestors", Value::Text(HTTP_HEADERS_FRAME_ANCESTORS)),
    row!("http.headers.referrer_policy", Value::Text(HTTP_HEADERS_REFERRER_POLICY)),
    row!("http.headers.hsts", Value::Time(HTTP_HEADERS_HSTS)),
    row!("http.headers.hsts_subdomains", Value::Flag(HTTP_HEADERS_HSTS_SUBDOMAINS)),
    row!("http.cors.methods", Value::List(HTTP_CORS_METHODS)),
    row!("http.cors.credentials", Value::Flag(HTTP_CORS_CREDENTIALS)),
    row!("http.cors.max_age", Value::Time(HTTP_CORS_MAX_AGE)),
    row!("http.cookies.secure", Value::Flag(HTTP_COOKIES_SECURE)),
    row!("http.cookies.http_only", Value::Flag(HTTP_COOKIES_HTTP_ONLY)),
    row!("http.cookies.same_site", Value::Text(HTTP_COOKIES_SAME_SITE)),
    row!("http.cookies.path", Value::Text(HTTP_COOKIES_PATH)),
    row!("http.client.connect_timeout", Value::Time(HTTP_CLIENT_CONNECT_TIMEOUT)),
    row!("http.client.deadline", Value::Time(HTTP_CLIENT_DEADLINE)),
    row!("http.client.max_redirects", Value::Count(HTTP_CLIENT_MAX_REDIRECTS as u64)),
    row!("http.client.idle", Value::Time(HTTP_CLIENT_IDLE)),
    row!("http.client.max_duration", Value::Time(HTTP_CLIENT_MAX_DURATION)),
    row!("http.client.pool_idle", Value::Count(HTTP_CLIENT_POOL_IDLE as u64)),
    row!("http.client.pool_idle_timeout", Value::Time(HTTP_CLIENT_POOL_IDLE_TIMEOUT)),
    row!("http.client.socket.max_message", Value::Count(HTTP_CLIENT_SOCKET_MAX_MESSAGE)),
    row!("http.client.socket.send_timeout", Value::Time(HTTP_CLIENT_SOCKET_SEND_TIMEOUT)),
    row!("http.client.tls.roots", Value::List(HTTP_CLIENT_TLS_ROOTS)),
    row!("http.client.tls.min_version", Value::Text(HTTP_CLIENT_TLS_MIN_VERSION)),
    row!("server.listen", Value::List(SERVER_LISTEN)),
    row!("server.socket_mode", Value::FileMode(SERVER_SOCKET_MODE)),
    row!("server.dispatch", Value::Startup {
        production: &Value::Text(SERVER_DISPATCH[0]),
        development: &Value::Text(SERVER_DISPATCH[1]),
    }),
    row!("server.static", Value::Startup {
        production: &Value::Flag(SERVER_STATIC[0]),
        development: &Value::Flag(SERVER_STATIC[1]),
    }),
    row!("server.max_in_flight", Value::Count(SERVER_MAX_IN_FLIGHT)),
    row!("server.workers", Value::Cores),
    row!("server.header_timeout", Value::Time(SERVER_HEADER_TIMEOUT)),
    row!("server.body_idle_timeout", Value::Time(SERVER_BODY_IDLE_TIMEOUT)),
    row!("server.write_idle_timeout", Value::Time(SERVER_WRITE_IDLE_TIMEOUT)),
    row!("server.keepalive_timeout", Value::Time(SERVER_KEEPALIVE_TIMEOUT)),
    row!("server.drain_timeout", Value::Time(SERVER_DRAIN_TIMEOUT)),
    row!("server.watchdog_margin", Value::Time(SERVER_WATCHDOG_MARGIN)),
    row!("server.connection.max_open", Value::Count(SERVER_CONNECTION_MAX_OPEN)),
    row!("server.connection.max_frame", Value::Size(SERVER_CONNECTION_MAX_FRAME as u64)),
    row!("server.connection.max_message", Value::Size(SERVER_CONNECTION_MAX_MESSAGE as u64)),
    row!("server.connection.idle_timeout", Value::Time(SERVER_CONNECTION_IDLE_TIMEOUT)),
    row!("server.connection.max_lifetime", Value::Time(SERVER_CONNECTION_MAX_LIFETIME)),
    row!("server.connection.send_timeout", Value::Time(SERVER_CONNECTION_SEND_TIMEOUT)),
    row!("session.ttl", Value::Time(SESSION_TTL)),
    row!("session.cookie", Value::Text(SESSION_COOKIE)),
    row!("db.pool.max", Scope::Unaddressed, Value::Count(DB_POOL_MAX as u64)),
    row!("db.pool.idle", Scope::Unaddressed, Value::Count(DB_POOL_IDLE as u64)),
    row!("db.pool.lifetime", Scope::Unaddressed, Value::Time(DB_POOL_LIFETIME)),
    row!("db.pool.acquire", Scope::Unaddressed, Value::Time(DB_POOL_ACQUIRE)),
    row!("db.<name>.statement_cache", Scope::Named, Value::Count(DB_STATEMENT_CACHE as u64)),
    row!("db.<name>.time_zone", Scope::Named, Value::Text(DB_TIME_ZONE)),
    row!("db.<name>.journal_mode", Scope::Named, Value::Text(DB_JOURNAL_MODE)),
    row!("db.<name>.pool.max", Scope::Named, Value::Count(DB_POOL_MAX as u64)),
    row!("db.<name>.pool.idle", Scope::Named, Value::Count(DB_POOL_IDLE as u64)),
    row!("db.<name>.pool.lifetime", Scope::Named, Value::Time(DB_POOL_LIFETIME)),
    row!("db.<name>.pool.acquire", Scope::Named, Value::Time(DB_POOL_ACQUIRE)),
    row!("ldap.<name>.tls", Scope::Named, Value::Text(LDAP_TLS)),
    row!("ldap.<name>.timeout", Scope::Named, Value::Time(LDAP_TIMEOUT)),
    row!("ldap.<name>.pool.max", Scope::Named, Value::Count(DB_POOL_MAX as u64)),
    row!("ldap.<name>.pool.idle", Scope::Named, Value::Count(DB_POOL_IDLE as u64)),
    row!("ldap.<name>.pool.lifetime", Scope::Named, Value::Time(DB_POOL_LIFETIME)),
    row!("ldap.<name>.pool.acquire", Scope::Named, Value::Time(DB_POOL_ACQUIRE)),
    row!("mail.<name>.port", Scope::Named, Value::Count(MAIL_PORT as u64)),
    row!("mail.<name>.timeout", Scope::Named, Value::Time(MAIL_TIMEOUT)),
    row!("cache.memory.max_size", Value::Size(CACHE_MAX_SIZE as u64)),
    row!("cache.memory.fill_wait", Value::Time(CACHE_MEMORY_FILL_WAIT)),
    row!("cache.shared.timeout", Scope::Written("cache.shared"), Value::Time(CACHE_SHARED_TIMEOUT)),
    row!("queue.workers", Value::Count(QUEUE_WORKERS as u64)),
    row!("queue.max_attempts", Value::Count(QUEUE_MAX_ATTEMPTS as u64)),
    row!("queue.visibility", Value::Time(QUEUE_VISIBILITY)),
    row!("deferred.max_concurrent", Value::Count(DEFERRED_MAX_CONCURRENT)),
    row!("schedule.timezone", Scope::Unaddressed, Value::Text(SCHEDULE_TIMEZONE)),
    row!("schedule.overlap", Scope::Unaddressed, Value::Text(SCHEDULE_OVERLAP)),
    row!("metrics.exporter", Value::Flag(false)),
    row!("metrics.max_series", Value::Count(METRICS_MAX_SERIES)),
    row!("trace.exporter", Value::Flag(false)),
    row!("trace.sample", Value::Ratio(TRACE_SAMPLE)),
    row!("trace.propagate", Value::Flag(TRACE_PROPAGATE)),
    row!("trace.response_header", Value::Flag(TRACE_RESPONSE_HEADER)),
    row!("opcache.validate", Value::Text(OPCACHE_VALIDATE)),
    row!("opcache.revalidate_freq", Value::Time(OPCACHE_REVALIDATE_FREQ)),
    row!("opcache.settle", Value::Startup {
        production: &Value::Time(OPCACHE_SETTLE[0]),
        development: &Value::Time(OPCACHE_SETTLE[1]),
    }),
    row!("opcache.file_cache", Value::Flag(OPCACHE_FILE_CACHE)),
    row!("opcache.file_cache_dir", Value::DataFolder(Folder::cache)),
    row!("opcache.file_cache_max_size", Value::Size(OPCACHE_FILE_CACHE_MAX_SIZE)),
    row!("opcache.file_cache_gc_probability", Value::Count(OPCACHE_FILE_CACHE_GC_PROBABILITY as u64)),
    row!("opcache.file_cache_gc_divisor", Value::Count(OPCACHE_FILE_CACHE_GC_DIVISOR as u64)),
    row!("include.optional", Scope::Unaddressed, Value::Flag(INCLUDE_OPTIONAL)),
];

/// The row whose key spells `key`, `<name>` matching any one segment, whatever its scope.
#[must_use]
pub fn row(key: &str) -> Option<&'static Builtin> {
    DEFAULTS.iter().find(|row| spells(row.key, key).is_some())
}

/// The default in force for `key` at `at`, as `Core\Config::get` renders it, or `None` where the
/// key has no default in force.
#[must_use]
pub fn in_force(key: &str, at: &At<'_>) -> Option<String> {
    let (index, row) = DEFAULTS
        .iter()
        .enumerate()
        .find(|(_, row)| applies(row, key, at.table))?;
    if let Some(text) = fixed(index) {
        return Some(text.to_owned());
    }
    render(&row.value, key, at)
}

/// The rendering of the [`DEFAULTS`] row at `index` where its value is fixed by the binary, and
/// `None` where it is computed when it is read or is a list.
///
/// Every fixed row is rendered once per process, on the first call. **What it spends:** one
/// `String` per fixed row, for the process. The rows are the binary's, so this never grows.
fn fixed(index: usize) -> Option<&'static str> {
    static RENDERED: std::sync::OnceLock<Box<[Option<String>]>> = std::sync::OnceLock::new();
    RENDERED
        .get_or_init(|| {
            DEFAULTS
                .iter()
                .map(|row| render_fixed(&row.value))
                .collect()
        })
        .get(index)?
        .as_deref()
}

/// A value fixed by the binary, rendered as [`render`] renders it, and `None` for any other value.
fn render_fixed(value: &Value) -> Option<String> {
    match *value {
        Value::Size(bytes) => Some(size(bytes)),
        Value::Time(duration) => Some(time(duration)),
        Value::Count(count) => Some(count.to_string()),
        Value::Ratio(fraction) => Some(format!("{fraction:?}")),
        Value::Flag(flag) => Some(flag.to_string()),
        Value::Text(text) => Some(text.to_string()),
        Value::FileMode(bits) => Some(format!("{bits:04o}")),
        _ => None,
    }
}

/// Every default in force at `at`, by dotted key, inserted into `out`.
pub fn fill(at: &At<'_>, out: &mut std::collections::BTreeMap<String, String>) {
    for row in DEFAULTS {
        for key in keys(row, at.table) {
            if let Some(value) = render(&row.value, &key, at) {
                out.insert(key, value);
            }
        }
    }
}

/// `value` rendered the way the shipped file writes it, without quotes, or `None` for a list and
/// for a computed value that is nothing here.
#[must_use]
pub fn render(value: &Value, key: &str, at: &At<'_>) -> Option<String> {
    match *value {
        Value::Size(_)
        | Value::Time(_)
        | Value::Count(_)
        | Value::Ratio(_)
        | Value::Flag(_)
        | Value::Text(_)
        | Value::FileMode(_) => render_fixed(value),
        Value::List(_) => None,
        Value::Serving(duration) => at.serving.then(|| time(duration)),
        Value::ByMode => mode::DERIVED
            .iter()
            .find(|row| row.key == key)
            .and_then(|row| row.value(at.mode))
            .map(str::to_string),
        Value::Startup {
            production,
            development,
        } => match at.started {
            mode::DEVELOPMENT => render(development, key, at),
            _ => render(production, key, at),
        },
        Value::StartedMode => Some(at.started.to_string()),
        Value::Cores => Some(
            std::thread::available_parallelism()
                .map_or(1, std::num::NonZeroUsize::get)
                .to_string(),
        ),
        Value::Inherit(other) => value_at(at.table, other)
            .and_then(crate::request::as_text)
            .or_else(|| in_force(other, at)),
        Value::InheritServing(other) => at
            .serving
            .then(|| render(&Value::Inherit(other), key, at))
            .flatten(),
        Value::DataFolder(path) => {
            crate::data::current().map(|folder| path(folder).display().to_string())
        }
    }
}

/// A list default as the shipped file writes it: `["GET", "HEAD", "POST"]`. Only the drift test
/// reads a list this way.
#[must_use]
pub fn render_list(items: &[&str]) -> String {
    let quoted: Vec<String> = items.iter().map(|item| format!("\"{item}\"")).collect();
    format!("[{}]", quoted.join(", "))
}

/// A size with the largest binary suffix that divides it exactly, and bare where none does.
pub(crate) fn size(bytes: u64) -> String {
    for (suffix, shift) in [("T", 40), ("G", 30), ("M", 20), ("K", 10)] {
        if bytes != 0 && bytes.is_multiple_of(1 << shift) {
            return format!("{}{suffix}", bytes >> shift);
        }
    }
    bytes.to_string()
}

/// A duration with the largest unit that divides it exactly. Days are used only for more than one
/// day, so one day reads `24h`.
fn time(duration: Duration) -> String {
    let nanos = duration.as_nanos();
    let second = 1_000_000_000_u128;
    let units = [
        ("d", u128::from(DAY) * second),
        ("h", u128::from(HOUR) * second),
        ("m", u128::from(MINUTE) * second),
        ("s", second),
        ("ms", 1_000_000),
        ("us", 1_000),
    ];
    for (suffix, unit) in units {
        if nanos != 0 && nanos.is_multiple_of(unit) && (suffix != "d" || nanos / unit > 1) {
            return format!("{}{suffix}", nanos / unit);
        }
    }
    format!("{nanos}ns")
}

/// Whether `row` is the default of `key` on the host whose table is `table`.
fn applies(row: &Builtin, key: &str, table: &toml::Table) -> bool {
    match row.scope {
        Scope::Always => row.key == key,
        Scope::Written(block) => row.key == key && is_table(table, block),
        Scope::Named => spells(row.key, key).is_some_and(|block| is_table(table, &block)),
        Scope::Unaddressed => false,
    }
}

/// Every key `row` is the default of on the host whose table is `table`.
fn keys(row: &Builtin, table: &toml::Table) -> Vec<String> {
    match row.scope {
        Scope::Always => vec![row.key.to_string()],
        Scope::Written(block) if is_table(table, block) => vec![row.key.to_string()],
        Scope::Named => {
            let Some((parent, rest)) = row.key.split_once(".<name>") else {
                return Vec::new();
            };
            let Some(toml::Value::Table(named)) = value_at(table, parent) else {
                return Vec::new();
            };
            named
                .iter()
                .filter(|(_, block)| block.is_table())
                .map(|(name, _)| format!("{parent}.{name}{rest}"))
                .collect()
        }
        Scope::Written(_) | Scope::Unaddressed => Vec::new(),
    }
}

/// Whether `pattern` spells `key`, and if so the dotted block its `<name>` segment names — the
/// whole pattern's prefix up to and including that segment, or `""` for a pattern with none.
fn spells(pattern: &str, key: &str) -> Option<String> {
    let mut block = String::new();
    let mut wanted = pattern.split('.');
    let mut given = key.split('.');
    let mut named = false;
    loop {
        match (wanted.next(), given.next()) {
            (None, None) => return Some(if named { block } else { String::new() }),
            (Some(want), Some(have)) if want == have || want == "<name>" => {
                if !named {
                    if !block.is_empty() {
                        block.push('.');
                    }
                    block.push_str(have);
                }
                named |= want == "<name>";
            }
            _ => return None,
        }
    }
}

/// Whether `table` has a table at the dotted `key`.
fn is_table(table: &toml::Table, key: &str) -> bool {
    value_at(table, key).is_some_and(toml::Value::is_table)
}
