//! `rule:packaging/a-command-that-needs-admin-rights-asks-for-them`: a command that needs admin rights
//! asks for them itself, and only a command on [`asks`]'s closed list ever does.
//!
//! **The list is one `match`.** `nvs service install`, `uninstall`, `start` and `stop` ask before
//! they start, and a dry run asks nothing because it changes nothing. `nvs init` asks only when it
//! must change a folder this account may not change ([`Asks::WhenDenied`]). `nvs run`, `nvs serve`,
//! `nvs test` and every other command never ask, because program code would then run as an
//! administrator.
//!
//! **Asking is starting this binary again, with the same arguments.** On Windows that is
//! `ShellExecuteExW` with the verb `runas` and a hidden window, and the first process waits for the
//! second and returns its exit code. The second process gets a hidden `--elevated-for <pid>` before
//! the arguments, frees its own console and attaches to the first process's, so its output and its
//! prompts are in the terminal the user typed the command in ([`platform::attach`]). It parses its arguments
//! again and runs every check again: nothing the first process found is trusted. A second process
//! that is still not elevated never asks again, so a refused prompt can never loop. On Unix the
//! same arguments run under `sudo` in the same terminal.
//!
//! **Where no prompt can show, nothing waits.** Without a terminal, over SSH or in CI on Windows,
//! under `--no-elevate`, or on Unix without `sudo`, the command prints one line and the command to
//! run, and exits `1` ([`route`]).
//!
//! Cost: one token query per run of a listed command, and nothing for any other command.

use std::ffi::OsString;
use std::process::ExitCode;

use crate::Command;

/// When a command asks for admin rights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Asks {
    /// Never: it runs with the rights it was started with.
    Never,
    /// Before it does anything, because every path through it needs the rights.
    Before,
    /// Only when the system denies it a change it was about to make.
    WhenDenied,
}

/// The closed list of `rule:packaging/a-command-that-needs-admin-rights-asks-for-them`.
pub(crate) fn asks(command: &Command) -> Asks {
    use crate::service::Command as Service;
    match command {
        Command::Service(args) => match &args.command {
            Service::Install(install) if !install.dry_run => Asks::Before,
            Service::Uninstall { dry_run: false, .. }
            | Service::Start { .. }
            | Service::Stop { .. } => Asks::Before,
            Service::Install(_)
            | Service::Uninstall { .. }
            | Service::Status { .. }
            | Service::Run { .. }
            | Service::Unit { .. } => Asks::Never,
        },
        Command::InstallService(install) if !install.dry_run => Asks::Before,
        Command::Init(_) => Asks::WhenDenied,
        _ => Asks::Never,
    }
}

/// What the command line said about asking: `--no-elevate`, and the hidden `--elevated-for` the
/// first process adds.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Options {
    pub(crate) no_elevate: bool,
    pub(crate) elevated_for: Option<u32>,
}

/// The facts [`route`] decides from, read once by [`facts`].
#[derive(Debug, Clone, Copy)]
struct Facts {
    /// This process already has the rights.
    elevated: bool,
    /// Standard input and standard error are both a terminal.
    terminal: bool,
    /// The session is remote, or a CI job, where a Windows consent prompt shows on no screen.
    unseen: bool,
    /// Something can ask: `ShellExecuteExW` on Windows, `sudo` on `PATH` elsewhere.
    asker: bool,
}

/// What a listed command does about its rights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    /// It has them, or never needs them: it runs.
    Run,
    /// It starts itself again with them.
    Ask,
    /// It prints the command to run with them, and exits `1`.
    Tell,
}

/// The one decision, kept free of the system so the tests reach every case without a prompt.
fn route(options: Options, facts: Facts) -> Route {
    if facts.elevated {
        return Route::Run;
    }
    // A process started to have the rights that still does not have them never asks again.
    if options.no_elevate || options.elevated_for.is_some() {
        return Route::Tell;
    }
    if !facts.terminal || facts.unseen || !facts.asker {
        return Route::Tell;
    }
    Route::Ask
}

