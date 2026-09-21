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
//! **Every verb is a plan of actions, and [`registration`] builds them.**
//! `install`, `uninstall`, `start`, `stop` and `status` are each a pure
//! function from a [`Plan`] to a list of [`registration::Action`]s — § 3's
//! registration, § 4's `PRESHUTDOWN`, failure actions, grants and event-log
//! source, or § 5's unit write and `daemon-reload` — applied in order through
//! one [`registration::Manager`]. A case drives the recording manager and
//! asserts the list, so every decision those sections take is held to with no
//! administrator rights and on either platform.
//!
//! **[`registration::scm::Scm`] applies the Windows half** — the SCM for a
//! registration and a control, the registry for the event-log source, the file
//! ACLs for a grant — and no case reaches it, because a real registration needs
//! administrator rights on a machine somebody chose. That is why 0093's own
//! *Verification* lists one among the end-to-end checks rather than here.
//! [`registration::Systemd`] applies the Linux half and is compiled everywhere,
//! for the reason [`registration::Platform`] is a parameter.
//!
//! **[`at_host`] is the one `cfg` in the surface.** A list of actions is the
//! same value on either machine, which is what makes every verb assertable
//! from either one — but the manager that *performs* one is this machine's real
//! SCM or its real `systemctl`, and only one of those exists to be named here.
//!
//! **An uninstall reads what it is undoing back from the platform**
//! ([`registration::Manager::stored`]): the argv out of § 3's `ImagePath` or
//! the unit's `ExecStart`, and the account beside it. § 4's closing property is
//! about the install that happened, and a second command line describes the one
//! the operator believes happened. The two directories § 4 grants read/write on
//! are the residue — neither manager holds them, so they are derived again from
//! the configuration the stored argv names, which is where the install derived
//! them from. An install given the installer's own `--log-file` over a
//! configuration that names no `file:` destination is the case that derivation
//! cannot reach, and it is `run`'s to close: that binding needs the same value
//! recoverable from what the platform stores.
//!
//! # The manager is told what state this process is in
//!
//! The `Type=notify` line [`unit()`] renders is a promise, and [`Notify`] is
//! what keeps it: a unit of that type whose process never sends `READY=1` is
//! one systemd ends at `TimeoutStartSec` however well it is serving.
//! `rule:packaging/the-generated-unit-is-hardened` is the list — `READY=1` once
//! every listener is bound, `RELOADING=1` and `READY=1` around a reload,
//! `STOPPING=1` once the drain has begun, and a `WATCHDOG=1` ping for as long
//! as the fleet is turning.
//!
//! **The ping is the `WatchdogSec=` line's promise, and it is gated rather than
//! timed.** [`Heartbeat`] owns what it is gated on and why one wedged core does
//! not withhold it; the short of it is that a beat a live thread writes proves
//! only that the process exists, which is not what the manager is asking.
//!
//! **And the manager's own controls are answered with the operations that
//! already exist.** [`hosted`] is that half of
//! `rule:packaging/a-service-answers-its-manager`: a `STOP` or a `PRESHUTDOWN`
//! enters [`crate::stop`]'s drain and is reported with a checkpoint that
//! advances while requests finish, and a `PARAMCHANGE` enters
//! [`crate::control`]'s one reload function. What is missing is the dispatcher
//! that hands it a control — the SCM's own, which only a process that manager
//! started has.
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
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

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
    /// The file a `[log] target` of `file:<path>` names, whose **directory** is
    /// one of § 4's grants. The installer's own `--log-file` is the other
    /// answer to the same row, and the one an uninstall cannot read back.
    pub(crate) log_file: Option<PathBuf>,
    /// `[opcache] file_cache_dir`, the artifact cache § 4 grants read/write on.
    /// Derived rather than asked for, so an uninstall names the same directory
    /// the install granted.
    pub(crate) cache_directory: Option<PathBuf>,
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
    /// [`registration::install_actions`] builds that write out of it, and
    /// [`at_host`] names the applier that performs one.
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
    all(not(test), not(windows)),
    expect(
        dead_code,
        reason = "the round-trip property's half, and the SCM read-back's on Windows"
    )
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

/// An `ExecStart` line read back into the words [`shell_word`] wrote it from,
/// which is how a Linux uninstall learns what it is undoing.
///
/// [`decode`] is the same half of the Windows round trip, and the two are
/// separate because the encodings are: `CommandLineToArgvW`'s backslash rule
/// has nothing to do with systemd's, and one function serving both would be a
/// third encoding neither platform reads.
#[cfg_attr(
    all(test, windows),
    expect(
        dead_code,
        reason = "the applier that reads a unit back is `Systemd`, which on Windows only a case \
                  could reach and a case drives the recording manager"
    )
)]
fn shell_words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut started = false;
    for character in line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' if quoted => escaped = true,
            '"' => {
                quoted = !quoted;
                started = true;
            }
            ' ' | '\t' if !quoted => {
                if started {
                    out.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            _ => {
                current.push(character);
                started = true;
            }
        }
    }
    if started {
        out.push(current);
    }
    out
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
    /// The fleet is still turning, which is the only thing a `WatchdogSec=`
    /// unit takes as evidence that this process is doing its job.
    ///
    /// A keepalive rather than a transition, so it is sent over and over while
    /// the others are sent once each — and it is a [`State`] anyway, because
    /// what a case has to be able to read back is the line, and the seam that
    /// hands a case the other three is the one holding [`Heartbeat`] up.
    Alive,
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
            State::Alive => "WATCHDOG=1",
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

/// What a service manager asks of a hosted server, answered with the operations
/// this process already has.
///
/// `rule:packaging/a-service-answers-its-manager`'s table is the whole of what
/// is here, and the rule's own wording is why nothing in it is re-implemented:
/// a hosted server answers with the operations it already has, *rather than a
/// shim reporting what it can see from outside*. So a stop enters
/// [`crate::stop::deliver_to`], the one drain a `SIGTERM` enters, and a
/// `PARAMCHANGE` enters [`nvs_server::control::Controlled::reload`], the one
/// reload every other spelling ends in ([`crate::control`]). What is left for
/// this module is the mapping, and watching the drain on the manager's behalf.
///
/// **The mapping is a value on both platforms.** A control arrives as one of
/// the SCM's ABI numbers, and those are matched here rather than behind a `cfg`
/// for [`registration::Platform`]'s reason: a case that could only be written
/// on the machine it describes is a case nobody runs, and a `PARAMCHANGE`
/// mapped to the wrong operation is silent until an administrator reloads a
/// machine somebody else owns.
///
/// What it spends, as `rule:programs/memory-priority` requires: one borrowed
/// thread for the length of a drain, and no allocation — nothing per request
/// and nothing that outlives the stop.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the caller is `nvs serve` registering this with the SCM, which is the dispatcher \
                  half; a case drives every item here"
    )
)]
pub(crate) mod hosted {
    use std::time::{Duration, Instant};

    use nvs_server::Draining;
    use nvs_server::control::Controlled;

    /// `SERVICE_CONTROL_STOP`.
    const STOP: u32 = 0x0000_0001;
    /// `SERVICE_CONTROL_PARAMCHANGE`.
    const PARAMCHANGE: u32 = 0x0000_0006;
    /// `SERVICE_CONTROL_PRESHUTDOWN`, which § 4 asks for at install because
    /// plain `SHUTDOWN` allows roughly five seconds and a drain needs more.
    const PRESHUTDOWN: u32 = 0x0000_000F;

    /// What a service manager asked for, and the whole set this process
    /// answers.
    ///
    /// Two, where the rule's table has three rows: `PRESHUTDOWN` and `STOP` ask
    /// this process for the same thing, and what differs between them — how
    /// long the machine agrees to wait — is settled at install by
    /// [`super::registration::Action::Preshutdown`] rather than here.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub(crate) enum Asked {
        /// Drain: answer what was accepted, accept nothing further, and say so
        /// while it happens.
        Stop,
        /// Re-read the configuration in-process, which `systemctl reload` is
        /// the other spelling of.
        Reload,
    }

    impl Asked {
        /// The control `code` names, or `None` where it names none.
        ///
        /// `None` is not a failure. The SCM sends `SERVICE_CONTROL_INTERROGATE`
        /// to any service at all, and what a handler owes it is the status it
        /// is already holding — so a caller's answer to everything outside this
        /// set is to say again what it last said.
        pub(crate) fn of(code: u32) -> Option<Self> {
            match code {
                STOP | PRESHUTDOWN => Some(Self::Stop),
                PARAMCHANGE => Some(Self::Reload),
                _ => None,
            }
        }
    }

    /// One thing the service manager is told while a drain runs.
    ///
    /// A reload has no member here, because the SCM has no state for one and
    /// the pair a `Type=notify` unit is owed is reported by the reload function
    /// itself ([`super::State`]) however the reload was asked for.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub(crate) enum Progress {
        /// The drain is running. `checkpoint` is the number the SCM reads as
        /// progress and `in_flight` is what it is advancing over.
        Stopping {
            /// Advanced on every report, and never repeated: a stop that stops
            /// advancing this is one the machine stops waiting for.
            checkpoint: u32,
            /// Requests still being answered at the moment of the report.
            in_flight: usize,
        },
        /// The drain is finished and this process may exit.
        Stopped,
    }

    /// Where a hosted server's drain progress goes.
    ///
    /// A seam for [`super::Supervisor`]'s reason and one more: the only
    /// implementation there can be is `SetServiceStatus` over a
    /// `SERVICE_STATUS_HANDLE` the SCM hands a process it started itself, so a
    /// case holding this surface to the rule's table could otherwise not be
    /// written on any machine at all.
    pub(crate) trait Reporting: std::fmt::Debug + Send + Sync {
        /// Say that the drain has reached `progress`.
        ///
        /// Nothing is returned for [`super::Supervisor::told`]'s reason: a
        /// drain that stopped because the manager stopped listening would be
        /// requests dropped to buy a status line.
        fn told(&self, progress: Progress);
    }

    /// How often a drain is reported, and how long it is watched for.
    ///
    /// The caller's, because the period a drain takes is `[server]
    /// drain_timeout` resolved out of the tree this process is serving
    /// (`nvs_config::Waits`), and a case may not wait one.
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct Pace {
        /// The period between two checkpoints.
        pub(crate) poll: Duration,
        /// The period after which the drain is reported finished whatever is
        /// still in flight.
        pub(crate) bound: Duration,
    }

    /// Answer `asked`, over the process it is about.
    ///
    /// Called on a thread of its own: the SCM ends a handler that does not
    /// return promptly, and a drain is bounded by `[server] drain_timeout`
    /// rather than by anything that fast.
    pub(crate) fn answer(
        asked: Asked,
        process: &dyn Controlled,
        draining: &Draining,
        manager: &dyn Reporting,
        pace: Pace,
    ) {
        match asked {
            // Nothing is done with the outcome, and that is the rule's own
            // arrangement rather than a dropped error: what a reload could not
            // apply is named by the reload function's report
            // (`rule:config/a-reload-names-what-it-could-not-apply`), and a
            // second account of it here would be a second place for it to be
            // wrong in.
            Asked::Reload => drop(process.reload()),
            Asked::Stop => stop(process, draining, manager, pace),
        }
    }

    /// The drain, and the manager watching it finish.
    ///
    /// Entered through [`crate::stop::deliver_to`] rather than begun here, so
    /// that a stop arriving from the SCM is the same state machine a
    /// terminating signal enters and reports the same `STOPPING=1`.
    ///
    /// Reaching [`Pace::bound`] still ends in [`Progress::Stopped`]. By then
    /// `drain_timeout` has closed what it was going to close, and a process
    /// reporting anything else would be one the machine waits out the rest of
    /// § 4's `PRESHUTDOWN` for before killing it anyway.
    fn stop(process: &dyn Controlled, draining: &Draining, manager: &dyn Reporting, pace: Pace) {
        crate::stop::deliver_to(draining);
        let began = Instant::now();
        let mut checkpoint = 0;
        loop {
            let in_flight = process.in_flight();
            checkpoint += 1;
            manager.told(Progress::Stopping {
                checkpoint,
                in_flight,
            });
            if in_flight == 0 || began.elapsed() >= pace.bound {
                break;
            }
            std::thread::sleep(pace.poll);
        }
        manager.told(Progress::Stopped);
    }
}

/// The `WATCHDOG=1` ping a `WatchdogSec=` unit is owed, and the thread sending
/// it.
///
/// [`unit()`] renders `WatchdogSec=30`, which means the manager stops this
/// process unless a ping arrives inside half that period — so the ping has to
/// be evidence of something. A beat written by a thread that lives whether or
/// not a core turns proves only that the process exists, and a process that
/// exists while answering nothing is the exact fault a watchdog is bought for.
/// So the gate is [`nvs_host::Watchdog::turning`]:
/// `rule:http-server/a-wedged-core-is-detected-by-its-deadline`'s detector,
/// read rather than written to, which is what keeps this off every request
/// path.
///
/// **One wedged core does not withhold the ping.** Withholding it there would
/// hand the manager a whole process to restart over a fault
/// `rule:http-server/a-wedged-core-is-shed-never-killed` sheds — every healthy
/// core's in-flight requests ended to answer one core's wedge, which spends
/// availability and buys nothing. The ping stops when *no* core is turning,
/// and there a restart is the only recovery left: nothing in this process can
/// kill a thread, and there is no core left to shed the work onto.
///
/// It spends one OS thread, and only under a manager that asked for a watchdog
/// at all.
#[derive(Debug)]
pub(crate) struct Heartbeat {
    shared: Arc<Beating>,
    /// Taken by [`Heartbeat::drop`], which joins it.
    thread: Option<JoinHandle<()>>,
}

