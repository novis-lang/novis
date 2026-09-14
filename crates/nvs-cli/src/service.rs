//! `nvs service` — `rule:packaging/a-service-is-one-stored-argv`'s operator surface: the one argv a service
//! manager stores, the closed set of things that argv may name, and the two
//! spellings of "print exactly what would be installed".
//!
//! # The installer is a sink
//!
//! `rule:security/sink-predicate` defines a sink as a parameter whose content becomes an
//! instruction rather than data, and § 2 of 0093 calls the trailing argv here
//! the sharpest case in the project: the command runs elevated, and what it
//! stores is executed by a privileged account at every boot until somebody
//! removes it. So [`plan`] is the whole surface's front door and its default is
//! refusal — every path out of it is either a [`Plan`], which is constructed
//! nowhere else, or one of § 2's refusal classes as an `E063x`
//! diagnostic. Nothing downstream re-checks, because nothing downstream can
//! obtain a `Plan` without having gone through it.
//!
//! The codes are `E0630` (the argv names something other than a server),
//! `E0631` (a relative path, or no `--config` at all), `E0632` (output with
//! nowhere to go), `E0633` (a password on a command line) and `E0634` (a
//! bundle installing itself). § 2's table carries more rows than there are
//! codes here, because rows that share a reason share a code: the
//! subcommand allowlist and `--fault-inject` are both "the stored argv names
//! an instruction that must not run under a privileged account", and the ADR
//! itself calls a missing `--config` "the same first-boot failure as a
//! relative path, one step less visible". Each code's own doc comment in
//! `nvs_diagnostics::code` owns the reasoning.
//!
//! **What counts as a path is a closed list, not a guess.** Every `--config`
//! value in the argv, the entry file `serve`/`run` names, and the installer's
//! own `--log-file` — and nothing else, because a rule that refused any word
//! that *looked* like a path would refuse a `--listen` and an `rule:tooling/terminal-output-is-a-sink`
//! program argument that happen to contain a separator. `Path::is_absolute` is
//! the test, so it answers for **the host the installer is running on**: a
//! Unix-shaped `/etc/nvs/nvs.toml` is not absolute on Windows, and refusing it
//! there is correct rather than a portability gap — a service is installed on
//! the machine it will run on.
//!
//! **A `[log] target` of `stderr` is not a destination.** § 4's *Output* is
//! explicit that a service has no console handle, so the process's stderr goes
//! nowhere; only `file:<path>` and `syslog` satisfy § 2's fourth row, and
//! reading `stderr` as satisfying it would let exactly the case that row exists
//! to prevent through.
//!
//! # What is on disk, and what is not
//!
//! [`unit()`] renders § 5's systemd unit and [`image_path`] § 3's Windows
//! `ImagePath`, both from a `Plan` and both pure. `nvs service unit` prints
//! one of them and touches nothing, which is § 5's whole design on Linux and
//! its `--print` on Windows.
//!
//! **Registration itself has not landed** — no `install`, `uninstall`,
//! `start`, `stop`, `status` or `run` subcommand exists, and neither the SCM
//! call of § 3 nor the unit-directory write and `daemon-reload` of § 5 is
//! implemented. That is deliberate rather than forgotten: both need
//! administrator rights on a machine somebody chose, and 0093's own
//! *Verification* lists them among the end-to-end checks. [`Delivery`] and
//! [`destination`] are the seam they will arrive at — `destination` already
//! answers where a unit *would* go, and answers `None` for the printing form,
//! which is what makes "printed, and written only on install" a property this
//! module can be asked about rather than a comment.
//!
//! # The manager is told what state this process is in
//!
//! The `Type=notify` line [`unit()`] renders is a promise, and [`Notify`] is
//! what keeps it: a unit of that type whose process never sends `READY=1` is
//! one systemd ends at `TimeoutStartSec` however well it is serving.
//! `rule:packaging/the-generated-unit-is-hardened` is the list — `READY=1` once
//! every listener is bound, `RELOADING=1` and `READY=1` around a reload, and
//! `STOPPING=1` once the drain has begun.
//!
//! **The protocol is written by hand.** `sd_notify` is one datagram of
//! `NAME=value` lines to whatever `$NOTIFY_SOCKET` names, so what a crate for
//! it would save is the page below, and `libsystemd` would be a C dependency
//! `rule:packaging/a-c-dependency-answers-two-questions` would have to answer
//! for. A process nothing started that way has no `NOTIFY_SOCKET` and sends
//! nothing at all, which is every `nvs serve` an operator runs by hand.
//!
//! **A report is a sink call, so the states a process goes through are
//! assertable without a service manager.** [`Supervisor`] is that seam, and the
//! datagram is one implementation of it. What it spends is one unbound socket
//! for the life of a process systemd started, and nothing per request or per
//! report.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, OnceLock};

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap, code};

use crate::render_diagnostics;

/// The closed allowlist of § 2's first row: the two subcommands that stay
/// running and mean something with no terminal attached.
const ALLOWED: [&str; 2] = ["serve", "run"];

/// Where a systemd unit goes when § 5's explicit request is made. Named here
/// rather than inside [`destination`] so a test can point that function at a
/// directory it owns.
const UNIT_DIRECTORY: &str = "/etc/systemd/system";

/// The options in a stored argv whose value is a path, so [`plan`] knows which
/// words § 2's third row applies to. `--listen` is deliberately absent: an
/// address is not a path.
const PATH_OPTIONS: [&str; 1] = ["--config"];

/// The options in a stored argv that take a separate value word, so the walk
/// looking for the entry file does not read one of them as the entry.
const VALUED: [&str; 3] = ["--config", "--listen", "--port"];

