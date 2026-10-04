//! Every argument goes through one check, and only one — and a member that
//! knows how long its result is allocates it once.
//!
//! Five guards, one rule each: a size becomes a refusal in exactly one place,
//! a `string` argument's UTF-8 is established by its tag and never re-derived,
//! a result is written into the allocation it is answered from, a callback
//! that declares no key parameter is handed no key, and a sort that renumbers
//! reads none.
//!
//! The first guard pins what was four hand-written copies of the same three
//! lines, with the members of largest appetite carrying none at all — which is
//! the failure mode the test exists for. A copy is easy to add and impossible
//! to notice, so the invariant is checked at the source rather than left to
//! review: `nvs_runtime::affordable` is the only place a size becomes a
//! refusal, and it is where `[limits.hard]` attaches when the M6 arena carries
//! it (`rule:programs/memory-priority`).
//!
//! The last two are `docs/perf/userland-gap.md` § D, and they are measured
//! here rather than from compiled code because the member is where the
//! decision is made: `nvs_runtime::callable_arity` is read once before the walk
//! and decides whether a key is *built*, so a native callback with an arity
//! slot is the whole of what the measurement needs. [`callable_of`] is that
//! callback, and it is the only thing in this file a compiler would otherwise
//! have to produce.

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
         `nvs_runtime::affordable`: {offenders:?}. That function is the one seam the per-request \
         ceiling attaches to; a private copy silently opts its member out of it."
    );
}

// ============================================================================
// One check per `string` argument
// ============================================================================

/// A `string`'s tag **is** `rule:types/bytes`'s UTF-8 guarantee, so a reader that has
/// already checked the tag may not then walk the payload to re-derive it.
///
/// The banned shape is one chain: [`nvs_runtime::Value::as_str_bytes`], which
/// answers only for a `Tag::Str`, feeding `std::str::from_utf8`.
/// `Value::as_text` is the same tag check and none of the walk —
/// `nvs_runtime`'s `string` module owns the argument in its § *Reading the
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
         The tag already is that guarantee (`rule:types/conversion`), so `Value::as_text` is the whole read \
         — `crates/nvs-stdlib/src/str.rs`'s `text` is the shape to copy, and it states why the \
         O(n) pass is not worth keeping `for safety`."
    );
}

// ============================================================================
// One allocation per result
// ============================================================================

/// How many allocations `call` makes, and what it answered.
///
/// The counter is `nvs_runtime::budget`'s, which every build maintains because
/// the memory limit is read off it; this binary installs no allocator of its
/// own, and could not, since a `#[global_allocator]` is chosen once per binary
/// and `nvs-runtime` now registers one in every `not(test)` build.
///
/// **Only run in a debug build.** What the gate keeps out is no longer a
/// missing counter but an optimized one's inlining: the numbers below are
/// pinned against what an unoptimized build allocates, and nothing has measured
/// them under `--release`.
#[cfg(debug_assertions)]
fn allocations_of(
    member: nvs_runtime::NvsFn,
    args: &[nvs_runtime::Value],
) -> (usize, nvs_runtime::Value) {
    let mut ctx = nvs_runtime::Ctx::buffered();
    // Every argument, the context and the message a failure would format are
    // built outside the window on purpose: what is being counted is what the
    // member spends on its *result*.
    let before = nvs_runtime::budget::allocations();
    let answer = nvs_runtime::call(member, &mut ctx, args);
    let spent = nvs_runtime::budget::allocations() - before;
    (spent, answer.expect("the member answered"))
}

/// A `string` argument.
#[cfg(debug_assertions)]
fn arg(text: &str) -> nvs_runtime::Value {
    nvs_runtime::Value::str(nvs_runtime::NvsStr::new(text.as_bytes()))
}

