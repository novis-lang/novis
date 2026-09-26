//! `Core\Process` — `rule:core-classes/process-is-argv-only`'s
//! one way to run another program, over
//! [`nvs_runtime::capability::exec`](nvs_runtime::capability::exec)'s door.
//!
//! # Decision: there is no shell-string form, so there is nothing here to escape
//!
//! `rule:core-classes/process-is-argv-only` is the whole of it, and the shape of this module is what enforces
//! it: `run` takes a `string $path` and an `array<string> $argv`, and no member
//! of this class takes a command line. PHP's `exec`, `system`, `shell_exec`,
//! `passthru` and the backtick operator are all *one* member here, because the
//! four of them differ only in what they do with output — and none of that
//! difference is worth a second parser between the program and the child. The
//! target kinds that would re-introduce one anyway (`.bat`, `.cmd`, `.ps1`) are
//! refused by the door, on every platform, for the reason
//! [`nvs_runtime::capability::exec`] states.
//!
//! # Decision: the result is a `Core`-owned instance, not a shape
//!
//! `rule:core-classes/process-run` writes `$result->exitCode()` and `$result->stderr()`, and this
//! module answers with exactly that: [`RESULT`] is an ordinary
//! [`crate::instance`] class with three zero-argument members over three slots.
//! A shape — `{exitCode, stdout, stderr}`, read as properties — was the
//! alternative, and it loses twice. Its field names land in the *interned type*
//! of every call site, so adding a fourth field later is a change to a type
//! programs have written down; and a shape has no room for a member that
//! computes, which is where § 1's later signatures (a `$result->ok()`, a
//! decoded `text()`) would have to go. `Core\Regex\Match` is the precedent, for
//! the same reasons stated there.
//!
//! **Captured output is `bytes`, never `string`** — `rule:core-classes/process-run`, over
//! `rule:types/bytes`'s UTF-8 guarantee,
//! which cannot be assumed of an arbitrary child's output. A caller who knows
//! the output is text writes `as string`, which is the checked conversion that
//! throws rather than the silent replacement-character mangling PHP gives.
//!
//! # Decision: the wait happens off the core, and nothing else about it moved
//!
//! `rule:core-classes/process-run`. A child process has no readiness a reactor can poll — no
//! descriptor of ours becomes ready when it exits — so waiting for one is
//! `rule:http-server/a-core-is-never-blocked-on-a-syscall`'s
//! other case, and [`nvs_host::blocking::run`] is the only spelling of it in
//! this tree. [`wait_off_core`] is that call and the whole of it: the core is
//! handed back while the child runs, the task resumes on a remote wake once
//! the pool thread has the output, and a neighbouring request served by the
//! same worker runs in between.
//!
//! Nothing else about the member changed when the wait moved — its signature,
//! its door and its result are what § 1 already specified, and what a caller
//! can observe is unchanged. Off a core, which is every CLI program, the
//! closure is simply called on this thread, so the cheap case stays free.
//!
//! # Decision: `[limits] max_output` bounds the capture, read once per call
//!
//! `rule:core-classes/process-run` reuses the directive an operator already
//! writes rather than adding a cap of this member's own, and
//! [`nvs_runtime::Ctx::intake_limit`] is that reading: the same number the
//! response ceiling is, in bytes-per-call rather than bytes-per-request. Both
//! pipes are drained through it, so neither stream is ever held unbounded, and a
//! child that keeps writing past it is killed by whichever reader notices —
//! [`drain`] owns the mechanism and what the second thread costs. The ceiling is
//! the *pair*: each stream stops one byte past it, and what
//! [`nvs_runtime::Ctx::intake_breach`] is asked is the two lengths added, so a
//! child that splits its output evenly between them is refused as well.
//!
//! The refusal is a **throw**, not `rule:errors/on-limit`'s `FATAL`: nothing has
//! reached the response, so the request has exceeded nothing and a caller who
//! ran a chattier child than it meant to can catch this and run it differently.
//! `Core\IO::read` refuses out of the same pair of methods, so the two members
//! answer a file and a child the same way.
//!
//! **What it spends:** the child's whole stdout and stderr, once each, as one
//! `bytes` value per stream held for as long as the program holds the result —
//! now bounded rather than trusted — plus one object allocation of three slots,
//! charged to the request that asked, and one OS thread for the child's
//! lifetime that [`drain`] explains.
//!
//! # Decision: a `spawn` files a trace event and a `run` does not
//!
//! A child process is a unit of work in the request's causal graph, which is
//! what `rule:observability/four-kinds-become-a-span` admits a span for, so
//! [`nvs_core_process_spawn`] opens `rule:observability/spawn-is-its-own-event`
//! 's event where the child starts and [`nvs_core_process_handle_wait`] closes
//! it where the child is joined. `run` opens none: it starts and joins inside
//! one call the call probe already brackets, and a second event over that same
//! interval is one interval every consumer would then count twice.
//!
//! The event carries the form and the two timestamps, and never the path or an
//! argument — a trace is a `secret` sink
//! (`rule:observability/trace-events-carry-a-kind`), and a target is the
//! request's business rather than the exporter's. A handle the program never
//! waits on leaves the event unjoined, which is exactly what a child killed
//! with its task is.
//!
//! **What it spends:** one trace event per `spawn` while `TRACE` or `PROFILE`
//! is on, charged to the request and released with it, and one `Option` per
//! held child; a flag test and a predicted-not-taken branch when both bits are
//! off.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Output};
use std::sync::Mutex;

use nvs_runtime::{Ctx, Fault, HeldChild, NvsStr, Tag, Value};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// This class's fully-qualified name, in one place so the registry row, the
/// [`crate::registry::CAPABILITIES`] row and every consumer that matches on it
/// cannot drift apart.
pub(crate) const NAME: &str = "Core\\Process";

/// The spelling both of `run`'s refusals name, and the one the door prints.
const RUN_MEMBER: &str = "Core\\Process::run";

/// [`RUN_MEMBER`]'s twin for the member that does not wait.
const SPAWN_MEMBER: &str = "Core\\Process::spawn";

/// How much one [`nvs_core_process_handle_read_stdout`] takes at a time.
///
/// A syscall-count choice and not a footprint one: the buffer is this call's
/// own and what comes back is a `bytes` the size of what was actually read, so
/// a bigger number costs nothing a child that writes little ever pays. Larger
/// than a pipe's own buffer on every platform this runs on, so a chunk is one
/// read rather than a loop.
const CHUNK: usize = 64 * 1024;

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "run",
            names: &["path", "argv"],
            // The path is a sink for `Core\IO`'s reason and one more of its own:
            // `rule:core-classes/process-run` says `$path` and every element of `$argv` are plain
            // `string`, because a name the program did not choose is the whole of
            // what a command injection is. The elements carry no mark of their own
            // — `nvs_types::core_lib::qual_of` reads no nested classification, and
            // none is needed: `array<tainted string>` is not `array<string>`, so a
            // tainted element is refused by the ordinary argument check.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(RESULT_NAME),
            symbol: "nvs_core_process_run",
            doc: Some(&RUN_DOC),
        },
        CoreMethod {
            name: "spawn",
            names: &["path", "argv"],
            // `run`'s parameters exactly, and deliberately not a shape of its
            // own: `rule:core-classes/process-is-argv-only` is a property of
            // the class and not of one member, and
            // `there_is_no_shell_string_form_of_run_or_spawn` reads this row
            // the same way it reads `run`'s.
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(HANDLE_NAME),
            symbol: "nvs_core_process_spawn",
            doc: Some(&SPAWN_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Process`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Runs other programs. A program is started directly with a list of arguments, never \
            through a shell, so an argument cannot add a second command. `run` waits for the \
            program to end, and `spawn` returns a handle while it runs. Both need the \
            `process.exec` capability. This replaces PHP's `exec`, `system`, `shell_exec`, \
            `passthru`, `proc_open` and the backtick operator.",
};

/// `Core\Process::run`'s reference card — `rule:core-api/reference-card`.
const RUN_DOC: MethodDoc = MethodDoc {
    short: "Runs the program at `$path` with the arguments in `$argv`, and waits until it ends. \
            The program is started directly, never through a shell, so nothing needs escaping. \
            Needs the `process.exec` capability for the program. This replaces PHP's `exec`, \
            `system`, `shell_exec`, `passthru` and the backtick operator.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The program to start, as an absolute path or a path relative to the working \
                   directory. `PATH` is not searched, so `ls` means a file named `ls` in the \
                   working directory.",
            shape: &[],
        },
        ParamDoc {
            name: "argv",
            desc: "The arguments, one in each element: `[\"-n\", \"1\", $host]`. An element with \
                   a space, a quote or a `;` in it is still one argument, on every platform.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Process\\Result` with the exit code and everything the program wrote to its \
          output and to its error output. The program cannot write to this program's own \
          output, so the two never mix.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`process.exec` does not allow this program, or it is a `.bat`, `.cmd` or \
                   `.ps1` file. Those are not allowed on any platform, because Windows starts \
                   them through a shell. The error is also thrown when the program writes more \
                   than `[limits] max_output` in total. Then the program is stopped and nothing \
                   is returned.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The program could not be started or waited for. For example, nothing is at \
                   the path, or the file is not a program.",
        },
    ],
};