/// What an operator asked to have stored, before § 2 has looked at any of it.
pub(crate) struct Request<'a> {
    /// The service's name — the same identity `nvs ctl --socket` addresses one
    /// of several servers on a host by (§ 1).
    pub(crate) name: &'a str,
    /// Everything to the right of `--`, stored untouched and never
    /// interpreted, which is what makes § 1's "every parameter is passable"
    /// true.
    pub(crate) argv: &'a [String],
    /// The installer's own `--log-file`, one of the two answers to § 2's
    /// fourth row.
    pub(crate) log_file: Option<&'a Path>,
    /// § 4's `--account`, for a deployment that needs a domain identity rather
    /// than the default virtual account.
    pub(crate) account: Option<&'a str>,
    /// A password written on the command line. It exists in order to be
    /// refused by name (`E0633`); the value is prompted for instead.
    pub(crate) password: Option<&'a str>,
}

/// What the process running the installer can answer about itself and about
/// the configuration the argv names.
///
/// A parameter rather than something [`plan`] reads for itself, so every § 2
/// refusal is reachable in a test with no service manager, no elevation and no
/// files — which is the whole of that section's *Verification* that does not
/// need a privileged machine.
pub(crate) struct Host {
    /// This binary, absolute. § 3 quotes it unconditionally.
    pub(crate) exe: PathBuf,
    /// Whether this binary is an `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host` bundle (`rule:programs/bundle-trust-domain`).
    ///
    pub(crate) from_a_bundle: bool,
    /// Whether the config the argv names sets `[log] target` to something a
    /// process with no console handle can actually write to.
    pub(crate) config_names_a_log_destination: bool,
    /// `[control] socket`, for the unit's `ExecReload` (§ 5).
    pub(crate) control_socket: Option<String>,
    /// `[limits] memory`, which § 5 derives `MemoryMax` from rather than
    /// inventing one.
    pub(crate) memory_max: Option<String>,
    /// Whether `[server] listen` includes a privileged port, which is the only
    /// condition under which § 5 emits `AmbientCapabilities` at all.
    pub(crate) privileged_port: bool,
}

/// A checked request: every § 2 refusal has already been made against it.
///
/// Constructed on exactly one code path — the far side of [`plan`] — so a
/// caller holding one is holding the installer's guarantee rather than an
/// intention to check later.
#[derive(Debug)]
pub(crate) struct Plan {
    name: String,
    exe: PathBuf,
    argv: Vec<String>,
    account: Option<String>,
    control_socket: Option<String>,
    memory_max: Option<String>,
    privileged_port: bool,
}

/// Whether a rendered unit is printed for review or written where the service
/// manager reads it.
///
/// § 5's asymmetry, as a value: on Linux the representation of a service *is*
/// a text file the operator's configuration management already owns the
/// directory for, so printing is the default and writing is the explicit
/// request.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Delivery {
    /// Write the unit to standard output and touch nothing.
    Print,
    /// Write it into the system unit directory.
    ///
    /// Nothing outside a test constructs this yet: § 5's write and its
    /// `daemon-reload` are the unlanded half the module doc names, and this
    /// variant is the shape they will arrive as.
    #[cfg_attr(not(test), expect(dead_code, reason = "§ 5's install has not landed"))]
    Install,
}

/// § 2, in order, and the only way to obtain a [`Plan`].
///
/// The order is the order the refusals are worth reading in: what the argv
/// *names* first, then what it could not find, then what it could not write,
/// so an operator fixing one refusal is not immediately handed a second about
/// a word they were already going to change.
///
/// # Errors
///
/// One of § 2's refusal classes, as an `E063x` diagnostic naming what was
/// refused and why — never a bare non-zero exit, which is that section's
/// closing sentence.
pub(crate) fn plan(request: &Request<'_>, host: &Host) -> Result<Plan, Diagnostic> {
    if host.from_a_bundle {
        return Err(Diagnostic::error(
            code::E_SERVICE_FROM_A_BUNDLE,
            "a bundled executable may not install itself as a service".to_owned(),
        )
        .with_note(
            "`rule:programs/bundle-trust-domain` makes a bundle one trust domain because the person who runs it is the \
             only principal involved; a service adds a privileged account running the same \
             payload at every boot"
                .to_owned(),
        )
        .with_help(
            "install the `nvs` binary and name the program in the stored argv instead".to_owned(),
        ));
    }

    let Some(subcommand) = request.argv.first() else {
        return Err(not_allowed(
            "the stored argv is empty",
            "a service manager starts this argv and expects a process that keeps running",
        ));
    };
    if !ALLOWED.contains(&subcommand.as_str()) {
        return Err(not_allowed(
            &format!("`nvs {subcommand}` may not be installed as a service"),
            "only `serve` and `run` are allowed: every other subcommand either exits immediately, \
             which a service manager reports as a crash loop forever, or is meaningless with no \
             terminal attached",
        ));
    }
    if let Some(word) = request
        .argv
        .iter()
        .find(|word| *word == "--fault-inject" || word.starts_with("--fault-inject="))
    {
        return Err(not_allowed(
            &format!("`{word}` may not be installed as a service"),
            "that hook provokes a contained engine panic and must never be reachable from a \
             served request, which a service carrying it is — with a privileged account attached",
        ));
    }

    if request.password.is_some() {
        return Err(Diagnostic::error(
            code::E_SERVICE_PASSWORD_ON_A_COMMAND_LINE,
            "an account password may not be passed on the command line".to_owned(),
        )
        .with_note(
            "a command line is readable by other users on this machine, and the value would \
             outlive the invocation in whatever recorded it"
                .to_owned(),
        )
        .with_help(
            "drop the option and enter the password when the installer prompts for it".to_owned(),
        ));
    }

    let configs = values_of(request.argv, "--config");
    if configs.is_empty() {
        return Err(Diagnostic::error(
            code::E_SERVICE_PATH_NOT_ABSOLUTE,
            "the stored argv names no `--config`".to_owned(),
        )
        .with_note(
            "without one the service would fall back to `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`'s `./nvs.toml`, making its \
             configuration a property of whatever directory the service manager happened to start \
             it in"
                .to_owned(),
        )
        .with_help("name the configuration file absolutely, as `--config <path>`".to_owned()));
    }
    // The first one only. An operator fixing a path re-runs this, and a list
    // of every relative word in an argv reads as a worse failure than the one
    // that is actually being reported.
    if let Some((option, written)) = relative_paths(request).into_iter().next() {
        return Err(Diagnostic::error(
            code::E_SERVICE_PATH_NOT_ABSOLUTE,
            format!("`{option}` is relative: `{written}`"),
        )
        .with_note(
            "a service starts under a working directory and an environment that are not this \
             shell's, so a relative path is a guaranteed first-boot failure reported as an opaque \
             service-manager error"
                .to_owned(),
        )
        .with_help("write the path absolutely".to_owned()));
    }

    if request.log_file.is_none() && !host.config_names_a_log_destination {
        return Err(Diagnostic::error(
            code::E_SERVICE_OUTPUT_GOES_NOWHERE,
            "the service would have nowhere to write diagnostics".to_owned(),
        )
        .with_note(
            "a service has no console handle, so its standard error is discarded: a refused \
             compile or a FATAL under this argv would leave no trace anywhere"
                .to_owned(),
        )
        .with_help(
            "pass `--log-file <path>`, or set `[log] target` to `file:<path>` or `syslog` in the \
             named configuration"
                .to_owned(),
        ));
    }

    Ok(Plan {
        name: request.name.to_owned(),
        exe: host.exe.clone(),
        argv: request.argv.to_vec(),
        account: request.account.map(str::to_owned),
        control_socket: host.control_socket.clone(),
        memory_max: host.memory_max.clone(),
        privileged_port: host.privileged_port,
    })
}