/// What the pinging thread and its handle share.
#[derive(Debug)]
struct Beating {
    /// Cleared by [`Heartbeat::drop`].
    beating: Mutex<bool>,
    /// Signalled to end the wait early, so a drop does not wait out an interval
    /// to join.
    change: Condvar,
}

impl Heartbeat {
    /// The ping this process's manager asked for, or `None` where it asked for
    /// none — which is every process an operator started by hand, and every
    /// process at all on Windows.
    pub(crate) fn start(
        told: &Notify,
        turning: impl Fn() -> bool + Send + 'static,
    ) -> Option<Self> {
        // A process reporting to nobody has nothing to ping, and the thread
        // would be an OS thread spent writing datagrams into silence.
        told.0.as_ref()?;
        Self::every(told, half_the_period()?, turning)
    }

    /// The same ping on an interval a caller names, which is what makes the
    /// gate assertable without a manager and without waiting out a real period.
    fn every(
        told: &Notify,
        interval: Duration,
        turning: impl Fn() -> bool + Send + 'static,
    ) -> Option<Self> {
        let shared = Arc::new(Beating {
            beating: Mutex::new(true),
            change: Condvar::new(),
        });
        let ticking = Arc::clone(&shared);
        let told = told.clone();
        match std::thread::Builder::new()
            .name("nvs-watchdog-ping".to_owned())
            .spawn(move || {
                loop {
                    {
                        let beating = ticking
                            .beating
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner);
                        let (beating, _) = ticking
                            .change
                            .wait_timeout(beating, interval)
                            .unwrap_or_else(PoisonError::into_inner);
                        if !*beating {
                            return;
                        }
                    }
                    // Outside the lock, because the gate walks the watched set
                    // and the report writes a datagram: neither is work a drop
                    // waiting to join should be held up by.
                    if turning() {
                        told.state(State::Alive);
                    }
                }
            }) {
            Ok(thread) => Some(Self {
                shared,
                thread: Some(thread),
            }),
            Err(error) => {
                eprintln!(
                    "warning: this process could not start the thread that answers \
                     `WatchdogSec`, so the manager that started it will stop it once the period \
                     is up: {error}"
                );
                None
            }
        }
    }
}

impl Drop for Heartbeat {
    fn drop(&mut self) {
        *self
            .shared
            .beating
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = false;
        self.shared.change.notify_all();
        if let Some(thread) = self.thread.take() {
            // Joined rather than left, so a fleet that has finished cannot be
            // followed by one more ping saying it is turning.
            let _ = thread.join();
        }
    }
}

/// Half of `WATCHDOG_USEC`, or `None` where this process is not the one being
/// watched.
///
/// The two variables are the manager's half of `WatchdogSec=`: the period in
/// microseconds, and — where it set one — the PID it will accept a ping from,
/// which is what keeps a child that inherited the environment from answering
/// for its parent. Half the period is the interval the protocol asks for,
/// because a ping that is late once still lands inside it.
fn half_the_period() -> Option<Duration> {
    let usec: u64 = std::env::var("WATCHDOG_USEC").ok()?.trim().parse().ok()?;
    let ours = match std::env::var("WATCHDOG_PID") {
        Ok(pid) => pid.trim().parse::<u32>().ok()? == std::process::id(),
        Err(_) => true,
    };
    (ours && usec > 0).then(|| Duration::from_micros((usec / 2).max(1)))
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

/// § 3's registration and § 5's write, as a list of actions and the manager
/// they go through.
///
/// **Every verb is a pure function from a [`Plan`] to a list of [`registration::Action`]s**,
/// and the manager that performs them is a parameter. That is what makes this
/// half of the surface assertable with no administrator rights and on either
/// platform: a case drives `Recording` and asserts the list, and the list is
/// where every decision § 3 and § 4 take actually lives — the encoded
/// `ImagePath`, the virtual account, `PRESHUTDOWN`, the failure actions, the
/// grants § 4 closes and the event-log source. `--dry-run` describes that same
/// list rather than a second rendering of it, so the change-management artifact
/// § 5 argues for and the thing that is performed cannot drift apart.
///
/// The appliers `Scm` and `Systemd` arrive at [`registration::Manager`], which is one method
/// rather than one per verb: a [`registration::Action`] already names what to do in its
/// platform's own terms, and a trait with a method per row would grow one every
/// time § 3 or § 5 gains a line.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the subcommands that reach this seam are the slice after it"
    )
)]
pub(crate) mod registration {
    use super::*;

    /// How long § 4's `PRESHUTDOWN` asks the machine to wait for this process.
    ///
    /// Plain `SHUTDOWN` allows roughly five seconds, and a drain finishes the
    /// requests already in flight rather than dropping them, so this is a
    /// ceiling over whatever bound `[server]` places on the drain rather than a
    /// guess at how long one takes.
    const PRESHUTDOWN: Duration = Duration::from_secs(180);

    /// § 4's reset period: how long the service stays up before the manager
    /// stops counting earlier failures against it.
    const FAILURE_RESET: Duration = Duration::from_secs(600);

    /// The `HKEY_LOCAL_MACHINE` key whose subkeys are event-log sources, which
    /// is what § 4's *Output* registers at install and removes at uninstall.
    const EVENT_SOURCES: &str = r"SYSTEM\CurrentControlSet\Services\EventLog\Application";

