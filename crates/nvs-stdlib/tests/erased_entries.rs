//! Every place native code enters compiled code, and who typed the values it passes.
//!
//! A compiled function reads each argument at its own declared representation, so a value of the
//! wrong type that reaches one is reinterpreted, not refused (`rule:types/callable-signature`).
//! A written call is typed by the checker. A call that native code makes is typed by nothing
//! unless the door it goes through checks it, or its caller built the values to fit.
//!
//! This test reads the code and is the list. A door into compiled code is one of two shapes: a
//! `transmute` of an address to `NvsFn`, or a call of `nvs_runtime`'s `dispatch::call_at` or of
//! `dispatch::call_slots`, which `call_at` builds its slots for. Each
//! function that holds one has a row in [`DRIVERS`]. A row names a test that passes wrong-typed
//! values through that door and expects a throw, or says which checked caller or compiler pass
//! typed them. A new door without a row fails here, naming the function, and so does a row whose
//! door is gone.

use std::fs;
use std::path::{Path, PathBuf};

/// Who answers for the values one door passes.
enum Driver {
    /// A test, by its function name, that passes wrong-typed values through the door and expects
    /// a throw. The scan checks that a function of that name exists under `crates/`.
    Refuses(&'static [&'static str]),
    /// The checked caller or compiler pass that typed every value the door passes.
    TypedBy(&'static str),
}

/// Each door, as the file it is in and the function that holds it.
const DRIVERS: &[(&str, &str, Driver)] = &[
    (
        "nvs-runtime/src/dispatch.rs",
        "call_erased_unchecked",
        Driver::Refuses(&[
            "a_call_by_name_refuses_an_argument_its_parameter_does_not_admit",
            "a_static_method_is_refused_by_name",
            "a_by_reference_parameter_is_refused_by_name",
            "construct_checks_its_arguments_as_call_does",
        ]),
    ),
    (
        "nvs-runtime/src/dispatch.rs",
        "call_static_bound",
        Driver::Refuses(&["an_isolate_entry_refuses_an_argument_its_parameter_does_not_admit"]),
    ),
    (
        "nvs-runtime/src/object.rs",
        "write_property",
        Driver::Refuses(&["a_hooked_property_write_is_checked_before_its_hook_runs"]),
    ),
    (
        "nvs-runtime/src/callable.rs",
        "call_callable_with",
        Driver::Refuses(&["a_bare_callable_call_checks_what_its_parameter_declares"]),
    ),
    (
        "nvs-runtime/src/dispatch.rs",
        "call_method",
        Driver::TypedBy(
            "the `Core` caller, against an interface member it names: `compareTo`, `toString`, \
             `encode`, the router's gate and responder members, the test runner's tests with no \
             argument, and a `Core\\Test` partial passing its double's typed arguments on",
        ),
    ),
    (
        "nvs-runtime/src/dispatch.rs",
        "call_static_on",
        Driver::TypedBy(
            "the `Core` caller: a command handler gets values its parser typed, `parse` one \
             string, and `fromRow` and a JSON `decode` one value the codec checked",
        ),
    ),
    (
        "nvs-runtime/src/object.rs",
        "construct",
        Driver::TypedBy(
            "the codec's field check, for a JSON or database row's constructor arguments; the \
             test runner passes none",
        ),
    ),
    (
        "nvs-runtime/src/object.rs",
        "read_erased_property_hinted",
        Driver::TypedBy("a `get` hook, which takes the receiver and no argument"),
    ),
    (
        "nvs-runtime/src/sequence.rs",
        "call_member",
        Driver::TypedBy("an iteration member, which takes the receiver and no argument"),
    ),
    (
        "nvs-runtime/src/dispatch.rs",
        "call_render",
        Driver::TypedBy("a `Core` member's own address, called with the receiver alone"),
    ),
    (
        "nvs-runtime/src/dispatch.rs",
        "call_compare_to",
        Driver::TypedBy(
            "a `Core` member's own address, called with the receiver and a value of its class",
        ),
    ),
    (
        "nvs-runtime/src/dispatch.rs",
        "call_unwind",
        Driver::TypedBy("the receiver alone"),
    ),
    (
        "nvs-runtime/src/dispatch.rs",
        "call_at",
        Driver::TypedBy(
            "the shared call itself: every caller of it is a door of its own, and has a row here",
        ),
    ),
    (
        "nvs-runtime/src/dispatch.rs",
        "call_slots",
        Driver::TypedBy(
            "the shared call under `call_at`: every caller of it is a door of its own, and has a \
             row here",
        ),
    ),
    (
        "nvs-codegen/src/lib.rs",
        "raw_function",
        Driver::TypedBy(
            "a script frame, which takes no argument; `Unit::call_static` is a test and embedder \
             door whose caller typed its arguments",
        ),
    ),
    (
        "nvs-cli/src/cache.rs",
        "script",
        Driver::TypedBy("a cached script's entry frame, which takes no argument"),
    ),
];

