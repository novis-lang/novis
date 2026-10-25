//! `nvs run` — check a Novis file, then compile and run it.
//!
//! The run itself is [`crate::run_run`], which a bundled executable also
//! starts. This module owns the command's arguments and the two ways a
//! command line reaches that function: from a terminal, and as the stored
//! argv of a service.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::config;

/// Check a Novis file, then compile and run it.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// The file to run.
    file: PathBuf,
    /// Print the lowered IR instead of compiling it.
    #[arg(long)]
    dump_ir: bool,
    /// Print the generated machine code instead of running it.
    #[arg(long, conflicts_with = "dump_ir")]
    dump_asm: bool,
    /// Provoke an engine failure at a named site, for testing containment.
    ///
    /// Scoped to `nvs run` and nothing else: a contained engine panic has
    /// no user-facing trigger by definition, so it needs a hook to be
    /// testable at all — and that hook must never be reachable from a
    /// served request, so `nvs serve` has none.
    // `nvs_runtime::FaultSite` documents each site.
    #[arg(long, value_name = "SITE")]
    fault_inject: Option<FaultSiteArg>,
    /// Count what the program did — statements executed, calls made,
    /// allocations, bytes — and print the four totals on stderr at exit.
    /// Before the program starts, it also prints the line that
    /// `nvs check --count` prints, with the IR instructions added.
    ///
    /// The counts are the same on every machine for the same program and
    /// binary, which is what `bun nv proofs --record-perf` keeps them
    /// for (`rule:testing/bench-counters`). Stdout is untouched, so a
    /// program's own output is what it always was. A counted run starts
    /// no queue worker, even when `[queue] workers` is set, so jobs in
    /// the queue stay there.
    #[arg(long, conflicts_with_all = ["dump_ir", "dump_asm"])]
    count: bool,
    /// Time every call and write a profile file when the program ends.
    ///
    /// `callgrind` writes the time spent in each function, for KCachegrind
    /// or Webgrind. `speedscope` writes every call on a timeline, for
    /// speedscope or the VS Code extension. The file is
    /// `callgrind.out.<pid>` or `profile.<pid>.speedscope.json` in the
    /// current folder. The timing adds a small cost to every call, so
    /// code that makes many small calls looks slower than it is.
    #[arg(long, value_name = "FORMAT", conflicts_with_all = ["dump_ir", "dump_asm", "count"])]
    profile: Option<crate::profile::Format>,
    /// Write the profile to this file instead.
    #[arg(long, value_name = "FILE", requires = "profile")]
    profile_out: Option<PathBuf>,
    /// Answer the request this file describes, rather than run as a
    /// program that is answering none.
    ///
    /// A flag and never an environment variable: whether a program is
    /// answering a request decides what every `Core\Request` member does,
    /// and a variable would make that a semantic change nothing at the
    /// call site shows. `nvs-test` writes one of these files per `.nvst`
    /// case that describes a request and owns the format
    /// (`nvs_test::request`); written by hand, it is how a request is
    /// reproduced without standing a listener up in front of it.
    #[arg(long, value_name = "FILE")]
    request: Option<PathBuf>,
    /// Run as a WebSocket connection whose peer sends the frames this
    /// file lists, and print every frame the program sends.
    ///
    /// What makes `Core\Socket::current()` answer outside a server, so a
    /// connection script can be run, shown and attacked from one file.
    /// The format is `peer`'s module doc. Given beside `--request`, the
    /// program is the request and the peer is the client of the
    /// connection its `Core\Socket::upgrade` opens, which runs once the
    /// request has ended.
    #[arg(long, value_name = "FILE")]
    peer: Option<PathBuf>,
    /// Run as an event-stream connection, and publish the values this
    /// file lists to the topics the program subscribes to.
    ///
    /// What makes `Core\Sse::current()` and `receive()` answer outside a
    /// server: the program is the script a `Core\Sse::upgrade` opens, and
    /// the stream it writes is printed as a client reads it. Beside a
    /// `--request`, the file feeds the stream that request's
    /// `Core\Sse::upgrade` opens instead. The format is `events`'s module
    /// doc.
    #[arg(long, value_name = "FILE", conflicts_with = "peer")]
    events: Option<PathBuf>,
    /// The program's own arguments. `Core\Command::run()` matches them
    /// against the commands the program declares.
    ///
    /// `nvs run` reads none of them, so a program can have its own
    /// `--verbose` option.
    ///
    /// A `--` right after the file is not passed to the program. To pass
    /// `--` as the first argument, write it twice.
    // Everything past the file is the program's, which is why it is
    // `trailing_var_arg`. A leading `--` is the parser's escape token and is
    // spent here, as `cargo run --` spends one: once, and only in that
    // position. A `--` anywhere later is an ordinary word.
    //
    // `rule:tooling/commands-are-compiled` is the command line this hands
    // over. Both halves of the `--` rule are pinned by
    // `tests/conformance/core/cli-arguments-hands-back-a-word-that-looks-like-syntax-as-the-value-it-is.nvst`,
    // since `Core\Cli::arguments` is where the difference is observable.
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        value_name = "ARGS"
    )]
    arguments: Vec<String>,
}

impl Args {
    /// Runs `nvs run` from a command line, logging where a service logs when
    /// a service manager started this process.
    pub(crate) fn run(self, config: &[PathBuf], init: config::Init) -> ExitCode {
        if crate::started_by_a_manager() {
            crate::log_like_a_service();
        }
        self.hosted(config, init)
    }

    /// Runs `nvs run` as `nvs service run` does, which has already set where
    /// the run logs.
    pub(crate) fn hosted(self, config: &[PathBuf], init: config::Init) -> ExitCode {
        crate::run_run(
            &self.file,
            self.dump_ir,
            self.dump_asm,
            self.fault_inject,
            self.count,
            self.profile.map(|format| (format, self.profile_out)),
            self.request.as_deref(),
            self.peer.as_deref(),
            self.events.as_deref(),
            config,
            self.arguments,
            init,
        )
    }
}

/// The closed set of sites `--fault-inject` accepts, one per
/// [`nvs_runtime::FaultSite`].
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(crate) enum FaultSiteArg {
    /// The request's second runtime helper call panics.
    HelperPanic,
}

impl From<FaultSiteArg> for nvs_runtime::FaultSite {
    fn from(arg: FaultSiteArg) -> Self {
        match arg {
            FaultSiteArg::HelperPanic => Self::HelperPanic,
        }
    }
}