/// `E0630`, whose two rows share a message shape.
fn not_allowed(message: &str, note: &str) -> Diagnostic {
    Diagnostic::error(code::E_SERVICE_ARGV_NOT_ALLOWED, message.to_owned())
        .with_note(note.to_owned())
        .with_help("a service runs `nvs serve` or `nvs run` and nothing else".to_owned())
}

/// Every relative path in the request, as the option that carried it and the
/// word itself, in the order an operator would fix them.
fn relative_paths<'a>(request: &'a Request<'_>) -> Vec<(&'a str, String)> {
    let mut out = Vec::new();
    for option in PATH_OPTIONS {
        for written in values_of(request.argv, option) {
            if !Path::new(written).is_absolute() {
                out.push((option, written.to_owned()));
            }
        }
    }
    if let Some(entry) = entry_file(request.argv)
        && !Path::new(entry).is_absolute()
    {
        out.push(("the entry file", entry.to_owned()));
    }
    if let Some(log) = request.log_file
        && !log.is_absolute()
    {
        out.push(("--log-file", log.display().to_string()));
    }
    out
}

/// Every value `option` was given in `argv`, in both the `--opt value` and
/// `--opt=value` spellings.
fn values_of<'a>(argv: &'a [String], option: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut words = argv.iter();
    while let Some(word) = words.next() {
        if word == option {
            if let Some(value) = words.next() {
                out.push(value.as_str());
            }
        } else if let Some(value) = word.strip_prefix(option)
            && let Some(value) = value.strip_prefix('=')
        {
            out.push(value);
        }
    }
    out
}

/// The file `serve` or `run` was given, if the argv names one.
///
/// The first bare word after the subcommand, with the value of a
/// separately-spelled option skipped so a `--config /etc/nvs.toml` is not read
/// as the entry.
fn entry_file(argv: &[String]) -> Option<&str> {
    let mut words = argv.iter().skip(1);
    while let Some(word) = words.next() {
        if VALUED.contains(&word.as_str()) {
            words.next();
        } else if !word.starts_with('-') {
            return Some(word);
        }
    }
    None
}

/// § 3's Windows `ImagePath`: the binary and the stored argv encoded into the
/// one string the SCM keeps, under `CommandLineToArgvW`'s rules.
///
/// The binary path is quoted **unconditionally**, whether or not it currently
/// contains a space — the rule that still holds after somebody moves the
/// installation, which is when the unquoted-path finding usually appears.
pub(crate) fn image_path(plan: &Plan) -> String {
    let mut out = quote(&plan.exe.display().to_string(), true);
    for word in &plan.argv {
        out.push(' ');
        out.push_str(&quote(word, false));
    }
    out
}

/// One argument, encoded so `CommandLineToArgvW` hands it back unchanged.
///
/// The rule that makes this worth confining to one function: a backslash is
/// literal *except* in the run immediately before a quote, where each one is
/// doubled — including the run at the end of the argument, which sits before
/// the closing quote this encoder adds. A directory path ending in `\` is
/// therefore the case a hand-written joiner gets wrong.
fn quote(argument: &str, force: bool) -> String {
    if !force && !argument.is_empty() && !argument.contains([' ', '\t', '"']) {
        return argument.to_owned();
    }
    let mut out = String::from('"');
    let mut backslashes = 0;
    for character in argument.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                for _ in 0..backslashes * 2 + 1 {
                    out.push('\\');
                }
                out.push('"');
                backslashes = 0;
            }
            _ => {
                for _ in 0..backslashes {
                    out.push('\\');
                }
                backslashes = 0;
                out.push(character);
            }
        }
    }
    for _ in 0..backslashes * 2 {
        out.push('\\');
    }
    out.push('"');
    out
}