/// Every `.rs` file under `dir`, sorted, so a failure names doors in one order on every runner.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    out.sort();
}

/// The shipped half of a file: every line before a `#[cfg(test)]` that opens a module. A
/// `#[cfg(test)]` on one item does not end it, so a test-only door above the test module is
/// still read and still owes a row.
fn shipped(text: &str) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        if line.trim_end() == "#[cfg(test)]"
            && lines
                .get(at + 1)
                .is_some_and(|next| next.starts_with("mod "))
        {
            break;
        }
        out.push((at + 1, *line));
    }
    out
}

/// The name of the last function opened at or before `line`.
fn enclosing<'a>(lines: &[(usize, &'a str)], upto: usize) -> Option<&'a str> {
    lines[..=upto].iter().rev().find_map(|(_, line)| {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            return None;
        }
        let after = trimmed.split_once("fn ")?.1;
        let before = &trimmed[..trimmed.len() - after.len() - 3];
        if !before.is_empty() && !before.ends_with(' ') {
            return None;
        }
        let name: &str = after
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .next()?;
        (!name.is_empty()).then_some(name)
    })
}

#[test]
#[cfg_attr(
    miri,
    ignore = "reads the source tree, which Miri's isolation does not allow"
)]
fn every_entry_into_compiled_code_has_a_driver() {
    let crates = nvs_repo::path("crates");
    let mut files = Vec::new();
    for entry in fs::read_dir(&crates).expect("the crates directory lists") {
        let src = entry.expect("a crate entry reads").path().join("src");
        if src.is_dir() {
            sources(&src, &mut files);
        }
    }
    files.sort();

    let mut doors: Vec<(String, String, usize)> = Vec::new();
    let mut every_fn = String::new();
    for file in &files {
        let text = fs::read_to_string(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        every_fn.push_str(&text);
        let rel = file
            .strip_prefix(&crates)
            .expect("under crates")
            .to_string_lossy()
            .replace('\\', "/");
        // `dispatch::call_at` is `pub(crate)` and `dispatch::call_slots` is private, so only
        // `nvs-runtime` can call them. A function of either name in another crate is not a door.
        let calls_at = rel.starts_with("nvs-runtime/src/");
        let lines = shipped(&text);
        for (index, (number, line)) in lines.iter().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            let transmute = code.contains("transmute") && code.contains("NvsFn");
            let call = calls_at
                && ((code.contains("call_at(") && !code.contains("fn call_at"))
                    || (code.contains("call_slots(") && !code.contains("fn call_slots")));
            if !(transmute || call) {
                continue;
            }
            let name = enclosing(&lines, index).unwrap_or_else(|| {
                panic!("{rel}:{number} enters compiled code outside a function")
            });
            doors.push((rel.clone(), name.to_owned(), *number));
        }
    }
    // The test modules count too: a driver test lives in one.
    for path in fs::read_dir(&crates).expect("the crates directory lists") {
        let tests = path.expect("a crate entry reads").path().join("tests");
        if tests.is_dir() {
            let mut more = Vec::new();
            sources(&tests, &mut more);
            for file in more {
                every_fn.push_str(&fs::read_to_string(&file).unwrap_or_default());
            }
        }
    }

    let mut missing = Vec::new();
    for (file, name, line) in &doors {
        if !DRIVERS.iter().any(|(f, n, _)| f == file && n == name) {
            missing.push(format!("crates/{file}:{line} in `{name}`"));
        }
    }
    assert!(
        missing.is_empty(),
        "these functions pass values into compiled code and have no row in `DRIVERS`. Add a test \
         that passes wrong-typed values through each and expects a throw, or name who typed them:\n{}",
        missing.join("\n")
    );

    for (file, name, driver) in DRIVERS {
        assert!(
            doors.iter().any(|(f, n, _)| f == file && n == name),
            "`DRIVERS` names `{name}` in crates/{file}, which no longer enters compiled code; \
             remove its row"
        );
        match driver {
            Driver::Refuses(tests) => {
                assert!(!tests.is_empty(), "`{name}` names no test that drives it");
                for test in *tests {
                    assert!(
                        every_fn.contains(&format!("fn {test}(")),
                        "`{name}`'s driver `{test}` is not a function under crates/"
                    );
                }
            }
            Driver::TypedBy(who) => {
                assert!(
                    !who.trim().is_empty(),
                    "`{name}` names nobody who typed its values"
                );
            }
        }
    }
}
