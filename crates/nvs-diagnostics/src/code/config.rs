//! The codes of the configuration and the capabilities, `E06xx`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `E06xx` fails the
//! crate's tests.

codes! {
    range = "E06";

    /// An `nvs.toml` directive that does not exist, or an invalid value.
    pub const E_BAD_DIRECTIVE: Code = Code::new("E0601").card(
        "This setting in the configuration file does not exist, or its value has the wrong form. \
         Check the name of the setting, and write the value in the form the message shows.",
    );
    // `E0602` is retired and is never reused: `Core\Config::set` answers every
    // refusal with `false` and never throws (`nvs_config::request`).
    // `E0603` is retired and is never reused: a request past a limit ends in a
    // `FATAL` that `Core\Fatal::onLimit` sees (`rule:errors/on-limit`), not a diagnostic.
    /// The same directive set twice in one configuration file. `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one` refuses
    /// it so that no assignment in a root-owned file is ever silently shadowed;
    /// across an `[[include]]` the same key is an override instead (`rule:config/later-wins-and-every-override-is-recorded`).
    pub const E_DUPLICATE_DIRECTIVE: Code = Code::new("E0604").card(
        "The same setting is written twice in one configuration file. Remove one of them. A \
         file you load with `[[include]]` may set it again, and the server uses that value.",
    );
    /// A file the configuration tree names cannot be read: a `--config` that does
    /// not exist, a non-`optional` `[[include]]` that does not, or an I/O failure
    /// on one that does. `rule:config/ownership-is-the-trust-boundary` makes every one of these a hard refusal —
    /// `optional` covers absence and nothing else, because otherwise a stray
    /// `chmod` silently drops half a configuration and the server comes up
    /// looking healthy.
    pub const E_UNREADABLE_CONFIG: Code = Code::new("E0605").card(
        "A configuration file cannot be read. It does not exist, or the server has no permission \
         to read it. An `[[include]]` marked `optional` may be missing, but it must be readable \
         when it exists.",
    );
    /// An `[[include]]` reached a file already on the chain that pulled it in, or
    /// nested deeper than `rule:config/include-takes-a-path-or-a-dir`'s cap of eight. The refusal names the whole
    /// chain, because the cycle is a property of the path and not of its last file.
    pub const E_INCLUDE_CYCLE: Code = Code::new("E0606").card(
        "The `[[include]]` files include each other in a loop, or they are nested more than eight \
         levels deep. The message shows the chain of files. Remove the include that goes back to \
         an earlier file.",
    );
    /// A file the configuration tree names fails `rule:config/ownership-is-the-trust-boundary`'s trust boundary:
    /// it is owned by an account that is neither this one nor an administrative
    /// one, or an account outside those can write it — the file itself, or the
    /// directory holding it, or the directory an absent `optional` include would
    /// appear in. Whoever can write one file in the tree can grant themselves
    /// every capability it carries, so this is a refusal to start rather than a
    /// warning, and it is re-run on every reload.
    pub const E_UNTRUSTED_CONFIG: Code = Code::new("E0607").card(
        "A configuration file is not safe to use. Another user owns it, or another user can change \
         the file or its directory. Change the owner or the permissions so that only you or an \
         administrator can write it.",
    );
    /// A `_file` sibling of a secret directive — `[db.<name>] password_file` —
    /// names a file the configuration cannot take a value from: it is
    /// set alongside the directive it stands in for, or its content is empty,
    /// whitespace-only, larger than `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s 64 KiB cap, or not valid
    /// UTF-8. Every one of those is a refusal to start rather than a value
    /// carried forward: an empty credential otherwise fails at the first
    /// request instead of at boot, and the size cap catches a path pointed at
    /// the wrong thing. A secret file that fails the § 6 trust boundary is
    /// `E0607` like any other configuration input, and one that cannot be read
    /// at all is `E0605`.
    pub const E_BAD_SECRET_FILE: Code = Code::new("E0608").card(
        "A `_file` setting such as `password_file` names the file that contains a secret. The file \
         must be UTF-8 text, not empty, and at most 64 KiB. Set either `password` or \
         `password_file`, never both.",
    );
    /// An `[[app]]` block that cannot be keyed on an entry file path: it names
    /// both `root` and `entry`, or neither, or it resolves to a path another
    /// block already claimed. `rule:config/an-application-is-its-entry-file-path` makes an application *be* its entry
    /// file path, so a block with no usable key silently covers nothing and
    /// hands every application it was meant for the global configuration
    /// instead — including one it was written to narrow. Two blocks on one path
    /// are § 2's duplicate rather than a refinement: specificity is what orders
    /// the layering, and equal paths have no order to decide with. A key naming
    /// something that cannot be examined at all is `E0605`, like any other
    /// configuration path.
    pub const E_BAD_APP_BLOCK: Code = Code::new("E0609").card(
        "An `[[app]]` block needs exactly one of `root` or `entry`. Two blocks cannot name the same \
         entry file, so merge them into one block.",
    );
    /// An `[[app]]` block asking for more than the host allows: a value in
    /// `[app.limits]` above the global `[limits.hard]`, or an `[app.limits.hard]`
    /// raising its own ceiling above it. `rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling` lets a block widen
    /// `[app.limits]` and lower its own ceiling, but leaves the bound where
    /// `rule:config/ceilings-are-their-own-directives` puts it — `[limits.hard]` is the host's answer and an application
    /// cannot exceed it. The refusal is at boot and the value is never clamped,
    /// exactly as a `Core\Config::set` above the ceiling is refused rather than
    /// reduced: a clamp would leave a block reading as though it got what it
    /// asked for. A value that is not a quantity at all is `E0601`, from the one
    /// parser both this check and `Core\Config::set` share (`rule:config/ini-set-is-core-config-set`).
    pub const E_APP_ABOVE_CEILING: Code = Code::new("E0610").card(
        "An `[[app]]` block sets a limit above the server's `[limits.hard]` value. Lower the value \
         in the block, or raise `[limits.hard]`.",
    );

    /// A `[[schedule]]` entry the scheduler could not arm: no `name` or a
    /// duplicate one, no `scope`, a `cron` outside `rule:config/cron-is-five-fields-and-nothing-more`'s five-field
    /// dialect, a `script` outside the `script.spawn` roots, or `scope =
    /// "fleet"` with no shared store to hold § 3's lease. Every one of them is
    /// refused at boot rather than at the first fire, because a schedule's
    /// failure mode is silence: an entry that never fires looks exactly like
    /// one whose interval has not come round yet, and the operator finds out
    /// from the work that did not happen. A path outside the roots is this
    /// code and not `E0602` — nothing has been denied at a door yet, the
    /// configuration simply does not say the file may be run.
    pub const E_BAD_SCHEDULE: Code = Code::new("E0611").card(
        "A `[[schedule]]` entry cannot start. Each entry needs its own `name`, a `scope`, a `cron` \
         with five fields, and a `script` the server is allowed to run. The message names the part \
         that is wrong.",
    );

    /// An `[http]` pair with no correct meaning: `rule:http-server/cors-is-closed-until-origins-are-named`'s `origins =
    /// ["*"]` with `credentials = true`, or § 3's `same_site = "None"` with
    /// `secure = false`. Both are refused rather than warned about because
    /// neither is a weak policy — every browser rejects both outright, so the
    /// deployment holds an access-control rule that does not run and nothing
    /// tells it so. This is the boot half; inside a request the same two
    /// combinations make `Core\Config::set` return `false` and leave the value
    /// unchanged (`rule:config/three-changeability-classes`'s existing rule), and the condition behind both
    /// halves is written once in `nvs_config::http`. Each half of a pair is
    /// legitimate alone — a wildcard origin is an ordinary public API — so
    /// there is no refusal here for a single key.
    pub const E_MEANINGLESS_HTTP_PAIR: Code = Code::new("E0612").card(
        "Browsers reject these two settings together, so they have no effect. With `origins = \
         [\"*\"]` in `[http.cors]`, set `credentials = false`. With `same_site = \"None\"` in \
         `[http.cookies]`, set `secure = true`.",
    );

    /// A `[log] target` that is none of `rule:errors/engine-floor`'s destinations —
    /// `stderr`, `file:<path>` or `syslog` — including a `file:` with no path
    /// behind it, and `syslog` on a host with no syslog socket, which is Windows.
    /// Refused at the boot that reads the tree rather than at the
    /// first record written through it, because the one moment the engine
    /// cannot report a configuration mistake is the moment it is already
    /// reporting a failure: tier 4 is the floor, and a diagnostic raised there
    /// would displace the record it exists to write. The grammar behind this
    /// refusal is `nvs_config::log::Target`, which is also what
    /// `nvs_runtime::Ctx::write_log_record` resolves a target through, so a
    /// spelling accepted here is a destination that opens.
    pub const E_UNSPELLED_LOG_TARGET: Code = Code::new("E0613").card(
        "`[log] target` must be `stderr`, `syslog`, or `file:` followed by a path, such as \
         `file:/var/log/app.log`. Windows has no syslog, so `syslog` works only on Linux, macOS \
         and other Unix systems.",
    );

    /// A `[log] level` that is none of `rule:errors/log-level`'s levels — refused at the
    /// same boot and for the same reason as `E0613` beside it, but against the
    /// opposite failure: an unspelled destination would route records nowhere,
    /// while an unspelled level leaves the floor at `Debug` and writes
    /// everything. An operator who wrote `level = "warning"` believes they
    /// raised it and did not, and no record they collect afterwards says so.
    /// The grammar is `nvs_render::Level::of`, which is also what
    /// `nvs_runtime::Ctx::write_log_record` resolves the floor through.
    pub const E_UNSPELLED_LOG_LEVEL: Code = Code::new("E0614").card(
        "`[log] level` must be one of `debug`, `info`, `warn`, `error` or `critical`. You can also \
         write them with a capital letter, such as `Info`.",
    );

    /// A `[log] format` that is neither of `rule:errors/renderings`'s two — the third
    /// refusal of the same block and for the third reason. An unspelled
    /// destination routes records nowhere and an unspelled level widens what is
    /// collected; an unspelled *rendering* leaves them as JSON Lines, so the
    /// deployment collects exactly the right records in the shape it asked not
    /// to have, and the pipeline reading them is the thing that breaks.
    ///
    /// § 3's own paragraph is why the near miss is worth a diagnostic rather
    /// than a fallback: the directive has **two** values and not three, because
    /// HTML is a rendering the response sink selects and never a log target's.
    /// An operator who writes `format = "html"` has asked for something that
    /// exists, somewhere this is not.
    pub const E_UNSPELLED_LOG_FORMAT: Code = Code::new("E0615").card(
        "`[log] format` must be `json` or `text`. There is no `html` format for a log.",
    );

    /// A written `Core\Cap::has("…")` naming something that is not a capability
    /// — `rule:security/capability-roster-is-closed`'s roster is closed, and § 6's whole point is that the
    /// answer decides which branch a package takes. A misspelling folds to
    /// `false` and so reads as *not granted*, which is the same answer the
    /// correct spelling gives on a deployment that granted nothing: the branch
    /// simply never runs, on every machine, and nothing at run time can tell
    /// the two apart. That is `E0798`'s reasoning one class over, and it is why
    /// only a *written* name is refused — a computed one keeps `rule:expressions/intrinsic-constant-arguments`'s
    /// rule that nothing is refused for being dynamic, and answers `false` at
    /// run time. In this band rather than the types one because what it checks
    /// is a capability name, which is `nvs_config::capability::Cap`'s roster and
    /// the same table a `[grants]` line is read against.
    pub const E_NOT_A_CAPABILITY: Code = Code::new("E0616").card(
        "`Core\\Cap::has` needs the name of a capability, and this name is not one. Check the \
         spelling. A wrong name always gives `false`, so the code that depends on it would never \
         run.",
    );

    /// A `[queue]` block the runtime could not arm: no `connection`, one naming
    /// a `[db.<name>]` block the tree does not hold, a `max_attempts` of `0`,
    /// or a `visibility` of `0`. Refused at boot for `E0611`'s reason one
    /// subsystem over — a queue fails silently by construction, since a job
    /// that is never claimed looks exactly like one whose turn has not come,
    /// and the operator learns about it from the work that did not happen. A
    /// `connection` naming nothing is this code and not `E0601`: the value is a
    /// well-formed name, and what is wrong with it is a fact about the rest of
    /// the tree rather than about the value.
    pub const E_BAD_QUEUE: Code = Code::new("E0617").card(
        "The `[queue]` block cannot start. Its `connection` must name a `[db.<name>]` block in the \
         configuration, and `max_attempts` and `visibility` must be greater than `0`.",
    );

    /// A **written** `Core\Db::open` host that the compiling machine's
    /// `db.open` grant does not cover — `rule:core-classes/db-compile-time-query-checking`'s second sentence, and
    /// the only capability question asked before a program runs.
    ///
    /// It refuses nothing `nvs_runtime::capability::require` would have
    /// allowed: the same grant list, walked by the same
    /// `nvs_config::capability::Capabilities::allows_host`, so this is `rule:expressions/preparation-preserves-behaviour`'s earlier answer and never a different one. It is therefore asked
    /// only where both halves are facts at check time — a host given as a string literal, and a
    /// configuration this machine actually read. A computed host, or a check
    /// run with no configuration in front of it, says nothing and leaves the
    /// refusal to the door. In this band rather than the types one for
    /// `E0616`'s reason: what it reads is a grant, not a type.
    pub const E_UNGRANTED_HOST: Code = Code::new("E0618").card(
        "The configuration does not allow a connection to this database host. Add the host to \
         `[capabilities] db.open` in `nvs.toml`, or name a `[db.<name>]` block and open it with \
         `Core\\Db::connect`.",
    );

    /// A `[server]` wait that would never end: `false`, or `0`. `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s
    /// headline is that Novis never waits forever and `rule:http-server/the-server-block-is-boot-class` states its
    /// four inbound waits as finite with nothing configured, so `false` — which
    /// removes a ceiling everywhere else in the tree (`rule:config/three-changeability-classes`) — has no
    /// meaning here and is refused rather than read as a default. `0` is the
    /// same refusal from the other side: a wait that expires as it is armed
    /// closes every connection before it can say anything, which is a server
    /// that accepts and answers nothing.
    ///
    /// A value that is not a duration at all is `E0601` in
    /// `nvs_config::value`'s own words; this code is only for a well-formed
    /// duration whose *magnitude* is the problem.
    ///
    pub const E_UNBOUNDED_WAIT: Code = Code::new("E0619").card(
        "A `[server]` wait cannot be `false` or `0`. Write how long the server waits, such as \
         `10s`, or remove the setting to use the default.",
    );

    /// A `[server] listen` entry the server cannot bind, or a written array
    /// with nothing in it. `rule:http-server/the-server-block-is-boot-class`'s array is one flat list whose entries
    /// are a `host:port` or an absolute path meaning a Unix socket, and the
    /// overload is unambiguous because no address can begin with a separator.
    ///
    /// A host *name* is refused rather than resolved: a name that answers with
    /// two addresses is two sockets rather than one, and resolving it at boot
    /// makes the server's start depend on a nameserver being up. An empty array
    /// is refused for the same reason `E0619` refuses `false` — it is a
    /// deployment that accepts nothing, and leaving the key out is how § 5's
    /// own default is kept.
    pub const E_BAD_LISTEN: Code = Code::new("E0620").card(
        "`[server] listen` needs at least one address, written as an IP address and a port, such as \
         `127.0.0.1:8000` or `[::1]:8000`. A host name such as `localhost` is not allowed. An \
         absolute path is a Unix socket.",
    );

    /// A `[[server.mount]]` block that does not resolve to a mount. `rule:http-server/a-path-is-never-derived-from-a-url`
    /// makes the set of paths the server can execute something enumerated
    /// before it accepts anything, so every way a block can fail to name one is
    /// a boot refusal rather than a mount quietly missing from the table.
    ///
    /// It covers the shape — naming both `scan` and `entry` or neither,
    /// matching on neither `prefix` nor `host`, a `{2}` whose glob has one `*`
    /// — and what only the disk answers: an entry that is not there, one that
    /// resolves outside `[server] root`, a captured segment `rule:errors/path-component-refusals`
    /// refuses, and two mounts answering at one key. The first set is checked
    /// wherever the tree is, so `nvs config check` reports it on a machine that
    /// holds none of the files; the second needs the tree it mounts.
    pub const E_BAD_MOUNT: Code = Code::new("E0621").card(
        "A `[[server.mount]]` block does not describe a valid mount. It needs exactly one of `scan` \
         or `entry`, and a `prefix` or a `host`. The message names the problem, such as an entry \
         file that does not exist.",
    );

    /// A `[server] max_in_flight` written as `0`. `rule:http-server/the-server-block-is-boot-class`'s ceiling is a
    /// safety valve, and that spelling asks for a process that accepts a
    /// connection and then refuses every request on it — the same deployment
    /// `E0620` refuses when `listen` is written empty, reached from the other
    /// end. `false` is not the other half of this refusal because the key is a
    /// plain count rather than a `Setting`: `rule:config/three-changeability-classes`'s ceiling-removing
    /// spelling does not typecheck against it, so removing this valve is not
    /// something the file can say.
    ///
    /// Only the written magnitude is this code's: admission also asks whether
    /// the memory the process holds leaves room for one more request
    /// (`rule:http-server/admission-is-arithmetic-not-a-number`), and that
    /// condition is separate from this count, so no number written here can
    /// disagree with it.
    pub const E_NO_ADMISSION: Code = Code::new("E0622").card(
        "`[server] max_in_flight` is `0`, so the server would answer every request with `503`. \
         Write a number above zero, or leave the key out to use the default.",
    );

    /// A `[server] health_path` that is not an absolute path: `healthz`,
    /// `/healthz?verbose=1`, or `/` on its own. `rule:http-server/the-server-block-is-boot-class`'s probe answers
    /// one exact URL ahead of the mount table, so what is written here is
    /// reserved from every application this server mounts — which is why the
    /// key is off by default, and why a spelling no request could ever carry
    /// is refused rather than reserved and then never reached. A relative path
    /// cannot equal a request target, a query or a fragment is not part of the
    /// path one is matched on, and `/` reserves every mount's own entry.
    ///
    /// Not `E0621`'s refusal reached from another direction: a mount path is a
    /// prefix that has to resolve inside `[server] root` on disk, and this is a
    /// whole request target that reaches no filesystem at all.
    pub const E_BAD_HEALTH_PATH: Code = Code::new("E0623").card(
        "`[server] health_path` must be an absolute path such as `/healthz`. It cannot be `/` on \
         its own, and it cannot contain a query (`?`) or a fragment (`#`).",
    );

    /// An `[http.cookies] same_site` that is none of `rule:http-server/cookies-are-secure-httponly-and-lax`'s three
    /// spellings — `Strictly`, `lax=true`, or a value left over from another
    /// framework's own key.
    ///
    /// Refused rather than defaulted because this block's whole job is to
    /// state what a cookie written with no options is: a browser drops an
    /// attribute it cannot parse and falls back to *its* default, so a tree
    /// with a fourth spelling has written a policy that the deployment
    /// believes is in force and nothing else does. It is also what lets
    /// `nvs_config::http::Cookies` resolve the key without an
    /// "or something else" arm, and so without ever repairing one
    /// (`rule:errors/ambiguous-input-refused`).
    ///
    /// Not `E0612`'s refusal reached from another direction: that one is a
    /// *pair* of individually meaningful values, and this is one value that
    /// has no meaning by itself — which is why it is asked first, the pair
    /// question being undecidable over a `same_site` nobody can read.
    pub const E_BAD_SAME_SITE: Code = Code::new("E0624").card(
        "`[http.cookies] same_site` must be `\"Strict\"`, `\"Lax\"` or `\"None\"`. Any other value \
         is not allowed.",
    );

    /// An `[http.headers]` policy value a header line cannot carry: a `\r\n`
    /// in a `referrer_policy`, a `content_security_policy` or a
    /// `permissions_policy`, or any other byte outside printable ASCII.
    ///
    /// These three are written onto every response verbatim, so a control
    /// character in one is a response-splitting attempt against every request
    /// the server will answer. `nvs_server::secure` already refuses to spell
    /// one and falls back to the shipped default, which is the right answer
    /// for a running server and the wrong one for a boot: the deployment
    /// believes the policy it wrote is in force, the shipped default is what
    /// is actually emitted, and nothing anywhere says so. Refusing at boot is
    /// what makes that fallback unreachable from a server that started, which
    /// is why both exist.
    ///
    /// Not `E0612`'s or `E0624`'s refusal reached from another direction:
    /// both of those are about a value's *meaning* under `rule:http-server/cors-is-closed-until-origins-are-named` and `rule:http-server/cookies-are-secure-httponly-and-lax`, and
    /// this is about whether the bytes can be transmitted at all.
    pub const E_UNCARRIABLE_HEADER: Code = Code::new("E0625").card(
        "A value in `[http.headers]` contains a line break or another character that a header \
         cannot contain. `referrer_policy`, `content_security_policy` and `permissions_policy` may \
         only contain printable ASCII characters.",
    );

    /// `[session] backend` names a store a session may not live in.
    ///
    /// `rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent` removed the memory
    /// cache tier from the candidates and said the removal is "enforced rather than documented";
    /// this code is that enforcement, and
    /// `rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot` is where the
    /// roster it checks against is written. A session read by one process and written by another
    /// must see one value, and the memory tier cannot give one — so a deployment that wrote
    /// `memory` has an authentication surface that holds until the deployment runs a second
    /// process, or until this one restarts.
    ///
    /// Refused where the key is written rather than where a session is started,
    /// for `E0613`'s reason applied to a worse failure: a session that vanishes
    /// looks like a user signing themselves out, so nothing in the running
    /// system ever reports it.
    pub const E_SESSION_BACKEND: Code = Code::new("E0626").card(
        "`[session] backend` must be `\"shared\"` or `\"db\"`. `\"memory\"` is not allowed, because \
         every server process must see the same session.",
    );

    /// `[metrics] exporter` or `[trace] exporter` names no exporter that block
    /// has.
    ///
    /// `rule:observability/metrics-and-trace-blocks-are-system` gives metrics a scrape (`prometheus`) or a push (`otlp`)
    /// and gives a trace only the push, with `false` the disabled state for
    /// both. The asymmetry is the section's, not a limitation of the exporters:
    /// a scrape answers with a series' current value, and a span is a finished
    /// record with no current value to answer with.
    ///
    /// Refused where the key is written rather than where an export would run,
    /// for `E0613`'s reason. Both blocks are `System`, so the value in force is
    /// the one the boot read, and a wrong one produces silence — a collector
    /// nothing writes to is indistinguishable from a deployment with nothing to
    /// say, so every dashboard over it is empty rather than wrong and nothing
    /// reports why.
    pub const E_UNSPELLED_EXPORTER: Code = Code::new("E0627").card(
        "The `exporter` key names an exporter that does not exist. `[metrics] exporter` accepts \
         `\"prometheus\"`, `\"otlp\"` or `false`. `[trace] exporter` accepts `\"otlp\"` or `false`.",
    );

    /// `[trace] sample` is not a fraction of one.
    ///
    /// `rule:observability/metrics-and-trace-blocks-are-system` writes the head sample as `0.0` to `1.0` inclusive. A
    /// number outside that names no smaller or larger sample — it names
    /// nothing, and what a reader does with `sample = 5` depends on which side
    /// of a comparison it lands on, where "record everything" and "record
    /// nothing" are both plausible readings of the same value.
    ///
    /// Non-finite is refused here rather than left to that comparison, because
    /// TOML spells `nan` and `inf` and every ordering against a `NaN` is false:
    /// a sample written as one would read as "record nothing" through the same
    /// expression that reads `0.0` that way.
    pub const E_SAMPLE_NOT_A_FRACTION: Code = Code::new("E0628").card(
        "`[trace] sample` must be a number from `0.0` to `1.0`. `0.0` records no requests, `0.5` \
         records half of them and `1.0` records every request.",
    );

    // `E0629` is retired and is never reused: it refused a configuration
    // directive that named a network address, and the server reads no such
    // directive.

    /// `nvs service` was asked to store an argv that names something other
    /// than a server.
    ///
    /// `rule:packaging/the-installer-is-a-sink`'s first two rows, under one code because they are one
    /// reason: the trailing argv is executed by a privileged account at every
    /// boot until somebody removes it, so what it names has to be a program
    /// that *stays running* and carries no testing hook. A subcommand outside
    /// the closed `serve`/`run` allowlist exits immediately, which every
    /// service manager reports as a crash loop forever; `--fault-inject` is
    /// `nvs-cli`'s own contained-panic hook, which that flag's doc comment
    /// says must never be reachable from a served request, and a service
    /// carrying it is exactly that with a privileged account attached.
    pub const E_SERVICE_ARGV_NOT_ALLOWED: Code = Code::new("E0630").card(
        "`nvs service` can only install a command that runs a server, `serve` or `run`. Other \
         commands and the `--fault-inject` option are not allowed in a service.",
    );

    /// A path in the argv `nvs service` was asked to store is relative: a
    /// `--config` or `--data` value, the entry file, or a `[log] target`
    /// file, `[opcache] file_cache_dir` or `[io] temp_root` in the
    /// configuration the service reads.
    ///
    /// `rule:packaging/the-installer-is-a-sink`'s third row: a service starts
    /// in `System32` or `/` under a minimal environment, so a relative path is
    /// a guaranteed first-boot failure surfacing as an opaque service-manager
    /// error. An argv with no `--config` is not this code's to refuse: the
    /// installer stores the absolute path of the data folder's `nvs.toml` in
    /// it, so the manager's starting directory is never searched for one.
    pub const E_SERVICE_PATH_NOT_ABSOLUTE: Code = Code::new("E0631").card(
        "Every path in the command that `nvs service` stores must be absolute. This includes the \
         paths in its configuration file. A service does not start in your current folder, so a \
         relative path is not found.",
    );

    /// An `--account` password was passed to `nvs service` on the command
    /// line.
    ///
    /// `rule:packaging/the-installer-is-a-sink`'s fifth row: a command line is readable by other users on
    /// the box, so the value is prompted for instead and is `secret` in
    /// `rule:security/secret-qualifier`'s
    /// sense for its whole life. The option exists in order to be refused by
    /// name — a bare "unrecognized argument" would read as a spelling mistake
    /// and send the operator looking for the right flag.
    pub const E_SERVICE_PASSWORD_ON_A_COMMAND_LINE: Code = Code::new("E0633").card(
        "`nvs service` does not accept the account password on the command line, because other \
         users on the machine can read it. Leave the password out, and `nvs service` asks for it.",
    );

    /// `nvs service` was run from an
    /// `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`
    /// bundle.
    ///
    /// `rule:packaging/a-bundle-may-not-install-itself`. A bundle is a single trust domain because the person who
    /// downloads and runs it is the only principal involved; installing a
    /// service creates a **second** principal — a privileged account executing
    /// that payload at every boot, with no operator having read what it
    /// contains — which is the boundary `rule:programs/bundle-trust-domain`
    /// declines to cross, arrived at from the other side.
    pub const E_SERVICE_FROM_A_BUNDLE: Code = Code::new("E0634").card(
        "A program built with `nvs build` cannot install itself as a service. Run `nvs service` \
         with the `nvs` program itself.",
    );

    /// `[cache.shared] url` or `[db.<name>] host` names a Unix-domain socket on a
    /// build that carries no `AF_UNIX` transport.
    ///
    /// `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot`.
    /// The transport is `#[cfg(unix)]`: `AF_UNIX` does exist on Windows, but the
    /// reactor's I/O layer does not carry it, so the spelling names a store this
    /// binary has no way to open — and every request until someone notices is one
    /// this refusal would have prevented.
    ///
    /// Not `E0626`'s refusal reached from another direction — that one is the
    /// same question answered the other way round. A session `backend = "db"`
    /// names a store the roster admits whose second half is unwritten, so
    /// refusing it at the key would claim the decision was wrong rather than that
    /// the build has not caught up; a Unix socket on a platform with no `AF_UNIX`
    /// is not waiting for this project, and the deployment has to be spelled
    /// differently. Reading it as loopback TCP instead is refused because a
    /// configuration that reads as one transport and runs as another is
    /// invisible in exactly the review that would have caught it.
    pub const E_NO_UNIX_TRANSPORT: Code = Code::new("E0635").card(
        "`[cache.shared] url` starts with `unix:`, which is a Unix socket, and this build cannot \
         open Unix sockets. This is the case on Windows. Write the url as `redis://host:port`.",
    );

    /// A `[server] workers` written as `0`.
    ///
    /// `rule:http-server/the-accept-fan-out-is-one-worker-per-core`
    /// : every listening socket this server binds is accepted on by a worker,
    /// so a count of zero is a process that takes the addresses the tree names
    /// and then answers nobody on any of them — `E0620`'s empty `listen`
    /// reached from the other end.
    ///
    /// Only zero is refused. The key is the bound rather than a request for
    /// one, so a count above what the machine answers
    /// `available_parallelism` with is what the operator asked for and is
    /// started: a heuristic that knew better than the block would make the
    /// core count something the file cannot state.
    pub const E_NO_WORKERS: Code = Code::new("E0636").card(
        "`[server] workers` is `0`, so no worker would answer a request. Write a number above zero, \
         or leave the key out to use one worker per CPU core.",
    );

    /// A **written** `Core\Queue::purge` queue name that the compiling
    /// machine's `queue.purge` grant does not cover —
    /// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s grant, read
    /// before the program runs rather than at the door alone.
    ///
    /// `E0618` one class over, and asked under `E0618`'s conditions and no
    /// others: a queue name given as a string literal, a configuration this machine actually
    /// read, and the same grant list walked by the same
    /// `nvs_config::capability::Capabilities`. So it refuses nothing
    /// `nvs_runtime::capability::require` would have allowed, which is
    /// `rule:expressions/preparation-preserves-behaviour`'s earlier answer and
    /// never a different one; a computed name, or a check run with no
    /// configuration in front of it, says nothing and leaves the refusal to
    /// the door.
    ///
    /// The scope is a queue name matched exactly, not `db.open`'s host
    /// pattern: a queue name is a flat string the program itself picked, so
    /// there are no labels for a `*.` to match at and a grant that covered a
    /// prefix would cover queues nobody had named yet.
    ///
    /// `Core\Queue::delete` has no code of its own. Its queue arrives inside a
    /// `Queue\Id` at run time, so there is no written name to read and the
    /// door is the only place the question can be asked. In this band rather
    /// than the types one for `E0618`'s reason: what it reads is a grant, not
    /// a type.
    pub const E_UNGRANTED_QUEUE: Code = Code::new("E0637").card(
        "`Core\\Queue::purge` names a queue that this machine's `queue.purge` capability does not \
         include. Add the queue name to `purge` under `[capabilities.queue]` in `nvs.toml`.",
    );

    /// `[http.client.tls] roots` naming no trust anchor at all — an empty
    /// list.
    ///
    /// Written out, that key is the whole answer to "whose certificates do
    /// you believe" for every outbound `https` call this process makes, and
    /// an empty list answers "nobody's". That is not a stricter deployment,
    /// it is one where every such call dies at the handshake with an unknown
    /// issuer — a message that sends an operator looking at the origin rather
    /// than at the key they wrote. Omitting the key is how a deployment asks
    /// for the compiled-in set, so there is nothing `[]` could have meant.
    pub const E_TLS_ROOTS_EMPTY: Code = Code::new("E0638").card(
        "`[http.client.tls] roots` is an empty list, so every outbound `https` call would fail. Name \
         at least one certificate file, or leave the key out to use the built-in certificates.",
    );

    /// `[http.client.tls] min_version` naming a version this build does not
    /// speak.
    ///
    /// The client implements TLS 1.2 and 1.3 and nothing beneath them, so
    /// `"1.0"` and `"1.1"` are not floors it can be lowered to: accepting
    /// either would leave the floor at 1.2 while telling an operator they had
    /// chosen otherwise, which is the one outcome worse than refusing the
    /// key.
    pub const E_TLS_MIN_VERSION: Code = Code::new("E0639").card(
        "`[http.client.tls] min_version` must be `\"1.2\"` or `\"1.3\"`. Older TLS versions are not \
         supported.",
    );

    /// `[http.client.tls] keylog` written on a host whose mode is
    /// `production`.
    ///
    /// The file it names collects every TLS session's secrets in the
    /// `SSLKEYLOGFILE` format, which is what decrypts this deployment's
    /// outbound traffic — the credentials inside it included — for anyone who
    /// can read the file. It is a debugging instrument, and the mode is where
    /// a deployment has already said whether it is debugging, so the key is
    /// announced in `development` and refused here rather than left for a
    /// reviewer to catch.
    pub const E_KEYLOG_IN_PRODUCTION: Code = Code::new("E0640").card(
        "`[http.client.tls] keylog` is not allowed in `production` mode. The file it writes can \
         decrypt every outbound connection. Use it only in `development`.",
    );

    /// `[http.client.tls]` resolved, and the outbound client it describes
    /// could not be built from it — a `roots` entry that cannot be opened or
    /// holds no certificate, or a `keylog` whose file will not open.
    ///
    /// The keys are checked where they are read and the anchors are parsed
    /// where they are believed, so this is the second half arriving at boot:
    /// the block passed `E0638`–`E0640` and the files it names still did not
    /// yield a client. It is a refusal to start rather than a warning,
    /// because the alternative is a process that answers every outbound
    /// `https` call with a handshake failure an operator reads as the
    /// origin's fault. The message names the file.
    pub const E_TLS_CLIENT_UNBUILDABLE: Code = Code::new("E0641").card(
        "A file named in `[http.client.tls]` cannot be used. A `roots` file cannot be opened or \
         contains no certificate, or the `keylog` file cannot be opened. The message names the file.",
    );

    /// A `[cache.memory] fill_wait` that is not a wait — `0`, or `false`.
    ///
    /// `rule:concurrency/a-secret-fill-runs-once-per-process`
    /// : on a miss exactly one caller in the process runs the fill while
    /// every other caller waits for it, so this key is what bounds that
    /// wait. Zero makes every concurrent caller but one throw
    /// `TimeoutError` — the stampede's failure mode with the fetches
    /// removed, and nothing bought with it — and `false` asks for a wait
    /// nothing ends, which
    /// `rule:http-server/no-spelling-for-an-unbounded-wait` gives no
    /// spelling for anywhere else either. A caller that wants a shorter
    /// wait than the operator's writes `wait` at its own call site, where
    /// that decision is visible in review; an operator who wants a longer
    /// one raises this key.
    pub const E_FILL_WAIT_NOT_A_WAIT: Code = Code::new("E0642").card(
        "`[cache.memory] fill_wait` must be a length of time above zero, such as `\"5s\"`. `0` and \
         `false` are not allowed.",
    );

    /// A `[http.client.proxy]` block that writes no `resolve`.
    ///
    /// `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`
    /// : the word is mandatory and there is no default, because both
    /// answers are commonly correct and either default silently does the
    /// wrong thing in somebody's production — `local` fails outright in a
    /// network where only the proxy resolves, and `proxy` gives up the
    /// address pin everywhere else. The refusal names both words rather
    /// than picking one, since the deployment's own network is what
    /// decides which is true and this file cannot know it.
    pub const E_PROXY_RESOLVE_MISSING: Code = Code::new("E0643").card(
        "A `[http.client.proxy]` block must set `resolve` to `\"local\"` or `\"proxy\"`. With \
         `\"local\"`, this server looks up host names. With `\"proxy\"`, the proxy looks them up.",
    );

    /// A `[http.client.proxy] resolve` that is neither `local` nor
    /// `proxy`.
    ///
    /// The two words are the whole roster, and they name who resolves the
    /// destination — which is the one question that decides whether
    /// `rule:security/net-address-policy` still sees an address. A third
    /// word has no reading to fall back on, and reading it as either of
    /// the two would pick a security posture out of a typo.
    pub const E_PROXY_RESOLVE_UNKNOWN: Code = Code::new("E0644").card(
        "`[http.client.proxy] resolve` must be `\"local\"` or `\"proxy\"`. With `\"local\"`, this \
         server looks up host names. With `\"proxy\"`, the proxy looks them up.",
    );

    /// A `[http.client.proxy]` block whose `url` this client cannot dial —
    /// absent, not `http://`, naming no host, or carrying a credential or
    /// a port that is not one.
    ///
    /// `rule:http-server/an-outbound-proxy-is-operator-configured`
    /// : every destination is reached by `CONNECT` over plain TCP to this
    /// address, so `https://` is a second trust decision with no spelling
    /// here and every other scheme is one this client does not speak. A
    /// block with no `url` is refused rather than read as "no proxy":
    /// leaving the block out is how a deployment asks for that, and a
    /// written block that proxies nothing is a deployment believing its
    /// egress is tunnelled when it is not. Userinfo in the URL is refused
    /// for the same reason — `username` and `password` are where a
    /// credential is read from, so one written here would be dropped in
    /// silence.
    pub const E_PROXY_URL_UNDIALABLE: Code = Code::new("E0645").card(
        "`[http.client.proxy] url` must be an `http://` address with a host, such as \
         `http://proxy.example.com:3128`. Put a user name and password in `username` and \
         `password`, not in the URL.",
    );

    /// A `[http.client.proxy] bypass` entry that is not a host name — one
    /// with a port, a scheme, a `*` or a `/`.
    ///
    /// `rule:http-server/an-outbound-proxy-is-operator-configured`
    /// : the list matches the URL's host text, each entry exact or with a
    /// leading `.` for a suffix, and it is matched before anything is
    /// resolved. A range or a wildcard in that position hands back more
    /// than the operator can see they are handing back — the destinations
    /// it covers are the ones that then leave without the proxy — and a
    /// port or a scheme is an entry that can never match, which is a
    /// bypass an operator believes is in force. The refusal names the
    /// entry.
    pub const E_PROXY_BYPASS_ENTRY: Code = Code::new("E0646").card(
        "An entry in `[http.client.proxy] bypass` must be a host name, such as `api.example.com`, or \
         `.example.com` for every host under it. A port, a scheme, `*` and `/` are not allowed.",
    );

    /// A `[http.client.socket]` bound written with no bound in it — a
    /// `max_message` or a `send_timeout` of `false` or of zero.
    ///
    /// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`
    /// : an outbound socket has no spelling for reassembling a message of any
    /// size or for waiting forever to write one, the same way
    /// `Core\Http\Options` has none for an unbounded call
    /// (`rule:http-server/no-spelling-for-an-unbounded-wait`). `false` removes
    /// a ceiling everywhere else in this file, and a socket whose message cap
    /// it removed is the memory of the task that opened it spent by whatever
    /// the peer decides to send. Zero is that value written the other way
    /// round: a cap of nothing admits no message and a wait of nothing writes
    /// no frame, so the block configured a socket that can hold no
    /// conversation. The refusal names the key and the two shipped values.
    pub const E_SOCKET_BOUND_REMOVED: Code = Code::new("E0647").card(
        "`[http.client.socket] max_message` and `send_timeout` need a value above zero. `false` and \
         `0` are not allowed. The message shows the default value.",
    );

    /// A `[server] socket_mode` that is not a file mode — not octal, wider
    /// than `0777`, or carrying a bit that is not a permission.
    ///
    /// Refused rather than read as far as it parses, because the mode on a
    /// Unix-domain socket is the whole of who may connect to it and a
    /// connection that arrives is implicitly trusted for the forwarded
    /// headers (`rule:http-server/a-unix-socket-listener`). A value this
    /// reader had to guess the intent of would be a trust boundary chosen
    /// out of a typo, and the guess an operator would least expect is the
    /// one that widens it.
    pub const E_BAD_SOCKET_MODE: Code = Code::new("E0648").card(
        "`[server] socket_mode` must be an octal file mode no larger than `0777`, such as \
         `\"0660\"`. It sets who may connect to the Unix socket.",
    );

    /// A `[server.connection]` bound written with no bound in it — any of
    /// that block's keys as `false` or as zero.
    ///
    /// `rule:concurrency/connection-bounds-are-finite` is the one bound set
    /// whose subject outlives the request that created it, so a connection
    /// held open with a ceiling an operator removed is memory and a
    /// descriptor nothing in the process ever reclaims. `false` removes a
    /// ceiling everywhere else in the tree
    /// (`rule:config/three-changeability-classes`) and has no meaning here;
    /// zero is the same value from the other side — no connection may be
    /// open, no frame may arrive and no wait may elapse — which is a server
    /// that upgrades nothing. The refusal names the key and the number the
    /// block ships with.
    ///
    /// A value that is not a size, a count or a duration at all is `E0601`
    /// in `nvs_config::value`'s own words; this code is only for a
    /// well-formed value whose *magnitude* is the problem.
    pub const E_CONNECTION_BOUND_REMOVED: Code = Code::new("E0649").card(
        "A key in `[server.connection]` is `false` or `0`. Every key in this block needs a limit \
         above zero. The message shows the default value.",
    );

    /// `[http] csrf_key` holding something that is not a key: text that is
    /// not base64, or base64 of any length other than the one a key is.
    ///
    /// Refused rather than ignored, because ignoring it is the silent
    /// direction. The key is what arms the token half of
    /// `rule:security/csrf-is-on-by-default`, so a deployment whose value
    /// the door could not read would go on serving with its unsafe verbs
    /// unverified while its configuration says they are checked — the one
    /// way a security directive must not fail. The refusal names the key
    /// and what a key is, and never a byte of the value.
    pub const E_BAD_CSRF_KEY: Code = Code::new("E0650").card(
        "`[http] csrf_key` is not a valid key. It must be base64 text of a 32-byte key.",
    );

    /// An `[[extension]]` entry with no `path`, no `sha256`, or a `sha256`
    /// that is not 64 hexadecimal digits —
    /// `rule:packaging/extension-loading-is-root-controlled`'s boot and
    /// reload refusal, raised by `nvs_config::extension::validate` and
    /// pointing at the entry's header line. Its own code rather than `E0601`,
    /// because a missing key is not a bad value: the entry is incomplete, and
    /// an unpinned entry is a binary the operator never vouched for.
    pub const E_BAD_EXTENSION_ENTRY: Code = Code::new("E0651").card(
        "An `[[extension]]` entry is not complete. It needs a `path` to the `.nvsx` file and a \
         `sha256`, which is the file's SHA-256 written as 64 hexadecimal digits.",
    );

    /// A complete `[[extension]]` entry whose file does not load: the
    /// digest differs from the pin, the component does not validate, the
    /// manifest does not match it, or its class is reserved or already
    /// declared — `rule:packaging/extension-loading-is-root-controlled`'s
    /// load refusals, raised by `nvs serve` at boot and at reload, pointing
    /// at the entry's header line. One entry refused is the whole set
    /// refused.
    pub const E_EXTENSION_NOT_LOADED: Code = Code::new("E0652").card(
        "An `[[extension]]` file does not load. The reason is in the message. The server does \
         not start, or a reload keeps the extensions it already has.",
    );

    /// `nvs service install` could not create or use the data folder of the
    /// service it was asked to install, and that folder's `nvs.toml` is the
    /// service's configuration: the folder, or a subfolder, could not be
    /// created, it fails `rule:config/ownership-is-the-trust-boundary`'s
    /// check, or the shipped `nvs.toml` could not be written into it.
    ///
    /// A refusal rather than the one warning every other command gives for an
    /// unusable data folder, because a service whose configuration cannot be
    /// read refuses to start at every boot. With a `--config` named elsewhere
    /// the service still boots, so the installer prints that warning instead
    /// and installs.
    pub const E_SERVICE_DATA_UNUSABLE: Code = Code::new("E0653").card(
        "`nvs service install` cannot create or use the data folder of the service. The message \
         shows the folder and the reason. Fix the folder, or add `--data <folder>` to the service \
         command with a folder that only administrators can change.",
    );

    /// `nvs service install --account <name>` or `nvs service unit` on Linux
    /// names an account the password database does not have.
    ///
    /// `rule:packaging/a-service-runs-as-a-virtual-account`'s Linux half gives
    /// the account its data folder by owner and group, so the installer needs
    /// the account's ids before it changes anything. A unit whose `User=` names
    /// no account is one systemd fails to start at every boot, so the install
    /// is refused in front of the manager rather than registered over it.
    pub const E_SERVICE_NO_SUCH_ACCOUNT: Code = Code::new("E0654").card(
        "On Linux, the account that `--account` names must exist before `nvs service` installs \
         the service. The service runs as this account. Create the account first, for example \
         with `useradd --system`, or name an account that exists.",
    );

    /// A `[grants]` table in the application's `package.toml` that
    /// `nvs_config::grants` cannot read: the file is not TOML, a key is not a
    /// namespace, a value is not a list, or a name in it is not a capability
    /// (`rule:security/grants-are-keyed-on-a-namespace`). An unknown name is
    /// refused because a grant line that grants nothing by a typo looks like
    /// one that works.
    pub const E_BAD_GRANT: Code = Code::new("E0655").card(
        "A line in the `[grants]` table of `package.toml` is not valid. Each key is a namespace, \
         such as `'Acme\\Http'`. Add `\\*` to the end of the key to also grant every namespace \
         below it. Each value is a list of capability names, such as `[\"net.connect\"]`. The \
         message shows the line and, for a wrong name, the list of capabilities.",
    );

    /// Two keys of `[grants]` that match the same namespaces, such as
    /// `'\Acme\Http'` and `'Acme\Http'`. `rule:security/longest-prefix-wins`
    /// refuses the tie, because neither key could win.
    pub const E_GRANT_KEYS_TIE: Code = Code::new("E0656").card(
        "Two keys in the `[grants]` table of `package.toml` name the same namespace. A `\\` at \
         the start of a key does not change the key. Keep one of the two lines.",
    );

    /// A key of `[grants]` under `Core`. The namespace is reserved
    /// (`rule:core-api/reserved-namespace`), so no line can grant it anything.
    pub const E_GRANT_UNDER_CORE: Code = Code::new("E0657").card(
        "A key in the `[grants]` table of `package.toml` names `Core` or a namespace below it. \
         `Core` is the standard library, and you cannot grant it capabilities. Delete the line.",
    );

    /// A call to a `Core` member that needs a capability the enclosing
    /// namespace's `[grants]` line does not grant
    /// (`rule:security/authority-is-the-enclosing-namespace`). The message names
    /// the namespace, the capability and the line to change.
    pub const E_CAPABILITY_NOT_GRANTED: Code = Code::new("E0658").card(
        "This code calls a `Core` method that needs a capability, such as `fs.read`. The code is \
         in a namespace that has a line in the `[grants]` table of `package.toml`. That line does \
         not list the capability, so the call does not compile. Add the capability to the line if \
         this code should have it. The message shows the line.",
    );

    /// A package's own `package.toml` that `nvs_config::package` cannot read:
    /// the file is not TOML, `[package] name` is not `Vendor\Package` or is
    /// under `Core`, a name in `[capabilities]` is not a capability, or one is
    /// both required and optional
    /// (`rule:packaging/a-package-name-is-vendor-slash-name`).
    pub const E_BAD_PACKAGE_MANIFEST: Code = Code::new("E0659").card(
        "The `package.toml` of a package is not valid. `[package] name` is a vendor and a \
         name, such as `'Acme\\Http'`. `[capabilities]` has two lists, `required` and \
         `optional`, of capability names such as `\"net.connect\"`. A capability is in one list \
         or the other. The message shows the line.",
    );

    /// A file of a package that declares a namespace outside the package's own
    /// (`rule:packaging/a-package-file-stays-under-its-declared-prefix`). The
    /// message names the file, the namespace and the package.
    pub const E_PACKAGE_NAMESPACE_OUTSIDE: Code = Code::new("E0660").card(
        "A file of a package declares a namespace that does not belong to the package. Every \
         file of the package `Acme\\Http` declares `namespace Acme\\Http;` or a namespace below \
         it, such as `Acme\\Http\\Client`. Change the `namespace` line of the file.",
    );

    /// A `require` or `autoload` path written in a package that leaves the
    /// package's directory through `..`, an absolute path or a symlink
    /// (`rule:packaging/a-package-cannot-reach-its-host`).
    pub const E_PACKAGE_PATH_ESCAPES: Code = Code::new("E0661").card(
        "A `require` or `autoload` path in a package names a file or directory outside the \
         package. A package can only load its own files. The path may leave the package with \
         `..`, with an absolute path or through a symbolic link. Change the path, or move the \
         file into the package.",
    );
}