/// `CommandLineToArgvW`'s rules, so [`image_path`]'s round trip is a property a
/// test can assert rather than a claim.
///
/// One deliberate narrowing: the `""`-inside-a-quoted-run spelling of a literal
/// quote is read as a close followed by a reopen rather than as one quote.
/// [`quote`] never emits that form — it writes `\"` — so the round trip holds,
/// and a decoder that is exact for everything this module produces is what the
/// property needs.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the round-trip property's half")
)]
pub(crate) fn decode(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quoted = false;
    let mut backslashes = 0usize;
    for character in line.chars() {
        match character {
            '\\' => {
                backslashes += 1;
                started = true;
            }
            '"' => {
                for _ in 0..backslashes / 2 {
                    current.push('\\');
                }
                if backslashes % 2 == 1 {
                    current.push('"');
                } else {
                    quoted = !quoted;
                }
                backslashes = 0;
                started = true;
            }
            ' ' | '\t' if !quoted => {
                for _ in 0..backslashes {
                    current.push('\\');
                }
                backslashes = 0;
                if started {
                    out.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            _ => {
                for _ in 0..backslashes {
                    current.push('\\');
                }
                backslashes = 0;
                current.push(character);
                started = true;
            }
        }
    }
    for _ in 0..backslashes {
        current.push('\\');
    }
    if started {
        out.push(current);
    }
    out
}

/// § 5's systemd unit, rendered from a checked plan.
///
/// Some of its lines are conditions rather than constants, and each is stated
/// in that section: `AmbientCapabilities` is emitted **only** when the
/// configured listen addresses include a privileged port, so the ordinary case
/// grants nothing at all, and `MemoryMax` is derived from `[limits]` rather
/// than invented. `ExecReload` is `rule:config/one-local-control-socket`'s control socket, so a unit
/// generated for a server with no `[control] socket` carries no reload line
/// rather than one that would fail.
///
pub(crate) fn unit(plan: &Plan) -> String {
    let exe = plan.exe.display();
    let mut out = String::new();
    out.push_str("[Unit]\n");
    out.push_str(&format!("Description=Novis service {}\n", plan.name));
    out.push_str("After=network.target\n\n");

    out.push_str("[Service]\n");
    out.push_str("Type=notify\n");
    out.push_str(&format!("ExecStart={exe}"));
    for word in &plan.argv {
        out.push(' ');
        out.push_str(&shell_word(word));
    }
    out.push('\n');
    if let Some(socket) = &plan.control_socket {
        out.push_str(&format!(
            "ExecReload={exe} ctl reload --socket {}\n",
            shell_word(socket)
        ));
    }
    out.push_str("WatchdogSec=30\n");
    if let Some(account) = &plan.account {
        out.push_str(&format!("User={account}\n"));
    }
    if let Some(memory) = &plan.memory_max {
        out.push_str(&format!("MemoryMax={memory}\n"));
    }
    out.push_str("NoNewPrivileges=true\n");
    out.push_str("ProtectSystem=strict\n");
    out.push_str("ProtectHome=true\n");
    out.push_str("PrivateTmp=true\n");
    out.push_str("CapabilityBoundingSet=\n");
    if plan.privileged_port {
        out.push_str("AmbientCapabilities=CAP_NET_BIND_SERVICE\n");
    }
    out.push_str("RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX\n");
    out.push_str("SystemCallFilter=@system-service\n\n");

    out.push_str("[Install]\n");
    out.push_str("WantedBy=multi-user.target\n");
    out
}

/// One word of a systemd `ExecStart`, quoted only where it has to be.
fn shell_word(word: &str) -> String {
    if word.is_empty() || word.contains([' ', '\t', '"', '\'']) {
        format!("\"{}\"", word.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        word.to_owned()
    }
}

/// What a service manager is told this process is doing, and the whole set.
///
/// There is no `STATUS=` line beside these: what an operator reads a refusal
/// out of is the diagnostic the reload rendered and `Core\Log`, and a summary
/// in a third place is a third place for it to be wrong in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    /// The boot is finished — every listener is bound and every mounted entry
    /// is compiled — so `systemctl start` returns when the port accepts rather
    /// than when this process was forked.
    Ready,
    /// A reload has begun. What ends it is [`State::Ready`] whatever the
    /// reload's own outcome was: a refused reload leaves this process serving
    /// the tree it already had, and a manager left in `reloading` over it would
    /// be reporting a state the process is not in.
    Reloading,
    /// The drain has begun, so this process is answering what it accepted and
    /// accepting nothing further.
    Stopping,
}

impl State {
    /// The assignment this state sends. A datagram carries this line, and for a
    /// reload the timestamp beneath it.
    ///
    /// Present where there is something to write it to: a datagram on Unix, and
    /// a case standing in for a manager anywhere.
    #[cfg(any(unix, test))]
    fn line(self) -> &'static str {
        match self {
            State::Ready => "READY=1",
            State::Reloading => "RELOADING=1",
            State::Stopping => "STOPPING=1",
        }
    }
}

/// Where this process's state changes go.
///
/// A seam rather than the datagram itself, so that what a boot, a reload and a
/// stop report is what a case reads back — a service manager is not something
/// a test may assume it is running under.
pub(crate) trait Supervisor: std::fmt::Debug + Send + Sync {
    /// Say that this process is now in `state`.
    ///
    /// Nothing is returned, because there is nothing a caller could do with a
    /// failure: a server that refused to serve because the manager that started
    /// it stopped listening would be an outage bought for a status line. An
    /// implementation reports its own.
    fn told(&self, state: State);
}

/// The service manager this process reports to, or nobody.
///
/// Cheap to clone and held by each half that has something to report — the boot
/// here, the reload in [`crate::control`] and the stop in [`crate::stop`].
#[derive(Clone, Debug)]
pub(crate) struct Notify(Option<Arc<dyn Supervisor>>);

/// What [`Notify::install`] left, for the reporter that is handed nothing.
static PROCESS: OnceLock<Notify> = OnceLock::new();

impl Notify {
    /// The manager `$NOTIFY_SOCKET` names, or silence where the variable is
    /// unset — which is every process an operator started by hand, and every
    /// process at all on Windows, where a service's state is the SCM's and not
    /// a datagram's.
    pub(crate) fn from_env() -> Self {
        #[cfg(unix)]
        {
            match std::env::var_os("NOTIFY_SOCKET") {
                Some(name) if !name.is_empty() => systemd::manager(&name),
                _ => Self::silent(),
            }
        }
        #[cfg(not(unix))]
        {
            Self::silent()
        }
    }

    /// A process that reports to nobody.
    pub(crate) fn silent() -> Self {
        Self(None)
    }

    /// A process that reports to `sink`.
    #[cfg(any(unix, test))]
    pub(crate) fn to(sink: Arc<dyn Supervisor>) -> Self {
        Self(Some(sink))
    }

    /// Report `state`, or do nothing at all.
    pub(crate) fn state(&self, state: State) {
        if let Some(sink) = &self.0 {
            sink.told(state);
        }
    }

    /// Keep this as the process's, for the one reporter that is handed nothing:
    /// a terminating signal's thread is given no boot and no server, which is
    /// why it takes this and `nvs_server::Draining::process()` the same way.
    ///
    /// Installed once — a second call keeps the first, because a process has
    /// one manager for its whole life.
    pub(crate) fn install(&self) {
        drop(PROCESS.set(self.clone()));
    }

    /// What [`Notify::install`] left, or silence before a boot installed
    /// anything.
    pub(crate) fn process() -> Self {
        PROCESS.get().cloned().unwrap_or_else(Self::silent)
    }
}

/// A [`Notify`] whose reports a case reads back, and the log it writes them to.
///
/// What it records is the line each state would have sent, so a case asserts
/// the protocol rather than the spelling of an enum.
#[cfg(test)]
pub(crate) fn recording() -> (Notify, Arc<std::sync::Mutex<Vec<&'static str>>>) {
    #[derive(Debug)]
    struct Recorder(Arc<std::sync::Mutex<Vec<&'static str>>>);

    impl Supervisor for Recorder {
        fn told(&self, state: State) {
            self.0
                .lock()
                .expect("the recorded states are only taken here")
                .push(state.line());
        }
    }

    let log = Arc::new(std::sync::Mutex::new(Vec::new()));
    (Notify::to(Arc::new(Recorder(Arc::clone(&log)))), log)
}

#[cfg(unix)]
mod systemd {
    //! `sd_notify` as the protocol is: one `AF_UNIX` datagram of `NAME=value`
    //! lines to the address `$NOTIFY_SOCKET` holds.

    use std::ffi::OsStr;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::net::{SocketAddr, UnixDatagram};
    use std::path::Path;
    use std::sync::Arc;

    use super::{Notify, State, Supervisor};

    /// The manager listening at `name`, or silence and one line saying why.
    ///
    /// A boot that cannot reach the manager that started it will be stopped at
    /// `TimeoutStartSec` with no explanation of its own, so the only useful
    /// place to say what went wrong is here.
    pub(super) fn manager(name: &OsStr) -> Notify {
        match opened(name) {
            Ok(manager) => Notify::to(Arc::new(manager)),
            Err(error) => {
                eprintln!(
                    "warning: `$NOTIFY_SOCKET` names `{}`, and this process cannot report its \
                     state to it: {error}",
                    Path::new(name).display()
                );
                Notify::silent()
            }
        }
    }

    /// The socket and the address, both taken once: a report is then a `send`
    /// and nothing else.
    fn opened(name: &OsStr) -> io::Result<Manager> {
        Ok(Manager {
            socket: UnixDatagram::unbound()?,
            at: address(name)?,
        })
    }

    /// `$NOTIFY_SOCKET` as an address: a path, or — where it begins with `@` —
    /// a name in Linux's abstract namespace, which is the same socket with no
    /// file to find it by.
    fn address(name: &OsStr) -> io::Result<SocketAddr> {
        if let [b'@', rest @ ..] = name.as_bytes() {
            #[cfg(target_os = "linux")]
            {
                use std::os::linux::net::SocketAddrExt;
                return SocketAddr::from_abstract_name(rest);
            }
            #[cfg(not(target_os = "linux"))]
            {
                drop(rest);
                return Err(io::Error::other(
                    "an abstract socket name is Linux's, and this platform has no namespace to \
                     look one up in",
                ));
            }
        }
        SocketAddr::from_pathname(Path::new(name))
    }

    /// One datagram per report, over a socket held for the life of the process.
    ///
    /// A report is rare — the boot's, a reload's pair, the stop's — so what
    /// this holds is one descriptor rather than one per report, and nothing
    /// that grows with either.
    #[derive(Debug)]
    struct Manager {
        /// Unbound, because `sd_notify` is one-way: a manager never answers,
        /// and a socket with a name of its own would be a file in a directory
        /// this process was not told it may write to.
        socket: UnixDatagram,
        /// Where `$NOTIFY_SOCKET` pointed when this process started.
        at: SocketAddr,
    }

    impl Supervisor for Manager {
        fn told(&self, state: State) {
            if let Err(error) = self
                .socket
                .send_to_addr(message(state).as_bytes(), &self.at)
            {
                eprintln!(
                    "warning: the service manager was not told `{}`: {error}",
                    state.line()
                );
            }
        }
    }

    /// The datagram's whole text, one assignment per line.
    ///
    /// A reload carries `MONOTONIC_USEC` as well: it is the clock reading a
    /// manager matches a reload it asked for against, and a `RELOADING` with
    /// none can be read as the answer to a request that had not been made when
    /// it was sent. A clock this platform will not read is left off rather than
    /// guessed at — the state itself is still the honest half of the message.
    fn message(state: State) -> String {
        match state {
            State::Reloading => match monotonic_usec() {
                Some(usec) => format!("{}\nMONOTONIC_USEC={usec}\n", state.line()),
                None => format!("{}\n", state.line()),
            },
            _ => format!("{}\n", state.line()),
        }
    }

    /// `CLOCK_MONOTONIC` in microseconds, which is the clock the protocol
    /// names. `Instant` is the same reading with no way to ask for its value.
    #[expect(
        unsafe_code,
        reason = "`clock_gettime` is the platform's only reading of `CLOCK_MONOTONIC`, and \
                  `std` hands back an opaque `Instant` instead of one"
    )]
    fn monotonic_usec() -> Option<u64> {
        // SAFETY: `timespec` is a plain C struct whose all-zero value is a
        // valid one, and the call below overwrites every field of it.
        let mut now: libc::timespec = unsafe { std::mem::zeroed() };
        // SAFETY: the platform writes one `timespec`, which this frame owns.
        if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &raw mut now) } != 0 {
            return None;
        }
        let seconds = u64::try_from(now.tv_sec).ok()?;
        let nanos = u64::try_from(now.tv_nsec).ok()?;
        seconds.checked_mul(1_000_000)?.checked_add(nanos / 1_000)
    }
}

