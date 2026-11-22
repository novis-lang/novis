//! `Core\Test::run` and `Core\Test\RunResult`: a test runs the program's entry
//! as a command and reads its exit code and output
//! (`rule:testing/a-test-runs-a-command`).
//!
//! The run itself is on the far side of `nvs_runtime::inproc`, as a test
//! request's is: the unit under test is `nvs_cli`'s, and this crate may not
//! hold one. This module reads the call's arguments, hands them over, and
//! builds the result from what came back.
//!
//! **What it spends:** per call, the child's isolate while it runs, then one
//! `RunResult` holding both captures, freed with it.

use nvs_runtime::{Fault, ThrownClass, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
    Qual, TextOrigin,
};

/// `Core\Test\RunResult`'s fully-qualified name.
pub(crate) const RUN_RESULT_NAME: &str = r"Core\Test\RunResult";

/// `Core\Test::run`'s `{stdin?: string|bytes}`. Without `stdin`, the input is
/// empty.
pub(crate) const RUN_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "stdin",
    ty: CoreTy::Union(&[CoreTy::Text(Qual::Neutral), CoreTy::Blob(Qual::Neutral)]),
    default: Const::Null,
}];

/// [`RUN_RESULT`]'s slots: the exit code, then standard output and standard
/// error, each as `bytes`. The readers that return text check the bytes.
const EXIT_CODE_SLOT: usize = 0;
/// See [`EXIT_CODE_SLOT`].
const STDOUT_SLOT: usize = 1;
/// See [`EXIT_CODE_SLOT`].
const STDERR_SLOT: usize = 2;

/// One reader: a member with no parameter.
const fn reader(
    name: &'static str,
    return_ty: CoreTy,
    symbol: &'static str,
    doc: &'static MethodDoc,
) -> CoreMethod {
    CoreMethod {
        name,
        names: &[],
        params: &[],
        defaults: &[],
        return_ty,
        text: TextOrigin::Clean(
            "A test's own program wrote this text, and the test reads it back.",
        )
        .on(&return_ty),
        symbol,
        doc: Some(doc),
    }
}

/// How one `Core\Test::run` ended: its exit code and its two captures, with
/// the names `Core\Process`'s result uses (`rule:core-classes/process-run`).
/// A `Core`-owned instance beside `Core\Test` for the reason
/// `Core\Test\Response` is one.
// rank: 475
pub(crate) const RUN_RESULT: CoreClass = CoreClass {
    name: RUN_RESULT_NAME,
    doc: Some(&RUN_RESULT_CARD),
    methods: &[],
    instance: &[
        reader(
            "exitCode",
            CoreTy::Int,
            "nvs_core_test_run_result_exit_code",
            &EXIT_CODE_DOC,
        ),
        reader(
            "stdout",
            CoreTy::Str,
            "nvs_core_test_run_result_stdout",
            &STDOUT_DOC,
        ),
        reader(
            "stderr",
            CoreTy::Str,
            "nvs_core_test_run_result_stderr",
            &STDERR_DOC,
        ),
        reader(
            "bytes",
            CoreTy::Bytes,
            "nvs_core_test_run_result_bytes",
            &BYTES_DOC,
        ),
    ],
    slots: &["exitCode", "stdout", "stderr"],
    constants: &[],
};

/// `Core\Test::run`'s reference card — `rule:core-api/reference-card`.
pub(crate) const RUN_DOC: MethodDoc = MethodDoc {
    short: "Runs your own program as a command and returns how it ended. It starts no new \
            process. The program runs the same way as with `nvs run`.",
    params: &[
        ParamDoc {
            name: "arguments",
            desc: "The words after the program name, such as `['greet', 'Ada', '--loud']`. \
                   `Core\\Command::run` and `Core\\Cli::arguments` read them.",
            shape: &[],
        },
        ParamDoc {
            name: "stdin",
            desc: "The input of the program, as a `string` or as `bytes`. `Core\\IO::stdin` reads \
                   it. If you give no input, the input is empty.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Test\\RunResult` with the exit code and the output. If the program calls \
          `exit(3)`, the exit code is `3`. If the program throws an error it does not catch, the \
          exit code is `1` and the error is on standard error.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The test program has no file that `nvs run` would run. This is the case when you \
               give `nvs test` a folder. The call is also an error inside a program that `run` or \
               `Core\\Test::request` started.",
    }],
};

/// `Core\Test\RunResult`'s class card — `rule:core-api/reference-card`.
const RUN_RESULT_CARD: ClassDoc = ClassDoc {
    short: "How one run of your program by `Core\\Test::run` ended. `exitCode()` returns the \
            exit code, `stdout()` and `bytes()` return the standard output, and `stderr()` \
            returns the standard error. You cannot create one yourself.",
};