/// `Core\Process::spawn`'s reference card — `rule:core-api/reference-card`.
const SPAWN_DOC: MethodDoc = MethodDoc {
    short: "Starts the program at `$path` with the arguments in `$argv`, and returns at once while \
            the program runs. The handle it returns reads the program's output and writes to its \
            input. The program is started directly, never through a shell. Needs the \
            `process.exec` capability for the program. This replaces PHP's `proc_open`.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The program to start, as an absolute path or a path relative to the working \
                   directory. `PATH` is not searched.",
            shape: &[],
        },
        ParamDoc {
            name: "argv",
            desc: "The arguments, one in each element. An element with a space, a quote or a `;` \
                   in it is still one argument, on every platform.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Process\\Handle` for the running program. Other requests keep running while this \
          one waits on the handle. The program is stopped when the request that started it ends.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "`process.exec` does not allow this program, or it is a `.bat`, `.cmd` or `.ps1` \
                   file. Those are not allowed on any platform, because Windows starts them \
                   through a shell.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The program could not be started. For example, nothing is at the path, or the \
                   file is not a program.",
        },
    ],
};

/// `Core\Process\Handle`'s fully-qualified name, written once for
/// [`RESULT_NAME`]'s reason.
const HANDLE_NAME: &str = r"Core\Process\Handle";

/// [`HANDLE`]'s first slot: the key its live child is filed under in the task.
const CHILD_SLOT: usize = 0;

/// [`HANDLE`]'s second slot: the path it was started on, kept so that every
/// refusal a member of this class raises can name the program the way the
/// program wrote it — `Core\IO\File`'s second slot, for its reason.
const HANDLE_PATH_SLOT: usize = 1;

/// `rule:core-classes/process-spawn`'s `ProcessHandle` — what
/// [`nvs_core_process_spawn`] answers with.
///
/// # Decision: one handle covers `passthru` and `proc_open` both
///
/// `rule:core-classes/process-spawn`. PHP splits streaming a child's output
/// straight through from controlling its pipes, and the difference between the
/// two is *which members a caller uses*, not two kinds of process — so there is
/// one type here with five members, and a program that only ever calls
/// `readStdout` has written `passthru` without there being a second class for
/// it.
///
/// # Decision: the slot holds a key, and the task holds the child
///
/// [`crate::instance`]'s first decision is the home of why a `Core` instance's
/// slots hold Novis values and a native thing therefore lives in a table the
/// runtime owns; `Core\IO\File` is the same shape one resource over. What is
/// this class's own is *which* table: a child is filed against the task, and
/// [`nvs_runtime::HeldChild`]'s `Drop` **kills** it, so a handle the program
/// stops reading from leaves no process behind. A descriptor is given back when
/// a request ends; a child has to be ended.
///
/// **What it spends:** one object allocation of two slots, plus the handle and
/// up to three pipes [`nvs_runtime::Ctx::hold_spawned_child`] accounts for —
/// O(children in flight), charged to the task that spawned them.
pub(crate) const HANDLE: CoreClass = CoreClass {
    name: HANDLE_NAME,
    doc: Some(&HANDLE_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "readStdout",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Bytes),
            symbol: "nvs_core_process_handle_read_stdout",
            doc: Some(&READ_STDOUT_DOC),
        },
        CoreMethod {
            name: "readStderr",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Bytes),
            symbol: "nvs_core_process_handle_read_stderr",
            doc: Some(&READ_STDERR_DOC),
        },
        CoreMethod {
            name: "writeStdin",
            names: &["data"],
            // The one parameter of this class that is not a program's name, and
            // the one that is **not** a sink: what goes down a child's standard
            // input is data the child parses on its own terms, exactly as a log
            // field is data, and the injection this class exists to close is in
            // the argv. `Qual::Neutral` rather than unclassified `Bytes`, which
            // would refuse `tainted` under
            // `rule:security/unclassified-parameter-refuses-tainted` and leave
            // piping a request body through a converter unwritable.
            params: &[CoreTy::Blob(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_process_handle_write_stdin",
            doc: Some(&WRITE_STDIN_DOC),
        },
        CoreMethod {
            name: "wait",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(RESULT_NAME),
            symbol: "nvs_core_process_handle_wait",
            doc: Some(&HANDLE_WAIT_DOC),
        },
        CoreMethod {
            name: "kill",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_process_handle_kill",
            doc: Some(&HANDLE_KILL_DOC),
        },
    ],
    slots: &["child", "path"],
    constants: &[],
};

/// `Core\Process\Handle`'s class card — `rule:core-api/reference-card`.
const HANDLE_CARD: ClassDoc = ClassDoc {
    short: "A program that `Core\\Process::spawn` started and that may still be running. \
            `readStdout` and `readStderr` read its output a part at a time, and `writeStdin` \
            writes to its input. `wait` waits for the end and returns a `Core\\Process\\Result`, \
            and `kill` stops the program at once. A program that is still running when the \
            request ends is stopped.",
};

/// `Core\Process\Handle::readStdout`'s reference card — `rule:core-api/reference-card`.
const READ_STDOUT_DOC: MethodDoc = MethodDoc {
    short: "Waits for the child to write something to its standard output and answers it — the \
            read half of `proc_open`'s pipes, and the whole of `passthru` when what a program does \
            with each chunk is write it to the response. The wait suspends this coroutine and \
            hands the core back, so other requests on it keep running.",
    params: &[],
    ret: "The octets the child wrote, as `bytes` for the reason `Core\\Process\\Result::stdout` \
          states, or `null` once the stream has ended. A chunk is whatever had arrived, never a \
          line and never the whole output: a program that wants all of it calls `run` instead.",
    errors: &[ErrorDoc {
        error: "IOError",
        desc: "The operating system failed the read. The stream is closed afterwards, so a \
               later call answers `null` rather than failing again.",
    }],
};

/// `Core\Process\Handle::readStderr`'s reference card — `rule:core-api/reference-card`.
const READ_STDERR_DOC: MethodDoc = MethodDoc {
    short: "`readStdout` on the other stream, kept separate from it exactly as a completed run's \
            two captures are.",
    params: &[],
    ret: "The octets the child wrote to its standard error, or `null` once that stream has ended.",
    errors: &[ErrorDoc {
        error: "IOError",
        desc: "The operating system failed the read, on `readStdout`'s terms.",
    }],
};

/// `Core\Process\Handle::writeStdin`'s reference card — `rule:core-api/reference-card`.
const WRITE_STDIN_DOC: MethodDoc = MethodDoc {
    short: "Writes `$data` to the child's standard input, suspending this coroutine until the \
            child has taken it. A `tainted` value is accepted here and nowhere else in this \
            class: standard input is data the child reads, where a path and an argument are a \
            command this process builds.",
    params: &[ParamDoc {
        name: "data",
        desc: "The octets to write, whole. A `string` converts on the way in.",
        shape: &[],
    }],
    ret: "Nothing. The child's input stays open for the next write and is closed by `wait`, which \
          is what lets a child reading to the end of its input ever see one.",
    errors: &[ErrorDoc {
        error: "IOError",
        desc: "The operating system failed the write — most often a child that has already \
               exited, which closes the pipe this end was writing into.",
    }],
};