/// A draw is written into the buffer it answers with, a pass at a time, and
/// never drawn into a scratch buffer and copied — at a width past one pass, so
/// a member that allocated per pass fails too.
// covers: Core\Random::bytes
#[cfg(debug_assertions)]
#[test]
fn a_random_draw_allocates_its_result_once() {
    use nvs_runtime::Value;
    use nvs_stdlib::random::{nvs_core_random_bytes, nvs_core_random_token};

    let cases: [(&str, nvs_runtime::NvsFn); 2] = [
        ("bytes", nvs_core_random_bytes),
        ("token", nvs_core_random_token),
    ];
    for (name, member) in cases {
        let (spent, answer) = allocations_of(member, &[Value::uint(1000)]);
        assert_eq!(
            spent, 1,
            "`Core\\Random::{name}` made {spent} allocations answering one draw, and it owes \
             exactly one: the draw is written into the value it answers with"
        );
        let width = answer
            .as_bytes()
            .or_else(|| answer.as_str_bytes())
            .map(<[u8]>::len);
        assert_eq!(
            width,
            Some(if name == "bytes" { 1000 } else { 2000 }),
            "`Core\\Random::{name}` answered the wrong width, so the count above measured the \
             wrong thing"
        );
        #[expect(
            unsafe_code,
            reason = "the value came back from `call`, which transfers the \
                      reference the helper produced"
        )]
        unsafe {
            nvs_runtime::nvs_value_release(u64::from(answer.tag_byte()), answer.bits());
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn a_str_member_allocates_its_result_once() {
    use nvs_runtime::Value;
    use nvs_stdlib::str::{
        nvs_core_str_pad_end, nvs_core_str_pad_start, nvs_core_str_repeat, nvs_core_str_replace,
    };

    // Not every member below knows its length exactly — `replace` writes into
    // a buffer the size of its subject and grows if the replacement is longer,
    // which is why this case's replacement is not. `join` and `reverse` are
    // absent for the same reason from the other side: the first grows past a
    // capacity that only counts its separators, the second spends the `Vec` of
    // borrowed pieces a backwards read needs. What is pinned here is the rule
    // they all follow — the result is never built somewhere else and copied in.
    let cases: Vec<(&str, nvs_runtime::NvsFn, Vec<Value>)> = vec![
        (
            "replace",
            nvs_core_str_replace,
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
            nvs_core_str_pad_start,
            vec![arg("42"), Value::uint(12), arg("·-")],
        ),
        (
            "padEnd",
            nvs_core_str_pad_end,
            vec![arg("42"), Value::uint(12), arg("·-")],
        ),
        (
            "repeat",
            nvs_core_str_repeat,
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
             pins against — see `crates/nvs-stdlib/src/str.rs`'s § *A result is written once*."
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
            nvs_runtime::nvs_value_release(u64::from(answer.tag_byte()), answer.bits());
        }
        for value in args {
            #[expect(
                unsafe_code,
                reason = "each argument was built here and holds the one \
                          reference this test owns"
            )]
            unsafe {
                nvs_runtime::nvs_value_release(u64::from(value.tag_byte()), value.bits());
            }
        }
    }
}

/// Hands back the one reference this test owns in `value`.
#[cfg(debug_assertions)]
fn release(value: nvs_runtime::Value) {
    #[expect(
        unsafe_code,
        reason = "every value passed here was built by this test or came back \
                  from `call`, which transfers the reference the helper produced"
    )]
    unsafe {
        nvs_runtime::nvs_value_release(u64::from(value.tag_byte()), value.bits());
    }
}

/// A packed list of `count` integers — the shape whose key is a *position*
/// rather than a stored string, and therefore the only one where synthesizing
/// a key costs an allocation at all.
#[cfg(debug_assertions)]
fn list_of(count: usize) -> nvs_runtime::Value {
    let mut list = nvs_runtime::NvsArray::new();
    for index in 0..count {
        list.append(nvs_runtime::Value::int(
            i64::try_from(index).expect("a test-sized index"),
        ));
    }
    nvs_runtime::Value::array(list)
}

/// A closure value whose `invoke` is a plain Rust function.
///
/// `nvs_runtime::call_callable` reads exactly four things off a closure — its
/// class's `ClassTable::set_callable` bit, which is what makes the value a
/// closure at all, slot `CALLABLE_ARITY_SLOT`, slot `CALLABLE_PARAM_TAGS_SLOT`,
/// and the `CALLABLE_INVOKE` method's address in its class — so a test in this
/// crate can hand a `Core` member a `callable` without a compiler in front of
/// it.
/// Everything else in `nvs_ir::lower::lower_anon_fn`'s representation is
/// captured state, and a native callback captures nothing.
///
/// Every parameter is recorded as `CALLABLE_PARAM_TAG_ANY`, which is what a
/// `mixed` one gets: the callback below is a Rust function taking whatever the
/// member hands it, so there is no declared type for the tag check to hold it
/// to.
///
/// The table is leaked because a descriptor's *address* is its identity and it
/// must outlive every instance made from it, which is the rule
/// `nvs_stdlib::instance`'s own docs state; the test process exiting is what
/// reclaims it.
#[cfg(debug_assertions)]
fn callable_of(arity: usize, invoke: nvs_runtime::NvsFn) -> nvs_runtime::Value {
    let mut table = nvs_runtime::ClassTable::new();
    let id = table.define("{callable}", &["arity", "params"], &[]);
    table.set_methods(
        id,
        vec![nvs_runtime::MethodRow {
            name: nvs_runtime::CALLABLE_INVOKE.to_owned(),
            code: invoke as *const u8,
            // A closure is called through `call_callable`, which reads the
            // arity and the tags off the *object*'s own two slots below rather
            // than off this row — see `nvs_runtime::MethodRow`.
            arity: 0,
            param_tags: 0,
            param_names: Vec::new(),
            param_types: Vec::new(),
            public: true,
            protected: false,
            native: false,
        }],
    );
    table.set_callable(id);
    let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
    #[expect(
        unsafe_code,
        reason = "the table above is leaked, so the descriptor outlives every \
                  instance made from it — `NvsObj::new`'s whole obligation"
    )]
    let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
    object.set_field(
        nvs_runtime::CALLABLE_ARITY_SLOT,
        nvs_runtime::Value::int(i64::try_from(arity).expect("a small arity")),
    );
    let mut tags: u64 = 0;
    for parameter in 0..arity {
        tags |= u64::from(nvs_runtime::CALLABLE_PARAM_TAG_ANY) << (parameter * 4);
    }
    object.set_field(
        nvs_runtime::CALLABLE_PARAM_TAGS_SLOT,
        nvs_runtime::Value::int(i64::from_ne_bytes(tags.to_ne_bytes())),
    );
    nvs_runtime::Value::object(object)
}