/// What this process can see right now.
fn facts() -> Facts {
    use std::io::IsTerminal as _;
    Facts {
        elevated: platform::elevated(),
        terminal: std::io::stdin().is_terminal() && std::io::stderr().is_terminal(),
        unseen: platform::UNSEEN
            .iter()
            .any(|var| std::env::var_os(var).is_some()),
        asker: platform::asker(),
    }
}

/// This process's [`Options`], set once by `main` before any command runs.
static OPTIONS: std::sync::OnceLock<Options> = std::sync::OnceLock::new();

/// Names this process's [`Options`], and in a process started with `--elevated-for`, attaches it
/// to that process's terminal before anything prints.
pub(crate) fn set(options: Options) {
    if let Some(pid) = options.elevated_for {
        platform::attach(pid);
    }
    // `main` calls this once, before any reader.
    let _ = OPTIONS.set(options);
}

/// Before a command on [`asks`]'s list that needs the rights now: `None` when it runs in this
/// process, and the exit code to return when it ran elsewhere or cannot run.
///
/// `extra` is added after this process's own arguments, as `nvs init` adds `--yes` once the user
/// has agreed to its change.
pub(crate) fn before(extra: &[&str]) -> Option<ExitCode> {
    let args = arguments(extra);
    let options = OPTIONS.get().copied().unwrap_or_default();
    match route(options, facts()) {
        Route::Run => None,
        Route::Tell => Some(tell(&args)),
        Route::Ask => Some(platform::ask(&args)),
    }
}

/// This process's arguments after the program, then `extra`.
fn arguments(extra: &[&str]) -> Vec<OsString> {
    std::env::args_os()
        .skip(1)
        .chain(extra.iter().map(OsString::from))
        .collect()
}

