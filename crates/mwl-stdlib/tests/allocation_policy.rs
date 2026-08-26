//! Every argument goes through one check, and only one — and a member that
//! knows how long its result is allocates it once.
//!
//! Three guards, one rule each: a size becomes a refusal in exactly one place,
//! a `string` argument's UTF-8 is established by its tag and never re-derived,
//! and a result is written into the allocation it is answered from.
//!
//! The guard this pins was four hand-written copies of the same three lines,
//! and the members with the largest appetite had none at all — which is the
//! failure mode the test exists for. A copy is easy to add and impossible to
//! notice, so the invariant is checked at the source rather than left to
//! review: `mwl_runtime::affordable` is the only place a size becomes a
//! refusal, and it is where `[limits.hard]` attaches when the M6 arena carries
//! it (ADR 0004).

use std::fs;
use std::path::Path;

/// Every `.rs` file under this crate's `src/`, as `(name, contents)`.
fn sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display())) {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let name = path
                .file_name()
                .expect("a file with an extension has a name")
                .to_string_lossy()
                .into_owned();
            let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{name}: {err}"));
            out.push((name, text));
        }
    }
    assert!(!out.is_empty(), "the stdlib source directory is not empty");
    out
}

#[test]
fn no_member_writes_its_own_allocation_guard() {
    // `isize::try_from(...)` on a size is the shape every one of the four
    // copies had. The point is not the spelling — it is that a member which
    // needs this check reaches for the shared one, so that the day a budget
    // replaces the overflow test, every member gets it at once.
    let offenders: Vec<String> = sources()
        .into_iter()
        .filter(|(_, text)| text.contains("isize::try_from"))
        .map(|(name, _)| name)
        .collect();

    assert!(
        offenders.is_empty(),
        "these files hand-write an allocation guard instead of calling \
         `mwl_runtime::affordable`: {offenders:?}. That function is the one seam the per-request \
         ceiling attaches to; a private copy silently opts its member out of it."
    );
}

// ============================================================================
// One check per `string` argument
// ============================================================================

/// A `string`'s tag **is** ADR 0009's UTF-8 guarantee, so a reader that has
/// already checked the tag may not then walk the payload to re-derive it.
///
/// The banned shape is one chain: [`mwl_runtime::Value::as_str_bytes`], which
/// answers only for a `Tag::Str`, feeding `std::str::from_utf8`.
/// `Value::as_text` is the same tag check and none of the walk —
/// `mwl_runtime`'s `string` module owns the argument in its § *Reading the
/// payload as text*, and a debug build still re-validates inside that one
/// reader, so the check is not lost, only paid once and in one place.
///
/// This scans the source rather than measuring, because what it pins is the
/// shape the *next* argument reader will be copied from: the pair sat in nine
/// sibling modules at once, each a faithful copy of the one before it, and no
/// measurement of any single member would have said so.
///
/// `Value::as_bytes` is deliberately untouched. A `bytes` carries no encoding
/// guarantee at all, so `Core\Encoding`'s `Utf8` scheme validating one is the
/// real check rather than a repeat of it.
#[test]
fn no_member_revalidates_a_string_argument() {
    /// How many lines after an `as_str_bytes` still count as the same chain.
    /// Wide enough to span a `ok_or_else` closure formatting a message, which
    /// is what every one of the nine put between the two halves.
    const WINDOW: usize = 12;

    let mut offenders: Vec<String> = Vec::new();
    for (name, text) in sources() {
        // A `#[cfg(test)]` module may hold either spelling for its own
        // reasons: building a `String` out of a member's *result* to assert on
        // it is not an argument reader re-validating anything.
        let source = text
            .split_once("\n#[cfg(test)]")
            .map_or(text.as_str(), |(head, _)| head);
        let lines: Vec<&str> = source.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !line.contains("as_str_bytes") {
                continue;
            }
            let end = lines.len().min(index + WINDOW);
            if lines[index..end].iter().any(|l| l.contains("from_utf8")) {
                offenders.push(format!("{name}:{}", index + 1));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "these sites read a `string`'s bytes and then re-validate them as UTF-8: {offenders:?}. \
         The tag already is that guarantee (ADR 0009 § 3), so `Value::as_text` is the whole read \
         — `crates/mwl-stdlib/src/str.rs`'s `text` is the shape to copy, and it states why the \
         O(n) pass is not worth keeping `for safety`."
    );
}

// ============================================================================
// One allocation per result
// ============================================================================

/// An allocator that counts the requests made on the calling thread, so the
/// guard below measures allocations rather than trusting a reading of the
/// member's source.
///
/// Thread-local for the reason `mwl_runtime`'s own `counting_alloc` states: a
/// test binary runs its tests concurrently, and a process-wide counter would
/// fold a neighbour's allocations into the delta.
///
/// **Only installed in a debug build.** A `#[global_allocator]` is chosen once
/// per binary, and an optimized build already has one: `mwl-runtime` installs
/// its pooled allocator in every `not(test)` optimized build, which is what
/// this binary links. `cargo test` — the profile `tools/verify.py` runs — is a
/// debug build, so the guard runs there; `cargo test --release` compiles it
/// out rather than failing to link.
#[cfg(debug_assertions)]
struct Counting;

#[cfg(debug_assertions)]
thread_local! {
    static ALLOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(debug_assertions)]
#[expect(
    unsafe_code,
    reason = "`GlobalAlloc` is an unsafe trait, and every method forwards its \
              own contract verbatim to `System`"
)]
unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        unsafe { std::alloc::System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        unsafe { std::alloc::System.realloc(ptr, layout, new_size) }
    }
}