/// What every callback below does: sweep the `slots` references a compiled
/// callee would release on exit — the receiver and each parameter, which
/// `call_callable` retained on the way in — and answer `true`.
///
/// `true` rather than anything derived from the arguments so that the callback
/// itself allocates nothing: what is being counted is what the *member* spends
/// per entry.
#[cfg(debug_assertions)]
#[expect(
    unsafe_code,
    reason = "`call_callable` passes exactly `slots` live values, each retained \
              for this callee to release, and `abi::call` passes the address \
              of a live `Value` for the result — neither is expressible in the \
              signature compiled code calls through"
)]
unsafe fn swept(
    args: *const nvs_runtime::Value,
    slots: usize,
    out: *mut nvs_runtime::Value,
) -> i32 {
    for index in 0..slots {
        release(unsafe { *args.add(index) });
    }
    unsafe {
        *out = nvs_runtime::Value::bool(true);
    }
    nvs_runtime::OK
}

/// `fn ($value)` — or `fn ($carry, $value)` read from `reduce`'s side: one
/// parameter, so the receiver plus one.
#[cfg(debug_assertions)]
#[expect(unsafe_code, reason = "forwarding this callee's own contract")]
unsafe extern "C" fn declares_one(
    _ctx: *mut nvs_runtime::Ctx,
    args: *const nvs_runtime::Value,
    out: *mut nvs_runtime::Value,
) -> i32 {
    unsafe { swept(args, 2, out) }
}

/// `fn ($value, $key)`, and `reduce`'s `fn ($carry, $value)`.
#[cfg(debug_assertions)]
#[expect(unsafe_code, reason = "forwarding this callee's own contract")]
unsafe extern "C" fn declares_two(
    _ctx: *mut nvs_runtime::Ctx,
    args: *const nvs_runtime::Value,
    out: *mut nvs_runtime::Value,
) -> i32 {
    unsafe { swept(args, 3, out) }
}

/// `reduce`'s `fn ($carry, $value, $key)`.
#[cfg(debug_assertions)]
#[expect(unsafe_code, reason = "forwarding this callee's own contract")]
unsafe extern "C" fn declares_three(
    _ctx: *mut nvs_runtime::Ctx,
    args: *const nvs_runtime::Value,
    out: *mut nvs_runtime::Value,
) -> i32 {
    unsafe { swept(args, 4, out) }
}

/// A callback declaring `arity` parameters and doing nothing with them.
///
/// # Panics
///
/// If `arity` is not one this file has a callback for.
#[cfg(debug_assertions)]
fn callback(arity: usize) -> nvs_runtime::Value {
    let invoke: nvs_runtime::NvsFn = match arity {
        1 => declares_one,
        2 => declares_two,
        3 => declares_three,
        other => panic!("no native callback declares {other} parameters"),
    };
    callable_of(arity, invoke)
}