/// `Core\Process\Handle::wait`'s reference card — `rule:core-api/reference-card`.
const HANDLE_WAIT_DOC: MethodDoc = MethodDoc {
    short: "Closes the child's standard input, waits for it to exit, and answers what it did — \
            `proc_close`, without the caller having to remember that closing the pipes comes \
            first. The wait suspends this coroutine exactly as `run`'s does.",
    params: &[],
    ret: "A `Core\\Process\\Result` carrying the exit status and whatever neither `readStdout` nor \
          `readStderr` had already taken. A program that streamed the whole output gets two empty \
          captures, which is the answer and not a loss.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "What the child had left to write is more than `[limits] max_output`, on \
                   `Core\\Process::run`'s terms — the ceiling on a response is the ceiling on one \
                   capture too.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The operating system failed to drain a pipe or to reap the child.",
        },
    ],
};

/// `Core\Process\Handle::kill`'s reference card — `rule:core-api/reference-card`.
const HANDLE_KILL_DOC: MethodDoc = MethodDoc {
    short: "Stops the program at once, whatever it is doing. If the program has already ended, \
            `kill` does nothing and throws no error. A program that nobody stops is stopped when \
            the request that started it ends.",
    params: &[],
    ret: "Nothing. Call `wait` afterwards to get the exit code. It is not `0` for a program that \
          was stopped.",
    errors: &[ErrorDoc {
        error: "IOError",
        desc: "The operating system could not stop the program.",
    }],
};

/// `Core\Process\Result`'s fully-qualified name, written once — [`RESULT`]
/// declares it and the [`CoreTy::Instance`] naming it resolves against
/// [`crate::registry::CLASSES`], so the two cannot drift apart.
const RESULT_NAME: &str = r"Core\Process\Result";

/// [`RESULT`]'s slots, in declaration order.
const EXIT_CODE_SLOT: usize = 0;
/// See [`EXIT_CODE_SLOT`].
const STDOUT_SLOT: usize = 1;
/// See [`EXIT_CODE_SLOT`].
const STDERR_SLOT: usize = 2;

/// The exit code answered for a child that exited without one — killed by a
/// signal, on the platforms that have them.
///
/// `-1` and not a throw: a child that a signal stopped *ran*, and its output is
/// as real as any other child's, so a caller who cares which happened compares
/// against a number rather than catching. No exit status a process can report
/// is negative, so the value cannot collide with one.
const SIGNALLED: i32 = -1;

/// `rule:core-classes/process-run`'s `ProcessResult` — what [`nvs_core_process_run`] answers with.
///
/// Three members over three slots and no static member at all: a result is only
/// ever produced by a run. This module's docs own why it is an instance rather
/// than a shape; what belongs here is that every member is a slot read, because
/// the child has already exited by the time one exists — there is nothing left
/// to compute and nothing left to fail.
pub(crate) const RESULT: CoreClass = CoreClass {
    name: RESULT_NAME,
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "exitCode",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_process_result_exit_code",
            doc: Some(&EXIT_CODE_DOC),
        },
        CoreMethod {
            name: "stdout",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_process_result_stdout",
            doc: Some(&STDOUT_DOC),
        },
        CoreMethod {
            name: "stderr",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_process_result_stderr",
            doc: Some(&STDERR_DOC),
        },
    ],
    slots: &["exitCode", "stdout", "stderr"],
    constants: &[],
};

/// `Core\Process\Result::exitCode`'s reference card — `rule:core-api/reference-card`.
const EXIT_CODE_DOC: MethodDoc = MethodDoc {
    short: "The status the child exited with — `$?`, and the third out-parameter `exec` writes.",
    params: &[],
    ret: "The exit status, `0` for success by the convention every operating system shares, and \
          `-1` for a child a signal stopped before it could report one.",
    errors: &[],
};

/// `Core\Process\Result::stdout`'s reference card — `rule:core-api/reference-card`.
const STDOUT_DOC: MethodDoc = MethodDoc {
    short: "Everything the child wrote to its standard output, captured whole.",
    params: &[],
    ret: "The octets, as `bytes` and not `string`: Novis guarantees a `string` is UTF-8, and a \
          child process makes no such promise about what it writes. A caller who knows the output \
          is text writes `as string`, which throws on a sequence that is not.",
    errors: &[],
};

/// `Core\Process\Result::stderr`'s reference card — `rule:core-api/reference-card`.
const STDERR_DOC: MethodDoc = MethodDoc {
    short: "Everything the child wrote to its standard error, captured whole and kept separate \
            from `stdout` — the stream PHP's `exec` discards and `shell_exec` merges.",
    params: &[],
    ret: "The octets, as `bytes`, for the reason `stdout` states.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_process_run" => (nvs_core_process_run as *const ()).cast(),
        "nvs_core_process_spawn" => (nvs_core_process_spawn as *const ()).cast(),
        "nvs_core_process_handle_read_stdout" => {
            (nvs_core_process_handle_read_stdout as *const ()).cast()
        }
        "nvs_core_process_handle_read_stderr" => {
            (nvs_core_process_handle_read_stderr as *const ()).cast()
        }
        "nvs_core_process_handle_write_stdin" => {
            (nvs_core_process_handle_write_stdin as *const ()).cast()
        }
        "nvs_core_process_handle_wait" => (nvs_core_process_handle_wait as *const ()).cast(),
        "nvs_core_process_handle_kill" => (nvs_core_process_handle_kill as *const ()).cast(),
        "nvs_core_process_result_exit_code" => {
            (nvs_core_process_result_exit_code as *const ()).cast()
        }
        "nvs_core_process_result_stdout" => (nvs_core_process_result_stdout as *const ()).cast(),
        "nvs_core_process_result_stderr" => (nvs_core_process_result_stderr as *const ()).cast(),
        _ => return None,
    })
}

/// One `string` argument, or the engine fault a wrongly-tagged one is: the
/// signature is checked at compile time, so a bad tag here is a lowering bug
/// and not something a program can provoke.
fn text<'a>(value: &'a Value, what: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{RUN_MEMBER} expected {:?} for {what}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

/// The `array<string>` argument, copied out element by element.
///
/// Owned `String`s rather than borrowed slices, because the argv has to outlive
/// the walk that reads it and a slot's `Value` is borrowed rather than
/// retained. An argv is a handful of short strings, so the copy is not a cost
/// worth an unsafe lifetime argument.
///
/// # Errors
///
/// A `Fault::fatal` for an element that is not text, on the same reading as
/// [`text`]: the parameter is `array<string>` in [`CLASS`], so a program cannot
/// reach this.
fn argv_of(value: &Value) -> Result<Vec<String>, Fault> {
    let array = value.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{RUN_MEMBER} expected {:?} for its argv, got tag {}",
            Tag::Array,
            value.tag_byte()
        ))
    })?;
    let mut argv = Vec::new();
    let mut from = 0usize;
    loop {
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live allocation, \
                      so it is live for the length of this call, and `from` only \
                      ever advances past a slot this same cursor reported"
        )]
        let (slot, element) = unsafe {
            let slot = nvs_runtime::nvs_array_next_slot(array, from);
            let Ok(slot) = usize::try_from(slot) else {
                break;
            };
            let mut element = Value::null();
            nvs_runtime::nvs_array_value_at(array, slot, &raw mut element);
            (slot, element)
        };
        from = slot + 1;
        argv.push(text(&element, "an argv element")?.to_owned());
    }
    Ok(argv)
}