#[cfg(debug_assertions)]
#[global_allocator]
static COUNTING: Counting = Counting;

/// How many allocations `call` makes, and what it answered.
#[cfg(debug_assertions)]
fn allocations_of(
    member: mwl_runtime::MwlFn,
    args: &[mwl_runtime::Value],
) -> (usize, mwl_runtime::Value) {
    let mut ctx = mwl_runtime::Ctx::buffered();
    // Every argument, the context and the message a failure would format are
    // built outside the window on purpose: what is being counted is what the
    // member spends on its *result*.
    let before = ALLOCATIONS.with(std::cell::Cell::get);
    let answer = mwl_runtime::call(member, &mut ctx, args);
    let spent = ALLOCATIONS.with(std::cell::Cell::get) - before;
    (spent, answer.expect("the member answered"))
}

/// A `string` argument.
#[cfg(debug_assertions)]
fn arg(text: &str) -> mwl_runtime::Value {
    mwl_runtime::Value::str(mwl_runtime::MwlStr::new(text.as_bytes()))
}

#[cfg(debug_assertions)]
#[test]
fn a_str_member_allocates_its_result_once() {
    use mwl_runtime::Value;
    use mwl_stdlib::str::{
        mwl_core_str_pad_end, mwl_core_str_pad_start, mwl_core_str_repeat, mwl_core_str_replace,
    };

    // Not every member below knows its length exactly — `replace` writes into
    // a buffer the size of its subject and grows if the replacement is longer,
    // which is why this case's replacement is not. `join` and `reverse` are
    // absent for the same reason from the other side: the first grows past a
    // capacity that only counts its separators, the second spends the `Vec` of
    // borrowed pieces a backwards read needs. What is pinned here is the rule
    // they all follow — the result is never built somewhere else and copied in.
    let cases: Vec<(&str, mwl_runtime::MwlFn, Vec<Value>)> = vec![
        (
            "replace",
            mwl_core_str_replace,
            vec![
                arg("the cat sat on the mat, and the cat stayed"),
                arg("cat"),
                arg("dog"),
                Value::bool(false),
                Value::uint(u64::MAX),
            ],
        ),
        (
            "padStart",
            mwl_core_str_pad_start,
            vec![arg("42"), Value::uint(12), arg("·-")],
        ),
        (
            "padEnd",
            mwl_core_str_pad_end,
            vec![arg("42"), Value::uint(12), arg("·-")],
        ),
        (
            "repeat",
            mwl_core_str_repeat,
            vec![arg("ha"), Value::uint(64)],
        ),
    ];

    for (name, member, args) in cases {
        let (spent, answer) = allocations_of(member, &args);
        assert_eq!(
            spent, 1,
            "`Core\\Str::{name}` made {spent} allocations answering one string, and it owes \
             exactly one: it writes into the allocation it answers from, at a capacity this \
             case does not make it exceed. Building a `String` and copying it in is what this \
             pins against — see `crates/mwl-stdlib/src/str.rs`'s § *A result is written once*."
        );
        assert!(
            answer.as_text().is_some_and(|text| !text.is_empty()),
            "`Core\\Str::{name}` answered something other than a non-empty `string`, so the \
             count above measured the wrong thing"
        );
        #[expect(
            unsafe_code,
            reason = "the value came back from `call`, which transfers the \
                      reference the helper produced"
        )]
        unsafe {
            mwl_runtime::mwl_value_release(u64::from(answer.tag_byte()), answer.bits());
        }
        for value in args {
            #[expect(
                unsafe_code,
                reason = "each argument was built here and holds the one \
                          reference this test owns"
            )]
            unsafe {
                mwl_runtime::mwl_value_release(u64::from(value.tag_byte()), value.bits());
            }
        }
    }
}