/// `docs/perf/userland-gap.md` § D, measured: the key a callback never
/// declared is never built.
///
/// `map`, `filter` and `reduce` each read `nvs_runtime::callable_arity` once
/// before their walk and pass `$key` only to a callback with somewhere to put
/// it. On a packed list that key is a *rendered decimal* — one `NvsStr` per
/// entry — so the difference between the two arities is exactly one allocation
/// per element, and it is the whole of what this pins.
///
/// **The key-free run is not zero**, and the assertion is a difference for
/// that reason: `call_callable` builds the argument slice it retains, which is
/// one allocation per call whatever the callback declares. That cost is
/// identical in both runs, so subtracting them isolates the key.
#[cfg(debug_assertions)]
#[test]
fn a_one_parameter_callback_synthesizes_no_key() {
    use nvs_runtime::{NvsFn, Value};
    use nvs_stdlib::arr::{nvs_core_arr_filter, nvs_core_arr_map, nvs_core_arr_reduce};

    const ENTRIES: usize = 64;

    // Member, the arity that wants no key, the arity that does, and whatever
    // trailing arguments the member's row declares after the callback.
    let cases: Vec<(&str, NvsFn, usize, usize, Vec<Value>)> = vec![
        ("map", nvs_core_arr_map, 1, 2, Vec::new()),
        ("filter", nvs_core_arr_filter, 1, 2, Vec::new()),
        ("reduce", nvs_core_arr_reduce, 2, 3, vec![Value::int(0)]),
    ];

    for (name, member, quiet, keyed, tail) in cases {
        let mut spent = Vec::new();
        for arity in [quiet, keyed] {
            let mut args = vec![list_of(ENTRIES), callback(arity)];
            args.extend_from_slice(&tail);
            let (count, answer) = allocations_of(member, &args);
            spent.push(count);
            release(answer);
            for value in args {
                release(value);
            }
        }
        let (quiet_spent, keyed_spent) = (spent[0], spent[1]);
        assert!(
            quiet_spent <= ENTRIES + ENTRIES / 4,
            "`Core\\Arr::{name}` spent {quiet_spent} allocations over {ENTRIES} entries for a \
             {quiet}-parameter callback, and the floor is one an entry — `call_callable`'s own \
             argument slice. Anything much above that is a key being built and dropped again."
        );
        assert!(
            keyed_spent >= quiet_spent + ENTRIES,
            "`Core\\Arr::{name}` spent {quiet_spent} allocations for a {quiet}-parameter \
             callback and {keyed_spent} for a {keyed}-parameter one over {ENTRIES} entries: \
             the gap is the rendered key, so if it has closed the walk is either building one \
             for a callback that never asked or handing the other a key it did not render."
        );
    }
}

/// The second half of § D: `Core\Arr::sort($list)` renumbers, so it reads no
/// key — and neither does one whose `by` declares a single parameter.
///
/// **Preserving a key is not what costs anything, and measuring said so.**
/// `preserveKeys: true` over a packed list was five allocations dearer than
/// `false` over sixty-four entries, not sixty-four: `NvsArray::slot_key`
/// answers a `SlotKey::Index` while the array is packed and renders nothing,
/// so what those five bought was the `Vec` holding them. The one place a sort
/// *renders* a key is `keys[index].to_str()`, reached only when a `by` closure
/// declared somewhere to put it — which is why the gap this test asserts is
/// between two arities of `by` rather than between the two `preserveKeys`.
///
/// The no-`by` run is asserted absolutely, which the callback cases above
/// could not be: it makes no closure call at all, so there is no per-entry
/// allocation left for anything but a key. What it may spend is the handful of
/// `Vec`s the permutation and the merge scratch need, and those grow with the
/// logarithm of the entry count rather than with the count.
#[cfg(debug_assertions)]
#[test]
fn a_sort_that_renumbers_builds_no_keys() {
    use nvs_runtime::Value;
    use nvs_stdlib::arr::nvs_core_arr_sort;

    const ENTRIES: usize = 64;

    // `by`, and nothing else varying: `order` ascending, no `comparator`, and
    // `preserveKeys` false throughout — the renumbering sort § D names.
    let mut spent = Vec::new();
    for by in [Value::null(), callback(1), callback(2)] {
        let args = vec![
            list_of(ENTRIES),
            by,
            Value::int(0),
            Value::null(),
            Value::bool(false),
        ];
        let (count, answer) = allocations_of(nvs_core_arr_sort, &args);
        spent.push(count);
        release(answer);
        for value in args {
            release(value);
        }
    }
    let (bare, quiet, keyed) = (spent[0], spent[1], spent[2]);

    assert!(
        bare < ENTRIES,
        "`Core\\Arr::sort($list)` made {bare} allocations over {ENTRIES} entries, which is at \
         least one an entry: a renumbering sort with no `by` reads no key and calls nothing, \
         so nothing in it should scale with the entry count"
    );
    assert!(
        quiet <= bare + ENTRIES + ENTRIES / 4,
        "a one-parameter `by` cost {quiet} allocations against the bare sort's {bare} over \
         {ENTRIES} entries, and it owes only `call_callable`'s argument slice per entry: \
         anything more is a key built for a callback that never declared one"
    );
    assert!(
        keyed >= quiet + ENTRIES,
        "a two-parameter `by` cost {keyed} allocations against the one-parameter `by`'s \
         {quiet} over {ENTRIES} entries: the gap is the key `keys[index].to_str()` renders, \
         so if it has closed the arity is no longer deciding anything"
    );
}
