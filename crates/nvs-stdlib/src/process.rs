//! `Core\Process` — [ADR 0044](../../../../docs/adr/0044-core-process-argv-only-no-shell.md)'s
//! one way to run another program, over
//! [`nvs_runtime::capability::exec`](nvs_runtime::capability::exec)'s door.
//!
//! # Decision: there is no shell-string form, so there is nothing here to escape
//!
//! ADR 0044 is the whole of it, and the shape of this module is what enforces
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
//! ADR 0044 § 1 writes `$result->exitCode()` and `$result->stderr()`, and this
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
//! **Captured output is `bytes`, never `string`** — ADR 0044 § 1, over
//! [ADR 0009](../../../../docs/adr/0009-string-and-bytes.md)'s UTF-8 guarantee,
//! which cannot be assumed of an arbitrary child's output. A caller who knows
//! the output is text writes `as string`, which is the checked conversion that
//! throws rather than the silent replacement-character mangling PHP gives.
//!
//! # Known gaps
//!
//! 1. **The wait blocks the calling worker thread.** ADR 0044 § 5 says `run`
//!    suspends the calling coroutine through the blocking pool instead, and the
//!    goal's `a_process_wait_suspends_its_coroutine_through_the_blocking_pool`
//!    is the check that closes it. Nothing about this module's surface changes
//!    when it does: the member's signature, its door and its result are already
//!    what § 1 specifies, and only [`nvs_core_process_run`]'s wait moves.
//! 2. **`[limits] max_output` does not bound the capture yet.** ADR 0044 § 1
//!    reuses that directive rather than adding a cap, and nothing reads it in
//!    this tree — so what bounds a capture today is the request's memory limit,
//!    which these two buffers are charged against like any other allocation.
//!    That is `Core\IO::read`'s reading of the same question, and the same
//!    later signature closes both.
//!
//! **What it spends:** the child's whole stdout and stderr, once each, as one
//! `bytes` value per stream held for as long as the program holds the result,
//! plus one object allocation of three slots — charged to the request that
//! asked.

use std::path::Path;