nvs_runtime::nvs_helper! {
    /// `Core\Process::run(string $path, array<string> $argv): Core\Process\Result`
    /// — replacing `exec`, `system`, `shell_exec`, `passthru` and the backtick
    /// operator, each in one call.
    ///
    /// The door is [`nvs_runtime::capability::exec`], which asks `process.exec`
    /// first and the target's kind second; everything this body adds is the
    /// wait and the capture, and the door pipes all three streams precisely so
    /// that no child inherits this process's own.
    ///
    /// [`wait_off_core`] owns which thread the wait occupies, and this module's
    /// *Decision: `[limits] max_output` bounds the capture* owns the ceiling the
    /// two captures are read through.
    fn nvs_core_process_run(ctx, args: [2]) {
        let program = text(&args[0], "its path")?;
        let argv = argv_of(&args[1])?;
        let borrowed: Vec<&str> = argv.iter().map(String::as_str).collect();
        let path = Path::new(program);
        let bound = ctx.intake_bound();
        let child = nvs_runtime::capability::exec(ctx, path, &borrowed, RUN_MEMBER)?;
        let output = wait_off_core(child, bound)
            .map_err(|err| nvs_runtime::capability::io_failure(RUN_MEMBER, path, &err))?;
        let taken = output.stdout.len() + output.stderr.len();
        if let Some(over) = ctx.intake_breach(RUN_MEMBER, taken) {
            return Err(over);
        }
        Ok(crate::instance::build(
            &RESULT,
            [
                Value::int(i64::from(output.status.code().unwrap_or(SIGNALLED))),
                Value::bytes(NvsStr::new(&output.stdout)),
                Value::bytes(NvsStr::new(&output.stderr)),
            ],
        ))
    }
}

/// The child's status and both of its streams, waited for **off this core** and
/// held to `bound` — `rule:core-classes/process-run`, and this module's
/// *Decision: the wait happens off the core*.
///
/// Both streams are read to the end before the status is taken: waiting first
/// and reading after deadlocks the moment a child fills a pipe buffer. That is
/// why the whole three-way wait goes to the pool as one job rather than the exit
/// alone — there is no point at which reading a pipe and waiting for the exit
/// are separable, so there is no smaller thing to hand off.
///
/// Named rather than written inline because it is the only part of
/// `Core\Process` a case can hold still while a neighbouring task runs:
/// `a_process_wait_suspends_its_coroutine_through_the_blocking_pool` drives
/// this on a scheduler, and the member around it needs a compiled program.
///
/// `bound` is [`nvs_runtime::Ctx::intake_bound`] — one byte past the ceiling, or
/// `u64::MAX` for a request under none, which is the whole of what this layer
/// knows about the directive.
///
/// **What it spends:** one pool thread for the child's lifetime, out of
/// [`nvs_host::blocking::bound`]'s per-worker bound — and off a core, where
/// `run` calls the closure on this thread, nothing at all.
///
/// # Errors
///
/// Whatever the operating system said about waiting for the child or draining
/// its pipes. The caller turns it into a `Fault`, since only it knows the path
/// to name.
fn wait_off_core(child: Child, bound: u64) -> std::io::Result<Output> {
    nvs_host::blocking::run(move || drain(child, bound))
}

/// Both pipes drained concurrently, neither past `bound`, and then the exit
/// status — what [`Child::wait_with_output`] does, plus the ceiling it has
/// nowhere to take.
///
/// **The second thread is the cost of the ceiling.** `wait_with_output` reads
/// both pipes at once without one, but it reads them to the end, and there is no
/// way to hand it a limit or to stop it once a child has decided to write
/// forever. Draining them here means one reader per pipe, since a single thread
/// reading one of them blocks while the other's buffer fills and the child stops
/// making progress — the deadlock the member has always been written around. One
/// thread spawn against a process spawn is noise, and it lasts exactly as long
/// as the child does.
///
/// The kill is what makes the bound a bound rather than a truncation: a reader
/// that fills its whole allowance stops reading, so without it the child would
/// keep writing into a pipe nobody drains and hang. It is taken through a
/// [`Mutex`] because either reader may be the one that notices, and the lock is
/// held for that call alone.
///
/// # Errors
///
/// Whatever the operating system said about draining a pipe or reaping the
/// child.
fn drain(mut child: Child, bound: u64) -> std::io::Result<Output> {
    let out_pipe = child.stdout.take();
    let err_pipe = child.stderr.take();
    let child = Mutex::new(child);
    let (stdout, stderr) = std::thread::scope(|scope| {
        let stderr = scope.spawn(|| read_bounded(err_pipe, bound, &child));
        let stdout = read_bounded(out_pipe, bound, &child);
        (stdout, stderr.join())
    });
    let stderr = stderr.unwrap_or_else(|panic| std::panic::resume_unwind(panic))?;
    let status = child
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .wait()?;
    Ok(Output {
        status,
        stdout: stdout?,
        stderr,
    })
}

/// One pipe read to its end or to `bound`, whichever comes first, killing the
/// child at the second.
///
/// A stream this process never piped answers empty rather than failing: the
/// caller took the handle out of the child, and a `None` there is a child
/// started without that pipe rather than an error to report.
///
/// # Errors
///
/// Whatever the operating system said about the read.
fn read_bounded<R: Read>(
    pipe: Option<R>,
    bound: u64,
    child: &Mutex<Child>,
) -> std::io::Result<Vec<u8>> {
    let Some(pipe) = pipe else {
        return Ok(Vec::new());
    };
    let mut held = Vec::new();
    let read = pipe.take(bound).read_to_end(&mut held)?;
    if u64::try_from(read).unwrap_or(u64::MAX) >= bound
        && let Ok(mut child) = child.lock()
    {
        let _ = child.kill();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `$result->exitCode(): int` — the status the child exited with.
    fn nvs_core_process_result_exit_code(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &RESULT, "exitCode")?;
        Ok(crate::instance::slot(receiver, EXIT_CODE_SLOT))
    }
}

nvs_runtime::nvs_helper! {
    /// `$result->stdout(): bytes` — everything the child wrote to its standard
    /// output.
    fn nvs_core_process_result_stdout(_ctx, args: [1]) {
        captured(args[0], STDOUT_SLOT, "stdout")
    }
}

nvs_runtime::nvs_helper! {
    /// `$result->stderr(): bytes` — everything the child wrote to its standard
    /// error.
    fn nvs_core_process_result_stderr(_ctx, args: [1]) {
        captured(args[0], STDERR_SLOT, "stderr")
    }
}

/// One captured stream out of a [`RESULT`] receiver, **retained** — the caller
/// takes over the reference this answers with, since
/// [`crate::instance::slot`] borrows.
///
/// # Errors
///
/// A `Fault::fatal` if the receiver is not one of this class's instances, which
/// the compile-time signature rules out.
fn captured(receiver: Value, index: usize, member: &str) -> Result<Value, Fault> {
    let object = crate::instance::receiver(receiver, &RESULT, member)?;
    let held = crate::instance::slot(object, index);
    #[expect(
        unsafe_code,
        reason = "the receiver owns the reference this borrowed read returned, \
                  so the caller needs one of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `Core\Process::spawn(string $path, array<string> $argv): Core\Process\Handle`
    /// — replacing `proc_open` and `passthru`.
    ///
    /// [`nvs_core_process_run`]'s first three lines exactly, and then the
    /// difference: nothing is waited for, and the child's own span is opened
    /// here for [`nvs_core_process_handle_wait`] to close. The door is the same
    /// [`nvs_runtime::capability::exec`], asked the same way and piping the
    /// same three streams, so a target this member starts is one `run` would
    /// have started and a target it refuses is one `run` refuses.
    fn nvs_core_process_spawn(ctx, args: [2]) {
        let program = text(&args[0], "its path")?;
        let argv = argv_of(&args[1])?;
        let borrowed: Vec<&str> = argv.iter().map(String::as_str).collect();
        let path = Path::new(program);
        let child = nvs_runtime::capability::exec(ctx, path, &borrowed, SPAWN_MEMBER)?;
        let mut held = HeldChild::new(child);
        held.spawn_event = ctx.open_spawn(nvs_runtime::SpawnForm::Process);
        let key = ctx.hold_spawned_child(held);
        #[expect(
            unsafe_code,
            reason = "the path is owned by the caller's argument, which outlives \
                      this call, so the copy this handle keeps for its refusals \
                      needs a reference of its own"
        )]
        unsafe {
            args[0].retain();
        }
        Ok(crate::instance::build(&HANDLE, [Value::uint(key), args[0]]))
    }
}