/// Where a rendered unit goes, which for [`Delivery::Print`] is nowhere.
///
/// `root` is the system unit directory rather than a constant, so the printing
/// half of § 5 can be asserted against a directory a test owns and can then
/// find still empty.
pub(crate) fn destination(delivery: Delivery, name: &str, root: &Path) -> Option<PathBuf> {
    match delivery {
        Delivery::Print => None,
        Delivery::Install => Some(root.join(format!("{name}.service"))),
    }
}

/// Deliver a rendered unit to wherever [`destination`] said, which for `None`
/// is standard output.
///
/// # Errors
///
/// The write's own error.
pub(crate) fn deliver(text: &str, destination: Option<&Path>) -> std::io::Result<()> {
    match destination {
        None => {
            print!("{text}");
            Ok(())
        }
        Some(path) => std::fs::write(path, text),
    }
}

/// `nvs service unit <name> [options] -- <argv…>` — print what would be
/// installed, and install nothing.
///
/// § 5 on Linux, and its `--print` on Windows: the same generation, the same
/// § 2 refusals in front of it, and the artifact a change-management review
/// actually wants rather than one a binary wrote behind the operator's
/// configuration management.
pub(crate) fn print_unit(
    config: &[PathBuf],
    name: &str,
    argv: &[String],
    log_file: Option<&Path>,
    account: Option<&str>,
    password: Option<&str>,
) -> ExitCode {
    let request = Request {
        name,
        argv,
        log_file,
        account,
        password,
    };
    let mut sources = SourceMap::new();
    let host = match describe_host(config, argv, &mut sources) {
        Ok(host) => host,
        Err(diagnostic) => return refuse(diagnostic, &mut sources),
    };
    let plan = match plan(&request, &host) {
        Ok(plan) => plan,
        Err(diagnostic) => return refuse(diagnostic, &mut sources),
    };

    let text = if cfg!(windows) {
        format!(
            "New-Service -Name {} -BinaryPathName '{}' -StartupType Automatic\n",
            plan.name,
            image_path(&plan)
        )
    } else {
        unit(&plan)
    };
    // Through the same seam an install will use, rather than around it: the
    // printing delivery is the one that has no destination, and routing this
    // command past `destination` would leave that a comment instead of the
    // thing that decides.
    let destination = destination(Delivery::Print, &plan.name, Path::new(UNIT_DIRECTORY));
    if let Err(error) = deliver(&text, destination.as_deref()) {
        eprintln!("error: could not write the unit: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// What this process and the named configuration answer about themselves.
///
/// The configuration questions are read off the merged table rather than the
/// typed tree, because each is one key and the tree would have to be
/// deserialized in full to reach them.
fn describe_host(
    config: &[PathBuf],
    argv: &[String],
    sources: &mut SourceMap,
) -> Result<Host, Diagnostic> {
    let exe = std::env::current_exe()
        .map_err(|error| {
            Diagnostic::error(
                code::E_SERVICE_PATH_NOT_ABSOLUTE,
                format!("could not locate this executable: {error}"),
            )
            .with_note("§ 3 stores an absolute path and nothing else".to_owned())
        })?
        .clone();

    // The argv's own `--config` is the tree the *service* will read, so it is
    // the tree these three questions are asked of — not whatever `nvs service`
    // was itself pointed at, which would answer for this shell instead.
    let named: Vec<PathBuf> = values_of(argv, "--config")
        .into_iter()
        .map(PathBuf::from)
        .collect();
    let files = crate::config::LocalFiles;
    let roots = crate::config::named_roots(config, &named);
    let cwd = crate::config::working_directory()?;
    let table = nvs_config::resolve::resolve(
        &nvs_config::resolve::roots(&roots, &cwd, &files),
        sources,
        &files,
    )
    .map(|resolved| resolved.table)
    .unwrap_or_default();

    let target = table
        .get("log")
        .and_then(toml::Value::as_table)
        .and_then(|log| log.get("target"))
        .and_then(toml::Value::as_str);
    let control_socket = table
        .get("control")
        .and_then(toml::Value::as_table)
        .and_then(|control| control.get("socket"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    let memory_max = table
        .get("limits")
        .and_then(toml::Value::as_table)
        .and_then(|limits| limits.get("memory"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    let listen = table
        .get("server")
        .and_then(toml::Value::as_table)
        .and_then(|server| server.get("listen"));

    Ok(Host {
        exe,
        from_a_bundle: crate::bundle::embedded().is_some(),
        config_names_a_log_destination: target
            .is_some_and(|target| target.starts_with("file:") || target == "syslog"),
        control_socket,
        memory_max,
        privileged_port: listen.is_some_and(privileged),
    })
}

/// Whether a `[server] listen` value — one address or a list of them — names a
/// port below 1024, which is § 5's one condition for granting a capability.
fn privileged(listen: &toml::Value) -> bool {
    match listen {
        toml::Value::String(address) => address
            .rsplit_once(':')
            .and_then(|(_, port)| port.parse::<u16>().ok())
            .is_some_and(|port| port < 1024),
        toml::Value::Array(addresses) => addresses.iter().any(privileged),
        _ => false,
    }
}

/// A refusal, rendered as the diagnostic it is.
fn refuse(diagnostic: Diagnostic, sources: &mut SourceMap) -> ExitCode {
    let mut diags = Diagnostics::new();
    diags.report(diagnostic);
    render_diagnostics(&mut diags, sources);
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The code a refusal carries. § 2's closing sentence is that each refusal
    /// is a diagnostic naming what was refused and why, never a bare non-zero
    /// exit, so an uncoded one is a failure of that rule rather than of the
    /// case reading it.
    fn coded(refusal: &Diagnostic) -> nvs_diagnostics::Code {
        refusal.code.expect("every § 2 refusal carries a code")
    }

    /// A path that is absolute on the host these tests run on.
    ///
    /// `Path::is_absolute` answers for the platform, and § 2's third row is a
    /// claim about the machine the service will start on, so a fixture that
    /// hardcoded a Unix root would assert nothing on Windows and vice versa.
    fn absolute(tail: &str) -> String {
        if cfg!(windows) {
            format!("C:\\nvs\\{}", tail.replace('/', "\\"))
        } else {
            format!("/etc/nvs/{tail}")
        }
    }

    /// A host with nothing wrong with it, so a refusal in a case below is the
    /// one that case is about.
    fn host() -> Host {
        Host {
            exe: PathBuf::from(absolute("bin/nvs")),
            from_a_bundle: false,
            config_names_a_log_destination: true,
            control_socket: Some(absolute("run/control.sock")),
            memory_max: Some("512M".to_owned()),
            privileged_port: false,
        }
    }

    /// An argv that survives § 2 whole.
    fn argv() -> Vec<String> {
        vec![
            "serve".to_owned(),
            absolute("app/index.nvs"),
            "--config".to_owned(),
            absolute("nvs.toml"),
        ]
    }

    fn request<'a>(argv: &'a [String]) -> Request<'a> {
        Request {
            name: "web",
            argv,
            log_file: None,
            account: None,
            password: None,
        }
    }

    #[test]
    fn a_request_that_survives_every_refusal_plans() {
        let argv = argv();
        assert!(plan(&request(&argv), &host()).is_ok());
    }

    /// `rule:packaging/the-installer-is-a-sink`, row 1 and row 2 — one code, because the reason is one:
    /// what the argv names has to keep running and must carry no testing hook.
    #[test]
    fn the_installer_refuses_a_subcommand_outside_the_serve_and_run_allowlist() {
        for subcommand in [
            "ast", "check", "test", "fmt", "info", "config", "ctl", "service",
        ] {
            let mut argv = argv();
            argv[0] = subcommand.to_owned();
            let refusal = plan(&request(&argv), &host()).expect_err(subcommand);
            assert_eq!(coded(&refusal), code::E_SERVICE_ARGV_NOT_ALLOWED);
            assert!(refusal.message.contains(subcommand), "{subcommand}");
        }

        // Both spellings of the hook, on a subcommand that *is* allowed — so
        // the refusal is the flag's and not the allowlist's reached twice.
        for hook in ["--fault-inject", "--fault-inject=helper-panic"] {
            let mut argv = argv();
            argv.push(hook.to_owned());
            let refusal = plan(&request(&argv), &host()).expect_err(hook);
            assert_eq!(coded(&refusal), code::E_SERVICE_ARGV_NOT_ALLOWED);
            assert!(refusal.message.contains("--fault-inject"), "{hook}");
        }

        // The empty argv is the same claim with nothing to name.
        assert_eq!(
            coded(&plan(&request(&[]), &host()).expect_err("empty")),
            code::E_SERVICE_ARGV_NOT_ALLOWED
        );
    }

    /// `rule:packaging/the-installer-is-a-sink`, row 3 and the `--config` row — the same first-boot
    /// failure, one of them one step less visible.
    #[test]
    fn the_installer_refuses_a_relative_path() {
        // In the argv's `--config`, in both spellings.
        for written in ["--config nvs.toml", "--config=nvs.toml"] {
            let argv: Vec<String> = format!("serve {} {written}", absolute("app/index.nvs"))
                .split(' ')
                .map(str::to_owned)
                .collect();
            let refusal = plan(&request(&argv), &host()).expect_err(written);
            assert_eq!(coded(&refusal), code::E_SERVICE_PATH_NOT_ABSOLUTE);
            assert!(refusal.message.contains("nvs.toml"), "{written}");
        }

        // As the entry file, with an absolute `--config` beside it, so the
        // walk is shown reading past the option's value rather than stopping
        // at the first bare word it sees.
        let entry = vec![
            "serve".to_owned(),
            "--config".to_owned(),
            absolute("nvs.toml"),
            "app/index.nvs".to_owned(),
        ];
        let refusal = plan(&request(&entry), &host()).expect_err("entry");
        assert_eq!(coded(&refusal), code::E_SERVICE_PATH_NOT_ABSOLUTE);
        assert!(refusal.message.contains("app/index.nvs"));

        // And in the installer's own option, which § 2 names beside the argv.
        let good = argv();
        let mut asked = request(&good);
        let relative = PathBuf::from("nvs.log");
        asked.log_file = Some(&relative);
        assert_eq!(
            coded(&plan(&asked, &host()).expect_err("log")),
            code::E_SERVICE_PATH_NOT_ABSOLUTE
        );

        // An argv with no `--config` at all is the same code: the fallback to
        // `./nvs.toml` is a relative path the operator never wrote.
        let bare = vec!["serve".to_owned(), absolute("app/index.nvs")];
        let refusal = plan(&request(&bare), &host()).expect_err("no config");
        assert_eq!(coded(&refusal), code::E_SERVICE_PATH_NOT_ABSOLUTE);
        assert!(refusal.message.contains("--config"));

        // The control: the same shapes, absolute, plan.
        assert!(plan(&request(&argv()), &host()).is_ok());
    }

    /// `rule:packaging/the-installer-is-a-sink`, row 5 — and the refusal is by name, so an operator is not
    /// left believing they mistyped a flag.
    #[test]
    fn the_installer_refuses_a_password_on_a_command_line() {
        let argv = argv();
        let mut asked = request(&argv);
        asked.account = Some("EXAMPLE\\nvs-web");
        asked.password = Some("hunter2");
        let refusal = plan(&asked, &host()).expect_err("password");
        assert_eq!(coded(&refusal), code::E_SERVICE_PASSWORD_ON_A_COMMAND_LINE);
        // The value itself is never echoed back — it is `secret` for its whole
        // life under `rule:security/secret-qualifier`, and a diagnostic quoting it would put it in a
        // second place.
        assert!(!refusal.message.contains("hunter2"));

        // The account alone is not a refusal: § 4 has it take a domain
        // identity for a deployment that needs one.
        let mut named = request(&argv);
        named.account = Some("EXAMPLE\\nvs-web");
        assert!(plan(&named, &host()).is_ok());
    }

    /// `rule:packaging/the-installer-is-a-sink`, row 4 and § 4's *Output*.
    #[test]
    fn the_installer_refuses_an_install_whose_output_would_go_nowhere() {
        let argv = argv();
        let mut nowhere = host();
        nowhere.config_names_a_log_destination = false;
        assert_eq!(
            coded(&plan(&request(&argv), &nowhere).expect_err("no destination")),
            code::E_SERVICE_OUTPUT_GOES_NOWHERE
        );

        // Either answer satisfies it, and nothing else does: `--log-file`…
        let mut logged = request(&argv);
        let path = PathBuf::from(absolute("log/web.log"));
        logged.log_file = Some(&path);
        assert!(plan(&logged, &nowhere).is_ok());

        // …or a `[log] target` the process can actually reach. `stderr` is
        // not one: a service has no console handle, which is the whole reason
        // this row exists.
        for target in ["file:/var/log/nvs.log", "syslog"] {
            let mut host = host();
            host.config_names_a_log_destination = target.starts_with("file:") || target == "syslog";
            assert!(plan(&request(&argv), &host).is_ok(), "{target}");
        }
    }

    /// `rule:packaging/a-bundle-may-not-install-itself`.
    #[test]
    fn the_installer_refuses_to_install_from_a_bundle() {
        let argv = argv();
        let mut bundled = host();
        bundled.from_a_bundle = true;
        let refusal = plan(&request(&argv), &bundled).expect_err("bundle");
        assert_eq!(coded(&refusal), code::E_SERVICE_FROM_A_BUNDLE);

        // It is refused first, before anything about the argv is looked at:
        // § 6's answer does not depend on what the payload would have run.
        let mut wrong = argv;
        wrong[0] = "ast".to_owned();
        assert_eq!(
            coded(&plan(&request(&wrong), &bundled).expect_err("bundle first")),
            code::E_SERVICE_FROM_A_BUNDLE
        );
    }

    /// `rule:packaging/the-argv-lives-in-imagepath`: the SCM stores one string, the process gets it back
    /// through `CommandLineToArgvW`, and that string is the only record of
    /// what the service runs — so the encoder's round trip is the property,
    /// and the quoting of the binary is unconditional.
    #[test]
    fn a_windows_image_path_is_quoted_and_absolute() {
        let corpus: Vec<Vec<String>> = vec![
            argv(),
            // A trailing backslash immediately before the closing quote, which
            // is the case a hand-written joiner gets wrong.
            vec![
                "serve".to_owned(),
                "--config".to_owned(),
                absolute("app dir\\"),
            ],
            // An embedded quote, and a backslash run in front of one.
            vec![
                "run".to_owned(),
                absolute("app/index.nvs"),
                "--config".to_owned(),
                absolute("nvs.toml"),
                "a\"b".to_owned(),
                "c\\\\\"d".to_owned(),
            ],
            // A space, an empty word, and a lone backslash.
            vec![
                "serve".to_owned(),
                absolute("app/index.nvs"),
                "--config".to_owned(),
                absolute("with space/nvs.toml"),
                String::new(),
                "\\".to_owned(),
            ],
        ];

        for argv in corpus {
            let plan = Plan {
                name: "web".to_owned(),
                exe: PathBuf::from(absolute("bin/nvs")),
                argv: argv.clone(),
                account: None,
                control_socket: None,
                memory_max: None,
                privileged_port: false,
            };
            let line = image_path(&plan);

            // Quoted unconditionally — the path here contains no space.
            assert!(line.starts_with('"'), "{line}");
            let decoded = decode(&line);
            let (image, rest) = decoded.split_first().expect("an image path");
            assert!(Path::new(image).is_absolute(), "{image}");
            assert_eq!(image, &absolute("bin/nvs"));
            assert_eq!(rest, argv.as_slice(), "{line}");
        }
    }

    /// `rule:packaging/the-unit-is-printed-and-install-is-the-opt-in`: `nvs service unit` writes the unit to stdout and touches
    /// nothing, and installing it is the explicit request rather than the
    /// default.
    #[test]
    fn a_systemd_unit_is_printed_and_written_only_on_install() {
        let plan = Plan {
            name: "web".to_owned(),
            exe: PathBuf::from(absolute("bin/nvs")),
            argv: argv(),
            account: Some("nvs-web".to_owned()),
            control_socket: Some(absolute("run/control.sock")),
            memory_max: Some("512M".to_owned()),
            privileged_port: false,
        };
        let text = unit(&plan);
        for line in [
            "Type=notify",
            "WatchdogSec=30",
            "User=nvs-web",
            "MemoryMax=512M",
            "NoNewPrivileges=true",
            "ProtectSystem=strict",
            "ProtectHome=true",
            "PrivateTmp=true",
            "CapabilityBoundingSet=",
            "RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX",
            "SystemCallFilter=@system-service",
        ] {
            assert!(text.contains(line), "missing `{line}` in\n{text}");
        }
        assert!(text.contains(&format!("ExecStart={} serve", absolute("bin/nvs"))));
        assert!(text.contains(" ctl reload --socket "));

        // § 5's capability condition: nothing is granted unless a privileged
        // port is configured, asserted on both sides so a generator that
        // always emitted it would fail here rather than read plausibly.
        assert!(!text.contains("AmbientCapabilities"));
        let mut privileged = plan;
        privileged.privileged_port = true;
        assert!(unit(&privileged).contains("AmbientCapabilities=CAP_NET_BIND_SERVICE"));

        // Printed, and written only on install: the printing delivery has no
        // destination at all, so a directory it was pointed at is still empty
        // afterwards, while the installing one names the unit file.
        let root = std::env::temp_dir().join(format!("nvs-service-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("a scratch directory");
        assert_eq!(destination(Delivery::Print, "web", &root), None);
        deliver(&text, destination(Delivery::Print, "web", &root).as_deref())
            .expect("the printing delivery");
        assert_eq!(
            std::fs::read_dir(&root).expect("readable").count(),
            0,
            "printing wrote into {}",
            root.display()
        );

        let installed = destination(Delivery::Install, "web", &root).expect("a destination");
        assert_eq!(installed, root.join("web.service"));
        deliver(&text, Some(&installed)).expect("the installing delivery");
        assert_eq!(std::fs::read_to_string(&installed).expect("written"), text);
        std::fs::remove_dir_all(&root).expect("a removed scratch directory");
    }
}