use nvs_runtime::{Fault, NvsStr, Tag, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// This class's fully-qualified name, in one place so the registry row, the
/// [`crate::registry::CAPABILITIES`] row and every consumer that matches on it
/// cannot drift apart.
pub(crate) const NAME: &str = "Core\\Process";

/// The spelling both of `run`'s refusals name, and the one the door prints.
const RUN_MEMBER: &str = "Core\\Process::run";

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[CoreMethod {
        name: "run",
        names: &["path", "argv"],
        // The path is a sink for `Core\IO`'s reason and one more of its own:
        // ADR 0044 § 1 says `$path` and every element of `$argv` are plain
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
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Process::run`'s reference card — ADR 0117.
const RUN_DOC: MethodDoc = MethodDoc {
    short: "Runs `$path` with `$argv`, waits for it to exit, and answers what it did — PHP's \
            `exec`, `system`, `shell_exec`, `passthru` and the backtick operator, all of which \
            differ only in what they do with the output. There is no command-line form of this \
            member anywhere in the surface: nothing is escaped because there is nothing to escape \
            into. Needs the `process.exec` capability for the target.",
    params: &[
        ParamDoc {
            name: "path",
            desc: "The program to start, absolute or relative to the working directory. It is \
                   started directly, never through a shell, so a `PATH` lookup is the caller's \
                   own to make.",
            shape: &[],
        },
        ParamDoc {
            name: "argv",
            desc: "The arguments, one element each — `[\"-n\", \"1\", $host]` and never \
                   `\"-n 1 $host\"`. An element carrying a space, a quote or a semicolon is one \
                   argument that contains those characters, on every platform.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Process\\Result` carrying the exit code and both captured streams. The child \
          inherits none of this process's own standard streams — all three are piped — so a \
          program that runs a child cannot have its own output interleaved with it.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The configuration does not grant `process.exec` for this target, or the target \
                   is a `.bat`, `.cmd` or `.ps1` file, which this API refuses on every platform \
                   because starting one hands the argv it just built to a second parser.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The capability allowed it and the operating system did not — nothing is at the \
                   path, it is not executable, or the child could not be waited for.",
        },
    ],
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

/// ADR 0044 § 1's `ProcessResult` — what [`nvs_core_process_run`] answers with.
///
/// Three members over three slots and no static member at all: a result is only
/// ever produced by a run. This module's docs own why it is an instance rather
/// than a shape; what belongs here is that every member is a slot read, because
/// the child has already exited by the time one exists — there is nothing left
/// to compute and nothing left to fail.
pub(crate) const RESULT: CoreClass = CoreClass {
    name: RESULT_NAME,
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

/// `Core\Process\Result::exitCode`'s reference card — ADR 0117.
const EXIT_CODE_DOC: MethodDoc = MethodDoc {
    short: "The status the child exited with — `$?`, and the third out-parameter `exec` writes.",
    params: &[],
    ret: "The exit status, `0` for success by the convention every operating system shares, and \
          `-1` for a child a signal stopped before it could report one.",
    errors: &[],
};

/// `Core\Process\Result::stdout`'s reference card — ADR 0117.
const STDOUT_DOC: MethodDoc = MethodDoc {
    short: "Everything the child wrote to its standard output, captured whole.",
    params: &[],
    ret: "The octets, as `bytes` and not `string`: Novis guarantees a `string` is UTF-8, and a \
          child process makes no such promise about what it writes. A caller who knows the output \
          is text writes `as string`, which throws on a sequence that is not.",
    errors: &[],
};

/// `Core\Process\Result::stderr`'s reference card — ADR 0117.
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
    /// wait and the capture. Both streams are read to the end before the status
    /// is taken, which is what `wait_with_output` is for — waiting first and
    /// reading after deadlocks the moment a child fills a pipe buffer, and the
    /// door pipes all three streams precisely so that no child inherits this
    /// process's own.
    ///
    /// This module's known gap 1 owns the thread this wait occupies, and gap 2
    /// owns what bounds the capture.
    fn nvs_core_process_run(ctx, args: [2]) {
        let program = text(&args[0], "its path")?;
        let argv = argv_of(&args[1])?;
        let borrowed: Vec<&str> = argv.iter().map(String::as_str).collect();
        let path = Path::new(program);
        let child = nvs_runtime::capability::exec(ctx, path, &borrowed, RUN_MEMBER)?;
        let output = child
            .wait_with_output()
            .map_err(|err| nvs_runtime::capability::io_failure(RUN_MEMBER, path, &err))?;
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nvs_runtime::{Ctx, Fault, ThrownClass};

    use super::{CLASS, CoreTy, Path, RESULT, RUN_MEMBER};

    /// The five spellings a port of PHP's shell family would reach for. None of them is a member
    /// of this class, because [`super::nvs_core_process_run`] is all five: they differ only in
    /// what they do with the output, and this module's own docs own that reading.
    const SHELL_SPELLINGS: &[&str] = &["exec", "system", "shellExec", "passthru", "backtick"];

    /// A snapshot built from the text an operator would have written, for the reason
    /// `nvs_runtime::capability`'s own cases state: the boot path deserializes, so a case that
    /// constructed the typed tree directly would pin a grant no configuration file can express.
    fn granting(written: &str) -> Arc<nvs_config::Snapshot> {
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        Arc::new(nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        })
    }

    /// Whether a parameter is somewhere a command line could be written — both spellings of
    /// `string`, since [`CoreTy::Str`] and [`CoreTy::Text`] differ in classification and not in
    /// what a caller can put in one.
    fn is_text(ty: CoreTy) -> bool {
        matches!(ty, CoreTy::Str | CoreTy::Text(_))
    }

    /// ADR 0044 § 1, asserted over the whole roster rather than off `run`'s signature: **no member
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
                 nowhere to put them but inside the name — ADR 0044 § 1",
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

    /// ADR 0044 § 4, driven through the door [`super::nvs_core_process_run`] calls, on a context
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
            panic!("a failed spawn is catchable too — ADR 0118 § 5");
        };
        assert_eq!(
            class,
            ThrownClass::Io,
            "a target that only looks like one of the three is started, and fails as the operating \
             system's problem: {message}"
        );
    }
}