/// The command shown when nothing can ask: this program's own path and `args`, quoted for the
/// shell it is pasted into.
fn shown(args: &[OsString]) -> String {
    let program = std::env::current_exe().map_or_else(|_| OsString::from("nvs"), OsString::from);
    std::iter::once(&program)
        .chain(args)
        .map(|arg| platform::quote(&arg.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `args` without `--no-elevate` and `--elevated-for`, which mean nothing to a process that
/// already has the rights. Everything after `--` is kept as written.
fn without_asking(args: &[OsString]) -> Vec<OsString> {
    let mut kept = Vec::with_capacity(args.len());
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == "--" {
            kept.push(arg.clone());
            kept.extend(rest.cloned());
            break;
        }
        if arg == "--elevated-for" {
            rest.next();
        } else if arg != "--no-elevate"
            && !arg
                .to_str()
                .is_some_and(|arg| arg.starts_with("--elevated-for="))
        {
            kept.push(arg.clone());
        }
    }
    kept
}

/// [`Route::Tell`]: the line, the command, and exit `1`.
fn tell(args: &[OsString]) -> ExitCode {
    let args = without_asking(args);
    eprintln!("{}\n  {}{}", platform::TELL, platform::PREFIX, shown(&args));
    ExitCode::FAILURE
}

/// An exit code another process returned, as this process's own.
fn exit_code(code: i64) -> ExitCode {
    match u8::try_from(code) {
        Ok(code) => ExitCode::from(code),
        Err(_) => ExitCode::FAILURE,
    }
}

#[cfg(windows)]
mod platform {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStrExt as _;
    use std::process::ExitCode;

    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_CANCELLED, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Security::{
        CheckTokenMembership, CreateWellKnownSid, WinBuiltinAdministratorsSid,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        AttachConsole, FreeConsole, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
        SetStdHandle,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, GetExitCodeProcess, INFINITE, WaitForSingleObject,
    };
    use windows_sys::Win32::UI::Shell::{
        SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
    };

    /// The first line [`super::tell`] prints.
    pub(super) const TELL: &str = "run this in a terminal started as administrator:";
    /// Nothing goes before the command: the terminal itself has the rights.
    pub(super) const PREFIX: &str = "";
    /// A consent prompt shows on the local desktop, so a remote session or a CI job never sees it.
    pub(super) const UNSEEN: [&str; 4] = ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY", "CI"];

    /// `ShellExecuteExW` is part of every Windows desktop and server.
    pub(super) fn asker() -> bool {
        true
    }

    /// Whether this process's token is a member of `BUILTIN\Administrators` with the group
    /// enabled, which a process started without elevation is not.
    #[expect(
        unsafe_code,
        reason = "`CreateWellKnownSid` writes at most `SECURITY_MAX_SID_SIZE` bytes into the buffer \
                  this function owns, and `CheckTokenMembership` with a null token reads that SID and \
                  writes one `BOOL` this function owns; both are unsafe only because they are `extern`"
    )]
    pub(super) fn elevated() -> bool {
        // `SECURITY_MAX_SID_SIZE`.
        let mut sid = [0u8; 68];
        let mut size = 68u32;
        // SAFETY: the buffer is `size` bytes and lives across both calls.
        let made = unsafe {
            CreateWellKnownSid(
                WinBuiltinAdministratorsSid,
                std::ptr::null_mut(),
                sid.as_mut_ptr().cast(),
                &raw mut size,
            )
        };
        if made == 0 {
            return false;
        }
        let mut member = 0;
        // SAFETY: a null token is this thread's effective token, and `sid` is the SID made above.
        let checked = unsafe {
            CheckTokenMembership(
                std::ptr::null_mut(),
                sid.as_mut_ptr().cast(),
                &raw mut member,
            )
        };
        checked != 0 && member != 0
    }

    /// `text` as a null-terminated wide string.
    fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Starts this binary again with `runas`, its window hidden and `--elevated-for` this process,
    /// waits for it, and returns its exit code.
    #[expect(
        unsafe_code,
        reason = "`ShellExecuteExW` reads the null-terminated strings and the struct this function \
                  owns for the length of the call, and the process handle it returns is waited on, \
                  read and closed here once; every call is unsafe only because it is `extern`"
    )]
    pub(super) fn ask(args: &[OsString]) -> ExitCode {
        let program = match std::env::current_exe() {
            Ok(program) => program,
            Err(err) => {
                eprintln!("error: the path of `nvs` could not be read: {err}");
                return ExitCode::FAILURE;
            }
        };
        // SAFETY: takes no argument and cannot fail.
        let pid = unsafe { GetCurrentProcessId() };
        let line = ["--elevated-for".to_string(), pid.to_string()]
            .into_iter()
            .chain(args.iter().map(|arg| arg.to_string_lossy().into_owned()))
            .map(|arg| quote(&arg))
            .collect::<Vec<_>>()
            .join(" ");
        let verb = wide("runas".as_ref());
        let file = wide(program.as_os_str());
        let parameters = wide(line.as_ref());
        let mut info = SHELLEXECUTEINFOW {
            cbSize: u32::try_from(size_of::<SHELLEXECUTEINFOW>()).unwrap_or(u32::MAX),
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            lpVerb: verb.as_ptr(),
            lpFile: file.as_ptr(),
            lpParameters: parameters.as_ptr(),
            // `SW_HIDE`: the new process attaches to this terminal and shows no window of its own.
            nShow: 0,
            ..Default::default()
        };
        // SAFETY: every pointer in `info` is to a buffer above that outlives the call.
        if unsafe { ShellExecuteExW(&raw mut info) } == 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error().and_then(|code| u32::try_from(code).ok()) == Some(ERROR_CANCELLED)
            {
                eprintln!("error: the request for administrator rights was declined");
            } else {
                eprintln!("error: Novis could not ask for administrator rights: {err}");
            }
            return ExitCode::FAILURE;
        }
        if info.hProcess.is_null() {
            eprintln!("error: Novis could not wait for the command started as administrator");
            return ExitCode::FAILURE;
        }
        let mut code = 1u32;
        // SAFETY: `hProcess` is the handle `SEE_MASK_NOCLOSEPROCESS` returned, and it is closed
        // once, after it is read.
        unsafe {
            WaitForSingleObject(info.hProcess, INFINITE);
            if GetExitCodeProcess(info.hProcess, &raw mut code) == 0 {
                code = 1;
            }
            CloseHandle(info.hProcess);
        }
        super::exit_code(i64::from(code))
    }

    /// In a process started with `--elevated-for <pid>`: its output and its prompts go to that
    /// process's terminal. Frees this process's own hidden console, attaches to `pid`'s, and points the three standard
    /// handles at it. When `pid` has no console, the process keeps none, and its exit code is still
    /// returned.
    #[expect(
        unsafe_code,
        reason = "`FreeConsole`, `AttachConsole`, `CreateFileW` on the console names and \
                  `SetStdHandle` take plain values and null-terminated strings this function owns; \
                  each handle it opens is kept as a standard handle for the life of the process"
    )]
    pub(super) fn attach(pid: u32) {
        // SAFETY: plain values, and the result of each is checked.
        unsafe {
            FreeConsole();
            if AttachConsole(pid) == 0 {
                return;
            }
        }
        for (name, slots) in [
            ("CONOUT$", &[STD_OUTPUT_HANDLE, STD_ERROR_HANDLE][..]),
            ("CONIN$", &[STD_INPUT_HANDLE][..]),
        ] {
            let name = wide(name.as_ref());
            // SAFETY: `name` is null-terminated and outlives the call.
            let handle = unsafe {
                CreateFileW(
                    name.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                continue;
            }
            for slot in slots {
                // SAFETY: `handle` is open, and is never closed while it is a standard handle.
                unsafe {
                    SetStdHandle(*slot, handle);
                }
            }
        }
    }

    /// One argument as `CommandLineToArgvW` reads it back: unchanged when it has no space, tab or
    /// quote, and otherwise in quotes, with each backslash before a quote doubled.
    pub(crate) fn quote(arg: &str) -> String {
        if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
            return arg.to_string();
        }
        let mut quoted = String::from("\"");
        let mut backslashes = 0;
        for c in arg.chars() {
            match c {
                '\\' => backslashes += 1,
                '"' => {
                    quoted.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                    backslashes = 0;
                }
                _ => {
                    quoted.extend(std::iter::repeat_n('\\', backslashes));
                    backslashes = 0;
                }
            }
            if c != '\\' {
                quoted.push(c);
            }
        }
        quoted.extend(std::iter::repeat_n('\\', backslashes * 2));
        quoted.push('"');
        quoted
    }
}