    /// Which service manager holds the service.
    ///
    /// A parameter rather than a `cfg!`, because § 3's registration and § 5's
    /// unit are each one list of actions, and a case that could only be written
    /// on the platform it describes is a case nobody runs.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub(crate) enum Platform {
        /// The SCM, whose own state is the only representation a service has.
        Windows,
        /// systemd, whose representation is a unit file.
        Linux,
    }

    impl Platform {
        /// The platform the installer is running on, which is the one the
        /// service will run on — the same reading § 2's third row takes paths
        /// against.
        pub(crate) fn host() -> Self {
            if cfg!(windows) {
                Self::Windows
            } else {
                Self::Linux
            }
        }
    }

    /// § 4's failure actions, as the one decision they carry.
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default, clap::ValueEnum)]
    pub(crate) enum Restart {
        /// The manager starts the service again after a failure, which is what
        /// `--restart on-failure` asks for and what an installer that was told
        /// nothing does.
        #[default]
        OnFailure,
        /// A failure leaves the service stopped.
        Never,
    }

    impl Restart {
        /// The word an operator wrote, for the line `--dry-run` prints.
        fn word(self) -> &'static str {
            match self {
                Self::OnFailure => "on-failure",
                Self::Never => "never",
            }
        }
    }

    /// When the manager starts the service after a boot (§ 4).
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default, clap::ValueEnum)]
    pub(crate) enum StartMode {
        /// At boot, with the rest of the automatic services.
        #[default]
        Automatic,
        /// After them, which is § 4's delayed auto-start.
        Delayed,
        /// Only when somebody asks.
        Manual,
    }

    impl StartMode {
        /// The word an operator wrote, for the line `--dry-run` prints.
        fn word(self) -> &'static str {
            match self {
                Self::Automatic => "automatic",
                Self::Delayed => "delayed",
                Self::Manual => "manual",
            }
        }

        /// Whether systemd is asked to want this service at boot, which is what
        /// `[Install] WantedBy` in the rendered unit is there for: on Linux the
        /// two auto-start modes are one `enable`, and the manual one is the
        /// unit sitting there unwanted.
        fn enabled(self) -> bool {
            matches!(self, Self::Automatic | Self::Delayed)
        }
    }

    /// The installer's own options that § 2 has nothing to refuse about, and
    /// the two directories § 4 grants the service's identity access to.
    #[derive(Debug, Default)]
    pub(crate) struct Registration {
        /// § 4's `--start`.
        pub(crate) start: StartMode,
        /// § 4's `--restart`.
        pub(crate) restart: Restart,
        /// § 4's `--depends-on`, for a database that must come up first.
        pub(crate) depends_on: Vec<String>,
        /// What an administrator reads beside the name. The service's own name,
        /// where the operator wrote nothing.
        pub(crate) description: Option<String>,
        /// The installer's `--log-file`, whose **directory** is granted
        /// read/write: a process that may write the file but not the directory
        /// cannot rotate it.
        pub(crate) log_file: Option<PathBuf>,
        /// `[opcache] file_cache_dir`, the artifact cache § 4 grants read/write
        /// on. `[cache]` is `Core\Cache`'s two tiers and holds no directory at
        /// all.
        pub(crate) cache_directory: Option<PathBuf>,
    }

    /// What the platform still holds about an installed service.
    ///
    /// On Windows that is the `ImagePath` and nothing else (§ 3), read back
    /// through [`decode`]; on Linux it is the unit's `ExecStart`. A value
    /// rather than something [`uninstall_actions`] reads for itself, so the
    /// property § 4 closes on — an uninstall leaves no key, no source, no unit
    /// and no granted ACL — is one a case can assert against the very install
    /// that granted them.
    #[derive(Debug)]
    pub(crate) struct Stored {
        /// The identity the manager holds it under.
        pub(crate) name: String,
        /// The stored argv, decoded, which is what names the configuration
        /// files the install granted read on.
        pub(crate) argv: Vec<String>,
        /// The account those grants were made to.
        pub(crate) account: String,
        /// The `--log-file` the install was given, if it was given one.
        pub(crate) log_file: Option<PathBuf>,
        /// The artifact cache directory it granted read/write on.
        pub(crate) cache_directory: Option<PathBuf>,
    }

    /// One step a verb performs, in its platform's own terms.
    ///
    /// A union over both platforms rather than one enum each: the verbs are one
    /// surface and [`Manager`] is one seam, and an applier never meets an action
    /// its platform has no call for, because the builder that produced the list
    /// was told which platform it was building for.
    #[derive(Clone, PartialEq, Eq, Debug)]
    pub(crate) enum Action {
        /// Register the service with the SCM, carrying § 3's encoded
        /// `ImagePath`.
        Register {
            name: String,
            image_path: String,
            account: String,
            start: StartMode,
            depends_on: Vec<String>,
            description: String,
        },
        /// Ask for `SERVICE_CONTROL_PRESHUTDOWN`, and for the time a drain
        /// needs.
        Preshutdown { name: String, timeout: Duration },
        /// Set § 4's failure actions and the period after which the manager
        /// forgets earlier failures.
        Failure {
            name: String,
            restart: Restart,
            reset: Duration,
        },
        /// Grant the service's identity access to one path, and § 4's list
        /// closes what may be granted at all.
        Grant {
            account: String,
            path: PathBuf,
            write: bool,
        },
        /// Register the event-log source § 4's *Output* writes lifecycle
        /// records to, which is a subkey under [`EVENT_SOURCES`].
        EventSource { name: String, exe: PathBuf },
        /// Take the registration away.
        Deregister { name: String },
        /// Take one granted entry away again.
        Revoke { account: String, path: PathBuf },
        /// Remove the event-log source's subkey.
        RemoveEventSource { name: String },
        /// Write § 5's rendered unit where systemd reads it.
        WriteUnit { path: PathBuf, text: String },
        /// Remove it.
        RemoveUnit { path: PathBuf },
        /// Run `systemctl` with these arguments — by argv and with no shell,
        /// per `rule:core-classes/process-is-argv-only`.
        Systemctl { argv: Vec<String> },
        /// Ask the SCM itself to start, stop or report on the service.
        Scm { name: String, control: Control },
    }

    /// The three verbs that act on an installed service rather than on its
    /// registration.
    ///
    /// `rule:packaging/a-service-is-one-stored-argv` has `status` report what no
    /// service manager knows — the in-flight request count and drain progress,
    /// asked over the control socket. That half belongs to the subcommand;
    /// here, `Status` is the manager's own answer and nothing more.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub(crate) enum Control {
        /// Start it now.
        Start,
        /// Stop it, which a hosted server answers with a drain
        /// (`rule:packaging/a-service-answers-its-manager`).
        Stop,
        /// Ask what state the manager has it in.
        Status,
    }

    impl Control {
        /// The `systemctl` verb, which is also the word the SCM line prints.
        fn word(self) -> &'static str {
            match self {
                Self::Start => "start",
                Self::Stop => "stop",
                Self::Status => "is-active",
            }
        }
    }

    /// The service manager an action is applied through.
    ///
    /// One method, because an [`Action`] already says what to do: `Scm` and
    /// `Systemd` are the real implementations, and `Recording` is what a case
    /// drives instead.
    pub(crate) trait Manager {
        /// Perform `action`, answering whatever the platform said about it —
        /// which for a [`Control::Status`] query is the state it reported.
        ///
        /// # Errors
        ///
        /// The platform's own failure, unchanged: nothing above this can
        /// anticipate what a service manager refuses.
        fn apply(&self, action: &Action) -> std::io::Result<Option<String>>;

        /// What the platform still holds under `name`: the argv it was
        /// registered with, and the account its grants were made to.
        ///
        /// Read back rather than asked for a second time, because § 4's
        /// closing property is about the install that happened and a second
        /// command line describes the one an operator believes happened. The
        /// two directories § 4 grants read/write on are left empty: neither
        /// manager holds them, and the caller derives them from the
        /// configuration the stored argv names.
        ///
        /// # Errors
        ///
        /// The platform's own failure, and `Unsupported` from a manager
        /// holding no registration to read — which is the recording one a case
        /// drives, where the [`Stored`] is the case's own value.
        fn stored(&self, name: &str, unit_root: &Path) -> std::io::Result<Stored> {
            let _ = (name, unit_root);
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "this service manager holds no registration to read back",
            ))
        }
    }

    /// Where a verb is being performed.
    ///
    /// `unit_root` is a parameter for the reason [`destination`] takes one: the
    /// directory § 5 writes into is the machine's, and a case asserting what an
    /// install writes has to own the directory it looks in.
    pub(crate) struct Site<'a> {
        /// Which manager holds the service.
        pub(crate) platform: Platform,
        /// The system unit directory, on the platform that has one.
        pub(crate) unit_root: &'a Path,
        /// The manager itself.
        pub(crate) manager: &'a dyn Manager,
    }

    /// Why a verb did not finish.
    #[derive(Debug)]
    pub(crate) enum Refused {
        /// § 2, reached before the manager was touched at all — which is
        /// `rule:packaging/the-installer-is-a-sink`'s whole claim.
        Installer(Diagnostic),
        /// The platform's own failure at the step it names.
        Manager(std::io::Error),
    }

    /// § 4's default identity: the virtual account the SCM creates and owns,
    /// with a per-service SID, no password to rotate or leak and no interactive
    /// logon. `--account` replaces it with a domain identity for a deployment
    /// that needs one, and `LocalSystem` is never the default.
    fn account_of(plan: &Plan) -> String {
        plan.account
            .clone()
            .unwrap_or_else(|| format!("NT SERVICE\\{}", plan.name))
    }

    /// § 4's grant list, closed: read on every configuration file the argv
    /// names, read/write on the log and artifact-cache directories, and nothing
    /// further.
    ///
    /// Read off the stored argv rather than off the installer's own options, so
    /// an uninstall that has only the `ImagePath` to go on derives the same list
    /// the install granted.
    fn granted(
        argv: &[String],
        log_file: Option<&Path>,
        cache_directory: Option<&Path>,
    ) -> Vec<(PathBuf, bool)> {
        let mut out: Vec<(PathBuf, bool)> = values_of(argv, "--config")
            .into_iter()
            .map(|config| (PathBuf::from(config), false))
            .collect();
        // The directory and not the file: a process that may write the log but
        // not the directory holding it cannot rotate one.
        if let Some(directory) = log_file.and_then(Path::parent) {
            out.push((directory.to_path_buf(), true));
        }
        if let Some(cache) = cache_directory {
            out.push((cache.to_path_buf(), true));
        }
        out
    }

    /// `install`'s steps, in the order the platform takes them.
    pub(crate) fn install_actions(
        platform: Platform,
        plan: &Plan,
        registration: &Registration,
        unit_root: &Path,
    ) -> Vec<Action> {
        match platform {
            Platform::Windows => {
                let account = account_of(plan);
                let mut out = vec![
                    Action::Register {
                        name: plan.name.clone(),
                        image_path: image_path(plan),
                        account: account.clone(),
                        start: registration.start,
                        depends_on: registration.depends_on.clone(),
                        description: registration
                            .description
                            .clone()
                            .unwrap_or_else(|| format!("Novis service {}", plan.name)),
                    },
                    Action::Preshutdown {
                        name: plan.name.clone(),
                        timeout: PRESHUTDOWN,
                    },
                    Action::Failure {
                        name: plan.name.clone(),
                        restart: registration.restart,
                        reset: FAILURE_RESET,
                    },
                ];
                for (path, write) in granted(
                    &plan.argv,
                    registration.log_file.as_deref(),
                    registration.cache_directory.as_deref(),
                ) {
                    out.push(Action::Grant {
                        account: account.clone(),
                        path,
                        write,
                    });
                }
                // Last, and after the grants: it is the destination a failure
                // to start is reported to, so it is registered while the
                // service still cannot have been started.
                out.push(Action::EventSource {
                    name: plan.name.clone(),
                    exe: plan.exe.clone(),
                });
                out
            }
            Platform::Linux => {
                let mut out = vec![Action::WriteUnit {
                    path: destination(Delivery::Install, &plan.name, unit_root)
                        .expect("an installing delivery names a unit file"),
                    text: unit(plan),
                }];
                // Then, and never before: `daemon-reload` is what makes the
                // file a unit systemd knows about, and an `enable` ahead of it
                // would be asked about a name it has not read yet.
                out.push(Action::Systemctl {
                    argv: vec!["daemon-reload".to_owned()],
                });
                if registration.start.enabled() {
                    out.push(Action::Systemctl {
                        argv: vec!["enable".to_owned(), plan.name.clone()],
                    });
                }
                out
            }
        }
    }

    /// `uninstall`'s steps, which leave no key, no source, no unit and no
    /// granted entry behind (`rule:packaging/a-service-answers-its-manager`).
    pub(crate) fn uninstall_actions(
        platform: Platform,
        stored: &Stored,
        unit_root: &Path,
    ) -> Vec<Action> {
        match platform {
            Platform::Windows => {
                // Stopped first: the SCM marks a running service for deletion
                // and removes it when the process exits, which would leave the
                // key behind for as long as it keeps running. An applier reads
                // "already stopped" as done.
                let mut out = vec![Action::Scm {
                    name: stored.name.clone(),
                    control: Control::Stop,
                }];
                for (path, _) in granted(
                    &stored.argv,
                    stored.log_file.as_deref(),
                    stored.cache_directory.as_deref(),
                ) {
                    out.push(Action::Revoke {
                        account: stored.account.clone(),
                        path,
                    });
                }
                out.push(Action::RemoveEventSource {
                    name: stored.name.clone(),
                });
                out.push(Action::Deregister {
                    name: stored.name.clone(),
                });
                out
            }
            Platform::Linux => vec![
                Action::Systemctl {
                    argv: vec![
                        "disable".to_owned(),
                        "--now".to_owned(),
                        stored.name.clone(),
                    ],
                },
                Action::RemoveUnit {
                    path: destination(Delivery::Install, &stored.name, unit_root)
                        .expect("an installing delivery names a unit file"),
                },
                Action::Systemctl {
                    argv: vec!["daemon-reload".to_owned()],
                },
            ],
        }
    }

    /// `start`, `stop` and `status`: the SCM directly on Windows, `systemctl`
    /// by argv with no shell on Linux.
    pub(crate) fn control_actions(platform: Platform, control: Control, name: &str) -> Vec<Action> {
        match platform {
            Platform::Windows => vec![Action::Scm {
                name: name.to_owned(),
                control,
            }],
            Platform::Linux => vec![Action::Systemctl {
                argv: vec![control.word().to_owned(), name.to_owned()],
            }],
        }
    }

    /// One line naming what an action does, which is what `--dry-run` prints.
    fn describe(action: &Action) -> String {
        match action {
            Action::Register {
                name,
                image_path,
                account,
                start,
                depends_on,
                description,
            } => {
                let depends = if depends_on.is_empty() {
                    String::new()
                } else {
                    format!(", after {}", depends_on.join(", "))
                };
                format!(
                    "register `{name}` as {account}, {} start, description `{description}`, \
                     ImagePath {image_path}{depends}",
                    start.word()
                )
            }
            Action::Preshutdown { name, timeout } => format!(
                "ask `{name}` for PRESHUTDOWN with {} seconds to drain in",
                timeout.as_secs()
            ),
            Action::Failure {
                name,
                restart,
                reset,
            } => format!(
                "set `{name}` to restart {}, forgetting failures after {} seconds",
                restart.word(),
                reset.as_secs()
            ),
            Action::Grant {
                account,
                path,
                write,
            } => format!(
                "grant {account} {} on {}",
                if *write { "read/write" } else { "read" },
                path.display()
            ),
            Action::EventSource { name, exe } => format!(
                "register the event-log source `{name}` under {EVENT_SOURCES}, reporting from {}",
                exe.display()
            ),
            Action::Deregister { name } => format!("remove the registration of `{name}`"),
            Action::Revoke { account, path } => {
                format!("revoke {account}'s access to {}", path.display())
            }
            Action::RemoveEventSource { name } => {
                format!("remove the event-log source `{name}` under {EVENT_SOURCES}")
            }
            Action::WriteUnit { path, text } => {
                format!("write {} ({} bytes)", path.display(), text.len())
            }
            Action::RemoveUnit { path } => format!("remove {}", path.display()),
            Action::Systemctl { argv } => format!("run systemctl {}", argv.join(" ")),
            Action::Scm { name, control } => format!("{} `{name}`", control.word()),
        }
    }

    /// Apply every action in order, or — for a dry run — describe them and
    /// touch nothing.
    ///
    /// The description goes to `out` rather than straight to standard output,
    /// because § 5's argument for the printed unit is that it is the artifact a
    /// change-management review wants, and an artifact nothing can read back is
    /// one nothing holds to the list that is actually performed.
    ///
    /// # Errors
    ///
    /// The manager's failure at the first action it refuses, so a verb stops
    /// where it stopped rather than carrying on past it.
    pub(crate) fn perform(
        actions: &[Action],
        site: &Site<'_>,
        dry_run: bool,
        out: &mut dyn std::io::Write,
    ) -> Result<Vec<String>, Refused> {
        if dry_run {
            for action in actions {
                writeln!(out, "{}", describe(action)).map_err(Refused::Manager)?;
            }
            return Ok(Vec::new());
        }
        let mut answers = Vec::new();
        for action in actions {
            match site.manager.apply(action) {
                Ok(Some(answer)) => answers.push(answer),
                Ok(None) => {}
                Err(error) => return Err(Refused::Manager(error)),
            }
        }
        Ok(answers)
    }

    /// `nvs service install` — § 2 first, then the platform's own steps.
    ///
    /// Nothing reaches [`Manager`] until [`plan`] has returned a [`Plan`],
    /// which is `rule:packaging/the-installer-is-a-sink`'s whole claim: a
    /// refusal touches nothing, and no step below re-checks, because none of
    /// them could have been reached without one.
    ///
    /// # Errors
    ///
    /// § 2's refusal, or the manager's own failure at the step it names.
    pub(crate) fn install(
        request: &Request<'_>,
        host: &Host,
        registration: &Registration,
        site: &Site<'_>,
        dry_run: bool,
        out: &mut dyn std::io::Write,
    ) -> Result<(), Refused> {
        let checked = plan(request, host).map_err(Refused::Installer)?;
        let actions = install_actions(site.platform, &checked, registration, site.unit_root);
        perform(&actions, site, dry_run, out).map(drop)
    }

    /// `nvs service uninstall` — the install's steps undone, from what the
    /// platform still holds.
    ///
    /// # Errors
    ///
    /// The manager's own failure at the step it names. There is no § 2 refusal
    /// here: nothing new is being stored.
    pub(crate) fn uninstall(
        stored: &Stored,
        site: &Site<'_>,
        dry_run: bool,
        out: &mut dyn std::io::Write,
    ) -> Result<(), Refused> {
        let actions = uninstall_actions(site.platform, stored, site.unit_root);
        perform(&actions, site, dry_run, out).map(drop)
    }

    /// `nvs service start`, `stop` and `status` — one manager call, and
    /// whatever it answered.
    ///
    /// # Errors
    ///
    /// The manager's own failure.
    pub(crate) fn control(
        control: Control,
        name: &str,
        site: &Site<'_>,
    ) -> Result<Vec<String>, Refused> {
        let actions = control_actions(site.platform, control, name);
        perform(&actions, site, false, &mut std::io::sink())
    }

    /// The Linux applier: § 5's unit written where systemd reads it, and
    /// `systemctl` run by argv with no shell
    /// (`rule:core-classes/process-is-argv-only`).
    ///
    /// Not `#[cfg(unix)]`, for the reason [`Platform`] is a parameter rather
    /// than a `cfg!`: a write, a remove and a child process are `std` on every
    /// platform, so compiling this everywhere is what keeps it compiled at all
    /// on the machine somebody happens to be working on. [`scm::Scm`] has no
    /// such choice — `windows-sys` is a dependency only where it exists.
    #[cfg_attr(
        all(test, windows),
        expect(
            dead_code,
            reason = "on Windows the applier `at_host` names is `scm::Scm`, and a case drives the \
                      recording manager"
        )
    )]
    #[derive(Debug)]
    pub(crate) struct Systemd;

    impl Manager for Systemd {
        fn apply(&self, action: &Action) -> std::io::Result<Option<String>> {
            match action {
                Action::WriteUnit { path, text } => {
                    if let Some(directory) = path.parent() {
                        std::fs::create_dir_all(directory)?;
                    }
                    std::fs::write(path, text)?;
                    Ok(None)
                }
                Action::RemoveUnit { path } => match std::fs::remove_file(path) {
                    // The unit being gone is the state an uninstall asks for,
                    // which is what lets one finish after an install that
                    // stopped part way through its own list.
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    other => other.map(|()| None),
                },
                Action::Systemctl { argv } => systemctl(argv),
                Action::Register { .. }
                | Action::Preshutdown { .. }
                | Action::Failure { .. }
                | Action::Grant { .. }
                | Action::EventSource { .. }
                | Action::Deregister { .. }
                | Action::Revoke { .. }
                | Action::RemoveEventSource { .. }
                | Action::Scm { .. } => Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "an SCM action on Linux",
                )),
            }
        }

        fn stored(&self, name: &str, unit_root: &Path) -> std::io::Result<Stored> {
            let path = destination(Delivery::Install, name, unit_root)
                .expect("an installing delivery names a unit file");
            let text = std::fs::read_to_string(path)?;
            let mut argv = Vec::new();
            // A unit with no `User=` runs as root, which is what the grants
            // were made to.
            let mut account = "root".to_owned();
            for line in text.lines() {
                if let Some(command) = line.strip_prefix("ExecStart=") {
                    // Past the first word, which is the binary the unit names
                    // and the stored argv does not.
                    argv = shell_words(command).into_iter().skip(1).collect();
                } else if let Some(user) = line.strip_prefix("User=") {
                    account = user.trim().to_owned();
                }
            }
            Ok(Stored {
                name: name.to_owned(),
                argv,
                account,
                log_file: None,
                cache_directory: None,
            })
        }
    }

    /// One `systemctl` run, by argv and with no shell, answering what it wrote
    /// to standard output.
    ///
    /// `is-active` answers by **exit status** — a unit that is not running is
    /// reported with a non-zero exit and the state on standard output — so for
    /// that verb alone the output is the answer rather than a failure. Every
    /// other verb here means what its status says, and a failing one carries
    /// whatever systemd wrote to standard error.
    #[cfg_attr(
        all(test, windows),
        expect(
            dead_code,
            reason = "on Windows the applier `at_host` names is `scm::Scm`, and a case drives the \
                      recording manager"
        )
    )]
    fn systemctl(argv: &[String]) -> std::io::Result<Option<String>> {
        let answered = argv.first().is_some_and(|verb| verb == "is-active");
        let run = std::process::Command::new("systemctl")
            .args(argv)
            .output()?;
        if run.status.success() || answered {
            let out = String::from_utf8_lossy(&run.stdout).trim().to_owned();
            return Ok((!out.is_empty()).then_some(out));
        }
        Err(std::io::Error::other(format!(
            "systemctl {} failed: {}",
            argv.join(" "),
            String::from_utf8_lossy(&run.stderr).trim()
        )))
    }

    /// The Windows applier: the SCM for a registration and a control, the
    /// registry for the event-log source, and the file ACLs for a grant.
    ///
    /// Nothing here decides anything. The builder that produced the [`Action`]
    /// already chose the start type, the timeout, the account and the rights,
    /// and every function below spells one of those the platform's way, so the
    /// cases that assert § 3's and § 4's lists keep asserting the decisions
    /// even though no case can reach this module: a real registration needs
    /// administrator rights on a machine somebody chose.
    ///
    /// The three verbs that ask for a state rather than a change — `Deregister`,
    /// a `Stop` and `RemoveEventSource` — answer for the state and not for the
    /// call, so a service already stopped, already gone or a source that was
    /// never there is a step that is done. That is what lets an uninstall of a
    /// half-finished install still leave no key, no source and no granted ACL.
    #[cfg(windows)]
    #[cfg_attr(
        test,
        expect(
            dead_code,
            reason = "the `ServiceCommand` variants that choose an applier are the slice after it"
        )
    )]
    #[expect(
        unsafe_code,
        reason = "registering a service, writing an event-log source and granting a right on a \
                  path are `advapi32` calls, and none of the three has a spelling in `std`"
    )]
    pub(crate) mod scm {
        use std::io::{Error, ErrorKind};
        use std::os::windows::ffi::OsStrExt;
        use std::path::Path;
        use std::time::Duration;

        use windows_sys::Win32::Foundation::{
            ERROR_FILE_NOT_FOUND, ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_DOES_NOT_EXIST,
            ERROR_SERVICE_MARKED_FOR_DELETE, ERROR_SERVICE_NOT_ACTIVE, ERROR_SUCCESS, LocalFree,
            WIN32_ERROR,
        };
        use windows_sys::Win32::Security::Authorization::{
            ACCESS_MODE, EXPLICIT_ACCESS_W, GRANT_ACCESS, GetNamedSecurityInfoW,
            NO_MULTIPLE_TRUSTEE, REVOKE_ACCESS, SE_FILE_OBJECT, SetEntriesInAclW,
            SetNamedSecurityInfoW, TRUSTEE_IS_NAME, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
        };
        use windows_sys::Win32::Security::{
            ACL, DACL_SECURITY_INFORMATION, NO_INHERITANCE, PSECURITY_DESCRIPTOR,
            SUB_CONTAINERS_AND_OBJECTS_INHERIT,
        };
        use windows_sys::Win32::Storage::FileSystem::{
            DELETE, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
        };
        use windows_sys::Win32::System::EventLog::{
            EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE,
        };
        use windows_sys::Win32::System::Registry::{
            HKEY, HKEY_LOCAL_MACHINE, KEY_WRITE, REG_DWORD, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE,
            RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW,
        };
        use windows_sys::Win32::System::Services::{
            ChangeServiceConfig2W, CloseServiceHandle, ControlService, CreateServiceW,
            DeleteService, OpenSCManagerW, OpenServiceW, QUERY_SERVICE_CONFIGW,
            QueryServiceConfigW, QueryServiceStatusEx, SC_ACTION, SC_ACTION_NONE,
            SC_ACTION_RESTART, SC_HANDLE, SC_MANAGER_CONNECT, SC_MANAGER_CREATE_SERVICE,
            SC_STATUS_PROCESS_INFO, SERVICE_AUTO_START, SERVICE_CHANGE_CONFIG,
            SERVICE_CONFIG_DELAYED_AUTO_START_INFO, SERVICE_CONFIG_DESCRIPTION,
            SERVICE_CONFIG_FAILURE_ACTIONS, SERVICE_CONFIG_FAILURE_ACTIONS_FLAG,
            SERVICE_CONFIG_PRESHUTDOWN_INFO, SERVICE_CONTINUE_PENDING, SERVICE_CONTROL_STOP,
            SERVICE_DELAYED_AUTO_START_INFO, SERVICE_DEMAND_START, SERVICE_DESCRIPTIONW,
            SERVICE_ERROR_NORMAL, SERVICE_FAILURE_ACTIONS_FLAG, SERVICE_FAILURE_ACTIONSW,
            SERVICE_PAUSE_PENDING, SERVICE_PAUSED, SERVICE_PRESHUTDOWN_INFO, SERVICE_QUERY_CONFIG,
            SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START, SERVICE_START_PENDING,
            SERVICE_STATUS, SERVICE_STATUS_PROCESS, SERVICE_STOP, SERVICE_STOP_PENDING,
            SERVICE_STOPPED, SERVICE_WIN32_OWN_PROCESS, StartServiceW,
        };
        use windows_sys::core::BOOL;

        use super::{Action, Control, EVENT_SOURCES, Manager, Restart, StartMode};

        /// How long the SCM waits before starting the service again, once per
        /// consecutive failure.
        ///
        /// A back-off rather than one delay: a server that cannot bind its
        /// listener fails again as fast as it is started, and a fixed short
        /// delay turns that into a restart loop that fills the event log
        /// instead of leaving the failure visible in it.
        const RESTART_DELAYS: [u32; 3] = [1_000, 10_000, 60_000];

        /// The record types the event-log source declares, which is what the
        /// viewer renders § 4's *Output* lifecycle records from.
        const REPORTED: u32 =
            (EVENTLOG_ERROR_TYPE | EVENTLOG_WARNING_TYPE | EVENTLOG_INFORMATION_TYPE) as u32;

        /// The Windows applier.
        ///
        /// A unit struct: an [`Action`] carries everything a step needs, and
        /// the handles one step opens belong to that step rather than to the
        /// process.
        #[derive(Debug)]
        pub(crate) struct Scm;

        impl Manager for Scm {
            fn apply(&self, action: &Action) -> std::io::Result<Option<String>> {
                match action {
                    Action::Register {
                        name,
                        image_path,
                        account,
                        start,
                        depends_on,
                        description,
                    } => register(name, image_path, account, *start, depends_on, description)
                        .map(|()| None),
                    Action::Preshutdown { name, timeout } => {
                        preshutdown(name, *timeout).map(|()| None)
                    }
                    Action::Failure {
                        name,
                        restart,
                        reset,
                    } => failure(name, *restart, *reset).map(|()| None),
                    Action::Grant {
                        account,
                        path,
                        write,
                    } => grant(account, path, *write).map(|()| None),
                    Action::EventSource { name, exe } => event_source(name, exe).map(|()| None),
                    Action::Deregister { name } => deregister(name).map(|()| None),
                    Action::Revoke { account, path } => revoke(account, path).map(|()| None),
                    Action::RemoveEventSource { name } => remove_event_source(name).map(|()| None),
                    Action::Scm { name, control } => ask(name, *control),
                    Action::WriteUnit { .. }
                    | Action::RemoveUnit { .. }
                    | Action::Systemctl { .. } => Err(Error::new(
                        ErrorKind::Unsupported,
                        "a systemd action on Windows",
                    )),
                }
            }

            fn stored(&self, name: &str, _unit_root: &Path) -> std::io::Result<super::Stored> {
                let (image_path, account) = configured(name)?;
                let argv = crate::service::decode(&image_path);
                Ok(super::Stored {
                    name: name.to_owned(),
                    // Past the first word: § 3 encodes this binary ahead of
                    // the stored argv, and the argv is what was stored.
                    argv: argv.into_iter().skip(1).collect(),
                    account,
                    log_file: None,
                    cache_directory: None,
                })
            }
        }

        /// § 3's `ImagePath` and the account the SCM holds under `name`, which
        /// together are everything a Windows uninstall undoes.
        ///
        /// `QueryServiceConfigW` is asked its size first and then answered
        /// with exactly that, which is the only way it is willing to be
        /// called: the structure it writes ends in the strings its own fields
        /// point at.
        fn configured(name: &str) -> std::io::Result<(String, String)> {
            let (_database, handle) = service(name, SERVICE_QUERY_CONFIG)?;
            let mut needed = 0u32;
            // SAFETY: a null buffer of length zero, which is how this call is
            // asked for the size it wants rather than given one.
            unsafe { QueryServiceConfigW(handle.0, std::ptr::null_mut(), 0, &mut needed) };
            // A `u64` buffer rather than a `u8` one: the structure holds
            // pointers, and the cast below has to land on their alignment.
            let mut buffer = vec![0u64; (needed as usize).div_ceil(8).max(1)];
            let config = buffer.as_mut_ptr().cast::<QUERY_SERVICE_CONFIGW>();
            // SAFETY: a buffer of exactly the size the call just asked for,
            // aligned for the structure it writes into it.
            ok(unsafe { QueryServiceConfigW(handle.0, config, needed, &mut needed) })?;
            // SAFETY: the call above filled the structure, and both strings
            // it points at live in the buffer that is still borrowed here.
            let (image_path, account) = unsafe {
                (
                    text((*config).lpBinaryPathName),
                    text((*config).lpServiceStartName),
                )
            };
            Ok((image_path, account))
        }

        /// A NUL-terminated UTF-16 string the SCM wrote, as a `String`.
        ///
        /// # Safety
        ///
        /// `start` is null, or points at a NUL-terminated run of `u16` that
        /// outlives the call.
        unsafe fn text(start: *const u16) -> String {
            if start.is_null() {
                return String::new();
            }
            let mut length = 0;
            // SAFETY: the caller's run is NUL-terminated, so this walk stops
            // inside it.
            while unsafe { *start.add(length) } != 0 {
                length += 1;
            }
            // SAFETY: `length` units from `start` is the run it just walked.
            String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(start, length) })
        }

        /// A NUL-terminated UTF-16 copy, which is what every `…W` call takes.
        fn wide(text: &str) -> Vec<u16> {
            text.encode_utf16().chain(std::iter::once(0)).collect()
        }

        /// The same for a path, whose characters are already UTF-16 here.
        fn wide_path(path: &Path) -> Vec<u16> {
            path.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        }

        /// The dependency list `CreateServiceW` takes: each name
        /// NUL-terminated, the list closed by an empty one.
        fn wide_list(names: &[String]) -> Vec<u16> {
            let mut out = Vec::new();
            for name in names {
                out.extend(name.encode_utf16());
                out.push(0);
            }
            out.push(0);
            out
        }

        /// What a `BOOL`-returning call said, as this thread's last error.
        fn ok(result: BOOL) -> std::io::Result<()> {
            if result == 0 {
                Err(Error::last_os_error())
            } else {
                Ok(())
            }
        }

        /// The same for a call that answers its error code directly.
        fn ok_status(status: WIN32_ERROR) -> std::io::Result<()> {
            if status == ERROR_SUCCESS {
                Ok(())
            } else {
                Err(Error::from_raw_os_error(
                    i32::try_from(status).unwrap_or(i32::MAX),
                ))
            }
        }

        /// The platform code behind one of these errors, for the steps that
        /// read "already in the state you asked for" as done.
        fn code(error: &Error) -> Option<WIN32_ERROR> {
            error.raw_os_error().and_then(|raw| u32::try_from(raw).ok())
        }

        /// A duration as the milliseconds the SCM counts in, saturating: a
        /// timeout past the range of a `u32` is one no machine waits out.
        fn millis(duration: Duration) -> u32 {
            u32::try_from(duration.as_millis()).unwrap_or(u32::MAX)
        }

        /// A duration as whole seconds, saturating for the same reason.
        fn seconds(duration: Duration) -> u32 {
            u32::try_from(duration.as_secs()).unwrap_or(u32::MAX)
        }

        /// An `SC_HANDLE` closed when the step that opened it ends.
        struct Handle(SC_HANDLE);

        impl Drop for Handle {
            fn drop(&mut self) {
                // SAFETY: a handle this value owns, closed exactly once.
                unsafe { CloseServiceHandle(self.0) };
            }
        }

        /// An open registry key, closed the same way.
        struct Key(HKEY);

        impl Drop for Key {
            fn drop(&mut self) {
                // SAFETY: a key this value owns, closed exactly once.
                unsafe { RegCloseKey(self.0) };
            }
        }

        /// A handle to this machine's service database.
        fn manager(access: u32) -> std::io::Result<Handle> {
            // SAFETY: two null names, which is the active database on this
            // machine; the access mask is one of this module's constants.
            let handle = unsafe { OpenSCManagerW(std::ptr::null(), std::ptr::null(), access) };
            if handle.is_null() {
                return Err(Error::last_os_error());
            }
            Ok(Handle(handle))
        }

        /// A handle to one registered service, beside the database handle it
        /// was opened through, so both close together.
        fn service(name: &str, access: u32) -> std::io::Result<(Handle, Handle)> {
            let database = manager(SC_MANAGER_CONNECT)?;
            let name = wide(name);
            // SAFETY: an open database handle and a NUL-terminated name that
            // outlives the call.
            let handle = unsafe { OpenServiceW(database.0, name.as_ptr(), access) };
            if handle.is_null() {
                return Err(Error::last_os_error());
            }
            Ok((database, Handle(handle)))
        }

        /// § 3's registration, and the two parameters `CreateServiceW` has no
        /// field for: the description, and whether an automatic start is the
        /// delayed one.
        fn register(
            name: &str,
            image_path: &str,
            account: &str,
            start: StartMode,
            depends_on: &[String],
            description: &str,
        ) -> std::io::Result<()> {
            let database = manager(SC_MANAGER_CONNECT | SC_MANAGER_CREATE_SERVICE)?;
            let wide_name = wide(name);
            let wide_image = wide(image_path);
            let wide_account = wide(account);
            let dependencies = wide_list(depends_on);
            let start_type = if matches!(start, StartMode::Manual) {
                SERVICE_DEMAND_START
            } else {
                SERVICE_AUTO_START
            };
            // SAFETY: every pointer is to a NUL-terminated local that outlives
            // the call. The nulls are the load-order group, the tag, and the
            // password a virtual account does not have.
            let handle = unsafe {
                CreateServiceW(
                    database.0,
                    wide_name.as_ptr(),
                    wide_name.as_ptr(),
                    SERVICE_CHANGE_CONFIG | SERVICE_QUERY_STATUS | SERVICE_START | SERVICE_STOP,
                    SERVICE_WIN32_OWN_PROCESS,
                    start_type,
                    SERVICE_ERROR_NORMAL,
                    wide_image.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    if depends_on.is_empty() {
                        std::ptr::null()
                    } else {
                        dependencies.as_ptr()
                    },
                    wide_account.as_ptr(),
                    std::ptr::null(),
                )
            };
            if handle.is_null() {
                return Err(Error::last_os_error());
            }
            let handle = Handle(handle);
            let mut wide_description = wide(description);
            let info = SERVICE_DESCRIPTIONW {
                lpDescription: wide_description.as_mut_ptr(),
            };
            // SAFETY: the structure and the string it points at are locals
            // that outlive the call, at the info level that names the
            // structure.
            ok(unsafe {
                ChangeServiceConfig2W(
                    handle.0,
                    SERVICE_CONFIG_DESCRIPTION,
                    std::ptr::from_ref(&info).cast(),
                )
            })?;
            if start.enabled() {
                let info = SERVICE_DELAYED_AUTO_START_INFO {
                    fDelayedAutostart: BOOL::from(matches!(start, StartMode::Delayed)),
                };
                // SAFETY: as above.
                ok(unsafe {
                    ChangeServiceConfig2W(
                        handle.0,
                        SERVICE_CONFIG_DELAYED_AUTO_START_INFO,
                        std::ptr::from_ref(&info).cast(),
                    )
                })?;
            }
            Ok(())
        }

        /// § 4's `PRESHUTDOWN`: the machine agreeing to wait while a drain
        /// finishes, rather than the few seconds plain `SHUTDOWN` allows.
        fn preshutdown(name: &str, timeout: Duration) -> std::io::Result<()> {
            let (_database, handle) = service(name, SERVICE_CHANGE_CONFIG)?;
            let info = SERVICE_PRESHUTDOWN_INFO {
                dwPreshutdownTimeout: millis(timeout),
            };
            // SAFETY: a local structure that outlives the call, at the info
            // level that names it.
            ok(unsafe {
                ChangeServiceConfig2W(
                    handle.0,
                    SERVICE_CONFIG_PRESHUTDOWN_INFO,
                    std::ptr::from_ref(&info).cast(),
                )
            })
        }

        /// § 4's failure actions and the period after which the SCM forgets
        /// earlier failures.
        fn failure(name: &str, restart: Restart, reset: Duration) -> std::io::Result<()> {
            let (_database, handle) = service(name, SERVICE_CHANGE_CONFIG)?;
            let mut actions: Vec<SC_ACTION> = Vec::new();
            match restart {
                Restart::OnFailure => {
                    for delay in RESTART_DELAYS {
                        actions.push(SC_ACTION {
                            Type: SC_ACTION_RESTART,
                            Delay: delay,
                        });
                    }
                }
                Restart::Never => actions.push(SC_ACTION {
                    Type: SC_ACTION_NONE,
                    Delay: 0,
                }),
            }
            let info = SERVICE_FAILURE_ACTIONSW {
                dwResetPeriod: seconds(reset),
                lpRebootMsg: std::ptr::null_mut(),
                lpCommand: std::ptr::null_mut(),
                cActions: u32::try_from(actions.len()).unwrap_or(u32::MAX),
                lpsaActions: actions.as_mut_ptr(),
            };
            // SAFETY: the structure and the action array are locals that
            // outlive the call, and `cActions` is that array's own length.
            ok(unsafe {
                ChangeServiceConfig2W(
                    handle.0,
                    SERVICE_CONFIG_FAILURE_ACTIONS,
                    std::ptr::from_ref(&info).cast(),
                )
            })?;
            // A server that cannot bind its listener exits non-zero rather
            // than crashing, and the SCM counts only a crash as a failure
            // unless this flag is set — so without it the restart § 4 asks for
            // would not happen in the case it exists for.
            let flag = SERVICE_FAILURE_ACTIONS_FLAG {
                fFailureActionsOnNonCrashFailures: BOOL::from(matches!(
                    restart,
                    Restart::OnFailure
                )),
            };
            // SAFETY: as above.
            ok(unsafe {
                ChangeServiceConfig2W(
                    handle.0,
                    SERVICE_CONFIG_FAILURE_ACTIONS_FLAG,
                    std::ptr::from_ref(&flag).cast(),
                )
            })
        }

        /// § 4's grant, and the whole of what it gives: read on a file the
        /// argv named, read/write on a directory the service writes in.
        ///
        /// Inheritance follows the object rather than the flag, because a
        /// grant on a log directory buys nothing if the file rotation creates
        /// in it does not carry the same entry.
        fn grant(account: &str, path: &Path, write: bool) -> std::io::Result<()> {
            let rights = if write {
                FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE | DELETE
            } else {
                FILE_GENERIC_READ
            };
            entry(account, path, GRANT_ACCESS, rights)
        }

        /// The same entry taken away.
        ///
        /// `REVOKE_ACCESS` removes every entry the account holds on the object
        /// rather than subtracting the rights the install added, which is what
        /// "no granted ACL" means when an install was interrupted part way
        /// through its own list.
        fn revoke(account: &str, path: &Path) -> std::io::Result<()> {
            entry(account, path, REVOKE_ACCESS, 0)
        }

        /// One explicit entry, merged into the object's own DACL and written
        /// back — the writing half of the walk
        /// `crates/nvs-config/src/trust.rs` reads.
        fn entry(
            account: &str,
            path: &Path,
            mode: ACCESS_MODE,
            rights: u32,
        ) -> std::io::Result<()> {
            let object = wide_path(path);
            let mut trustee = wide(account);
            let access = EXPLICIT_ACCESS_W {
                grfAccessPermissions: rights,
                grfAccessMode: mode,
                grfInheritance: if path.is_dir() {
                    SUB_CONTAINERS_AND_OBJECTS_INHERIT
                } else {
                    NO_INHERITANCE
                },
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: std::ptr::null_mut(),
                    MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                    TrusteeForm: TRUSTEE_IS_NAME,
                    TrusteeType: TRUSTEE_IS_UNKNOWN,
                    ptstrName: trustee.as_mut_ptr(),
                },
            };
            let mut dacl: *mut ACL = std::ptr::null_mut();
            let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
            // SAFETY: a NUL-terminated path that outlives the call, and
            // out-parameters of the types the signature names. A failing read
            // allocates no descriptor, which is why the early return below
            // frees nothing.
            let read = unsafe {
                GetNamedSecurityInfoW(
                    object.as_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut dacl,
                    std::ptr::null_mut(),
                    &mut descriptor,
                )
            };
            ok_status(read)?;
            let mut merged: *mut ACL = std::ptr::null_mut();
            // SAFETY: one entry, the DACL the read above returned, and an
            // out-parameter for the list `advapi32` allocates.
            let built = unsafe { SetEntriesInAclW(1, &access, dacl, &mut merged) };
            let written = if built == ERROR_SUCCESS {
                // SAFETY: the merged list, onto the same object the read
                // named; both outlive the call.
                unsafe {
                    SetNamedSecurityInfoW(
                        object.as_ptr(),
                        SE_FILE_OBJECT,
                        DACL_SECURITY_INFORMATION,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        merged,
                        std::ptr::null(),
                    )
                }
            } else {
                built
            };
            // SAFETY: two blocks `advapi32` allocated, each freed once and
            // neither read afterwards; `LocalFree` of a null pointer is a
            // no-op, which is the path a failed merge takes.
            unsafe {
                LocalFree(merged.cast());
                LocalFree(descriptor);
            }
            ok_status(written)
        }

        /// § 4's *Output*: the event-log source is a subkey under
        /// [`EVENT_SOURCES`], and `EventMessageFile` is the binary the viewer
        /// reads a record's text out of.
        fn event_source(name: &str, exe: &Path) -> std::io::Result<()> {
            let path = wide(&format!(r"{EVENT_SOURCES}\{name}"));
            let mut created: HKEY = std::ptr::null_mut();
            // SAFETY: a NUL-terminated subkey path that outlives the call, and
            // an out-parameter for the key; the disposition is not read.
            let opened = unsafe {
                RegCreateKeyExW(
                    HKEY_LOCAL_MACHINE,
                    path.as_ptr(),
                    0,
                    std::ptr::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_WRITE,
                    std::ptr::null(),
                    &mut created,
                    std::ptr::null_mut(),
                )
            };
            ok_status(opened)?;
            let key = Key(created);
            let message_file = wide_path(exe);
            let value = wide("EventMessageFile");
            // SAFETY: both strings outlive the call, and the byte count is the
            // UTF-16 length including its terminator.
            ok_status(unsafe {
                RegSetValueExW(
                    key.0,
                    value.as_ptr(),
                    0,
                    REG_EXPAND_SZ,
                    message_file.as_ptr().cast::<u8>(),
                    u32::try_from(message_file.len() * 2).unwrap_or(u32::MAX),
                )
            })?;
            let value = wide("TypesSupported");
            let reported = REPORTED;
            // SAFETY: four bytes from a local `u32`, at the type that names
            // that width.
            ok_status(unsafe {
                RegSetValueExW(
                    key.0,
                    value.as_ptr(),
                    0,
                    REG_DWORD,
                    std::ptr::from_ref(&reported).cast::<u8>(),
                    4,
                )
            })
        }

        /// The source's subkey taken away again.
        fn remove_event_source(name: &str) -> std::io::Result<()> {
            let path = wide(&format!(r"{EVENT_SOURCES}\{name}"));
            // SAFETY: a NUL-terminated subkey path that outlives the call.
            let removed = unsafe { RegDeleteTreeW(HKEY_LOCAL_MACHINE, path.as_ptr()) };
            if removed == ERROR_FILE_NOT_FOUND {
                return Ok(());
            }
            ok_status(removed)
        }

        /// The registration taken away.
        fn deregister(name: &str) -> std::io::Result<()> {
            let (_database, handle) = match service(name, DELETE) {
                Ok(handles) => handles,
                Err(error) if code(&error) == Some(ERROR_SERVICE_DOES_NOT_EXIST) => return Ok(()),
                Err(error) => return Err(error),
            };
            // SAFETY: a handle opened for `DELETE` on the line above.
            match ok(unsafe { DeleteService(handle.0) }) {
                Err(error) if code(&error) == Some(ERROR_SERVICE_MARKED_FOR_DELETE) => Ok(()),
                other => other,
            }
        }

        /// `start`, `stop` and `status`, through the SCM itself rather than an
        /// `sc.exe` this process would have to quote a name for.
        fn ask(name: &str, control: Control) -> std::io::Result<Option<String>> {
            match control {
                Control::Start => {
                    let (_database, handle) = service(name, SERVICE_START)?;
                    // SAFETY: a handle opened for `SERVICE_START`, and no
                    // arguments — the argv a service runs is its `ImagePath`.
                    match ok(unsafe { StartServiceW(handle.0, 0, std::ptr::null()) }) {
                        Err(error) if code(&error) == Some(ERROR_SERVICE_ALREADY_RUNNING) => {
                            Ok(None)
                        }
                        other => other.map(|()| None),
                    }
                }
                Control::Stop => {
                    let (_database, handle) = match service(name, SERVICE_STOP) {
                        Ok(handles) => handles,
                        Err(error) if code(&error) == Some(ERROR_SERVICE_DOES_NOT_EXIST) => {
                            return Ok(None);
                        }
                        Err(error) => return Err(error),
                    };
                    // SAFETY: a `repr(C)` structure of integers, whose all-zero
                    // value the call below overwrites.
                    let mut status: SERVICE_STATUS = unsafe { std::mem::zeroed() };
                    // SAFETY: a handle opened for `SERVICE_STOP`, and a local
                    // out-parameter the call fills in.
                    match ok(unsafe { ControlService(handle.0, SERVICE_CONTROL_STOP, &mut status) })
                    {
                        Err(error) if code(&error) == Some(ERROR_SERVICE_NOT_ACTIVE) => Ok(None),
                        other => other.map(|()| None),
                    }
                }
                Control::Status => {
                    let (_database, handle) = service(name, SERVICE_QUERY_STATUS)?;
                    // SAFETY: as above.
                    let mut status: SERVICE_STATUS_PROCESS = unsafe { std::mem::zeroed() };
                    let mut needed = 0_u32;
                    // SAFETY: a buffer that is exactly the structure the info
                    // level names, its own size, and an out-parameter for the
                    // size the call would have wanted.
                    ok(unsafe {
                        QueryServiceStatusEx(
                            handle.0,
                            SC_STATUS_PROCESS_INFO,
                            std::ptr::from_mut(&mut status).cast::<u8>(),
                            u32::try_from(size_of::<SERVICE_STATUS_PROCESS>()).unwrap_or(u32::MAX),
                            &mut needed,
                        )
                    })?;
                    Ok(Some(state(status.dwCurrentState)))
                }
            }
        }

        /// The word an SCM state number means, which is all a service manager
        /// knows: what `nvs service status` adds to it comes from the control
        /// socket.
        fn state(current: u32) -> String {
            match current {
                SERVICE_STOPPED => "stopped".to_owned(),
                SERVICE_START_PENDING => "start-pending".to_owned(),
                SERVICE_STOP_PENDING => "stop-pending".to_owned(),
                SERVICE_RUNNING => "running".to_owned(),
                SERVICE_CONTINUE_PENDING => "continue-pending".to_owned(),
                SERVICE_PAUSE_PENDING => "pause-pending".to_owned(),
                SERVICE_PAUSED => "paused".to_owned(),
                other => format!("state {other}"),
            }
        }
    }

    /// A manager that performs nothing and remembers every action it was
    /// handed.
    ///
    /// What every case drives: § 3's registration and § 5's write both need
    /// administrator rights on a machine somebody chose, while the property a
    /// case is about is the list of actions rather than the platform's answer
    /// to one of them.
    #[cfg(test)]
    #[derive(Debug, Default)]
    pub(crate) struct Recording {
        applied: Mutex<Vec<Action>>,
    }

    #[cfg(test)]
    impl Recording {
        /// Everything applied through it, in order.
        pub(crate) fn applied(&self) -> Vec<Action> {
            self.applied
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    #[cfg(test)]
    impl Manager for Recording {
        fn apply(&self, action: &Action) -> std::io::Result<Option<String>> {
            self.applied
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(action.clone());
            Ok(None)
        }
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
    let host = match describe_host(config, argv, Unresolved::Refuses, &mut sources) {
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

/// The site this installer acts in: the platform it is running on, that
/// platform's real manager, and the directory § 5 writes a unit into.
///
/// The one `cfg` in the surface, and it is here rather than in a verb because
/// every verb is the same list of actions on either machine — what differs is
/// only which manager performs one, and only this machine's exists.
fn at_host<T>(body: impl FnOnce(&registration::Site<'_>) -> T) -> T {
    #[cfg(windows)]
    let manager = registration::scm::Scm;
    #[cfg(not(windows))]
    let manager = registration::Systemd;
    body(&registration::Site {
        platform: registration::Platform::host(),
        unit_root: Path::new(UNIT_DIRECTORY),
        manager: &manager,
    })
}

/// A refusal, rendered as whichever of the two it is: § 2's diagnostic, or the
/// platform's own failure at the step the verb stopped on.
fn report(refused: registration::Refused, sources: &mut SourceMap) -> ExitCode {
    match refused {
        registration::Refused::Installer(diagnostic) => refuse(diagnostic, sources),
        registration::Refused::Manager(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// What the platform holds under `name`, with the name in the failure.
///
/// The service manager's own error is "the specified service is not an
/// installed service" and nothing else, which in an operator's locale and with
/// no name in it is a sentence about no particular service.
fn held(
    site: &registration::Site<'_>,
    name: &str,
) -> Result<registration::Stored, registration::Refused> {
    site.manager.stored(name, site.unit_root).map_err(|error| {
        registration::Refused::Manager(std::io::Error::new(
            error.kind(),
            format!("this machine's service manager holds no `{name}`: {error}"),
        ))
    })
}

/// The installer's own options — everything `nvs service install` was given to
/// the left of `--`, where the argv to its right is the request itself.
pub(crate) struct InstallOptions<'a> {
    /// Where the service writes diagnostics, if the named configuration does
    /// not say (§ 2's fourth row).
    pub(crate) log_file: Option<&'a Path>,
    /// § 4's `--account`, replacing the per-service virtual account.
    pub(crate) account: Option<&'a str>,
    /// § 2 refuses it and exists to name it (`E0633`).
    pub(crate) password: Option<&'a str>,
    /// § 4's `--start`.
    pub(crate) start: registration::StartMode,
    /// § 4's `--restart`.
    pub(crate) restart: registration::Restart,
    /// § 4's `--depends-on`.
    pub(crate) depends_on: &'a [String],
    /// What an administrator reads beside the name.
    pub(crate) description: Option<&'a str>,
    /// Print the actions and touch nothing.
    pub(crate) dry_run: bool,
}

/// `nvs service install <name> [options] -- <argv…>` — § 2's refusals, then
/// this platform's own steps.
///
/// The grants are derived here rather than passed in: the log destination is
/// the installer's `--log-file` or the `file:` target the named configuration
/// carries, and the artifact cache is that configuration's
/// `[opcache] file_cache_dir`. An uninstall re-derives both from the same
/// configuration, which is what makes § 4's closing property hold against the
/// install that granted them.
pub(crate) fn install(
    config: &[PathBuf],
    name: &str,
    argv: &[String],
    options: &InstallOptions<'_>,
) -> ExitCode {
    let request = Request {
        name,
        argv,
        log_file: options.log_file,
        account: options.account,
        password: options.password,
    };
    let mut sources = SourceMap::new();
    let host = match describe_host(config, argv, Unresolved::Refuses, &mut sources) {
        Ok(host) => host,
        Err(diagnostic) => return refuse(diagnostic, &mut sources),
    };
    let registration = registration::Registration {
        start: options.start,
        restart: options.restart,
        depends_on: options.depends_on.to_vec(),
        description: options.description.map(str::to_owned),
        log_file: options
            .log_file
            .map(Path::to_path_buf)
            .or_else(|| host.log_file.clone()),
        cache_directory: host.cache_directory.clone(),
    };
    let performed = at_host(|site| {
        registration::install(
            &request,
            &host,
            &registration,
            site,
            options.dry_run,
            &mut std::io::stdout(),
        )
    });
    match performed {
        Ok(()) => ExitCode::SUCCESS,
        Err(refused) => report(refused, &mut sources),
    }
}

/// `nvs service uninstall <name>` — the install's steps undone, from what the
/// platform still holds.
pub(crate) fn uninstall(config: &[PathBuf], name: &str, dry_run: bool) -> ExitCode {
    let mut sources = SourceMap::new();
    let performed = at_host(|site| {
        let mut stored = held(site, name)?;
        let host = describe_host(config, &stored.argv, Unresolved::ReadsAsEmpty, &mut sources)
            .map_err(registration::Refused::Installer)?;
        // The two the platform does not hold, back from where the install
        // read them. An install given the installer's own `--log-file` over a
        // configuration naming no destination is the one case this misses, and
        // the module doc owns it.
        stored.log_file = host.log_file;
        stored.cache_directory = host.cache_directory;
        registration::uninstall(&stored, site, dry_run, &mut std::io::stdout())
    });
    match performed {
        Ok(()) => ExitCode::SUCCESS,
        Err(refused) => report(refused, &mut sources),
    }
}

/// `nvs service start <name>`.
pub(crate) fn start(name: &str) -> ExitCode {
    answered(name, registration::Control::Start)
}

/// `nvs service stop <name>`, which a hosted server answers with a drain
/// (`rule:packaging/a-service-answers-its-manager`).
pub(crate) fn stop(name: &str) -> ExitCode {
    answered(name, registration::Control::Stop)
}

/// `nvs service status <name>` — the manager's own answer, and then the one
/// § 1 gives the verb its second spelling for: what no service manager knows,
/// asked over the control socket the service's own configuration names.
///
/// That configuration is read out of the stored argv rather than out of this
/// shell's `--config`, for the reason [`describe_host`] reads its three
/// questions there: the tree this command is pointed at answers for this
/// shell, and the question is about the service.
pub(crate) fn status(config: &[PathBuf], name: &str) -> ExitCode {
    let mut sources = SourceMap::new();
    let asked = at_host(|site| {
        let stored = held(site, name)?;
        let answers = registration::control(registration::Control::Status, name, site)?;
        Ok((stored, answers))
    });
    let (stored, answers) = match asked {
        Ok(answered) => answered,
        Err(refused) => return report(refused, &mut sources),
    };
    for answer in answers {
        println!("{answer}");
    }
    crate::ctl::status(&service_configuration(&stored.argv, config), None)
}

/// The configuration whose `[control] socket` `nvs service status` asks: the
/// one the **stored** argv names, and this shell's only where it names none.
///
/// Apart from [`status`] because it is the whole of what that verb adds to
/// `nvs ctl status`, and a case can then assert which endpoint is chosen
/// without a service manager holding a registration to read one out of.
pub(crate) fn service_configuration(argv: &[String], fallback: &[PathBuf]) -> Vec<PathBuf> {
    let named: Vec<PathBuf> = values_of(argv, "--config")
        .into_iter()
        .map(PathBuf::from)
        .collect();
    if named.is_empty() {
        fallback.to_vec()
    } else {
        named
    }
}

/// `nvs service run <name>` — the argv the platform holds under `name`, so
/// this process can run the line a service manager would have started.
///
/// A manager never invokes this verb: § 3's `ImagePath` and § 5's `ExecStart`
/// both carry the stored argv directly, so what the manager starts is
/// `nvs serve …` itself. What this is for is running that same line in the
/// foreground, where its output is on a terminal — which is how an operator
/// watches a boot that fails under the manager with nothing in the log.
///
/// # Errors
///
/// The platform's own failure to say what it holds, already reported.
pub(crate) fn stored_argv(name: &str) -> Result<Vec<String>, ExitCode> {
    let mut sources = SourceMap::new();
    at_host(|site| held(site, name))
        .map(|stored| stored.argv)
        .map_err(|refused| report(refused, &mut sources))
}

/// One control verb, and whatever the manager answered.
fn answered(name: &str, control: registration::Control) -> ExitCode {
    let mut sources = SourceMap::new();
    match at_host(|site| registration::control(control, name, site)) {
        Ok(answers) => {
            for answer in answers {
                println!("{answer}");
            }
            ExitCode::SUCCESS
        }
        Err(refused) => report(refused, &mut sources),
    }
}

/// What [`describe_host`] does with a named configuration that does not
/// resolve.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Unresolved {
    /// The configuration's own diagnostic is the refusal. `install` and `unit`
    /// take this one: the service would refuse the same file at every boot, and
    /// reading it as empty reports `E0632` about a file whose `[log] target`
    /// the operator can see — a key under a header left commented out is the
    /// usual way to get there.
    Refuses,
    /// It reads as a configuration that says nothing. `uninstall` takes this
    /// one, because a service whose configuration has since been broken or
    /// deleted is exactly one an operator still has to be able to remove.
    ReadsAsEmpty,
}

/// What this process and the named configuration answer about themselves.
///
/// The configuration questions are read off the merged table rather than the
/// typed tree, because each is one key and the tree would have to be
/// deserialized in full to reach them.
fn describe_host(
    config: &[PathBuf],
    argv: &[String],
    unresolved: Unresolved,
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
    let resolved = nvs_config::resolve::resolve(
        &nvs_config::resolve::roots(&roots, &cwd, &files),
        sources,
        &files,
    )
    .map(|resolved| resolved.table);
    // An argv naming no `--config` is `E0631`'s to refuse, and what resolved in
    // its place is this shell's tree, whose faults are not the service's.
    let table = match resolved {
        Err(diagnostic) if unresolved == Unresolved::Refuses && !named.is_empty() => {
            return Err(diagnostic);
        }
        resolved => resolved.unwrap_or_default(),
    };

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
    let cache_directory = table
        .get("opcache")
        .and_then(toml::Value::as_table)
        .and_then(|opcache| opcache.get("file_cache_dir"))
        .and_then(toml::Value::as_str)
        .map(PathBuf::from);

    Ok(Host {
        exe,
        from_a_bundle: crate::bundle::embedded().is_some(),
        config_names_a_log_destination: target
            .is_some_and(|target| target.starts_with("file:") || target == "syslog"),
        control_socket,
        memory_max,
        privileged_port: listen.is_some_and(privileged),
        log_file: target
            .and_then(|target| target.strip_prefix("file:"))
            .map(PathBuf::from),
        cache_directory,
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
            log_file: None,
            cache_directory: None,
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

    /// A named configuration that does not resolve is refused as itself. The
    /// file here is the shipped template with `target` uncommented and `[log]`
    /// left commented out above it, which makes `target` a root key: read as
    /// empty, that file is refused as `E0632` with its destination in plain
    /// sight. An uninstall reads the same file as saying nothing, so a broken
    /// configuration never keeps a service installed.
    #[test]
    fn a_configuration_that_does_not_resolve_is_refused_as_itself_and_never_blocks_an_uninstall() {
        let root = unit_root("unresolved");
        let file = root.join("nvs.toml");
        std::fs::write(&file, "#[log]\ntarget = \"file:/var/log/nvs.log\"\n").expect("a file");
        let named = file.to_str().expect("a UTF-8 path").to_owned();
        let argv = vec!["serve".to_owned(), "--config".to_owned(), named];

        let refusal = describe_host(&[], &argv, Unresolved::Refuses, &mut SourceMap::new())
            .err()
            .expect("a root `target` is not a key");
        assert_eq!(coded(&refusal), code::E_BAD_DIRECTIVE);

        let host = describe_host(&[], &argv, Unresolved::ReadsAsEmpty, &mut SourceMap::new())
            .expect("an uninstall reads it as empty");
        assert!(!host.config_names_a_log_destination);
        assert_eq!(host.log_file, None);

        // An argv naming no `--config` is `E0631`'s to refuse, whatever this
        // shell's own tree looks like, so it is described and left to `plan`.
        let bare = vec!["serve".to_owned()];
        assert!(describe_host(&[file], &bare, Unresolved::Refuses, &mut SourceMap::new()).is_ok());
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

    /// The `WatchdogSec=30` line [`unit`] renders is a promise: the manager
    /// stops this process unless the ping arrives inside half that period. So
    /// both halves of what the ping *means* are asserted here — it is sent
    /// while the fleet turns, and it stops the moment the fleet does, which is
    /// the difference between evidence and a beat any live thread can write.
    ///
    /// The interval is this case's rather than a manager's, because the only
    /// thing [`Heartbeat::start`] adds is reading two environment variables,
    /// and a case may not set those out from under another running beside it.
    #[test]
    fn the_watchdog_ping_is_sent_while_the_fleet_turns_and_stops_when_it_does_not() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let (told, sent) = recording();
        let turning = Arc::new(AtomicBool::new(true));
        let gate = Arc::clone(&turning);
        let interval = Duration::from_millis(5);
        let beat = Heartbeat::every(&told, interval, move || gate.load(Ordering::Relaxed))
            .expect("the thread that answers `WatchdogSec`");
        let pings = || {
            sent.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .iter()
                .filter(|line| **line == "WATCHDOG=1")
                .count()
        };
        // Generous on purpose: what this waits for is that pings arrive at all,
        // and a loaded machine can take far longer than the interval to
        // schedule the thread sending them.
        let patience = Duration::from_secs(10);
        let deadline = std::time::Instant::now() + patience;
        while pings() < 2 && std::time::Instant::now() < deadline {
            std::thread::sleep(interval);
        }
        assert!(pings() >= 2, "a turning fleet's manager was told nothing");

        // No core turning, so the process stops saying otherwise. Counted twice
        // with intervals in between rather than once, so that a ping already
        // past the gate when the fleet stopped is not read as a live one.
        turning.store(false, Ordering::Relaxed);
        std::thread::sleep(interval * 20);
        let last = pings();
        std::thread::sleep(interval * 20);
        assert_eq!(
            pings(),
            last,
            "a process whose every core had stopped went on telling its manager it was alive"
        );

        // And a fleet that comes back is reported again, rather than left
        // condemned by the sweep that found it wedged.
        turning.store(true, Ordering::Relaxed);
        let deadline = std::time::Instant::now() + patience;
        while pings() == last && std::time::Instant::now() < deadline {
            std::thread::sleep(interval);
        }
        assert!(pings() > last, "a fleet that came back was not reported");

        drop(beat);
        let stopped = pings();
        std::thread::sleep(interval * 20);
        assert_eq!(
            pings(),
            stopped,
            "the thread outlived the handle that owns it and is still pinging"
        );
    }

    /// A scratch unit directory this case owns, so what an install would write
    /// is asserted against a directory nothing else is looking at.
    fn unit_root(case: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("nvs-service-{case}-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("a scratch directory");
        root
    }

    /// An installer's options with both grantable directories named, so § 4's
    /// grant list is exercised whole rather than in its empty case.
    fn registration() -> registration::Registration {
        registration::Registration {
            log_file: Some(PathBuf::from(absolute("log/web.log"))),
            cache_directory: Some(PathBuf::from(absolute("cache"))),
            ..registration::Registration::default()
        }
    }

    /// `rule:packaging/the-argv-lives-in-imagepath` and § 4's identity and
    /// controls: what an install *is* on Windows is one registration carrying
    /// the encoded `ImagePath`, a virtual account nobody holds a password for,
    /// and a shutdown control that leaves room for a drain.
    #[test]
    fn service_install_on_windows_registers_the_encoded_image_path_a_virtual_account_and_preshutdown()
     {
        let argv = argv();
        let checked = plan(&request(&argv), &host()).expect("a plan");
        let root = unit_root("install-windows");
        let actions = registration::install_actions(
            registration::Platform::Windows,
            &checked,
            &registration(),
            &root,
        );

        let registered = actions
            .iter()
            .find_map(|action| match action {
                registration::Action::Register {
                    image_path: stored,
                    account,
                    start,
                    description,
                    ..
                } => Some((stored, account, *start, description)),
                _ => None,
            })
            .expect("a registration");
        // The encoder's output and not a second rendering of it: § 3 has that
        // one string be the only record of what the service runs.
        assert_eq!(registered.0, &image_path(&checked));
        assert_eq!(registered.1, "NT SERVICE\\web");
        assert_eq!(registered.2, registration::StartMode::Automatic);
        assert!(registered.3.contains("web"), "{}", registered.3);

        // `--account` replaces the virtual account and nothing else about the
        // registration.
        let mut named = request(&argv);
        named.account = Some("EXAMPLE\\nvs-web");
        let domain = plan(&named, &host()).expect("a plan");
        assert!(
            registration::install_actions(
                registration::Platform::Windows,
                &domain,
                &registration(),
                &root
            )
            .iter()
            .any(|action| matches!(
                action,
                registration::Action::Register { account, .. } if account == "EXAMPLE\\nvs-web"
            ))
        );

        // PRESHUTDOWN, with room for a drain: plain SHUTDOWN allows roughly
        // five seconds, which is the whole reason § 4 asks for this control.
        let timeout = actions
            .iter()
            .find_map(|action| match action {
                registration::Action::Preshutdown { timeout, .. } => Some(*timeout),
                _ => None,
            })
            .expect("a preshutdown request");
        assert!(timeout > Duration::from_secs(5), "{timeout:?}");

        // § 4's failure actions and its *Output*: both are part of installing,
        // not a second command an operator has to remember.
        assert!(actions.iter().any(|action| matches!(
            action,
            registration::Action::Failure {
                restart: registration::Restart::OnFailure,
                ..
            }
        )));
        assert!(
            actions
                .iter()
                .any(|action| matches!(action, registration::Action::EventSource { .. }))
        );

        // `--restart never` is the other half of that decision, and it changes
        // nothing else about the registration.
        let never = registration::Registration {
            restart: registration::Restart::Never,
            ..registration()
        };
        assert!(
            registration::install_actions(registration::Platform::Windows, &checked, &never, &root)
                .iter()
                .any(|action| matches!(
                    action,
                    registration::Action::Failure {
                        restart: registration::Restart::Never,
                        ..
                    }
                ))
        );

        // Windows has no unit file, and nothing here writes one.
        assert!(!actions.iter().any(|action| matches!(
            action,
            registration::Action::WriteUnit { .. } | registration::Action::Systemctl { .. }
        )));
        std::fs::remove_dir_all(&root).expect("a removed scratch directory");
    }

    /// `rule:packaging/the-unit-is-printed-and-install-is-the-opt-in`: an
    /// install on Linux is the same generation `unit` prints, written into the
    /// system unit directory and followed by a `daemon-reload` — in that order,
    /// because a reload is what makes the file a unit systemd knows the name
    /// of.
    #[test]
    fn service_install_on_linux_writes_the_unit_then_runs_daemon_reload() {
        let argv = argv();
        let checked = plan(&request(&argv), &host()).expect("a plan");
        let root = unit_root("install-linux");
        let actions = registration::install_actions(
            registration::Platform::Linux,
            &checked,
            &registration(),
            &root,
        );

        match &actions[0] {
            registration::Action::WriteUnit { path, text } => {
                assert_eq!(path, &root.join("web.service"));
                assert_eq!(text, &unit(&checked));
            }
            other => panic!("the first action is not the unit write: {other:?}"),
        }
        assert_eq!(
            actions[1],
            registration::Action::Systemctl {
                argv: vec!["daemon-reload".to_owned()]
            }
        );
        // Automatic start is an `enable`, after the reload; a manual one is the
        // unit sitting there unwanted.
        assert_eq!(
            actions[2],
            registration::Action::Systemctl {
                argv: vec!["enable".to_owned(), "web".to_owned()]
            }
        );
        let manual = registration::Registration {
            start: registration::StartMode::Manual,
            ..registration()
        };
        assert_eq!(
            registration::install_actions(registration::Platform::Linux, &checked, &manual, &root)
                .len(),
            2
        );

        // Delayed auto-start is still an auto-start: on Linux the difference is
        // the manager's own ordering and not whether the unit is wanted, so the
        // list is the same one.
        let delayed = registration::Registration {
            start: registration::StartMode::Delayed,
            ..registration()
        };
        assert_eq!(
            registration::install_actions(registration::Platform::Linux, &checked, &delayed, &root),
            actions
        );

        // Linux holds no registration and no event-log source.
        assert!(!actions.iter().any(|action| matches!(
            action,
            registration::Action::Register { .. }
                | registration::Action::EventSource { .. }
                | registration::Action::Grant { .. }
        )));

        // Building the list writes nothing: the write is an action an applier
        // performs, which is what makes `--dry-run` possible at all.
        assert_eq!(
            std::fs::read_dir(&root).expect("readable").count(),
            0,
            "planning wrote into {}",
            root.display()
        );
        std::fs::remove_dir_all(&root).expect("a removed scratch directory");
    }

    /// § 5's change-management artifact, as the thing that is actually
    /// performed rather than a second rendering of it: a dry run describes
    /// every action and reaches the manager with none of them.
    #[test]
    fn service_install_dry_run_prints_the_actions_and_touches_nothing() {
        let argv = argv();
        let checked = plan(&request(&argv), &host()).expect("a plan");
        let root = unit_root("dry-run");

        for platform in [
            registration::Platform::Windows,
            registration::Platform::Linux,
        ] {
            let manager = registration::Recording::default();
            let site = registration::Site {
                platform,
                unit_root: &root,
                manager: &manager,
            };
            let actions = registration::install_actions(platform, &checked, &registration(), &root);
            let mut printed = Vec::new();
            registration::install(
                &request(&argv),
                &host(),
                &registration(),
                &site,
                true,
                &mut printed,
            )
            .expect("a dry run");

            // One line per action, and the manager was not asked for any of
            // them.
            let text = String::from_utf8(printed).expect("utf-8");
            assert_eq!(text.lines().count(), actions.len(), "{platform:?}\n{text}");
            assert!(manager.applied().is_empty(), "{platform:?}");
            assert_eq!(
                std::fs::read_dir(&root).expect("readable").count(),
                0,
                "a dry run wrote into {}",
                root.display()
            );

            // And the same call without it applies exactly that list, so the
            // artifact and the steps cannot drift apart.
            registration::install(
                &request(&argv),
                &host(),
                &registration(),
                &site,
                false,
                &mut std::io::sink(),
            )
            .expect("an install");
            assert_eq!(manager.applied(), actions, "{platform:?}");
        }
        std::fs::remove_dir_all(&root).expect("a removed scratch directory");
    }

    /// `rule:packaging/a-service-answers-its-manager`'s closing sentence: an
    /// uninstall leaves no registration, no event-log source, no unit file and
    /// no granted access — and the grants it revokes are exactly the ones the
    /// install made, because both derive them from the stored argv.
    #[test]
    fn service_uninstall_leaves_no_key_no_event_source_no_unit_and_no_acl() {
        let argv = argv();
        let checked = plan(&request(&argv), &host()).expect("a plan");
        let root = unit_root("uninstall");
        let options = registration();
        let stored = registration::Stored {
            name: "web".to_owned(),
            argv: argv.clone(),
            account: "NT SERVICE\\web".to_owned(),
            log_file: options.log_file.clone(),
            cache_directory: options.cache_directory.clone(),
        };

        let installed = registration::install_actions(
            registration::Platform::Windows,
            &checked,
            &options,
            &root,
        );
        let removed =
            registration::uninstall_actions(registration::Platform::Windows, &stored, &root);

        let mut granted: Vec<&PathBuf> = installed
            .iter()
            .filter_map(|action| match action {
                registration::Action::Grant { path, .. } => Some(path),
                _ => None,
            })
            .collect();
        let mut revoked: Vec<&PathBuf> = removed
            .iter()
            .filter_map(|action| match action {
                registration::Action::Revoke { path, .. } => Some(path),
                _ => None,
            })
            .collect();
        assert!(!granted.is_empty(), "nothing was granted to revoke");
        granted.sort();
        revoked.sort();
        assert_eq!(granted, revoked);

        assert!(
            removed
                .iter()
                .any(|action| matches!(action, registration::Action::Deregister { .. }))
        );
        assert!(
            removed
                .iter()
                .any(|action| matches!(action, registration::Action::RemoveEventSource { .. }))
        );
        // Stopped before the key is taken away: the SCM removes a running
        // service when its process exits, which would leave the key behind for
        // as long as it kept running.
        assert_eq!(
            removed[0],
            registration::Action::Scm {
                name: "web".to_owned(),
                control: registration::Control::Stop
            }
        );
        assert!(!removed.iter().any(|action| matches!(
            action,
            registration::Action::Register { .. } | registration::Action::Grant { .. }
        )));

        // On Linux the unit file is the registration, so removing it is what
        // leaves nothing behind — and the reload is what makes systemd forget
        // the name.
        let linux = registration::uninstall_actions(registration::Platform::Linux, &stored, &root);
        assert!(linux.contains(&registration::Action::RemoveUnit {
            path: root.join("web.service")
        }));
        assert!(linux.contains(&registration::Action::Systemctl {
            argv: vec!["daemon-reload".to_owned()]
        }));
        assert!(
            !linux
                .iter()
                .any(|action| matches!(action, registration::Action::WriteUnit { .. }))
        );
        // Through the front door, and nothing added on the way: what an
        // uninstall performs is that list, in that order.
        let manager = registration::Recording::default();
        let site = registration::Site {
            platform: registration::Platform::Linux,
            unit_root: &root,
            manager: &manager,
        };
        registration::uninstall(&stored, &site, false, &mut std::io::sink()).expect("an uninstall");
        assert_eq!(manager.applied(), linux);

        std::fs::remove_dir_all(&root).expect("a removed scratch directory");
    }

    /// `rule:packaging/a-service-is-one-stored-argv`: the three thin verbs go
    /// to the SCM directly on Windows and to `systemctl` on Linux **by argv**,
    /// which is `rule:core-classes/process-is-argv-only` — the service's name
    /// is one element and never a word inside a command line somebody has to
    /// quote.
    #[test]
    fn service_start_stop_and_status_reach_the_manager_by_argv_with_no_shell() {
        let root = unit_root("control");

        // The manager is the host's, read rather than chosen: a service is
        // installed on the machine it will run on, which is the reading § 2's
        // third row already takes paths against.
        assert_eq!(
            registration::Platform::host(),
            if cfg!(windows) {
                registration::Platform::Windows
            } else {
                registration::Platform::Linux
            }
        );
        for (control, verb) in [
            (registration::Control::Start, "start"),
            (registration::Control::Stop, "stop"),
            (registration::Control::Status, "is-active"),
        ] {
            assert_eq!(
                registration::control_actions(registration::Platform::Windows, control, "web"),
                vec![registration::Action::Scm {
                    name: "web".to_owned(),
                    control
                }]
            );
            assert_eq!(
                registration::control_actions(registration::Platform::Linux, control, "a name"),
                vec![registration::Action::Systemctl {
                    argv: vec![verb.to_owned(), "a name".to_owned()]
                }],
                "{verb}"
            );

            // One manager call, and whatever it answered comes back.
            let manager = registration::Recording::default();
            let site = registration::Site {
                platform: registration::Platform::Linux,
                unit_root: &root,
                manager: &manager,
            };
            registration::control(control, "web", &site).expect("a control");
            assert_eq!(manager.applied().len(), 1, "{verb}");
        }
        std::fs::remove_dir_all(&root).expect("a removed scratch directory");
    }

    /// `rule:packaging/the-installer-is-a-sink`: `plan` is the front door, so
    /// every § 2 refusal happens with the manager untouched. The cases above
    /// assert each refusal's code; this one asserts that reaching one costs the
    /// machine nothing.
    #[test]
    fn every_install_refusal_runs_before_the_manager_is_touched() {
        let root = unit_root("refusals");
        let good = argv();

        // One fixture per refusal class § 2 lists, built as the cases above
        // build them.
        let mut wrong_subcommand = good.clone();
        wrong_subcommand[0] = "ast".to_owned();
        let mut relative = good.clone();
        relative[3] = "nvs.toml".to_owned();
        let hook = {
            let mut argv = good.clone();
            argv.push("--fault-inject".to_owned());
            argv
        };

        for (case, argv, bundled, logged, password) in [
            ("a bundle", good.clone(), true, true, None),
            ("the allowlist", wrong_subcommand, false, true, None),
            ("a testing hook", hook, false, true, None),
            ("a relative path", relative, false, true, None),
            ("a password", good.clone(), false, true, Some("hunter2")),
            ("nowhere to write", good.clone(), false, false, None),
        ] {
            let mut asked = request(&argv);
            asked.password = password;
            if password.is_some() {
                asked.account = Some("EXAMPLE\\nvs-web");
            }
            let mut host = host();
            host.from_a_bundle = bundled;
            host.config_names_a_log_destination = logged;

            for platform in [
                registration::Platform::Windows,
                registration::Platform::Linux,
            ] {
                let manager = registration::Recording::default();
                let site = registration::Site {
                    platform,
                    unit_root: &root,
                    manager: &manager,
                };
                let refusal = registration::install(
                    &asked,
                    &host,
                    &registration(),
                    &site,
                    false,
                    &mut std::io::sink(),
                )
                .expect_err(case);
                match refusal {
                    registration::Refused::Installer(diagnostic) => {
                        assert!(diagnostic.code.is_some(), "{case}");
                    }
                    registration::Refused::Manager(error) => {
                        panic!("{case} reached the manager: {error}")
                    }
                }
                assert!(manager.applied().is_empty(), "{case} on {platform:?}");
                assert_eq!(
                    std::fs::read_dir(&root).expect("readable").count(),
                    0,
                    "{case} wrote into {}",
                    root.display()
                );
            }
        }
        std::fs::remove_dir_all(&root).expect("a removed scratch directory");
    }

    /// The process a control is answered over: a count that falls the way a
    /// drain makes one fall, and a reload that records having been entered.
    ///
    /// The count is a list rather than a request that really finishes, because
    /// what the cases below are about is the sequence the manager is told —
    /// a real drain would make that sequence a property of how fast this
    /// machine happens to be.
    #[derive(Debug, Default)]
    struct Hosted {
        /// What `in_flight` answers, one call at a time, and `0` once the list
        /// is spent.
        falling: Mutex<Vec<usize>>,
        /// How many times the one reload function was entered.
        reloads: Mutex<usize>,
    }

    impl nvs_server::control::Controlled for Hosted {
        fn reload(&self) -> Result<nvs_server::control::Report, String> {
            *self.reloads.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            Ok(nvs_server::control::Report {
                applied: vec!["limits.memory".to_owned()],
                ignored: vec!["server.listen"],
                invalidated: 0,
            })
        }

        fn snapshot(&self) -> Arc<nvs_config::snapshot::Snapshot> {
            Arc::new(nvs_config::snapshot::Snapshot::default())
        }

        fn unapplied(&self) -> Vec<&'static str> {
            Vec::new()
        }

        fn in_flight(&self) -> usize {
            let mut falling = self.falling.lock().unwrap_or_else(PoisonError::into_inner);
            if falling.is_empty() {
                0
            } else {
                falling.remove(0)
            }
        }

        fn draining(&self) -> bool {
            self.falling
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_empty()
        }
    }

    /// The service manager a case reads the drain's progress back out of.
    #[derive(Debug, Default)]
    struct Watching(Mutex<Vec<hosted::Progress>>);

    impl hosted::Reporting for Watching {
        fn told(&self, progress: hosted::Progress) {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(progress);
        }
    }

    impl Watching {
        /// Everything the manager was told, in order.
        fn reported(&self) -> Vec<hosted::Progress> {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    /// A pace that waits for nothing, for a case whose in-flight count falls
    /// because the case said so.
    fn at_once() -> hosted::Pace {
        hosted::Pace {
            poll: Duration::ZERO,
            bound: Duration::from_secs(30),
        }
    }

    /// `rule:packaging/a-service-answers-its-manager`, row 1 and row 3: a
    /// machine restart drains in-flight requests instead of killing them, and
    /// what keeps the machine waiting while that happens is a checkpoint the
    /// SCM watches advance.
    ///
    /// The control arrives as its ABI number rather than as [`hosted::Asked`],
    /// because the mapping is half of what this case is about — `PRESHUTDOWN`
    /// is the control § 4 pays for at install, and one read as "no operation"
    /// would drain nothing at the only moment that row exists for.
    ///
    /// The drain is `Draining::detached`: this server's stopping is not this
    /// process's, and the bit a terminating signal sets stays the case about
    /// signals' own (`crate::stop`).
    #[test]
    fn service_run_turns_a_stop_into_a_drain_with_advancing_checkpoints() {
        // The numbers are matched on both platforms and defined by one, so on
        // the one that defines them they are held to it.
        #[cfg(windows)]
        assert_eq!(
            (
                windows_sys::Win32::System::Services::SERVICE_CONTROL_STOP,
                windows_sys::Win32::System::Services::SERVICE_CONTROL_PARAMCHANGE,
                windows_sys::Win32::System::Services::SERVICE_CONTROL_PRESHUTDOWN,
            ),
            (0x0000_0001, 0x0000_0006, 0x0000_000F),
            "the SCM's own constants are not what `hosted` matches",
        );
        assert_eq!(
            hosted::Asked::of(0x0000_000F),
            Some(hosted::Asked::Stop),
            "PRESHUTDOWN is what § 4 asks for at install, and it means drain",
        );
        let process = Hosted {
            falling: Mutex::new(vec![3, 2, 1, 0]),
            reloads: Mutex::new(0),
        };
        let draining = nvs_server::Draining::detached();
        let manager = Watching::default();

        hosted::answer(
            hosted::Asked::of(0x0000_0001).expect("STOP is a control this process answers"),
            &process,
            &draining,
            &manager,
            at_once(),
        );

        assert!(
            draining.is_draining(),
            "the SCM's stop did not enter the drain every other stop enters",
        );
        assert_eq!(
            manager.reported(),
            vec![
                hosted::Progress::Stopping {
                    checkpoint: 1,
                    in_flight: 3,
                },
                hosted::Progress::Stopping {
                    checkpoint: 2,
                    in_flight: 2,
                },
                hosted::Progress::Stopping {
                    checkpoint: 3,
                    in_flight: 1,
                },
                hosted::Progress::Stopping {
                    checkpoint: 4,
                    in_flight: 0,
                },
                hosted::Progress::Stopped,
            ],
            "the checkpoint advances on every report and the last word is STOPPED",
        );
        assert_eq!(
            *process
                .reloads
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
            0,
            "a stop re-read the configuration on its way out",
        );
    }

    /// `rule:packaging/a-service-answers-its-manager`, row 2: a `PARAMCHANGE`
    /// is the configuration reload performed in-process, which is the standing
    /// decision that a reload is **one** function — the control socket,
    /// `systemctl reload` by `ExecReload`, and this all end in
    /// [`nvs_server::control::Controlled::reload`].
    ///
    /// What that function reports is `crate::control`'s case and not this one.
    /// Asserted here instead: it was entered exactly once, nothing was drained
    /// on the way, and the manager was told nothing — the SCM has no state for
    /// a reload, and a service that reported `STOP_PENDING` over one would be
    /// telling the machine it was going away.
    #[test]
    fn service_run_turns_paramchange_into_the_reload() {
        assert_eq!(
            hosted::Asked::of(0x0000_0006),
            Some(hosted::Asked::Reload),
            "PARAMCHANGE is the reload's control",
        );
        assert_eq!(
            hosted::Asked::of(0x0000_0004),
            None,
            "INTERROGATE asks for no operation: a handler answers it with the status it holds",
        );
        let process = Hosted::default();
        let draining = nvs_server::Draining::detached();
        let manager = Watching::default();

        hosted::answer(
            hosted::Asked::Reload,
            &process,
            &draining,
            &manager,
            at_once(),
        );

        assert_eq!(
            *process
                .reloads
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
            1,
            "a PARAMCHANGE did not end in the one reload function",
        );
        assert!(
            !draining.is_draining(),
            "a reload stopped the server it was supposed to re-configure",
        );
        assert!(
            manager.reported().is_empty(),
            "the manager was told about a reload it has no state for",
        );
    }
}