/// The key a [`HANDLE`] receiver's child is filed under, and the path it was
/// started on — [`crate::io`]'s `handle_of` one resource over, for its reason.
///
/// # Errors
///
/// A `Fault::fatal` if the receiver is not one of this class's instances or its
/// first slot holds the wrong tag: both slots are written by
/// [`nvs_core_process_spawn`] and by nothing else.
fn handle_of(value: Value, member: &str) -> Result<(u64, Value), Fault> {
    let receiver = crate::instance::receiver(value, &HANDLE, member)?;
    let key = crate::instance::slot(receiver, CHILD_SLOT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{HANDLE_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                HANDLE.slots[CHILD_SLOT]
            ))
        })?;
    Ok((key, crate::instance::slot(receiver, HANDLE_PATH_SLOT)))
}

/// The child this handle names, or the engine fault an unfiled key is.
///
/// There is no catchable refusal here and there is no `close` for there to be
/// one about: a handle names a child for as long as its task runs, and the one
/// window in which its slot is empty is the inside of a
/// [`nvs_core_process_handle_wait`] — which the coroutine that would have to
/// ask is itself suspended in. So an empty slot is a value that reached another
/// task, and failing closed on it is the whole of what this checks.
///
/// # Errors
///
/// A `Fault::fatal`, on the reading above.
fn child_of<'a>(ctx: &'a mut Ctx, key: u64, member: &str) -> Result<&'a mut HeldChild, Fault> {
    ctx.spawned_child_mut(key).ok_or_else(|| {
        Fault::fatal(format!(
            "{HANDLE_NAME}::{member} was asked of a child this task does not hold"
        ))
    })
}

/// What the operating system said, as this class's `IOError` naming the program
/// the handle was opened on.
fn failed(member: &str, path: &Value, err: &std::io::Error) -> Fault {
    nvs_runtime::capability::io_failure(
        &format!("{HANDLE_NAME}::{member}"),
        Path::new(path.as_text().unwrap_or("?")),
        err,
    )
}

/// One chunk off a child's pipe, **off this core** — the pipe is moved to the
/// blocking pool and handed back, because
/// `rule:http-server/a-core-is-never-blocked-on-a-syscall` has no readiness to
/// park a child's pipe on and [`wait_off_core`]'s module doc owns why that
/// leaves exactly one spelling.
///
/// Moved rather than borrowed for the reason
/// [`nvs_runtime::Ctx::take_spawned_child`] gives about a wait: the closure
/// outlives this stack frame, so it owns what it reads from and gives it back.
fn chunk_off_core<R: Read + Send + 'static>(mut pipe: R) -> (R, std::io::Result<Vec<u8>>) {
    nvs_host::blocking::run(move || {
        let mut held = vec![0u8; CHUNK];
        let read = pipe.read(&mut held).map(|read| {
            held.truncate(read);
            held
        });
        (pipe, read)
    })
}

/// One `bytes` written to a child's pipe, off this core for
/// [`chunk_off_core`]'s reason.
fn write_off_core<W: Write + Send + 'static>(
    mut pipe: W,
    data: Vec<u8>,
) -> (W, std::io::Result<()>) {
    nvs_host::blocking::run(move || {
        let written = pipe.write_all(&data).and_then(|()| pipe.flush());
        (pipe, written)
    })
}

/// Whatever is left of one pipe, to its end or to `bound` — [`read_bounded`]
/// without the kill, which a drain that is about to reap the child anyway has
/// no use for.
///
/// # Errors
///
/// Whatever the operating system said about the read.
fn rest_of<R: Read>(pipe: Option<R>, bound: u64) -> std::io::Result<Vec<u8>> {
    let Some(pipe) = pipe else {
        return Ok(Vec::new());
    };
    let mut held = Vec::new();
    pipe.take(bound).read_to_end(&mut held)?;
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `$handle->readStdout(): ?bytes` — the next chunk the child wrote, or
    /// `null` at the end of the stream.
    fn nvs_core_process_handle_read_stdout(ctx, args: [1]) {
        let (key, path) = handle_of(args[0], "readStdout")?;
        let Some(pipe) = child_of(ctx, key, "readStdout")?.stdout.take() else {
            return Ok(Value::null());
        };
        let (pipe, read) = chunk_off_core(pipe);
        let octets = read.map_err(|err| failed("readStdout", &path, &err))?;
        if !octets.is_empty()
            && let Ok(child) = child_of(ctx, key, "readStdout")
        {
            child.stdout = Some(pipe);
        }
        Ok(ended_or(&octets))
    }
}

nvs_runtime::nvs_helper! {
    /// `$handle->readStderr(): ?bytes` — [`nvs_core_process_handle_read_stdout`]
    /// on the other stream.
    fn nvs_core_process_handle_read_stderr(ctx, args: [1]) {
        let (key, path) = handle_of(args[0], "readStderr")?;
        let Some(pipe) = child_of(ctx, key, "readStderr")?.stderr.take() else {
            return Ok(Value::null());
        };
        let (pipe, read) = chunk_off_core(pipe);
        let octets = read.map_err(|err| failed("readStderr", &path, &err))?;
        if !octets.is_empty()
            && let Ok(child) = child_of(ctx, key, "readStderr")
        {
            child.stderr = Some(pipe);
        }
        Ok(ended_or(&octets))
    }
}

/// A chunk as the two answers a read has: the octets, or the `null` that is the
/// end of a stream. **A pipe answers zero bytes exactly once**, at its end, so
/// the empty `bytes` a caller might otherwise have to tell apart from `null`
/// never occurs.
fn ended_or(octets: &[u8]) -> Value {
    if octets.is_empty() {
        Value::null()
    } else {
        Value::bytes(NvsStr::new(octets))
    }
}