/// `Core\Test\RunResult::exitCode`'s reference card.
const EXIT_CODE_DOC: MethodDoc = MethodDoc {
    short: "Returns the exit code of the program.",
    params: &[],
    ret: "`0` when the program ended normally. The number given to `exit` when the program \
          called it. `2` when `Core\\Command::run` printed its usage page. `1` when the program \
          threw an error it did not catch.",
    errors: &[],
};

/// `Core\Test\RunResult::stdout`'s reference card.
const STDOUT_DOC: MethodDoc = MethodDoc {
    short: "Returns the text the program wrote to standard output.",
    params: &[],
    ret: "Everything the program wrote with `echo`, in order. If the program wrote nothing, the \
          result is an empty string.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The output is not valid UTF-8, so it is not a string. Read it with `bytes()`.",
    }],
};

/// `Core\Test\RunResult::stderr`'s reference card.
const STDERR_DOC: MethodDoc = MethodDoc {
    short: "Returns the text the program wrote to standard error.",
    params: &[],
    ret: "Everything the program wrote to standard error, in order. A usage page from \
          `Core\\Command::run` is here, and so is an error the program did not catch. If the \
          program wrote nothing there, the result is an empty string.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The text is not valid UTF-8, so it is not a string.",
    }],
};

/// `Core\Test\RunResult::bytes`'s reference card.
const BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns what the program wrote to standard output, as `bytes`.",
    params: &[],
    ret: "Every byte of the standard output, in order. The bytes do not have to be valid UTF-8.",
    errors: &[],
};

