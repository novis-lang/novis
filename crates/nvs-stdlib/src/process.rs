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
//! [ADR 0106](/docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md) § 6's
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
//! # Known gaps
//!
//! 1. **`[limits] max_output` does not bound the capture yet.** `rule:core-classes/process-run`
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
use std::process::{Child, Output};

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
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Process::run`'s reference card — `rule:core-api/reference-card`.
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

/// `rule:core-classes/process-run`'s `ProcessResult` — what [`nvs_core_process_run`] answers with.
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
    /// known gap 1 owns what bounds the capture.
    fn nvs_core_process_run(ctx, args: [2]) {
        let program = text(&args[0], "its path")?;
        let argv = argv_of(&args[1])?;
        let borrowed: Vec<&str> = argv.iter().map(String::as_str).collect();
        let path = Path::new(program);
        let child = nvs_runtime::capability::exec(ctx, path, &borrowed, RUN_MEMBER)?;
        let output = wait_off_core(child)
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

/// The child's status and both of its streams, waited for **off this core** —
/// `rule:core-classes/process-run`, and this module's *Decision: the wait happens off the core*.
///
/// Both streams are read to the end before the status is taken, which is what
/// [`Child::wait_with_output`] is for: waiting first and reading after
/// deadlocks the moment a child fills a pipe buffer. That is why the whole
/// three-way wait goes to the pool as one job rather than the exit alone —
/// there is no point at which reading a pipe and waiting for the exit are
/// separable, so there is no smaller thing to hand off.
///
/// Named rather than written inline because it is the only part of
/// `Core\Process` a case can hold still while a neighbouring task runs:
/// `a_process_wait_suspends_its_coroutine_through_the_blocking_pool` drives
/// this on a scheduler, and the member around it needs a compiled program.
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
fn wait_off_core(child: Child) -> std::io::Result<Output> {
    nvs_host::blocking::run(move || child.wait_with_output())
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
    use std::cell::RefCell;
    use std::rc::Rc;

    use nvs_host::blocking::pool_size;
    use nvs_host::reactor::install;
    use nvs_host::{Reactor, Scheduler, run_until_idle};
    use nvs_runtime::{Ctx, Fault, TaskRoot, ThrownClass};

    use crate::tests::granting;

    use super::{CLASS, CoreTy, Path, RESULT, RUN_MEMBER, wait_off_core};

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
            panic!("a failed spawn is catchable too — ADR 0118 § 5");
        };
        assert_eq!(
            class,
            ThrownClass::Io,
            "a target that only looks like one of the three is started, and fails as the operating \
             system's problem: {message}"
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
            let output = wait_off_core(child).expect("the child never ended");
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