nvs_runtime::nvs_helper! {
    /// `$handle->writeStdin(bytes $data): void` — the octets written whole,
    /// suspending until the child has taken them.
    fn nvs_core_process_handle_write_stdin(ctx, args: [2]) {
        let (key, path) = handle_of(args[0], "writeStdin")?;
        let data = args[1]
            .as_bytes()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "{HANDLE_NAME}::writeStdin expected a `bytes`, got tag {}",
                    args[1].tag_byte()
                ))
            })?
            .to_vec();
        let Some(pipe) = child_of(ctx, key, "writeStdin")?.stdin.take() else {
            return Err(failed(
                "writeStdin",
                &path,
                &std::io::Error::from(std::io::ErrorKind::BrokenPipe),
            ));
        };
        let (pipe, written) = write_off_core(pipe, data);
        written.map_err(|err| failed("writeStdin", &path, &err))?;
        if let Ok(child) = child_of(ctx, key, "writeStdin") {
            child.stdin = Some(pipe);
        }
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$handle->wait(): Core\Process\Result` — the child's status, and
    /// whatever neither read had already taken.
    ///
    /// The drain is not optional and is why this is more than a `wait` call: a
    /// child blocked writing into a pipe nobody is reading never exits, so a
    /// member that only waited would hang on exactly the program that streamed
    /// one stream and forgot the other. [`drain`]'s two readers, for its reason,
    /// and [`nvs_runtime::Ctx::intake_bound`] is the ceiling both are held to —
    /// `rule:core-classes/process-run`'s, since what this answers is a capture.
    ///
    /// This is also the join `rule:observability/spawn-is-its-own-event` closes
    /// the child's event at, which is why the event travels in the
    /// [`nvs_runtime::HeldChild`] rather than in the handle the program holds.
    fn nvs_core_process_handle_wait(ctx, args: [1]) {
        let (key, path) = handle_of(args[0], "wait")?;
        let bound = ctx.intake_bound();
        let Some(mut held) = ctx.take_spawned_child(key) else {
            return Err(Fault::fatal(format!(
                "{HANDLE_NAME}::wait was asked of a child this task does not hold"
            )));
        };
        let open = held.spawn_event.take();
        let (held, stdout, stderr, status) = nvs_host::blocking::run(move || {
            let mut held = held;
            drop(held.stdin.take());
            let out = held.stdout.take();
            let err = held.stderr.take();
            let (stdout, stderr) = std::thread::scope(|scope| {
                let stderr = scope.spawn(|| rest_of(err, bound));
                (rest_of(out, bound), stderr.join())
            });
            let stderr = stderr.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            let status = held.reap();
            (held, stdout, stderr, status)
        });
        ctx.restore_spawned_child(key, held);
        if let Some(open) = open {
            // A child process reports no wall time of its own, so the join
            // carries the parent-observed wall and no split. It is closed
            // before the three refusals below, since a wait that throws on
            // what the child wrote has still joined it.
            ctx.close_spawn(open, None);
        }

        let status = status.map_err(|err| failed("wait", &path, &err))?;
        let stdout = stdout.map_err(|err| failed("wait", &path, &err))?;
        let stderr = stderr.map_err(|err| failed("wait", &path, &err))?;
        let taken = stdout.len() + stderr.len();
        if let Some(over) = ctx.intake_breach(SPAWN_MEMBER, taken) {
            return Err(over);
        }
        Ok(crate::instance::build(
            &RESULT,
            [
                Value::int(i64::from(status.code().unwrap_or(SIGNALLED))),
                Value::bytes(NvsStr::new(&stdout)),
                Value::bytes(NvsStr::new(&stderr)),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `$handle->kill(): void` — the child ended now, and nothing said about a
    /// child that had already ended.
    fn nvs_core_process_handle_kill(ctx, args: [1]) {
        let (key, path) = handle_of(args[0], "kill")?;
        child_of(ctx, key, "kill")?
            .kill()
            .map_err(|err| failed("kill", &path, &err))?;
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use nvs_host::blocking::pool_size;
    use nvs_host::reactor::install;
    use nvs_host::{Reactor, Scheduler, run_until_idle};
    use nvs_runtime::{
        Ctx, DebugFlags, Fault, NvsArray, NvsStr, TaskRoot, ThrownClass, TraceKind, Value,
    };

    use crate::tests::granting;

    use crate::registry;

    use super::{
        CLASS, CoreTy, EXIT_CODE_SLOT, HANDLE, HANDLE_NAME, Path, RESULT, RESULT_NAME, RUN_MEMBER,
        Read, wait_off_core,
    };

    /// The five spellings a port of PHP's shell family would reach for. None of them is a member
    /// of this class, because [`super::nvs_core_process_run`] is all five: they differ only in
    /// what they do with the output, and this module's own docs own that reading.
    const SHELL_SPELLINGS: &[&str] = &["exec", "system", "shellExec", "passthru", "backtick"];

    /// Whether a parameter is somewhere a command line could be written — both spellings of
    /// `string`, since [`CoreTy::Str`] and [`CoreTy::Text`] differ in classification and not in
    /// what a caller can put in one.
    fn is_text(ty: CoreTy) -> bool {
        matches!(ty, CoreTy::Str | CoreTy::Text(_))
    }

    /// `rule:core-classes/process-run`, asserted over the whole roster rather than off `run`'s signature: **no member
    /// of this class takes a command line**, and what proves it is that every member naming a
    /// program carries exactly one text parameter — the name — with its arguments in an
    /// `array<string>` beside it. A `spawn` that arrived later with a second text parameter, or
    /// with no argv at all, fails here while still looking reasonable on its own line.
    #[test]
    fn there_is_no_shell_string_form_of_run_or_spawn() {
        let mut starters = 0usize;
        for method in CLASS.methods.iter().chain(CLASS.instance) {
            assert!(
                !SHELL_SPELLINGS.contains(&method.name),
                "`{}` is one of PHP's shell entry points, and this class has one member for all \
                 five: {}",
                method.name,
                RUN_MEMBER
            );
            let texts = method.params.iter().filter(|ty| is_text(**ty)).count();
            if texts == 0 {
                continue;
            }
            starters += 1;
            assert_eq!(
                texts, 1,
                "`{}` takes {texts} `string` parameters: a member that names a program takes the \
                 name and nothing else as text, or the second one is a command line by another \
                 route",
                method.name
            );
            let argvs = method
                .params
                .iter()
                .filter(|ty| matches!(**ty, CoreTy::Array(element) if is_text(*element)))
                .count();
            assert_eq!(
                argvs, 1,
                "`{}` names a program and takes no `array<string>` of arguments, so its caller has \
                 nowhere to put them but inside the name — `rule:core-classes/process-run`",
                method.name
            );
        }
        assert!(
            starters >= 1,
            "the sweep asserted nothing: this class has no member that starts a program at all"
        );
        assert!(
            RESULT.methods.is_empty() && RESULT.instance.iter().all(|m| m.params.is_empty()),
            "a result is read, never re-run: no member of it takes an argument, let alone a \
             command line"
        );
    }

    /// `rule:core-classes/process-spawn`, asserted off the registry rather than off a run: the
    /// member exists, it answers the handle, and the handle carries the five members the rule
    /// names. A `spawn` that landed answering a `Core\Process\Result` — which compiles, and which
    /// every other test in this module would still pass — is the mistake this is written against,
    /// since a member that waited would be `run` under a second name.
    #[test]
    fn process_spawn_is_a_registered_member() {
        let spawn = CLASS
            .methods
            .iter()
            .find(|method| method.name == "spawn")
            .expect("`Core\\Process` registers `spawn` beside `run`");
        assert!(
            matches!(spawn.return_ty, CoreTy::Instance(named) if named == HANDLE_NAME),
            "`spawn` answers {:?} rather than a handle, so nothing it started can be read from \
             until it has finished — which is `run`",
            spawn.return_ty
        );
        assert_eq!(
            spawn.names, CLASS.methods[0].names,
            "`spawn` and `run` take the same two things under the same two names, or one of them \
             has grown a shape the other has not"
        );
        assert!(
            registry::CLASSES
                .iter()
                .any(|class| class.name == HANDLE_NAME),
            "{HANDLE_NAME} is what `spawn` answers and is not on the roster, so the checker \
             cannot resolve a call on one — `every_instance_type_names_a_registered_class`"
        );

        for named in ["readStdout", "readStderr", "writeStdin", "wait", "kill"] {
            assert!(
                HANDLE.instance.iter().any(|method| method.name == named),
                "`{named}` is one of the five members `rule:core-classes/process-spawn` writes on \
                 a handle, and this class has no such member"
            );
        }
        assert!(
            HANDLE.methods.is_empty(),
            "a handle is only ever received from a `spawn`, so it has no static member"
        );
        let waits = HANDLE
            .instance
            .iter()
            .find(|method| method.name == "wait")
            .expect("the roster above holds `wait`");
        assert!(
            matches!(waits.return_ty, CoreTy::Instance(named) if named == RESULT_NAME),
            "a handle's `wait` answers what a completed `run` answers, or the two members describe \
             one child two ways"
        );
    }

    /// The long-running child two of the cases below need, named the way the test harness names
    /// it. Not a path a program can take: this suite's own binary is the one program every
    /// platform it runs on is certain to have, and [`a_child_that_runs_until_it_is_killed`] is the
    /// argument that makes it outlast the case acting on it.
    const SLEEPER: &[&str] = &[
        "--exact",
        "process::tests::a_child_that_runs_until_it_is_killed",
        "--ignored",
        "--nocapture",
    ];

    /// Not a case. It is the **child** [`SLEEPER`] starts, and `#[ignore]` is what keeps an
    /// ordinary run from sitting here: the two cases that need a process which is still running
    /// when they act on it reach this one by name.
    #[test]
    #[ignore = "started by name as the child two of this module's cases need, never run alone"]
    fn a_child_that_runs_until_it_is_killed() {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }

    /// `Core\Process::spawn(<this test binary>, <argv>)` on `ctx`, answering the handle it built
    /// and releasing the two arguments it was built from.
    fn spawn_on(ctx: &mut Ctx, argv: &[&str]) -> Value {
        let me = std::env::current_exe().expect("a test binary knows its own path");
        let program = me.to_str().expect("this suite is built under a UTF-8 path");
        let mut array = NvsArray::new();
        for one in argv {
            array.append(Value::str(NvsStr::new(one.as_bytes())));
        }
        let args = [
            Value::str(NvsStr::new(program.as_bytes())),
            Value::array(array),
        ];
        let answered = nvs_runtime::call(super::nvs_core_process_spawn, ctx, &args)
            .expect("`exec = true` admits this suite's own binary");
        for argument in args {
            #[expect(
                unsafe_code,
                reason = "the list holds exactly the references it built, and the \
                          handle carries a reference of its own to the path"
            )]
            unsafe {
                argument.release();
            }
        }
        answered
    }

    /// The exit status out of a `Core\Process\Result` the way a program reads it, so that a case
    /// asserting a status is asserting what `$result->exitCode()` would answer.
    fn status_of(result: Value) -> i64 {
        let object = crate::instance::receiver(result, &RESULT, "exitCode")
            .expect("a `wait` answers a result and nothing else");
        crate::instance::slot(object, EXIT_CODE_SLOT)
            .as_int()
            .expect("the first slot of a result is its status")
    }

    /// `rule:core-classes/process-spawn`'s "every read suspends the calling coroutine exactly as
    /// `run`'s wait does", in the same two observable ways
    /// [`a_process_wait_suspends_its_coroutine_through_the_blocking_pool`] reads that claim off a
    /// wait — and this is the half that member cannot cover, because a read happens while the
    /// child is still running and a wait happens only once it is not.
    ///
    /// The neighbour is what makes the first half an assertion rather than a hope: a read that
    /// held the core would answer the same octets and finish both tasks, and only the *order*
    /// tells the two apart. The child is this test binary asked to list its cases — a real process
    /// writing several kilobytes on every platform the suite runs on, with nothing to build first.
    // covers: Core\Process::spawn
    #[test]
    fn a_spawned_childs_reads_suspend_the_coroutine_and_free_the_core() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        assert_eq!(
            pool_size().0,
            0,
            "this thread's pool had already started threads, so the count below proves nothing"
        );

        let order = Rc::new(RefCell::new(Vec::new()));
        let reader = Rc::clone(&order);
        let mut granted = Ctx::buffered();
        granted.set_config(granting("[capabilities.process]\nexec = true\n"));
        sched.spawn(granted, TaskRoot::Worker, move |ctx| {
            let handle = spawn_on(ctx, &["--list"]);
            let chunk =
                nvs_runtime::call(super::nvs_core_process_handle_read_stdout, ctx, &[handle])
                    .expect("the child listed its cases");
            assert!(
                chunk.as_bytes().is_some_and(|octets| !octets.is_empty()),
                "a child that wrote its listing answered no first chunk"
            );
            #[expect(
                unsafe_code,
                reason = "the member hands back a reference of its own for the chunk, \
                          and this scope owns the handle it was read through"
            )]
            unsafe {
                chunk.release();
                handle.release();
            }
            reader.borrow_mut().push("read");
        });
        let neighbour = Rc::clone(&order);
        sched.spawn(Ctx::buffered(), TaskRoot::Worker, move |_ctx| {
            neighbour.borrow_mut().push("neighbour");
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.finished, 2, "a task never came back off the pool");
        assert_eq!(
            *order.borrow(),
            ["neighbour", "read"],
            "the read held the core instead of handing it back"
        );
        assert!(
            pool_size().0 >= 1,
            "the read ran on the worker: this thread's blocking pool never started a thread"
        );
    }

    /// `rule:core-classes/process-spawn`'s `kill` and `wait`, asserted together because neither is
    /// worth anything alone: a `kill` that signalled nothing would still leave `wait` answering a
    /// status, and a `wait` that never reaped would still let a killed child look ended.
    ///
    /// The status is compared against `0` rather than named. A signal has no exit code on the
    /// platforms that have signals, so [`SIGNALLED`] is the answer there and the number the system
    /// chose is the answer elsewhere; what is common to both — and what a caller asks — is that a
    /// child somebody ended is not one that finished its work.
    // covers: Core\Process\Handle::kill
    #[test]
    fn killing_a_spawned_child_ends_it_and_wait_answers_its_status() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting("[capabilities.process]\nexec = true\n"));
        let handle = spawn_on(&mut ctx, SLEEPER);

        nvs_runtime::call(super::nvs_core_process_handle_kill, &mut ctx, &[handle])
            .expect("a live child is one the operating system will end");
        assert!(
            ctx.take_pending().is_none(),
            "a `kill` of a child that is running refused"
        );
        let result = nvs_runtime::call(super::nvs_core_process_handle_wait, &mut ctx, &[handle])
            .expect("a killed child is still one to be waited for");
        assert_ne!(
            status_of(result),
            0,
            "the child answered the status of a program that ran to its own end, so either the \
             kill reached nothing or the wait reaped something else"
        );

        #[expect(
            unsafe_code,
            reason = "this scope owns the handle `spawn` built and the result `wait` \
                      answered with"
        )]
        unsafe {
            result.release();
            handle.release();
        }
    }

    /// `rule:observability/four-kinds-become-a-span`'s `spawn`, over the member that starts a child
    /// process: one event for the spawn, and one that reads as joined once the `wait` joining the
    /// child has answered.
    ///
    /// Counted off the whole trace rather than read at an index, which is what makes this an
    /// assertion about the member rather than about the test's own arithmetic: a member that filed
    /// its event twice — once where the child starts and once where it is joined — prints
    /// plausibly at index zero and fails here.
    #[test]
    fn a_spawn_produces_exactly_one_span() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting("[capabilities.process]\nexec = true\n"));
        ctx.set_debug_flags(DebugFlags::TRACE);
        let handle = spawn_on(&mut ctx, SLEEPER);
        nvs_runtime::call(super::nvs_core_process_handle_kill, &mut ctx, &[handle])
            .expect("a live child is one the operating system will end");
        let result = nvs_runtime::call(super::nvs_core_process_handle_wait, &mut ctx, &[handle])
            .expect("a killed child is still one to be waited for");

        let spans: Vec<&str> = ctx
            .trace()
            .iter()
            .filter(|event| event.kind == TraceKind::Spawn)
            .map(|event| event.callee.as_str())
            .collect();
        assert_eq!(
            spans.len(),
            1,
            "a spawn and the wait that joined it are one event, and the request recorded {spans:?}"
        );
        assert!(
            spans[0].starts_with(r"Core\Process::spawn"),
            "the event does not name the form that filed it: {}",
            spans[0]
        );
        assert!(
            !spans[0].ends_with("unjoined"),
            "a child that has been reaped is a child whose event has been closed: {}",
            spans[0]
        );

        #[expect(
            unsafe_code,
            reason = "this scope owns the handle `spawn` built and the result `wait` \
                      answered with"
        )]
        unsafe {
            result.release();
            handle.release();
        }
    }

    /// `rule:core-classes/process-spawn`'s "a child still running when its task ends is killed",
    /// which is the half of the rule no program can observe and every operator eventually does.
    ///
    /// The observation is the child's own output pipe, taken out of the table before the context
    /// goes: this end stays open, and the write end is the child's alone, so the read below
    /// reaches its end exactly when the child does. A child that survived would leave it open for
    /// the thirty seconds [`a_child_that_runs_until_it_is_killed`] sleeps, which is why the wait
    /// here is bounded rather than a `join` — a failure has to report, not hang.
    #[test]
    fn a_spawned_child_is_killed_when_its_task_ends() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting("[capabilities.process]\nexec = true\n"));
        let handle = spawn_on(&mut ctx, SLEEPER);
        let (key, _path) = super::handle_of(handle, "test").expect("`spawn` answered a handle");
        assert_eq!(
            ctx.spawned_children(),
            1,
            "the child was never filed against the task, so there is nothing for its end to kill"
        );
        let pipe = ctx
            .spawned_child_mut(key)
            .expect("the child is this context's")
            .stdout
            .take()
            .expect("the door pipes all three of a child's streams");
        #[expect(
            unsafe_code,
            reason = "this scope owns the handle `spawn` built, and the child itself is \
                      the context's rather than the handle's"
        )]
        unsafe {
            handle.release();
        }

        let (done, ended) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut sink = Vec::new();
            let mut pipe = pipe;
            let _ = pipe.read_to_end(&mut sink);
            let _ = done.send(());
        });

        drop(ctx);
        ended
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect(
                "the child outlived the task that spawned it: its output pipe never reached its \
                 end, so something is still running that nothing is charged for",
            );
    }

    /// `rule:core-classes/process-refuses-a-shell-target`, driven through the door [`super::nvs_core_process_run`] calls, on a context
    /// that grants everything — so a refusal here cannot be the capability denial wearing the same
    /// class. Both halves of the rule: the three kinds are refused **however they are spelled**,
    /// since the extension is lower-cased before it is matched, and the refusal names the kind in
    /// that one spelling regardless of how the target was written.
    #[test]
    fn a_windows_batch_or_powershell_target_is_refused() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting("[capabilities.process]\nexec = true\n"));
        for (target, named) in [
            ("examples/process/say.bat", "bat"),
            ("examples/process/SAY.BAT", "bat"),
            ("C:/deploy/release.cmd", "cmd"),
            ("C:/deploy/RELEASE.CMD", "cmd"),
            ("./build.ps1", "ps1"),
            ("./BUILD.PS1", "ps1"),
            ("./Build.Ps1", "ps1"),
        ] {
            let refused = nvs_runtime::capability::exec(&ctx, Path::new(target), &[], RUN_MEMBER)
                .expect_err("a second command-line parser is not a target this API has");
            let Fault::Thrown(class, message) = refused else {
                panic!("§ 4's refusal is catchable, like every refusal this member writes");
            };
            assert_eq!(class, ThrownClass::Runtime);
            assert!(
                message.contains(&format!(".{named}")) && message.contains(RUN_MEMBER),
                "the refusal names the kind and the member, lower-cased however `{target}` was \
                 spelled: {message}"
            );
            assert!(
                !message.contains("process.exec"),
                "and is not the capability denial, which `exec = true` does not produce: {message}"
            );
        }

        // The control the sweep above cannot supply: under the same grant a target of any other
        // kind reaches the spawn, so the refusals are about the kind and not about the door being
        // shut. Nothing is at this path, so what comes back is the operating system's answer.
        let missing =
            nvs_runtime::capability::exec(&ctx, Path::new("./say.bat.gz"), &[], RUN_MEMBER)
                .expect_err("nothing is at that path");
        let Fault::Thrown(class, message) = missing else {
            panic!("a failed spawn is catchable too — `rule:security/denial-is-a-runtime-error`");
        };
        assert_eq!(
            class,
            ThrownClass::Io,
            "a target that only looks like one of the three is started, and fails as the operating \
             system's problem: {message}"
        );
    }

    /// `Core\Process::run(<this test binary>, ["--list"])` under one `[limits] max_output`
    /// ceiling, answering what the member answered and releasing what it built.
    ///
    /// The child is the suite's own binary asked to list its cases, which is a real process
    /// writing several kilobytes on every platform the suite runs on, with nothing to build
    /// first — the same trick the wait's case uses for a child that writes nothing.
    fn list_under(program: &str, ceiling: &str) -> Result<(), String> {
        let mut ctx = Ctx::buffered();
        ctx.set_config(granting(&format!(
            "[capabilities.process]\nexec = true\n\n[limits]\nmax_output = \"{ceiling}\"\n"
        )));
        let mut argv = NvsArray::new();
        argv.append(Value::str(NvsStr::new(b"--list")));
        let args = [
            Value::str(NvsStr::new(program.as_bytes())),
            Value::array(argv),
        ];
        let answered = nvs_runtime::call(super::nvs_core_process_run, &mut ctx, &args);
        let refusal = ctx.take_pending().map(std::borrow::Cow::into_owned);
        for argument in args.into_iter().chain(answered) {
            #[expect(
                unsafe_code,
                reason = "the list holds exactly the references it built, and the member \
                          hands back a reference of its own on the path that succeeds"
            )]
            unsafe {
                argument.release();
            }
        }
        refusal.map_or(Ok(()), Err)
    }

    /// `rule:core-classes/process-run`'s reuse of `[limits] max_output`, asserted **on both sides
    /// of the bound** over one child: the same listing is answered whole under a ceiling above it
    /// and refused under one below it, so a member that refused everything — or that had nothing
    /// to refuse — fails here while looking right on either half alone.
    ///
    /// What the control buys is the second half of "bounded rather than unbounded". A refusal
    /// alone proves only that a small number stops something; reading the child's real output
    /// first is what says the tight ceiling was crossed by a child that had more to write, and it
    /// is `wait_off_core` under `u64::MAX` — the no-ceiling spelling — that reads it.
    // covers: Core\Process::run
    #[test]
    fn a_child_whose_output_exceeds_limits_max_output_is_bounded_rather_than_unbounded() {
        let me = std::env::current_exe().expect("a test binary knows its own path");
        let program = me.to_str().expect("this suite is built under a UTF-8 path");

        let mut ctx = Ctx::buffered();
        ctx.set_config(granting("[capabilities.process]\nexec = true\n"));
        let child = nvs_runtime::capability::exec(&ctx, &me, &["--list"], RUN_MEMBER)
            .expect("`exec = true` admits an ordinary executable");
        let listing = wait_off_core(child, u64::MAX)
            .expect("the child never ended")
            .stdout
            .len();
        assert!(
            listing > 64,
            "this binary listed {listing} bytes of cases, so the ceiling below is not one the \
             child crosses and the refusal it produces would prove nothing"
        );

        list_under(program, "8M").expect("a ceiling the child is nowhere near refuses nothing");

        let refused = list_under(program, "64")
            .expect_err("a child writing past `[limits] max_output` is not captured");
        assert!(
            refused.contains("max_output") && refused.contains(RUN_MEMBER),
            "the refusal names neither the directive an operator would raise nor the member \
             that hit it: {refused}"
        );
        assert!(
            refused.contains("64"),
            "the refusal does not say what the ceiling was, which is the one number its reader \
             has to change: {refused}"
        );
    }

    /// `rule:core-classes/process-run`, in the only two ways it is observable: the core is **given back** while the
    /// child runs, and the wait lands on the blocking pool rather than on the worker.
    ///
    /// The neighbour is what makes the first half an assertion rather than a hope. A wait that
    /// held the core would still answer correctly and still finish both tasks — it would only be
    /// slower, and nothing about one task's own result can tell the two apart. What can is the
    /// *order*: the neighbour is spawned second and must run first, which happens only if the
    /// waiter suspended. The second half is read off `pool_size`, which starts at zero threads for
    /// a thread that has never made a blocking call, so a run where it is still zero is a run
    /// where the wait never left this one.
    ///
    /// The child is this test binary with a filter that matches nothing: a real process, started
    /// through the real door, on every platform the suite runs on and with nothing to build first.
    #[test]
    fn a_process_wait_suspends_its_coroutine_through_the_blocking_pool() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        assert_eq!(
            pool_size().0,
            0,
            "this thread's pool had already started threads, so the count below proves nothing"
        );

        let order = Rc::new(RefCell::new(Vec::new()));
        let waiter = Rc::clone(&order);
        let mut granted = Ctx::buffered();
        granted.set_config(granting("[capabilities.process]\nexec = true\n"));
        sched.spawn(granted, TaskRoot::Worker, move |ctx| {
            let me = std::env::current_exe().expect("a test binary knows its own path");
            let child = nvs_runtime::capability::exec(
                ctx,
                &me,
                &["--exact", "__nvs_no_such_case__"],
                RUN_MEMBER,
            )
            .expect("`exec = true` admits an ordinary executable");
            let output = wait_off_core(child, u64::MAX).expect("the child never ended");
            assert!(
                output.status.success(),
                "a filter matching no case is not a failing run"
            );
            waiter.borrow_mut().push("waited");
        });
        let neighbour = Rc::clone(&order);
        sched.spawn(Ctx::buffered(), TaskRoot::Worker, move |_ctx| {
            neighbour.borrow_mut().push("neighbour");
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.finished, 2, "a task never came back off the pool");
        assert_eq!(
            *order.borrow(),
            ["neighbour", "waited"],
            "the wait held the core instead of handing it back"
        );
        assert!(
            pool_size().0 >= 1,
            "the wait ran on the worker: this thread's blocking pool never started a thread"
        );
    }
}