nvs_runtime::nvs_helper! {
    /// `Core\Test::run(array<string> $arguments, {stdin?: string|bytes}): Core\Test\RunResult`
    /// — `rule:testing/a-test-runs-a-command`.
    ///
    /// A run from inside a run or a request, and a unit with no entry `nvs run`
    /// would run, are mistakes in the test, so each is a `LogicError`.
    fn nvs_core_test_run(ctx, args: [2]) {
        let arguments = super::lines_of(args[0])?;
        let stdin = match (args[1].as_bytes(), args[1].as_text()) {
            (Some(octets), _) => octets.to_vec(),
            (None, Some(text)) => text.as_bytes().to_vec(),
            (None, None) => Vec::new(),
        };
        let ran = nvs_runtime::inproc::run(ctx, arguments, stdin).map_err(|refusal| match refusal {
            nvs_runtime::inproc::AnswerError::InsideRun
            | nvs_runtime::inproc::AnswerError::NoEntry => {
                Fault::thrown_as(ThrownClass::Logic, format!("Core\\Test::run(): {refusal}"))
            }
            refusal => Fault::thrown(format!("Core\\Test::run could not run the program: {refusal}")),
        })?;
        Ok(crate::instance::build(
            &RUN_RESULT,
            [
                Value::int(ran.exit_code),
                Value::bytes(nvs_runtime::NvsStr::new(&ran.stdout)),
                Value::bytes(nvs_runtime::NvsStr::new(&ran.stderr)),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\RunResult::exitCode(): int`.
    fn nvs_core_test_run_result_exit_code(_ctx, args: [1]) {
        crate::instance::read_slot(args, &RUN_RESULT, EXIT_CODE_SLOT, "exitCode")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\RunResult::bytes(): bytes` — standard output as it was
    /// written.
    fn nvs_core_test_run_result_bytes(_ctx, args: [1]) {
        crate::instance::read_slot(args, &RUN_RESULT, STDOUT_SLOT, "bytes")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\RunResult::stdout(): string`. Each call makes a new
    /// `string` from the kept bytes.
    fn nvs_core_test_run_result_stdout(_ctx, args: [1]) {
        text_of(args, STDOUT_SLOT, "stdout")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\RunResult::stderr(): string`, as `stdout` reads its slot.
    fn nvs_core_test_run_result_stderr(_ctx, args: [1]) {
        text_of(args, STDERR_SLOT, "stderr")
    }
}

/// Where each member's helper lives.
pub(crate) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    Some(nvs_runtime::Entry::borrowed(match symbol {
        "nvs_core_test_run" => (nvs_core_test_run as *const ()).cast(),
        "nvs_core_test_run_result_exit_code" => {
            (nvs_core_test_run_result_exit_code as *const ()).cast()
        }
        "nvs_core_test_run_result_stdout" => (nvs_core_test_run_result_stdout as *const ()).cast(),
        "nvs_core_test_run_result_stderr" => (nvs_core_test_run_result_stderr as *const ()).cast(),
        "nvs_core_test_run_result_bytes" => (nvs_core_test_run_result_bytes as *const ()).cast(),
        _ => return None,
    }))
}

/// The capture in `slot` of the receiver, as a `string`.
///
/// # Errors
///
/// A `RuntimeError` for bytes that are not UTF-8.
fn text_of(args: &[Value], slot: usize, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(args[0], &RUN_RESULT, member)?;
    let held = crate::instance::slot(receiver, slot);
    // Unreachable from source: `nvs_core_test_run` fills the slot with `bytes`
    // and nothing else writes it.
    let octets = held.as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{RUN_RESULT_NAME}::{member} found a non-`bytes` slot"
        ))
    })?;
    let text = std::str::from_utf8(octets).map_err(|err| {
        Fault::thrown(format!(
            "{RUN_RESULT_NAME}::{member}(): the output is not valid UTF-8 at byte {}, so it is \
             not a string",
            err.valid_up_to()
        ))
    })?;
    Ok(Value::text(text))
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsStr};

    use super::*;

    /// The release a compiled call site owes for a value a member answered.
    fn dropped(value: Value) {
        #[expect(unsafe_code, reason = "the member transferred what it answered")]
        // SAFETY: a member's answer carries a reference of its own, and this
        // driver is the caller that would otherwise hold it.
        unsafe {
            value.release();
        }
    }

    /// A `RunResult` built as `nvs_core_test_run` builds one from a run.
    fn ran(exit_code: i64, stdout: &[u8], stderr: &[u8]) -> Value {
        crate::instance::build(
            &RUN_RESULT,
            [
                Value::int(exit_code),
                Value::bytes(NvsStr::new(stdout)),
                Value::bytes(NvsStr::new(stderr)),
            ],
        )
    }

    /// Each reader returns its slot with a reference of the caller's own, so
    /// reading twice returns the same value and releasing a reading leaves the
    /// result whole. `bytes` returns standard output whatever it holds, while
    /// `stdout` and `stderr` throw for bytes that are not UTF-8, and a receiver
    /// that is not a `RunResult` throws.
    // covers: Core\Test\RunResult::exitCode, Core\Test\RunResult::stdout
    // covers: Core\Test\RunResult::stderr, Core\Test\RunResult::bytes
    #[test]
    fn run_result_readers_return_the_exit_code_and_both_outputs_and_text_must_be_utf8() {
        let mut ctx = Ctx::buffered();
        let result = ran(3, b"giving up\n", b"the disk is full\n");
        for round in 0..2 {
            let code = nvs_runtime::call(nvs_core_test_run_result_exit_code, &mut ctx, &[result])
                .expect("reading the exit code cannot fail");
            let out = nvs_runtime::call(nvs_core_test_run_result_stdout, &mut ctx, &[result])
                .expect("UTF-8 standard output is a string");
            let err = nvs_runtime::call(nvs_core_test_run_result_stderr, &mut ctx, &[result])
                .expect("UTF-8 standard error is a string");
            let raw = nvs_runtime::call(nvs_core_test_run_result_bytes, &mut ctx, &[result])
                .expect("reading the bytes cannot fail");
            assert_eq!(code.as_int(), Some(3), "round {round}: the exit code");
            assert_eq!(out.as_text(), Some("giving up\n"), "round {round}: stdout");
            assert_eq!(
                err.as_text(),
                Some("the disk is full\n"),
                "round {round}: stderr"
            );
            assert_eq!(
                raw.as_bytes(),
                Some(&b"giving up\n"[..]),
                "round {round}: bytes"
            );
            for value in [code, out, err, raw] {
                dropped(value);
            }
        }
        dropped(result);

        let binary = ran(0, &[0x89, b'P', b'N', b'G'], &[0xff]);
        let raw = nvs_runtime::call(nvs_core_test_run_result_bytes, &mut ctx, &[binary])
            .expect("bytes that are not UTF-8 are still bytes");
        assert_eq!(raw.as_bytes(), Some(&[0x89, b'P', b'N', b'G'][..]));
        dropped(raw);
        for reader in [
            nvs_core_test_run_result_stdout as nvs_runtime::NvsFn,
            nvs_core_test_run_result_stderr,
        ] {
            assert!(
                nvs_runtime::call(reader, &mut ctx, &[binary]).is_err(),
                "output that is not UTF-8 is not a string"
            );
            let message = ctx.take_pending().unwrap_or_default();
            assert!(
                message.contains("not valid UTF-8 at byte 0"),
                "the error names the first bad byte: {message}"
            );
        }
        dropped(binary);

        let stranger = Value::int(7);
        assert!(
            nvs_runtime::call(nvs_core_test_run_result_exit_code, &mut ctx, &[stranger]).is_err(),
            "a receiver that is not a run result throws"
        );
        let _ = ctx.take_pending();
    }
}