#[cfg(not(windows))]
mod platform {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::process::ExitCode;

    /// The first line [`super::tell`] prints.
    pub(super) const TELL: &str = "run this as root:";
    /// `sudo` before the command.
    pub(super) const PREFIX: &str = "sudo ";
    /// `sudo` asks in the terminal itself, so a remote session sees it.
    pub(super) const UNSEEN: [&str; 0] = [];

    /// `sudo` on `PATH`.
    fn sudo() -> Option<PathBuf> {
        std::env::split_paths(&std::env::var_os("PATH")?)
            .map(|dir| dir.join("sudo"))
            .find(|path| path.is_file())
    }

    pub(super) fn asker() -> bool {
        sudo().is_some()
    }

    /// Whether this process runs as root.
    #[expect(
        unsafe_code,
        reason = "`geteuid` has no safe spelling in `std`; it takes no argument, cannot fail, and \
                  returns a plain `uid_t`, so the call is unsafe only because it is `extern`"
    )]
    pub(super) fn elevated() -> bool {
        // SAFETY: see the reason above.
        unsafe { libc::geteuid() == 0 }
    }

    /// Runs this binary again under `sudo` in this terminal, with `--elevated-for` this process,
    /// and returns its exit code.
    pub(super) fn ask(args: &[OsString]) -> ExitCode {
        let (Some(sudo), Ok(program)) = (sudo(), std::env::current_exe()) else {
            return super::tell(args);
        };
        let ran = std::process::Command::new(sudo)
            .arg("--")
            .arg(program)
            .arg("--elevated-for")
            .arg(std::process::id().to_string())
            .args(args)
            .status();
        match ran {
            Ok(status) => super::exit_code(status.code().map_or(1, i64::from)),
            Err(err) => {
                eprintln!("error: `sudo` did not start: {err}");
                ExitCode::FAILURE
            }
        }
    }

    /// In a process started with `--elevated-for <pid>`: nothing, because `sudo` runs in the same
    /// terminal.
    pub(super) fn attach(_pid: u32) {}

    /// One argument as a POSIX shell reads it back: unchanged when it has only safe characters,
    /// and otherwise in single quotes.
    pub(crate) fn quote(arg: &str) -> String {
        let safe = |c: char| c.is_ascii_alphanumeric() || "-_./=:,+@%".contains(c);
        if !arg.is_empty() && arg.chars().all(safe) {
            return arg.to_string();
        }
        format!("'{}'", arg.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::{Asks, Facts, Options, Route, asks, route};
    use crate::Cli;

    /// The command `nvs` parses from `line`.
    fn parsed(line: &str) -> crate::Command {
        let cli = Cli::try_parse_from(std::iter::once("nvs").chain(line.split(' ')))
            .unwrap_or_else(|err| panic!("`nvs {line}` did not parse: {err}"));
        cli.command.expect("a command")
    }

    #[test]
    fn only_listed_commands_ask_for_admin_rights() {
        for line in [
            "service install shop -- serve /srv/shop/index.nvs",
            "install-service shop -- serve /srv/shop/index.nvs",
            "service uninstall shop",
            "service start shop",
            "service stop shop",
        ] {
            assert_eq!(asks(&parsed(line)), Asks::Before, "nvs {line}");
        }
        assert_eq!(asks(&parsed("init")), Asks::WhenDenied);
        for line in [
            "service install shop --dry-run -- serve /srv/shop/index.nvs",
            "service uninstall shop --dry-run",
            "service status shop",
            "service run shop",
            "service unit shop -- serve /srv/shop/index.nvs",
            "run index.nvs",
            "serve index.nvs",
            "test tests",
            "check index.nvs",
            "config check",
            "info",
        ] {
            assert_eq!(asks(&parsed(line)), Asks::Never, "nvs {line}");
        }
    }

    #[test]
    fn no_command_asks_where_no_prompt_can_show() {
        let seen = Facts {
            elevated: false,
            terminal: true,
            unseen: false,
            asker: true,
        };
        let plain = Options::default();
        assert_eq!(route(plain, seen), Route::Ask);
        for (why, options, facts) in [
            (
                "no terminal",
                plain,
                Facts {
                    terminal: false,
                    ..seen
                },
            ),
            (
                "a remote session or CI",
                plain,
                Facts {
                    unseen: true,
                    ..seen
                },
            ),
            (
                "nothing can ask",
                plain,
                Facts {
                    asker: false,
                    ..seen
                },
            ),
            (
                "`--no-elevate`",
                Options {
                    no_elevate: true,
                    ..plain
                },
                seen,
            ),
            (
                "a process already started to have the rights",
                Options {
                    elevated_for: Some(7),
                    ..plain
                },
                seen,
            ),
        ] {
            assert_eq!(route(options, facts), Route::Tell, "{why}");
        }
        for facts in [
            seen,
            Facts {
                terminal: false,
                ..seen
            },
        ] {
            let elevated = Facts {
                elevated: true,
                ..facts
            };
            assert_eq!(route(plain, elevated), Route::Run);
        }
        // `--elevated-for` and `--no-elevate` parse before and after the command.
        let cli = Cli::try_parse_from(["nvs", "--elevated-for", "7", "service", "start", "shop"])
            .expect("parses");
        assert_eq!(cli.elevated_for, Some(7));
        let cli = Cli::try_parse_from(["nvs", "service", "stop", "shop", "--no-elevate"])
            .expect("parses");
        assert!(cli.no_elevate);
        // The command printed instead drops both, and keeps what follows `--`.
        let args = [
            "--elevated-for",
            "7",
            "service",
            "install",
            "shop",
            "--no-elevate",
            "--",
            "serve",
            "--no-elevate",
        ];
        let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
        assert_eq!(
            super::without_asking(&args),
            ["service", "install", "shop", "--", "serve", "--no-elevate"]
        );
    }

    #[cfg(windows)]
    #[test]
    fn an_argument_is_quoted_as_windows_reads_it_back() {
        use super::platform::quote;
        assert_eq!(quote("serve"), "serve");
        assert_eq!(quote(r"C:\Program Files\nvs"), r#""C:\Program Files\nvs""#);
        assert_eq!(quote(r"a\"), r"a\");
        assert_eq!(quote(r"a b\"), r#""a b\\""#);
        assert_eq!(quote(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(quote(""), r#""""#);
    }
}
